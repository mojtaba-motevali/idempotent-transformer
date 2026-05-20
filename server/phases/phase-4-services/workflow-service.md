# Phase 4 — WorkflowService

## File

`src/services/workflow_service.rs` (full rewrite)

## Depends on

- `WorkflowRepository` (Phase 3)
- `FencingTokenRepository` (Phase 3)
- `Arc<openraft::Raft<TypeConfig>>` from `RaftNode` (Phase 2)

---

## Struct

```rust
use std::sync::Arc;
use openraft::Raft;
use tracing::{debug, instrument};
use crate::{
    error::WorkflowError,
    repositories::{
        workflows::WorkflowRepository,
        checkpoints::CheckpointRepository,
        workflows_fencing_tokens::FencingTokenRepository,
    },
    storage::type_config::{TypeConfig, NodeId},
};

pub struct WorkflowService {
    workflows: Arc<WorkflowRepository>,
    fencing_tokens: Arc<FencingTokenRepository>,
    checkpoints: Arc<CheckpointRepository>,
    raft: Arc<Raft<TypeConfig>>,
    local_node_id: NodeId,
}

impl WorkflowService {
    pub fn new(
        workflows: Arc<WorkflowRepository>,
        fencing_tokens: Arc<FencingTokenRepository>,
        checkpoints: Arc<CheckpointRepository>,
        raft: Arc<Raft<TypeConfig>>,
        local_node_id: NodeId,
    ) -> Self {
        Self { workflows, fencing_tokens, checkpoints, raft, local_node_id }
    }
}
```

---

## Methods

### `is_leader`

Replaces `client.is_leader_db()`.

```rust
pub async fn is_leader(&self) -> bool {
    self.raft.current_leader().await == Some(self.local_node_id)
}
```

### `create`

Preserves the current `tokio::join!` parallelism for the two independent writes.
Both writes go through Raft (`raft.client_write(WorkflowCommand::...)`) so they are
replicated before the response is returned.

```rust
#[instrument(skip(self), fields(workflow_id = %workflow_id), err)]
pub async fn create(
    &self,
    workflow_id: &str,
    name: Option<&str>,
) -> Result<CreateWorkflowOutput, WorkflowError> {
    let created_at = chrono::Utc::now().timestamp_millis();

    let (wf_result, token_result) = tokio::join!(
        self.raft.client_write(WorkflowCommand::CreateOrGetWorkflow {
            workflow_id: workflow_id.to_owned(),
            name: name.map(str::to_owned),
            created_at,
        }),
        self.raft.client_write(WorkflowCommand::IncrementFencingToken {
            workflow_id: workflow_id.to_owned(),
        }),
    );

    wf_result.map_err(|e| WorkflowError::Raft(e.to_string()))?;
    let fencing_token = match token_result.map_err(|e| WorkflowError::Raft(e.to_string()))?.data {
        WorkflowResponse::FencingToken(t) => t,
        _ => return Err(WorkflowError::Raft("unexpected response variant".to_owned())),
    };

    Ok(CreateWorkflowOutput { fencing_token })
}
```

### `finish`

Validates fencing token then writes through Raft.

```rust
#[instrument(skip(self), fields(workflow_id = %input.workflow_id, fencing_token = input.fencing_token), err)]
pub async fn finish(
    &self,
    input: FinishWorkflowInput,
) -> Result<FinishWorkflowOutput, WorkflowError> {
    let stored_token = self.fencing_tokens
        .get(&input.workflow_id).await?
        .ok_or(WorkflowError::FencingTokenNotFound {
            workflow_id: input.workflow_id.clone(),
        })?;

    if stored_token > input.fencing_token {
        return Err(WorkflowError::FencingTokenExpired {
            stored: stored_token,
            received: input.fencing_token,
        });
    }

    let expire_at = chrono::Utc::now().timestamp_millis() + input.expire_after;
    let completed_at = chrono::Utc::now().timestamp_millis();

    self.raft.client_write(WorkflowCommand::CompleteWorkflow {
        workflow_id: input.workflow_id,
        expire_at,
        completed_at,
    })
    .await
    .map_err(|e| WorkflowError::Raft(e.to_string()))?;

    Ok(FinishWorkflowOutput {})
}
```

### `status`

Read-only — goes directly to the local repository, no Raft write needed.

```rust
#[instrument(skip(self), fields(workflow_id), err)]
pub async fn status(
    &self,
    workflow_id: &str,
) -> Result<Workflow, WorkflowError> {
    self.workflows
        .get_by_id(workflow_id).await?
        .ok_or_else(|| WorkflowError::NotFound { id: workflow_id.to_owned() })
}
```

### `cleanup_expired`

Leader-only; called by the cron job every 10 s.
`CheckpointRepository` is injected at construction (see `WorkflowService::new` above).

```rust
#[instrument(skip(self), err)]
pub async fn cleanup_expired(&self) -> Result<(), WorkflowError> {
    if !self.is_leader().await {
        return Ok(());
    }
    let before_ms = chrono::Utc::now().timestamp_millis();
    debug!("running expired workflow cleanup");

    // Delete fencing tokens and checkpoints in parallel; both go through SQLite pool directly.
    // Raft log is not involved: cleanup is leader-only, so applying locally is sufficient.
    let (tokens_result, checkpoints_result) = tokio::join!(
        self.fencing_tokens.delete_for_completed(before_ms),
        self.checkpoints.delete_for_completed(before_ms),
    );
    tokens_result?;
    checkpoints_result?;

    let n = self.workflows.delete_expired(before_ms).await?;
    debug!(deleted = n, "expired workflows removed");
    Ok(())
}
```

---

## Input / Output types

Keep existing types from `src/services/workflow_service.rs` unchanged:
`CreateWorkflowInput`, `CreateWorkflowOutput`, `FinishWorkflowInput`, `FinishWorkflowOutput`.

---

## Verification

```bash
cargo check -p idempotency-server
# WorkflowService must compile; no hiqlite imports
```
