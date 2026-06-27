use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowStatus {
    Running = 0,
    Completed = 1,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub id: String,
    pub status: i64,
    pub expire_at: Option<i64>,
    pub completed_at: Option<i64>,
    pub name: Option<String>,
    pub created_at: i64,
}
