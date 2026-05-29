use std::collections::HashSet;

use tracing::instrument;

use crate::{
    error::RepositoryError,
    schema::workflow::WorkflowStatus,
    storage::state_machine::SharedStore,
};

#[derive(Clone)]
pub struct FencingTokenRepository {
    store: SharedStore,
}

impl FencingTokenRepository {
    pub fn new(store: SharedStore) -> Self {
        Self { store }
    }

    #[instrument(skip(self), fields(workflow_id), err)]
    pub async fn get(&self, workflow_id: &str) -> Result<Option<i64>, RepositoryError> {
        Ok(self.store.lock().fencing_tokens.get(workflow_id).copied())
    }

    /// Deletes fencing tokens for completed workflows whose `expire_at` is
    /// before `before_ms`.  Acquires the store once for both the scan and
    /// the removal.
    #[instrument(skip(self), err)]
    pub async fn delete_for_completed(&self, before_ms: i64) -> Result<u64, RepositoryError> {
        let mut store = self.store.lock();
        let expired: HashSet<String> = store
            .workflows
            .values()
            .filter(|wf| {
                wf.status == WorkflowStatus::Completed as i64
                    && wf.expire_at.map_or(false, |e| e < before_ms)
            })
            .map(|wf| wf.id.clone())
            .collect();

        let count = expired.len() as u64;
        for id in &expired {
            store.fencing_tokens.remove(id);
        }
        Ok(count)
    }
}
