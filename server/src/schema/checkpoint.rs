use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointValue {
    pub position: i64,
    pub idempotency_key: String,
    pub value: Option<Vec<u8>>,
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

impl Checkpoint {
    pub fn from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            workflow_id:     row.get(0)?,
            position:        row.get(1)?,
            value:           row.get(2)?,
            idempotency_key: row.get(3)?,
            created_at:      row.get(4)?,
            lease_timeout:   0, // not stored in DB; set by LeaseRepository
        })
    }
}
