use hiqlite::Row;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowFencingToken {
    pub workflow_id: String,
    pub fencing_token: i64,
}

impl From<Row<'_>> for WorkflowFencingToken {
    fn from(mut row: Row<'_>) -> Self {
        Self {
            workflow_id: row.get("id"),
            fencing_token: row.get("fencing_token"),
        }
    }
}
