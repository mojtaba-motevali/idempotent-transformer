use std::io::ErrorKind;
use std::sync::Arc;

use openraft::{
    EntryPayload, LogId, RaftSnapshotBuilder, RaftTypeConfig, SnapshotMeta, StorageError,
    StorageIOError, StoredMembership,
};
use openraft::storage::RaftStateMachine;
use tokio::sync::Mutex;

use crate::repositories::{
    checkpoints::CheckpointRepository,
    workflows::WorkflowRepository,
    workflows_fencing_tokens::FencingTokenRepository,
};
use crate::storage::command::{WorkflowCommand, WorkflowResponse};
use crate::storage::type_config::{Node, TypeConfig};

pub struct StateMachine {
    pub pool: deadpool_sqlite::Pool,
    last_applied: Arc<Mutex<Option<LogId<u64>>>>,
    last_membership: Arc<Mutex<StoredMembership<u64, Node>>>,
}

impl StateMachine {
    pub async fn new(pool: deadpool_sqlite::Pool) -> anyhow::Result<Self> {
        {
            let conn = pool.get().await?;
            conn.interact(|conn| {
                conn.execute_batch(include_str!("../../migrations/1_init.sql"))
            })
            .await
            .map_err(|e| anyhow::anyhow!("interact error: {e}"))??;
        }

        {
            let conn = pool.get().await?;
            conn.interact(|conn| {
                conn.execute_batch(
                    "CREATE TABLE IF NOT EXISTS RaftMeta (
                        key   TEXT NOT NULL PRIMARY KEY,
                        value TEXT NOT NULL
                    );",
                )
            })
            .await
            .map_err(|e| anyhow::anyhow!("interact error: {e}"))??;
        }

        let last_applied = Self::load_meta_json::<Option<LogId<u64>>>(&pool, "last_applied")
            .await?
            .unwrap_or(None);

        let last_membership =
            Self::load_meta_json::<StoredMembership<u64, Node>>(&pool, "last_membership")
                .await?
                .unwrap_or_default();

        Ok(Self {
            pool,
            last_applied: Arc::new(Mutex::new(last_applied)),
            last_membership: Arc::new(Mutex::new(last_membership)),
        })
    }

    async fn load_meta_json<T: serde::de::DeserializeOwned>(
        pool: &deadpool_sqlite::Pool,
        key: &'static str,
    ) -> anyhow::Result<Option<T>> {
        use rusqlite::OptionalExtension;
        let conn = pool.get().await?;
        let result: Option<String> = conn
            .interact(move |conn| {
                conn.query_row(
                    "SELECT value FROM RaftMeta WHERE key = ?1",
                    rusqlite::params![key],
                    |row| row.get::<_, String>(0),
                )
                .optional()
            })
            .await
            .map_err(|e| anyhow::anyhow!("interact error: {e}"))?
            .map_err(|e| anyhow::anyhow!("sqlite error: {e}"))?;

        match result {
            Some(json) => Ok(Some(serde_json::from_str::<T>(&json)?)),
            None => Ok(None),
        }
    }

    fn sm_err(e: &dyn std::error::Error) -> StorageError<u64> {
        StorageError::IO {
            source: StorageIOError::write_state_machine(&std::io::Error::new(
                ErrorKind::Other,
                e.to_string(),
            )),
        }
    }

    async fn put_meta(&self, key: &'static str, value: String) -> Result<(), StorageError<u64>> {
        let conn = self.pool.get().await.map_err(|e| Self::sm_err(&e))?;
        conn.interact(move |conn| {
            conn.execute(
                "INSERT INTO RaftMeta (key, value) VALUES (?1, ?2)
                 ON CONFLICT (key) DO UPDATE SET value = excluded.value",
                rusqlite::params![key, value],
            )
        })
        .await
        .map_err(|e| {
            StorageError::IO {
                source: StorageIOError::write_state_machine(&std::io::Error::new(
                    ErrorKind::Other,
                    e.to_string(),
                )),
            }
        })?
        .map_err(|e| Self::sm_err(&e))?;
        Ok(())
    }

    async fn apply_command(
        &self,
        cmd: WorkflowCommand,
        wf: &WorkflowRepository,
        cp: &CheckpointRepository,
        ft: &FencingTokenRepository,
    ) -> Result<WorkflowResponse, StorageError<u64>> {
        match cmd {
            WorkflowCommand::CreateOrGetWorkflow {
                workflow_id,
                name,
                created_at,
            } => {
                wf.create_or_get(&workflow_id, name.as_deref(), created_at)
                    .await
                    .map_err(|e| Self::sm_err(&e))?;
                Ok(WorkflowResponse::Empty)
            }

            WorkflowCommand::IncrementFencingToken { workflow_id } => {
                let token = ft
                    .increment(&workflow_id)
                    .await
                    .map_err(|e| Self::sm_err(&e))?;
                Ok(WorkflowResponse::FencingToken(token))
            }

            WorkflowCommand::UpsertCheckpoint {
                workflow_id,
                position,
                idempotency_key,
                value,
                created_at,
            } => {
                cp.upsert(
                    &workflow_id,
                    position,
                    &idempotency_key,
                    value.as_deref(),
                    created_at,
                )
                .await
                .map_err(|e| Self::sm_err(&e))?;
                Ok(WorkflowResponse::Empty)
            }

            WorkflowCommand::CompleteWorkflow {
                workflow_id,
                expire_at,
                completed_at,
            } => {
                wf.complete(&workflow_id, expire_at, completed_at)
                    .await
                    .map_err(|e| Self::sm_err(&e))?;
                Ok(WorkflowResponse::Empty)
            }

            WorkflowCommand::DeleteExpired { before_ms } => {
                let (r1, r2, r3) = tokio::join!(
                    cp.delete_for_completed(before_ms),
                    ft.delete_for_completed(before_ms),
                    wf.delete_expired(before_ms),
                );
                // Log errors but don't fail the Raft entry — cleanup is best-effort
                if let Err(e) = r1 {
                    tracing::warn!(err = %e, "checkpoint cleanup error");
                }
                if let Err(e) = r2 {
                    tracing::warn!(err = %e, "fencing token cleanup error");
                }
                if let Err(e) = r3 {
                    tracing::warn!(err = %e, "workflow cleanup error");
                }
                Ok(WorkflowResponse::Empty)
            }
        }
    }
}

pub struct NoopSnapshotBuilder;

impl RaftSnapshotBuilder<TypeConfig> for NoopSnapshotBuilder {
    async fn build_snapshot(
        &mut self,
    ) -> Result<openraft::Snapshot<TypeConfig>, StorageError<u64>> {
        Err(StorageError::IO {
            source: StorageIOError::write_snapshot(
                None,
                &std::io::Error::new(
                    std::io::ErrorKind::Unsupported,
                    "snapshots not implemented",
                ),
            ),
        })
    }
}

impl RaftStateMachine<TypeConfig> for StateMachine {
    type SnapshotBuilder = NoopSnapshotBuilder;

    async fn applied_state(
        &mut self,
    ) -> Result<(Option<LogId<u64>>, StoredMembership<u64, Node>), StorageError<u64>> {
        let last_applied = *self.last_applied.lock().await;
        let last_membership = self.last_membership.lock().await.clone();
        Ok((last_applied, last_membership))
    }

    async fn apply<I>(
        &mut self,
        entries: I,
    ) -> Result<Vec<WorkflowResponse>, StorageError<u64>>
    where
        I: IntoIterator<Item = openraft::Entry<TypeConfig>> + Send,
    {
        let wf = WorkflowRepository::new(self.pool.clone());
        let cp = CheckpointRepository::new(self.pool.clone());
        let ft = FencingTokenRepository::new(self.pool.clone());

        let mut responses = Vec::new();
        let mut last_log_id: Option<LogId<u64>> = None;

        for entry in entries {
            let log_id = entry.log_id;

            let response = match entry.payload {
                EntryPayload::Normal(cmd) => {
                    self.apply_command(cmd, &wf, &cp, &ft).await?
                }
                EntryPayload::Membership(m) => {
                    let membership = StoredMembership::new(Some(log_id), m);
                    *self.last_membership.lock().await = membership.clone();
                    let json =
                        serde_json::to_string(&membership).map_err(|e| Self::sm_err(&e))?;
                    self.put_meta("last_membership", json).await?;
                    WorkflowResponse::Empty
                }
                EntryPayload::Blank => WorkflowResponse::Empty,
            };

            *self.last_applied.lock().await = Some(log_id);
            last_log_id = Some(log_id);
            responses.push(response);
        }

        if let Some(lid) = last_log_id {
            let json = serde_json::to_string(&lid).map_err(|e| Self::sm_err(&e))?;
            self.put_meta("last_applied", json).await?;
        }

        Ok(responses)
    }

    async fn get_snapshot_builder(&mut self) -> Self::SnapshotBuilder {
        NoopSnapshotBuilder
    }

    async fn begin_receiving_snapshot(
        &mut self,
    ) -> Result<Box<<TypeConfig as RaftTypeConfig>::SnapshotData>, StorageError<u64>> {
        Ok(Box::new(std::io::Cursor::new(Vec::new())))
    }

    async fn install_snapshot(
        &mut self,
        meta: &SnapshotMeta<u64, Node>,
        _snapshot: Box<<TypeConfig as RaftTypeConfig>::SnapshotData>,
    ) -> Result<(), StorageError<u64>> {
        *self.last_applied.lock().await = meta.last_log_id;
        *self.last_membership.lock().await = meta.last_membership.clone();
        Ok(())
    }

    async fn get_current_snapshot(
        &mut self,
    ) -> Result<Option<openraft::Snapshot<TypeConfig>>, StorageError<u64>> {
        Ok(None)
    }
}
