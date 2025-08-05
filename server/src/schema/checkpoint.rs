use hiqlite::Row;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointValue {
    pub position: i64,
    pub idempotency_key: String,
    pub value: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub workflow_id: String,
    pub value: Option<Vec<u8>>,
    pub lease_timeout: i64,
    pub position: i64,
    pub idempotency_key: String,
    pub created_at: i64,
}

impl From<Row<'_>> for CheckpointValue {
    fn from(mut row: Row<'_>) -> Self {
        Self {
            position: row.get("position"),
            idempotency_key: row.get("idempotency_key"),
            value: row.get("value"),
        }
    }
}

impl From<Row<'_>> for Checkpoint {
    fn from(mut row: Row<'_>) -> Self {
        Self {
            workflow_id: row.get("workflow_id"),
            value: row.get("value"),
            lease_timeout: row.get("lease_timeout"),
            position: row.get("position"),
            idempotency_key: row.get("idempotency_key"),
            created_at: row.get("created_at"),
        }
    }
}
