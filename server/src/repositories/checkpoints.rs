use deadpool_sqlite::Pool;
use rusqlite::OptionalExtension;
use tracing::instrument;

use crate::{error::RepositoryError, schema::checkpoint::Checkpoint};

#[derive(Clone)]
pub struct CheckpointRepository {
    pool: Pool,
}

impl CheckpointRepository {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    #[instrument(skip(self), fields(workflow_id, position), err)]
    pub async fn upsert(
        &self,
        workflow_id: &str,
        position: i64,
        idempotency_key: &str,
        value: Option<&[u8]>,
        created_at: i64,
    ) -> Result<(), RepositoryError> {
        let conn = self.pool.get().await?;
        let wf_id = workflow_id.to_owned();
        let ikey = idempotency_key.to_owned();
        let val = value.map(|v| v.to_vec());

        conn.interact(move |conn| {
            conn.execute(
                "INSERT INTO Checkpoints (workflow_id, position, idempotency_key, value, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT (workflow_id, position)
                 DO UPDATE SET value = excluded.value",
                rusqlite::params![wf_id, position, ikey, val, created_at],
            )
        })
        .await??;
        Ok(())
    }

    #[instrument(skip(self), fields(workflow_id, position), err)]
    pub async fn get(
        &self,
        workflow_id: &str,
        position: i64,
    ) -> Result<Option<Checkpoint>, RepositoryError> {
        let conn = self.pool.get().await?;
        let wf_id = workflow_id.to_owned();

        let result = conn
            .interact(move |conn| {
                conn.query_row(
                    "SELECT workflow_id, position, value, idempotency_key, created_at \
                     FROM Checkpoints WHERE workflow_id = ?1 AND position = ?2",
                    rusqlite::params![wf_id, position],
                    Checkpoint::from_row,
                )
                .optional()
            })
            .await??;
        Ok(result)
    }

    #[instrument(skip(self), err)]
    pub async fn delete_for_completed(&self, before_ms: i64) -> Result<u64, RepositoryError> {
        let conn = self.pool.get().await?;

        let n = conn
            .interact(move |conn| {
                conn.execute(
                    "DELETE FROM Checkpoints
                     WHERE workflow_id IN (
                         SELECT id FROM Workflows
                         WHERE status = 1 AND expire_at < ?1
                     )",
                    rusqlite::params![before_ms],
                )
            })
            .await??;
        Ok(n as u64)
    }
}
