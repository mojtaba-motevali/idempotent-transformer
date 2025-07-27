CREATE TABLE IF NOT EXISTS Workflows (
    id VARCHAR(255) NOT NULL PRIMARY KEY,
    status INTEGER NOT NULL,
    expire_at TIMESTAMP,
    completed_at TIMESTAMP,
    created_at TIMESTAMP NOT NULL
);

CREATE TABLE IF NOT EXISTS WorkflowFencingTokens (
    workflow_id VARCHAR(255) NOT NULL PRIMARY KEY,
    fencing_token INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS Checkpoints (
    workflow_id VARCHAR(255) NOT NULL,
    position_checksum INTEGER NOT NULL,
    value BYTEA,
    idempotency_checksum INTEGER NOT NULL,
    created_at TIMESTAMP NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_checkpoints_workflow_id_position ON Checkpoints (
    workflow_id,
    position_checksum
);