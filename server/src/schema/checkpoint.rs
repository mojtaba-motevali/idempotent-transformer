use hiqlite::Row;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointValue {
    pub position: i64,
    pub value: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub workflow_id: String,
    pub value: Option<Vec<u8>>,
    pub lease_timeout: i64,
    pub created_at: i64,
}

impl From<Row<'_>> for CheckpointValue {
    fn from(mut row: Row<'_>) -> Self {
        Self {
            position: row.get("position"),
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
            created_at: row.get("created_at"),
        }
    }
}
