CREATE TABLE IF NOT EXISTS Workflows (
    id VARCHAR(255) NOT NULL PRIMARY KEY,
    status INTEGER NOT NULL,
    expire_at TIMESTAMP,
    created_at TIMESTAMP NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_workflows_id ON Workflows (id);

CREATE TABLE IF NOT EXISTS WorkflowFencingTokens (
    workflow_id VARCHAR(255) NOT NULL REFERENCES Workflows (id) ON DELETE CASCADE,
    fencing_token INTEGER NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_workflow_fencing_tokens_workflow_id ON WorkflowFencingTokens (workflow_id);

CREATE TABLE IF NOT EXISTS Checkpoints (
    workflow_id VARCHAR(255) NOT NULL REFERENCES Workflows (id) ON DELETE CASCADE,
    position_checksum INTEGER NOT NULL,
    value BYTEA,
    workflow_context_name VARCHAR(255) NOT NULL,
    checkpoint_context_name VARCHAR(255) NOT NULL,
    created_at TIMESTAMP NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_checkpoints_workflow_id_position ON Checkpoints (
    workflow_id,
    position_checksum
);

CREATE TABLE IF NOT EXISTS CheckpointLeases (
    workflow_id VARCHAR(255) NOT NULL REFERENCES Workflows (id) ON DELETE CASCADE,
    position_checksum INTEGER NOT NULL,
    lease_timeout INTEGER NOT NULL,
    created_at TIMESTAMP NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_checkpoint_leases_workflow_id_checkpoint_position ON CheckpointLeases (
    workflow_id,
    position_checksum
);