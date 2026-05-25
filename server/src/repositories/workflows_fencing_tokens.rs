use deadpool_sqlite::Pool;
use rusqlite::OptionalExtension;
use tracing::instrument;

use crate::error::RepositoryError;

#[derive(Clone)]
pub struct FencingTokenRepository {
    pool: Pool,
}

impl FencingTokenRepository {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    #[instrument(skip(self), fields(workflow_id), err)]
    pub async fn increment(&self, workflow_id: &str) -> Result<i64, RepositoryError> {
        let conn = self.pool.get().await?;
        let wf_id = workflow_id.to_owned();

        let token = conn
            .interact(move |conn| {
                conn.query_row(
                    "INSERT INTO WorkflowFencingTokens (workflow_id, fencing_token)
                     VALUES (?1, 1)
                     ON CONFLICT (workflow_id)
                     DO UPDATE SET fencing_token = WorkflowFencingTokens.fencing_token + 1
                     RETURNING fencing_token",
                    rusqlite::params![wf_id],
                    |row| row.get::<_, i64>(0),
                )
            })
            .await??;

        Ok(token)
    }

    #[instrument(skip(self), fields(workflow_id), err)]
    pub async fn get(&self, workflow_id: &str) -> Result<Option<i64>, RepositoryError> {
        let conn = self.pool.get().await?;
        let wf_id = workflow_id.to_owned();

        conn.interact(move |conn| {
            conn.query_row(
                "SELECT fencing_token FROM WorkflowFencingTokens WHERE workflow_id = ?1",
                rusqlite::params![wf_id],
                |row| row.get::<_, i64>(0),
            )
            .optional()
        })
        .await?
        .map_err(RepositoryError::Sqlite)
    }

    #[instrument(skip(self), err)]
    pub async fn delete_for_completed(&self, before_ms: i64) -> Result<u64, RepositoryError> {
        let conn = self.pool.get().await?;

        let n = conn
            .interact(move |conn| {
                conn.execute(
                    "DELETE FROM WorkflowFencingTokens
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
