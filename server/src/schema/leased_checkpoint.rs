use hiqlite::Row;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LeasedCheckpointValue {
    pub lease_timeout: i64,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeasedCheckpoint {
    pub workflow_id: String,
    pub position_checksum: i64,
    pub lease_timeout: i64,
    pub created_at: i64,
}

impl From<Row<'_>> for LeasedCheckpointValue {
    fn from(mut row: Row<'_>) -> Self {
        Self {
            lease_timeout: row.get("lease_timeout"),
            created_at: row.get("created_at"),
        }
    }
}
