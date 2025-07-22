use hiqlite::Row;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointValue {
    pub position_checksum: i64,
    pub idempotency_checksum: i64,
    pub value: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub workflow_id: String,
    pub value: Option<Vec<u8>>,
    pub lease_timeout: i64,
    pub checkpoint_context_name: String,
    pub workflow_context_name: String,
    pub created_at: i64,
}

impl From<Row<'_>> for CheckpointValue {
    fn from(mut row: Row<'_>) -> Self {
        Self {
            position_checksum: row.get("position_checksum"),
            idempotency_checksum: row.get("idempotency_checksum"),
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
            checkpoint_context_name: row.get("checkpoint_context_name"),
            workflow_context_name: row.get("workflow_context_name"),
            created_at: row.get("created_at"),
        }
    }
}
