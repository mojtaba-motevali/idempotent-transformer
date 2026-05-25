use deadpool_sqlite::Pool;
use rusqlite::OptionalExtension;
use tracing::instrument;

use crate::{
    error::RepositoryError,
    schema::workflow::{Workflow, WorkflowStatus},
};

#[derive(Clone)]
pub struct WorkflowRepository {
    pool: Pool,
}

impl WorkflowRepository {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    #[instrument(skip(self), fields(workflow_id), err)]
    pub async fn create_or_get(
        &self,
        workflow_id: &str,
        name: Option<&str>,
        created_at: i64,
    ) -> Result<Workflow, RepositoryError> {
        let conn = self.pool.get().await?;
        let wf_id = workflow_id.to_owned();
        let wf_name = name.map(str::to_owned);
        let status = WorkflowStatus::Running as i64;

        conn.interact(move |conn| {
            conn.query_row(
                "INSERT INTO Workflows (id, status, name, created_at)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT (id) DO UPDATE SET name = excluded.name
                 RETURNING *",
                rusqlite::params![wf_id, status, wf_name, created_at],
                Workflow::from_row,
            )
        })
        .await?
        .map_err(RepositoryError::Sqlite)
    }

    #[instrument(skip(self), fields(workflow_id), err)]
    pub async fn get_by_id(
        &self,
        workflow_id: &str,
    ) -> Result<Option<Workflow>, RepositoryError> {
        let conn = self.pool.get().await?;
        let wf_id = workflow_id.to_owned();

        conn.interact(move |conn| {
            conn.query_row(
                "SELECT * FROM Workflows WHERE id = ?1",
                rusqlite::params![wf_id],
                Workflow::from_row,
            )
            .optional()
        })
        .await?
        .map_err(RepositoryError::Sqlite)
    }

    #[instrument(skip(self), fields(workflow_id), err)]
    pub async fn complete(
        &self,
        workflow_id: &str,
        expire_at: i64,
        completed_at: i64,
    ) -> Result<(), RepositoryError> {
        let conn = self.pool.get().await?;
        let wf_id = workflow_id.to_owned();
        let status = WorkflowStatus::Completed as i64;

        conn.interact(move |conn| {
            conn.execute(
                "UPDATE Workflows SET status = ?1, expire_at = ?2, completed_at = ?3
                 WHERE id = ?4",
                rusqlite::params![status, expire_at, completed_at, wf_id],
            )
        })
        .await?
        .map_err(RepositoryError::Sqlite)?;
        Ok(())
    }

    #[instrument(skip(self), err)]
    pub async fn delete_expired(&self, before_ms: i64) -> Result<u64, RepositoryError> {
        let conn = self.pool.get().await?;
        let status = WorkflowStatus::Completed as i64;

        let n = conn
            .interact(move |conn| {
                conn.execute(
                    "DELETE FROM Workflows WHERE expire_at < ?1 AND status = ?2",
                    rusqlite::params![before_ms, status],
                )
            })
            .await?
            .map_err(RepositoryError::Sqlite)?;
        Ok(n as u64)
    }
}
