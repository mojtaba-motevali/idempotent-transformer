use tracing::instrument;

use crate::{
    error::RepositoryError,
    schema::workflow::{Workflow, WorkflowStatus},
    storage::state_machine::SharedStore,
};

#[derive(Clone)]
pub struct WorkflowRepository {
    store: SharedStore,
}

impl WorkflowRepository {
    pub fn new(store: SharedStore) -> Self {
        Self { store }
    }

    #[instrument(skip(self), fields(workflow_id), err)]
    pub async fn get_by_id(
        &self,
        workflow_id: &str,
    ) -> Result<Option<Workflow>, RepositoryError> {
        Ok(self.store.lock().workflows.get(workflow_id).cloned())
    }


    #[instrument(skip(self), err)]
    pub async fn delete_expired(&self, before_ms: i64) -> Result<u64, RepositoryError> {
        let mut store = self.store.lock();
        let expired: Vec<String> = store
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
            store.workflows.remove(id);
        }
        Ok(count)
    }
}
