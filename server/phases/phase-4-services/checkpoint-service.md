# Phase 4 — CheckpointService

## File

`src/services/checkpoint_service.rs` (full rewrite)

## Depends on

- `CheckpointRepository` (Phase 3)
- `LeaseRepository` (Phase 3)
- `FencingTokenRepository` (Phase 3)

---

## Struct

```rust
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

pub struct CheckpointService {
    checkpoints: Arc<CheckpointRepository>,
    leases: Arc<LeaseRepository>,
    fencing_tokens: Arc<FencingTokenRepository>,
    /// Per-position in-process mutex. Replaces client.lock().
    /// Key: "{workflow_id}:{position}"
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

    /// Returns (or creates) the Mutex for this (workflow_id, position) pair.
    async fn lock_position(&self, workflow_id: &str, position: i64) -> Arc<Mutex<()>> {
        let key = Self::lock_key(workflow_id, position);
        self.locks
            .entry(key)
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }
}
```

---

## Methods

### `checkpoint`

Validates fencing token, checks lease expiry, persists value.

```rust
#[instrument(skip(self), fields(workflow_id = %input.workflow_id, position = input.position, fencing_token = input.fencing_token), err)]
pub async fn checkpoint(
    &self,
    input: CheckpointInput,
) -> Result<CheckpointOutput, CheckpointError> {
    // Fetch stored fencing token and remove any active lease in parallel
    let (token_result, lease) = tokio::join!(
        self.fencing_tokens.get(&input.workflow_id),
        self.leases.get(&input.workflow_id, input.position),
    );

    let stored_token = token_result?
        .ok_or(CheckpointError::FencingTokenNotFound {
            workflow_id: input.workflow_id.clone(),
        })?;

    // If a lease existed, remove it now
    if lease.is_some() {
        self.leases.release(&input.workflow_id, input.position).await;
    }

    let is_fencing_token_expired = stored_token > input.fencing_token;

    // A lease that existed but has now expired means a zombie worker wrote this;
    // if the fencing token is also stale, mark abort.
    let abort = match &lease {
        Some(l) => {
            let lease_expired = chrono::Utc::now().timestamp_millis()
                > l.created_at + l.lease_timeout;
            if lease_expired {
                is_fencing_token_expired
            } else {
                false
            }
        }
        None => is_fencing_token_expired,
    };

    // Always persist the checkpoint value regardless of abort —
    // the abort flag tells the client to stop, but the value is still recorded.
    self.checkpoints
        .upsert(
            &input.workflow_id,
            input.position,
            &input.idempotency_key,
            Some(&input.value),
            chrono::Utc::now().timestamp_millis(),
        )
        .await?;

    if abort {
        debug!(workflow_id = %input.workflow_id, position = input.position, "checkpoint aborted — fencing token expired");
    }

    Ok(CheckpointOutput { abort })
}
```

### `lease`

Acquires a per-position lock, checks for an existing checkpoint or lease, then issues a new lease.

```rust
#[instrument(skip(self), fields(workflow_id = %input.workflow_id, position = input.position, fencing_token = input.fencing_token), err)]
pub async fn lease(
    &self,
    input: LeaseCheckpointInput,
) -> Result<LeaseCheckpointOutput, CheckpointError> {
    // Acquire in-process lock — replaces client.lock()
    let mutex = self.lock_position(&input.workflow_id, input.position).await;
    let _guard = mutex.lock().await;

    // Parallel: check existing lease and fencing token
    let (existing_lease, token_result) = tokio::join!(
        self.leases.get(&input.workflow_id, input.position),
        self.fencing_tokens.get(&input.workflow_id),
    );

    let stored_token = token_result?
        .ok_or(CheckpointError::FencingTokenNotFound {
            workflow_id: input.workflow_id.clone(),
        })?;

    if stored_token > input.fencing_token {
        return Err(CheckpointError::FencingTokenExpired);
    }

    // If a lease already exists and has not expired, return remaining time
    if let Some(ref l) = existing_lease {
        let now = chrono::Utc::now().timestamp_millis();
        let remaining = (l.created_at + l.lease_timeout) - now;
        if remaining > 0 {
            return Ok(LeaseCheckpointOutput::RemainingLeaseTimeout(remaining));
        }
    }

    // Check if a committed checkpoint value already exists
    if let Some(cp) = self.checkpoints.get(&input.workflow_id, input.position).await? {
        // Non-determinism guard: same position must have the same idempotency key
        if cp.idempotency_key != input.idempotency_key {
            return Err(CheckpointError::NonDeterministicCheckpoint { position: cp.position });
        }

        if let Some(value) = cp.value {
            return Ok(LeaseCheckpointOutput::CheckpointValue(value));
        }
        return Err(CheckpointError::CheckpointValueNotFound { position: cp.position });
    }

    // Issue a new lease
    self.leases
        .acquire(&input.workflow_id, input.position, input.lease_timeout)
        .await;

    Ok(LeaseCheckpointOutput::Leased)
}
```

### `release`

```rust
#[instrument(skip(self), fields(workflow_id, position), err)]
pub async fn release(
    &self,
    workflow_id: &str,
    position: i64,
) -> Result<(), CheckpointError> {
    self.leases.release(workflow_id, position).await;
    Ok(())
}
```

### `create_idempotency_key`

Acquires lock, validates fencing token, creates a placeholder checkpoint, returns UUID key.

```rust
#[instrument(skip(self), fields(workflow_id = %input.workflow_id, position = input.position, fencing_token = input.fencing_token), err)]
pub async fn create_idempotency_key(
    &self,
    input: CreateDurableIdempotencyKeyInput,
) -> Result<CreateDurableIdempotencyKeyOutput, CheckpointError> {
    let mutex = self.lock_position(&input.workflow_id, input.position).await;
    let _guard = mutex.lock().await;

    let stored_token = self.fencing_tokens.get(&input.workflow_id).await?
        .ok_or(CheckpointError::FencingTokenNotFound {
            workflow_id: input.workflow_id.clone(),
        })?;

    if stored_token > input.fencing_token {
        return Err(CheckpointError::FencingTokenExpired);
    }

    let idempotency_key = uuid::Uuid::new_v4().to_string();

    // Placeholder checkpoint with no value — marks the position as reserved
    self.checkpoints
        .upsert(
            &input.workflow_id,
            input.position,
            &idempotency_key,
            None,
            chrono::Utc::now().timestamp_millis(),
        )
        .await?;

    Ok(CreateDurableIdempotencyKeyOutput { idempotency_key })
}
```

---

## Output type variants

```rust
pub enum LeaseCheckpointOutput {
    CheckpointValue(Vec<u8>),
    RemainingLeaseTimeout(i64),
    Leased, // new variant: lease was freshly issued, no value yet
}
```

---

## Verification

```bash
cargo check -p idempotency-server
# CheckpointService must compile; no hiqlite imports; DashMap lock key must be Send
```
