use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowCommand {
    CreateOrGetWorkflow {
        workflow_id: String,
        name: Option<String>,
        created_at: i64,
    },
    IncrementFencingToken {
        workflow_id: String,
    },
    UpsertCheckpoint {
        workflow_id: String,
        position: i64,
        idempotency_key: String,
        value: Option<Vec<u8>>,
        created_at: i64,
    },
    CompleteWorkflow {
        workflow_id: String,
        expire_at: i64,
        completed_at: i64,
    },
    DeleteExpired {
        before_ms: i64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowResponse {
    Workflow { id: String, fencing_token: i64 },
    FencingToken(i64),
    Empty,
}
