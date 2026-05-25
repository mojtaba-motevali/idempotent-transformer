use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowFencingToken {
    pub workflow_id: String,
    pub fencing_token: i64,
}
