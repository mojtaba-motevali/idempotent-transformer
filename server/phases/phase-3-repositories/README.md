# Phase 3 — Repository Layer

**Status: DONE**

## What was done

All four repository structs implemented, compiling, and fully instrumented with `#[instrument]`.

## Repository structs

### WorkflowRepository (`src/repositories/workflows.rs`)

```
pool: deadpool_sqlite::Pool
```

| Method | SQL | Notes |
|--------|-----|-------|
| `create_or_get(id, name, created_at)` | `INSERT ... ON CONFLICT DO UPDATE SET name ... RETURNING *` | Idempotent |
| `get_by_id(id)` | `SELECT * WHERE id = ?` | Returns `Option<Workflow>` |
| `complete(id, expire_at, completed_at)` | `UPDATE ... SET status = Completed` | |
| `delete_expired(before_ms)` | `DELETE WHERE expire_at < ? AND status = Completed` | Returns row count |

### CheckpointRepository (`src/repositories/checkpoints.rs`)

| Method | SQL | Notes |
|--------|-----|-------|
| `upsert(wf_id, pos, ikey, value, created_at)` | `INSERT ... ON CONFLICT DO UPDATE SET value` | Idempotent on (workflow_id, position) |
| `get(wf_id, pos)` | `SELECT WHERE workflow_id = ? AND position = ?` | Returns `Option<Checkpoint>` |
| `delete_for_completed(before_ms)` | `DELETE WHERE workflow_id IN (SELECT id FROM Workflows WHERE ...)` | Cascading cleanup |

### FencingTokenRepository (`src/repositories/workflows_fencing_tokens.rs`)

| Method | SQL | Notes |
|--------|-----|-------|
| `increment(wf_id)` | `INSERT ... ON CONFLICT DO UPDATE SET fencing_token = fencing_token + 1 RETURNING fencing_token` | Atomic; returns new token |
| `get(wf_id)` | `SELECT fencing_token WHERE workflow_id = ?` | Returns `Option<i64>` |
| `delete_for_completed(before_ms)` | `DELETE WHERE workflow_id IN (SELECT ...)` | Cascading cleanup |

### LeaseRepository (`src/repositories/lease_checkpoint.rs`)

```
cache: moka::future::Cache<String, LeasedCheckpointValue>
```

Key format: `"{workflow_id}:{position}"`.

| Method | Behaviour |
|--------|-----------|
| `acquire(wf_id, pos, timeout_ms)` | Inserts with TTL; errors if already leased |
| `get(wf_id, pos)` | Returns `Option<LeasedCheckpointValue>` |
| `release(wf_id, pos)` | Removes the entry |

Not persisted to disk — leases are in-memory only. TTL expiry is automatic via moka.

## Error pattern

All repository methods return `Result<T, RepositoryError>`.

The `??` double-unwrap pattern is idiomatic here:
```rust
// interact returns Result<Result<T, rusqlite::Error>, InteractError>
conn.interact(|conn| { ... }).await??  // outer ? = InteractError, inner ? = rusqlite::Error
// Both errors convert to RepositoryError via From impls in error.rs
```

## Tracing

Every method has `#[instrument(skip(self), fields(...), err)]`.
`err` causes an automatic `ERROR` event when the method returns `Err`.

## Detail docs

- `workflow-repository.md`
- `checkpoint-repository.md`
- `fencing-token-repository.md`
- `lease-repository.md`
