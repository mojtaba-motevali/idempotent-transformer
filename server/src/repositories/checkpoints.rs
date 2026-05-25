use std::sync::Arc;

use rocksdb::{Direction, IteratorMode, DB};
use tracing::instrument;

use crate::{
    error::RepositoryError,
    schema::checkpoint::Checkpoint,
    storage::state_machine::CF_CHECKPOINTS,
};

use super::workflows::expired_workflow_ids;

#[derive(Clone)]
pub struct CheckpointRepository {
    db: Arc<DB>,
}

impl CheckpointRepository {
    pub fn new(db: Arc<DB>) -> Self {
        Self { db }
    }

    fn cf(&self) -> &rocksdb::ColumnFamily {
        self.db.cf_handle(CF_CHECKPOINTS).unwrap()
    }
}

/// Key encoding: `{workflow_id bytes} \x00 {position as 8-byte big-endian}`
fn cp_key(workflow_id: &str, position: i64) -> Vec<u8> {
    let mut key = workflow_id.as_bytes().to_vec();
    key.push(0);
    key.extend_from_slice(&position.to_be_bytes());
    key
}

fn cp_prefix(workflow_id: &str) -> Vec<u8> {
    let mut p = workflow_id.as_bytes().to_vec();
    p.push(0);
    p
}

impl CheckpointRepository {
    #[instrument(skip(self), fields(workflow_id, position), err)]
    pub async fn upsert(
        &self,
        workflow_id: &str,
        position: i64,
        idempotency_key: &str,
        value: Option<&[u8]>,
        created_at: i64,
    ) -> Result<(), RepositoryError> {
        let cp = Checkpoint {
            workflow_id: workflow_id.to_owned(),
            position,
            value: value.map(|v| v.to_vec()),
            idempotency_key: idempotency_key.to_owned(),
            created_at,
        };
        let bytes = serde_json::to_vec(&cp)
            .map_err(|e| RepositoryError::Decode(e.to_string()))?;
        self.db.put_cf(self.cf(), cp_key(workflow_id, position), bytes)?;
        Ok(())
    }

    #[instrument(skip(self), fields(workflow_id, position), err)]
    pub async fn get(
        &self,
        workflow_id: &str,
        position: i64,
    ) -> Result<Option<Checkpoint>, RepositoryError> {
        match self.db.get_cf(self.cf(), cp_key(workflow_id, position))? {
            Some(bytes) => {
                let cp = serde_json::from_slice(&bytes)
                    .map_err(|e| RepositoryError::Decode(e.to_string()))?;
                Ok(Some(cp))
            }
            None => Ok(None),
        }
    }

    #[instrument(skip(self), err)]
    pub async fn delete_for_completed(&self, before_ms: i64) -> Result<u64, RepositoryError> {
        let expired_ids = expired_workflow_ids(&self.db, before_ms)?;
        let cf = self.cf();
        let mut count = 0u64;
        for workflow_id in &expired_ids {
            let prefix = cp_prefix(workflow_id);
            let iter = self.db.iterator_cf(
                cf,
                IteratorMode::From(&prefix, Direction::Forward),
            );
            for item in iter {
                let (k, _) = item?;
                if !k.starts_with(&prefix) {
                    break;
                }
                self.db.delete_cf(cf, &k)?;
                count += 1;
            }
        }
        Ok(count)
    }
}
