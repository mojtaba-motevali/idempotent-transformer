# repositories/ — Data access layer

All repository structs wrap a `SharedStore` (`Arc<parking_lot::Mutex<Store>>`) and expose
async methods returning `Result<T, RepositoryError>`.  The underlying data lives entirely
in RAM inside the state machine's `Store` struct — no RocksDB in this layer.

Every method is instrumented with `#[instrument(skip(self), fields(...), err)]`.

## In-memory data layout (`Store`)

| Field | Type | Key |
|-------|------|-----|
| `workflows` | `HashMap<String, Workflow>` | `workflow_id` |
| `fencing_tokens` | `HashMap<String, i64>` | `workflow_id` |
| `checkpoints` | `BTreeMap<(String, i64), Checkpoint>` | `(workflow_id, position)` |

`Store` is defined in `storage::state_machine` and protected by a `parking_lot::Mutex`.

## Mutex discipline

All repository methods lock `SharedStore` for the duration of the in-memory operation
(HashMap/BTreeMap read or write) and release before returning.  No `.await` is held
across the lock — `parking_lot::Mutex` (sync, non-poisoning) is appropriate.

## Files

### `workflows.rs` — `WorkflowRepository`

```rust
pub struct WorkflowRepository { store: SharedStore }
impl WorkflowRepository {
    pub fn new(store: SharedStore) -> Self

    pub async fn create_or_get(
        &self, workflow_id: &str, name: Option<&str>, created_at: i64,
    ) -> Result<Workflow, RepositoryError>
    // HashMap::entry → Occupied: optionally update name; Vacant: insert new Workflow

    pub async fn get_by_id(&self, workflow_id: &str) -> Result<Option<Workflow>, RepositoryError>
    // store.workflows.get(workflow_id).cloned()

    pub async fn complete(
        &self, workflow_id: &str, expire_at: i64, completed_at: i64,
    ) -> Result<(), RepositoryError>
    // get_mut → set status = Completed, expire_at, completed_at

    pub async fn delete_expired(&self, before_ms: i64) -> Result<u64, RepositoryError>
    // collect expired IDs → remove from store.workflows
}
```

### `checkpoints.rs` — `CheckpointRepository`

```rust
pub struct CheckpointRepository { store: SharedStore }
impl CheckpointRepository {
    pub fn new(store: SharedStore) -> Self

    pub async fn upsert(
        &self, workflow_id, position, idempotency_key, value: Option<&[u8]>, created_at,
    ) -> Result<(), RepositoryError>
    // BTreeMap::insert((workflow_id, position), Checkpoint { ... })
    // Idempotent: same key overwrites the value

    pub async fn get(&self, workflow_id, position) -> Result<Option<Checkpoint>, RepositoryError>
    // BTreeMap::get(&(workflow_id, position)).cloned()

    pub async fn delete_for_completed(&self, before_ms: i64) -> Result<u64, RepositoryError>
    // 1. Scan store.workflows for expired IDs (same lock)
    // 2. BTreeMap::retain to remove all matching checkpoints
}
```

### `workflows_fencing_tokens.rs` — `FencingTokenRepository`

```rust
pub struct FencingTokenRepository { store: SharedStore }
impl FencingTokenRepository {
    pub fn new(store: SharedStore) -> Self

    pub async fn increment(&self, workflow_id) -> Result<i64, RepositoryError>
    // HashMap::entry → and_modify(+1) / or_insert(1)
    // Safe: store lock serializes all writers

    pub async fn get(&self, workflow_id) -> Result<Option<i64>, RepositoryError>
    // store.fencing_tokens.get(workflow_id).copied()

    pub async fn delete_for_completed(&self, before_ms: i64) -> Result<u64, RepositoryError>
    // Scan store.workflows for expired IDs → remove from store.fencing_tokens (same lock)
}
```

### `lease_checkpoint.rs` — `LeaseRepository`

```rust
pub struct LeaseRepository {
    cache: moka::future::Cache<String, LeasedCheckpointValue>,
}
impl LeaseRepository {
    pub fn new() -> Self  // capacity 100,000; no auto-TTL

    pub async fn acquire(&self, workflow_id, position, lease_timeout_ms) -> LeasedCheckpointValue
    // inserts { lease_timeout, created_at: now_ms }; key = "{wf_id}:{pos}"

    pub async fn get(&self, workflow_id, position) -> Option<LeasedCheckpointValue>
    // returns None if missing OR now > created_at + lease_timeout

    pub async fn release(&self, workflow_id, position)
    // cache.invalidate("{wf_id}:{pos}")
}
```

**Not persisted** — in-memory only. Server restart clears all leases.

## Error handling

```rust
pub enum RepositoryError {}  // empty — in-memory HashMap/BTreeMap ops are infallible
```

All repository methods return `Ok(...)` unconditionally; `RepositoryError` exists only
to satisfy the `Result<T, RepositoryError>` return type required by service callers.

## Concurrency

- **Write path**: `StateMachine::apply()` (Raft, sequential) + direct service-layer writes
  (`CheckpointService::checkpoint` → `CheckpointRepository::upsert`).
- **Read path**: service layer reads concurrently. Parking_lot lock is uncontended on reads.
- Cleanup (`delete_for_completed`, `delete_expired`): acquires the store once per call,
  scanning workflows and deleting in the same critical section — consistent, no TOCTOU.

## Dependencies

- `storage::state_machine::SharedStore`
- `schema::{Workflow, WorkflowStatus, Checkpoint, LeasedCheckpointValue}`
- `error::RepositoryError`
- `moka::future::Cache` (lease only)
