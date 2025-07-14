use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub id: String,
    pub execution_order: u64,
    pub value: Vec<u8>,
    pub expire_at: Option<i64>,
}
