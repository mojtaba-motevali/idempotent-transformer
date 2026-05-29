use thiserror::Error;

#[derive(Error, Debug)]
pub enum RepositoryError {}

#[derive(Error, Debug)]
pub enum WorkflowError {
    #[error("workflow not found: {id}")]
    NotFound { id: String },

    #[error("fencing token not found for workflow: {workflow_id}")]
    FencingTokenNotFound { workflow_id: String },

    #[error("fencing token expired (stored: {stored}, received: {received})")]
    FencingTokenExpired { stored: i64, received: i64 },

    #[error("raft error: {0}")]
    Raft(String),

    #[error(transparent)]
    Repository(#[from] RepositoryError),
}

#[derive(Error, Debug)]
pub enum CheckpointError {
    #[error("fencing token not found for workflow: {workflow_id}")]
    FencingTokenNotFound { workflow_id: String },

    #[error("fencing token expired")]
    FencingTokenExpired,

    #[error("non-deterministic checkpoint: position {position} already has a different idempotency key")]
    NonDeterministicCheckpoint { position: i64 },

    #[error("checkpoint value not found at position {position}")]
    CheckpointValueNotFound { position: i64 },

    #[error(transparent)]
    Repository(#[from] RepositoryError),
}

#[derive(Error, Debug)]
pub enum NetworkError {
    #[error("TCP connect failed to {addr}: {source}")]
    Connect {
        addr: String,
        #[source]
        source: std::io::Error,
    },

    #[error("TCP I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("FlatBuffers serialize error: {0}")]
    Serialize(String),

    #[error("FlatBuffers deserialize error: {0}")]
    Deserialize(String),

    #[error("unexpected message type: {0:#x}")]
    InvalidMessageType(u8),

    #[error("connection lost: {0}")]
    ConnectionLost(String),
}
