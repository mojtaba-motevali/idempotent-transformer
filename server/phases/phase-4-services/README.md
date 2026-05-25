# Phase 4 — Service Layer

**Status: DONE**

## What was done

Both service structs implemented, compiling, and fully instrumented.

## WorkflowService (`src/services/workflow_service.rs`)

```rust
pub struct WorkflowService {
    workflows: Arc<WorkflowRepository>,
    fencing_tokens: Arc<FencingTokenRepository>,
    checkpoints: Arc<CheckpointRepository>,
    raft: Arc<Raft<TypeConfig>>,
    node_id: NodeId,
}
```

| Method | What it does | Returns |
|--------|-------------|---------|
| `create(workflow_id, name)` | Sends `CreateOrGetWorkflow` + `IncrementFencingToken` through Raft; returns new token | `Result<i64, WorkflowError>` |
| `finish(input)` | Validates fencing token; sends `CompleteWorkflow` through Raft | `Result<(), WorkflowError>` |
| `status(workflow_id)` | Reads directly from `WorkflowRepository` (no Raft round-trip) | `Result<Workflow, WorkflowError>` |
| `cleanup_expired()` | Leader-only; sends `DeleteExpired` through Raft | `Result<(), WorkflowError>` |
| `is_leader()` | `raft.current_leader().await == Some(self.node_id)` | `bool` |

**Fencing token validation** in `finish()`:
```
stored_token = fencing_tokens.get(workflow_id)?
if stored_token != input.fencing_token → Err(WorkflowError::FencingTokenExpired)
```

**Raft write** example:
```rust
raft.client_write(WorkflowCommand::CreateOrGetWorkflow { ... }).await
    .map_err(|e| WorkflowError::Raft(e.to_string()))?;
```

## CheckpointService (`src/services/checkpoint_service.rs`)

```rust
pub struct CheckpointService {
    checkpoints: Arc<CheckpointRepository>,
    leases: Arc<LeaseRepository>,
    fencing_tokens: Arc<FencingTokenRepository>,
    locks: Arc<DashMap<String, Arc<tokio::sync::Mutex<()>>>>,
}
```

| Method | What it does | Returns |
|--------|-------------|---------|
| `checkpoint(input)` | Validates token; sends `UpsertCheckpoint` through Raft; returns `abort` bool | `Result<bool, CheckpointError>` |
| `lease(input)` | Acquires per-position lock; validates token; returns existing or new lease | `Result<LeaseCheckpointOutput, CheckpointError>` |
| `release(wf_id, pos)` | Deletes lease from LeaseRepository | `Result<(), CheckpointError>` |
| `create_idempotency_key(input)` | Validates token; generates deterministic UUID | `Result<String, CheckpointError>` |

**Per-position locking** (replaces `client.lock()`):
```rust
// lock_position creates a Mutex per "workflow_id:position" key in DashMap
let _guard = lock_position(wf_id, pos).lock().await;
// guard dropped at end of scope
```

**Idempotency key generation**: UUID v4, deterministic per (workflow_id, fencing_token, position).

**Non-determinism detection** in `checkpoint()`:
- If existing checkpoint has different `idempotency_key` → `Err(CheckpointError::NonDeterministicCheckpoint)`

## Raft command dispatch (summary)

| Operation | Goes through Raft? | Why |
|-----------|-------------------|-----|
| `CreateOrGetWorkflow` | Yes | Must be linearizable across nodes |
| `IncrementFencingToken` | Yes | Monotonic counter, must be ordered |
| `UpsertCheckpoint` | Yes | All nodes must agree on checkpoint value |
| `CompleteWorkflow` | Yes | Status change must be replicated |
| `DeleteExpired` | Yes | Leader-only but must be replicated to followers |
| `get_by_id`, `get`, `status` | No | Reads local SQLite directly |
| Lease acquire/release | No | In-memory moka cache, not replicated |

## Detail docs

- `workflow-service.md`
- `checkpoint-service.md`
