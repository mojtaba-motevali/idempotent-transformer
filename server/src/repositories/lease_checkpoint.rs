use moka::future::Cache;
use tracing::instrument;

use crate::schema::leased_checkpoint::LeasedCheckpointValue;

#[derive(Clone)]
pub struct LeaseRepository {
    cache: Cache<String, LeasedCheckpointValue>,
}

impl LeaseRepository {
    pub fn new() -> Self {
        Self {
            cache: Cache::builder()
                .max_capacity(100_000)
                .build(),
        }
    }

    fn key(workflow_id: &str, position: i64) -> String {
        format!("{workflow_id}:{position}")
    }

    #[instrument(skip(self), fields(workflow_id, position))]
    pub async fn acquire(
        &self,
        workflow_id: &str,
        position: i64,
        lease_timeout_ms: i64,
    ) -> LeasedCheckpointValue {
        let created_at = chrono::Utc::now().timestamp_millis();
        let key = Self::key(workflow_id, position);
        let value = LeasedCheckpointValue {
            lease_timeout: lease_timeout_ms,
            created_at,
        };
        self.cache.insert(key, value.clone()).await;
        value
    }

    #[instrument(skip(self), fields(workflow_id, position))]
    pub async fn get(&self, workflow_id: &str, position: i64) -> Option<LeasedCheckpointValue> {
        let key = Self::key(workflow_id, position);
        let value = self.cache.get(&key).await?;
        let now = chrono::Utc::now().timestamp_millis();
        if now <= value.created_at + value.lease_timeout {
            Some(value)
        } else {
            None
        }
    }

    #[instrument(skip(self), fields(workflow_id, position))]
    pub async fn release(&self, workflow_id: &str, position: i64) {
        let key = Self::key(workflow_id, position);
        self.cache.invalidate(&key).await;
    }
}
