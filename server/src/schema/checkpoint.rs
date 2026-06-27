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
    pub position: i64,
    pub value: Option<Vec<u8>>,
    pub idempotency_key: String,
    pub created_at: i64,
}
