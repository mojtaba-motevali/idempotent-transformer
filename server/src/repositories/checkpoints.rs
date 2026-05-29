use std::collections::HashSet;

use tracing::instrument;

use crate::{
    error::RepositoryError,
    schema::{checkpoint::Checkpoint, workflow::WorkflowStatus},
    storage::state_machine::SharedStore,
};

#[derive(Clone)]
pub struct CheckpointRepository {
    store: SharedStore,
}

impl CheckpointRepository {
    pub fn new(store: SharedStore) -> Self {
        Self { store }
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
        let mut store = self.store.lock();
        store.checkpoints.insert(
            (workflow_id.to_owned(), position),
            Checkpoint {
                workflow_id:     workflow_id.to_owned(),
                position,
                value:           value.map(<[u8]>::to_vec),
                idempotency_key: idempotency_key.to_owned(),
                created_at,
            },
        );
        Ok(())
    }

    #[instrument(skip(self), fields(workflow_id, position), err)]
    pub async fn get(
        &self,
        workflow_id: &str,
        position: i64,
    ) -> Result<Option<Checkpoint>, RepositoryError> {
        Ok(self
            .store
            .lock()
            .checkpoints
            .get(&(workflow_id.to_owned(), position))
            .cloned())
    }

    /// Deletes all checkpoints belonging to completed workflows whose
    /// `expire_at` is before `before_ms`.  Acquires the store once for both
    /// the expired-workflow scan and the checkpoint removal.
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

        let before = store.checkpoints.len();
        store.checkpoints.retain(|(wf_id, _), _| !expired.contains(wf_id));
        Ok((before - store.checkpoints.len()) as u64)
    }
}
