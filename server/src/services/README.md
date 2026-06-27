# services/ — Business logic layer

Services implement the workflow engine protocol. They hold repository references and
the Raft handle, but contain no SQL — that lives in `repositories/`. All service
methods return typed errors (`WorkflowError` / `CheckpointError`) via `thiserror`.

Every method is instrumented with `#[instrument(skip(self), fields(...), err)]`.

## Files

### `workflow_service.rs` — `WorkflowService`

```rust
pub struct WorkflowService {
    workflows:      Arc<WorkflowRepository>,
    fencing_tokens: Arc<FencingTokenRepository>,
    checkpoints:    Arc<CheckpointRepository>,
    raft:           Arc<openraft::Raft<TypeConfig>>,
    local_node_id:  NodeId,
}
```

| Method | Behaviour |
|--------|-----------|
| `is_leader() -> bool` | `raft.current_leader().await == Some(local_node_id)` |
| `create(workflow_id, name) -> Result<CreateWorkflowOutput, WorkflowError>` | `tokio::join!` sends `CreateOrGetWorkflow` + `IncrementFencingToken` through Raft in parallel. Returns `FencingToken` from the increment response. |
| `finish(input: FinishWorkflowInput) -> Result<(), WorkflowError>` | Reads current fencing token; rejects if `stored > input.fencing_token` (`FencingTokenExpired`). Sends `CompleteWorkflow` through Raft. |
| `status(workflow_id) -> Result<Workflow, WorkflowError>` | Reads directly from `WorkflowRepository` — no Raft round-trip. Returns `NotFound` if absent. |
| `cleanup_expired() -> Result<(), WorkflowError>` | Leader-only guard. Runs `tokio::join!` on `fencing_tokens.delete_for_completed` + `checkpoints.delete_for_completed`, then `workflows.delete_expired`. Does NOT go through Raft — direct repo calls. |

**Input/output types** defined in the same file:
```rust
pub struct CreateWorkflowOutput { pub fencing_token: i64 }
pub struct FinishWorkflowInput  { pub workflow_id: String, pub fencing_token: i64, pub expire_after: i64 }
```

### `checkpoint_service.rs` — `CheckpointService`

```rust
pub struct CheckpointService {
    checkpoints:    Arc<CheckpointRepository>,
    leases:         Arc<LeaseRepository>,
    fencing_tokens: Arc<FencingTokenRepository>,
    locks:          Arc<DashMap<String, Arc<tokio::sync::Mutex<()>>>>,
}
```

`locks` is a per-position in-process mutex map. Key format: `"{workflow_id}:{position}"`.
Replaces the distributed `client.lock()` from the hiqlite era — serialises concurrent
`lease` and `create_idempotency_key` calls for the same position on the same node.

| Method | Behaviour |
|--------|-----------|
| `checkpoint(input) -> Result<CheckpointOutput, CheckpointError>` | `tokio::join!` reads fencing token + current lease. Releases lease if present. Computes `abort` flag based on token expiry and lease state. Upserts checkpoint into `CheckpointRepository` (via Raft). Returns `{ abort: bool }`. |
| `lease(input) -> Result<LeaseCheckpointOutput, CheckpointError>` | Acquires `lock_position`. Validates fencing token. Returns `RemainingLeaseTimeout` if active lease exists. Returns `CheckpointValue` if checkpoint already committed. Acquires new lease otherwise. Returns one of `{ CheckpointValue(Vec<u8>), RemainingLeaseTimeout(i64), Leased }`. |
| `release(workflow_id, position) -> Result<(), CheckpointError>` | Calls `leases.release()`. |
| `create_idempotency_key(input) -> Result<CreateDurableIdempotencyKeyOutput, CheckpointError>` | Acquires `lock_position`. Validates fencing token. Generates `uuid::Uuid::new_v4()`. Upserts a checkpoint row with `value = None` and the new key. |

**Input/output types** defined in the same file:
```rust
pub struct CheckpointInput { workflow_id, value: Vec<u8>, fencing_token, position, idempotency_key }
pub struct CheckpointOutput { abort: bool }
pub struct LeaseCheckpointInput { workflow_id, fencing_token, lease_timeout, position, idempotency_key }
pub enum   LeaseCheckpointOutput { CheckpointValue(Vec<u8>), RemainingLeaseTimeout(i64), Leased }
pub struct CreateDurableIdempotencyKeyInput  { workflow_id, fencing_token, position }
pub struct CreateDurableIdempotencyKeyOutput { idempotency_key: String }
```

## Raft vs direct access

| Operation | Path |
|-----------|------|
| `create` (workflow + fencing token) | → Raft `client_write` |
| `finish` (complete workflow) | → Raft `client_write` |
| `checkpoint` (upsert) | → Raft `client_write` (via `CheckpointRepository`) |
| `status` (read workflow) | → Direct in-memory read |
| `lease` (read token + lease) | → Direct reads; lease acquire is moka only |
| `cleanup_expired` | → Direct in-memory (leader-only, best-effort) |

## Dependencies

- `repositories::{WorkflowRepository, CheckpointRepository, FencingTokenRepository, LeaseRepository}`
- `storage::command::{WorkflowCommand, WorkflowResponse}`
- `storage::type_config::{TypeConfig, NodeId}`
- `error::{WorkflowError, CheckpointError}`
- `schema::workflow::Workflow`
- `dashmap::DashMap`
- `tokio::sync::Mutex`
- `openraft::Raft`
