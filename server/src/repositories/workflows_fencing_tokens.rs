use std::sync::Arc;

use rocksdb::DB;
use tracing::instrument;

use crate::{
    error::RepositoryError,
    storage::state_machine::CF_FENCING_TOKENS,
};

use super::workflows::expired_workflow_ids;

#[derive(Clone)]
pub struct FencingTokenRepository {
    db: Arc<DB>,
}

impl FencingTokenRepository {
    pub fn new(db: Arc<DB>) -> Self {
        Self { db }
    }

    fn cf(&self) -> &rocksdb::ColumnFamily {
        self.db.cf_handle(CF_FENCING_TOKENS).unwrap()
    }
}

fn encode_token(token: i64) -> [u8; 8] {
    token.to_be_bytes()
}

fn decode_token(bytes: &[u8]) -> Result<i64, RepositoryError> {
    bytes
        .try_into()
        .map(i64::from_be_bytes)
        .map_err(|_| RepositoryError::Decode("fencing token: invalid byte length".into()))
}

impl FencingTokenRepository {
    /// Atomically increments the fencing token for `workflow_id` and returns the new value.
    /// Safe without a transaction because `StateMachine::apply()` is single-writer.
    #[instrument(skip(self), fields(workflow_id), err)]
    pub async fn increment(&self, workflow_id: &str) -> Result<i64, RepositoryError> {
        let cf = self.cf();
        let current = match self.db.get_cf(cf, workflow_id.as_bytes())? {
            Some(bytes) => decode_token(&bytes)?,
            None => 0,
        };
        let new_token = current + 1;
        self.db.put_cf(cf, workflow_id.as_bytes(), encode_token(new_token))?;
        Ok(new_token)
    }

    #[instrument(skip(self), fields(workflow_id), err)]
    pub async fn get(&self, workflow_id: &str) -> Result<Option<i64>, RepositoryError> {
        match self.db.get_cf(self.cf(), workflow_id.as_bytes())? {
            Some(bytes) => Ok(Some(decode_token(&bytes)?)),
            None => Ok(None),
        }
    }

    #[instrument(skip(self), err)]
    pub async fn delete_for_completed(&self, before_ms: i64) -> Result<u64, RepositoryError> {
        let expired_ids = expired_workflow_ids(&self.db, before_ms)?;
        let cf = self.cf();
        let mut count = 0u64;
        for id in &expired_ids {
            self.db.delete_cf(cf, id.as_bytes())?;
            count += 1;
        }
        Ok(count)
    }
}
