use std::sync::Arc;

use dashmap::DashMap;
use tokio::sync::Mutex;
use tracing::{debug, instrument};

use crate::{
    error::CheckpointError,
    repositories::{
        checkpoints::CheckpointRepository,
        lease_checkpoint::LeaseRepository,
        workflows_fencing_tokens::FencingTokenRepository,
    },
};

// ── Input / Output types ────────────────────────────────────────────────────

#[derive(Debug)]
pub struct CheckpointInput {
    pub workflow_id: String,
    pub value: Vec<u8>,
    pub fencing_token: i64,
    pub position: i64,
    pub idempotency_key: String,
}

#[derive(Debug)]
pub struct CheckpointOutput {
    pub abort: bool,
}

#[derive(Debug)]
pub struct LeaseCheckpointInput {
    pub workflow_id: String,
    pub fencing_token: i64,
    pub lease_timeout: i64,
    pub position: i64,
    pub idempotency_key: String,
}

#[derive(Debug)]
pub enum LeaseCheckpointOutput {
    CheckpointValue(Vec<u8>),
    RemainingLeaseTimeout(i64),
    Leased,
}

#[derive(Debug)]
pub struct CreateDurableIdempotencyKeyInput {
    pub workflow_id: String,
    pub fencing_token: i64,
    pub position: i64,
}

#[derive(Debug)]
pub struct CreateDurableIdempotencyKeyOutput {
    pub idempotency_key: String,
}

// ── Service ─────────────────────────────────────────────────────────────────

pub struct CheckpointService {
    checkpoints: Arc<CheckpointRepository>,
    leases: Arc<LeaseRepository>,
    fencing_tokens: Arc<FencingTokenRepository>,
    locks: Arc<DashMap<String, Arc<Mutex<()>>>>,
}

impl CheckpointService {
    pub fn new(
        checkpoints: Arc<CheckpointRepository>,
        leases: Arc<LeaseRepository>,
        fencing_tokens: Arc<FencingTokenRepository>,
    ) -> Self {
        Self {
            checkpoints,
            leases,
            fencing_tokens,
            locks: Arc::new(DashMap::new()),
        }
    }

    fn lock_key(workflow_id: &str, position: i64) -> String {
        format!("{workflow_id}:{position}")
    }

    async fn lock_position(&self, workflow_id: &str, position: i64) -> Arc<Mutex<()>> {
        let key = Self::lock_key(workflow_id, position);
        self.locks
            .entry(key)
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }

    // ── checkpoint ──────────────────────────────────────────────────────────

    #[instrument(
        skip(self),
        fields(
            workflow_id = %input.workflow_id,
            position    = input.position,
            fencing_token = input.fencing_token
        ),
        err
    )]
    pub async fn checkpoint(
        &self,
        input: CheckpointInput,
    ) -> Result<CheckpointOutput, CheckpointError> {
        let (token_result, lease_option) = tokio::join!(
            self.fencing_tokens.get(&input.workflow_id),
            self.leases.get(&input.workflow_id, input.position),
        );

        let stored_token = token_result?
            .ok_or_else(|| CheckpointError::FencingTokenNotFound {
                workflow_id: input.workflow_id.clone(),
            })?;

        // Release lease unconditionally if one existed (ignoring result)
        if lease_option.is_some() {
            self.leases.release(&input.workflow_id, input.position).await;
        }

        let is_token_expired = stored_token > input.fencing_token;

        let now = chrono::Utc::now().timestamp_millis();
        let abort = match &lease_option {
            Some(l) => {
                let lease_expired = now > l.created_at + l.lease_timeout;
                if lease_expired {
                    is_token_expired
                } else {
                    false
                }
            }
            None => is_token_expired,
        };

        let now_ms = chrono::Utc::now().timestamp_millis();
        self.checkpoints
            .upsert(
                &input.workflow_id,
                input.position,
                &input.idempotency_key,
                Some(&input.value),
                now_ms,
            )
            .await?;

        if abort {
            debug!(
                workflow_id = %input.workflow_id,
                position    = input.position,
                "checkpoint aborted: fencing token expired"
            );
        }

        Ok(CheckpointOutput { abort })
    }

    // ── lease ───────────────────────────────────────────────────────────────

    #[instrument(
        skip(self),
        fields(
            workflow_id   = %input.workflow_id,
            position      = input.position,
            fencing_token = input.fencing_token
        ),
        err
    )]
    pub async fn lease(
        &self,
        input: LeaseCheckpointInput,
    ) -> Result<LeaseCheckpointOutput, CheckpointError> {
        let mutex = self.lock_position(&input.workflow_id, input.position).await;
        let _guard = mutex.lock().await;

        let (lease_result, token_result) = tokio::join!(
            self.leases.get(&input.workflow_id, input.position),
            self.fencing_tokens.get(&input.workflow_id),
        );

        let stored_token = token_result?
            .ok_or_else(|| CheckpointError::FencingTokenNotFound {
                workflow_id: input.workflow_id.clone(),
            })?;

        if stored_token > input.fencing_token {
            return Err(CheckpointError::FencingTokenExpired);
        }

        // If there is an active lease with remaining time, return the remaining duration.
        if let Some(lease) = lease_result {
            let now = chrono::Utc::now().timestamp_millis();
            let expires_at = lease.created_at + lease.lease_timeout;
            if now < expires_at {
                let remaining = expires_at - now;
                return Ok(LeaseCheckpointOutput::RemainingLeaseTimeout(remaining));
            }
        }

        // Check whether a committed checkpoint already exists.
        if let Some(cp) = self
            .checkpoints
            .get(&input.workflow_id, input.position)
            .await?
        {
            if cp.idempotency_key != input.idempotency_key {
                return Err(CheckpointError::NonDeterministicCheckpoint {
                    position: input.position,
                });
            }
            return match cp.value {
                Some(v) => Ok(LeaseCheckpointOutput::CheckpointValue(v)),
                None => Err(CheckpointError::CheckpointValueNotFound {
                    position: input.position,
                }),
            };
        }

        // Acquire a new lease.
        self.leases
            .acquire(&input.workflow_id, input.position, input.lease_timeout)
            .await;

        Ok(LeaseCheckpointOutput::Leased)
    }

    // ── release ─────────────────────────────────────────────────────────────

    #[instrument(skip(self), fields(workflow_id, position), err)]
    pub async fn release(
        &self,
        workflow_id: &str,
        position: i64,
    ) -> Result<(), CheckpointError> {
        self.leases.release(workflow_id, position).await;
        Ok(())
    }

    // ── create_idempotency_key ───────────────────────────────────────────────

    #[instrument(
        skip(self),
        fields(
            workflow_id   = %input.workflow_id,
            position      = input.position,
            fencing_token = input.fencing_token
        ),
        err
    )]
    pub async fn create_idempotency_key(
        &self,
        input: CreateDurableIdempotencyKeyInput,
    ) -> Result<CreateDurableIdempotencyKeyOutput, CheckpointError> {
        let mutex = self.lock_position(&input.workflow_id, input.position).await;
        let _guard = mutex.lock().await;

        let stored_token = self
            .fencing_tokens
            .get(&input.workflow_id)
            .await?
            .ok_or_else(|| CheckpointError::FencingTokenNotFound {
                workflow_id: input.workflow_id.clone(),
            })?;

        if stored_token > input.fencing_token {
            return Err(CheckpointError::FencingTokenExpired);
        }

        let idempotency_key = uuid::Uuid::new_v4().to_string();
        let now_ms = chrono::Utc::now().timestamp_millis();

        self.checkpoints
            .upsert(
                &input.workflow_id,
                input.position,
                &idempotency_key,
                None,
                now_ms,
            )
            .await?;

        Ok(CreateDurableIdempotencyKeyOutput { idempotency_key })
    }
}
