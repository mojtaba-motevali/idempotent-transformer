# rpc_server/ — gRPC handler and server startup

Thin translation layer: maps gRPC request/response types to service method calls.
No business logic here — all logic is in `services/`.

## Files

### `server.rs`

#### `WorkflowRpcHandler`

```rust
pub struct WorkflowRpcHandler {
    workflow_service:    Arc<WorkflowService>,
    checkpoint_service:  Arc<CheckpointService>,
}
impl WorkflowRpcHandler {
    pub fn new(workflow_service: Arc<WorkflowService>, checkpoint_service: Arc<CheckpointService>) -> Self
}
```

Implements the tonic-generated `WorkflowServiceImpl` trait. All 7 RPCs are instrumented
with `#[instrument(skip(self), fields(...), err)]`.

**7 RPC methods:**

| gRPC method | Service call | Fields captured |
|-------------|-------------|----------------|
| `workflow_start` | `workflow_svc.create(workflow_id, context_name)` | `workflow_id` |
| `checkpoint` | `checkpoint_svc.checkpoint(CheckpointInput { ... })` | `workflow_id`, `position`, `fencing_token` |
| `lease_checkpoint` | `checkpoint_svc.lease(LeaseCheckpointInput { ... })` | `workflow_id`, `position`, `fencing_token` |
| `complete_workflow` | `workflow_svc.finish(FinishWorkflowInput { ... })` | `workflow_id`, `fencing_token` |
| `workflow_status` | `workflow_svc.status(workflow_id)` | `workflow_id` |
| `release_checkpoint` | `checkpoint_svc.release(workflow_id, position)` | `workflow_id`, `position` |
| `generate_idempotency_key` | `checkpoint_svc.create_idempotency_key(...)` | `workflow_id`, `position`, `fencing_token` |

#### Error mappers

```rust
fn workflow_err(e: WorkflowError) -> tonic::Status
fn checkpoint_err(e: CheckpointError) -> tonic::Status
```

Status mappings:
```
WorkflowError::NotFound                 → Status::not_found
WorkflowError::FencingTokenNotFound     → Status::not_found
WorkflowError::FencingTokenExpired      → Status::aborted
WorkflowError::Raft(_)                  → Status::internal
WorkflowError::Repository(_)            → Status::internal

CheckpointError::FencingTokenNotFound   → Status::not_found
CheckpointError::FencingTokenExpired    → Status::aborted
CheckpointError::NonDeterministicCheckpoint → Status::aborted
CheckpointError::CheckpointValueNotFound → Status::not_found
CheckpointError::Repository(_)          → Status::internal
```

#### `start_server`

```rust
pub async fn start_server(addr: &str, handler: WorkflowRpcHandler) -> anyhow::Result<()>
```

Binds the tonic gRPC server on `addr`. Logs `info!(addr, "gRPC server listening")` before
serving. Runs until the future is dropped (via `tokio::select!` in `main.rs`).

## Proto file

`proto/workflow_service.proto` — 7 endpoints. **Not modified during the migration.**
The generated code is included via `tonic::include_proto!("workflow_service")`.

## Dependencies

- `services::{WorkflowService, CheckpointService}` and their input/output types
- `error::{WorkflowError, CheckpointError}`
- `tonic::{Request, Response, Status}`
- `tracing::instrument`
