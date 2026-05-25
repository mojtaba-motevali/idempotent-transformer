use std::sync::Arc;

use rocksdb::{DB, IteratorMode};
use tracing::instrument;

use crate::{
    error::RepositoryError,
    schema::workflow::{Workflow, WorkflowStatus},
    storage::state_machine::CF_WORKFLOWS,
};

#[derive(Clone)]
pub struct WorkflowRepository {
    db: Arc<DB>,
}

impl WorkflowRepository {
    pub fn new(db: Arc<DB>) -> Self {
        Self { db }
    }

    fn cf(&self) -> &rocksdb::ColumnFamily {
        self.db.cf_handle(CF_WORKFLOWS).unwrap()
    }

    #[instrument(skip(self), fields(workflow_id), err)]
    pub async fn create_or_get(
        &self,
        workflow_id: &str,
        name: Option<&str>,
        created_at: i64,
    ) -> Result<Workflow, RepositoryError> {
        let cf = self.cf();

        if let Some(bytes) = self.db.get_cf(cf, workflow_id.as_bytes())? {
            let mut wf: Workflow = serde_json::from_slice(&bytes)
                .map_err(|e| RepositoryError::Decode(e.to_string()))?;
            if name.is_some() {
                wf.name = name.map(str::to_owned);
                let updated = serde_json::to_vec(&wf)
                    .map_err(|e| RepositoryError::Decode(e.to_string()))?;
                self.db.put_cf(cf, workflow_id.as_bytes(), updated)?;
            }
            return Ok(wf);
        }

        let wf = Workflow {
            id: workflow_id.to_owned(),
            status: WorkflowStatus::Running as i64,
            name: name.map(str::to_owned),
            expire_at: None,
            completed_at: None,
            created_at,
        };
        let bytes = serde_json::to_vec(&wf)
            .map_err(|e| RepositoryError::Decode(e.to_string()))?;
        self.db.put_cf(cf, workflow_id.as_bytes(), bytes)?;
        Ok(wf)
    }

    #[instrument(skip(self), fields(workflow_id), err)]
    pub async fn get_by_id(
        &self,
        workflow_id: &str,
    ) -> Result<Option<Workflow>, RepositoryError> {
        let cf = self.cf();
        match self.db.get_cf(cf, workflow_id.as_bytes())? {
            Some(bytes) => {
                let wf = serde_json::from_slice(&bytes)
                    .map_err(|e| RepositoryError::Decode(e.to_string()))?;
                Ok(Some(wf))
            }
            None => Ok(None),
        }
    }

    #[instrument(skip(self), fields(workflow_id), err)]
    pub async fn complete(
        &self,
        workflow_id: &str,
        expire_at: i64,
        completed_at: i64,
    ) -> Result<(), RepositoryError> {
        let cf = self.cf();
        if let Some(bytes) = self.db.get_cf(cf, workflow_id.as_bytes())? {
            let mut wf: Workflow = serde_json::from_slice(&bytes)
                .map_err(|e| RepositoryError::Decode(e.to_string()))?;
            wf.status = WorkflowStatus::Completed as i64;
            wf.expire_at = Some(expire_at);
            wf.completed_at = Some(completed_at);
            let updated = serde_json::to_vec(&wf)
                .map_err(|e| RepositoryError::Decode(e.to_string()))?;
            self.db.put_cf(cf, workflow_id.as_bytes(), updated)?;
        }
        Ok(())
    }

    #[instrument(skip(self), err)]
    pub async fn delete_expired(&self, before_ms: i64) -> Result<u64, RepositoryError> {
        let ids = expired_workflow_ids(&self.db, before_ms)?;
        let cf = self.cf();
        for id in &ids {
            self.db.delete_cf(cf, id.as_bytes())?;
        }
        Ok(ids.len() as u64)
    }
}

/// Returns IDs of completed workflows whose `expire_at` is before `before_ms`.
/// Used by all three cleanup methods across the repository layer.
pub(crate) fn expired_workflow_ids(
    db: &DB,
    before_ms: i64,
) -> Result<Vec<String>, RepositoryError> {
    let cf = db.cf_handle(CF_WORKFLOWS).unwrap();
    let mut ids = Vec::new();
    for item in db.iterator_cf(cf, IteratorMode::Start) {
        let (_, v) = item?;
        let wf: Workflow = serde_json::from_slice(&v)
            .map_err(|e| RepositoryError::Decode(e.to_string()))?;
        if wf.status == WorkflowStatus::Completed as i64
            && wf.expire_at.map_or(false, |e| e < before_ms)
        {
            ids.push(wf.id);
        }
    }
    Ok(ids)
}

