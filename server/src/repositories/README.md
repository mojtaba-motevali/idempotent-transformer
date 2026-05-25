# repositories/ — Data access layer

All repository structs wrap an `Arc<rocksdb::DB>` (or `moka` cache for leases) and expose
async methods returning `Result<T, RepositoryError>`.

Every method is instrumented with `#[instrument(skip(self), fields(...), err)]`.

## RocksDB schema (state machine DB at `{data_dir}/state`)

| Column family | Key | Value |
|---------------|-----|-------|
| `workflows` | `workflow_id` (UTF-8 bytes) | serde_json `Workflow` |
| `fencing_tokens` | `workflow_id` (UTF-8 bytes) | 8-byte big-endian `i64` |
| `checkpoints` | `{workflow_id bytes}\x00{position 8-byte BE}` | serde_json `Checkpoint` |
| `raft_meta` | key string bytes | serde_json value (used by StateMachine only) |

The null-byte separator in checkpoint keys allows prefix scanning by `workflow_id`
(a workflow ID never contains a null byte).

CF names are defined as constants in `storage::state_machine`:
`CF_WORKFLOWS`, `CF_FENCING_TOKENS`, `CF_CHECKPOINTS`, `CF_RAFT_META`.

## Files

### `workflows.rs` — `WorkflowRepository`

```rust
pub struct WorkflowRepository { db: Arc<DB> }
impl WorkflowRepository {
    pub fn new(db: Arc<DB>) -> Self

    pub async fn create_or_get(
        &self, workflow_id: &str, name: Option<&str>, created_at: i64,
    ) -> Result<Workflow, RepositoryError>
    // get → if exists update name and return; else put new Workflow

    pub async fn get_by_id(&self, workflow_id: &str) -> Result<Option<Workflow>, RepositoryError>
    // get_cf(CF_WORKFLOWS, workflow_id)

    pub async fn complete(
        &self, workflow_id: &str, expire_at: i64, completed_at: i64,
    ) -> Result<(), RepositoryError>
    // get → mutate status/expire_at/completed_at → put

    pub async fn delete_expired(&self, before_ms: i64) -> Result<u64, RepositoryError>
    // iterate CF_WORKFLOWS, delete Completed entries with expire_at < before_ms
}
```

**`expired_workflow_ids(db, before_ms) -> Result<Vec<String>, RepositoryError>`**
`pub(crate)` free function — iterates `CF_WORKFLOWS`, returns IDs of `Completed`
workflows with `expire_at < before_ms`. Used by all three cleanup methods.

### `checkpoints.rs` — `CheckpointRepository`

```rust
pub struct CheckpointRepository { db: Arc<DB> }
impl CheckpointRepository {
    pub fn new(db: Arc<DB>) -> Self

    pub async fn upsert(
        &self, workflow_id, position, idempotency_key, value: Option<&[u8]>, created_at,
    ) -> Result<(), RepositoryError>
    // put_cf(CF_CHECKPOINTS, cp_key(wf_id, pos), json(Checkpoint))
    // Idempotent: same key overwrites the value

    pub async fn get(&self, workflow_id, position) -> Result<Option<Checkpoint>, RepositoryError>
    // get_cf(CF_CHECKPOINTS, cp_key(wf_id, pos))

    pub async fn delete_for_completed(&self, before_ms) -> Result<u64, RepositoryError>
    // 1. expired_workflow_ids(db, before_ms)
    // 2. For each expired id: prefix-scan CF_CHECKPOINTS by cp_prefix(id), delete all
}
```

Checkpoint key format: `{workflow_id_bytes} ++ [0x00] ++ {position.to_be_bytes()}`.
Prefix = `{workflow_id_bytes} ++ [0x00]` (all checkpoints for a workflow share this prefix).

### `workflows_fencing_tokens.rs` — `FencingTokenRepository`

```rust
pub struct FencingTokenRepository { db: Arc<DB> }
impl FencingTokenRepository {
    pub fn new(db: Arc<DB>) -> Self

    pub async fn increment(&self, workflow_id) -> Result<i64, RepositoryError>
    // get current (default 0) → +1 → put as 8-byte BE → return new value
    // Safe without a transaction: StateMachine::apply() is single-writer

    pub async fn get(&self, workflow_id) -> Result<Option<i64>, RepositoryError>
    // get_cf(CF_FENCING_TOKENS, workflow_id) → decode 8-byte BE i64

    pub async fn delete_for_completed(&self, before_ms) -> Result<u64, RepositoryError>
    // expired_workflow_ids(db, before_ms) → delete each from CF_FENCING_TOKENS
}
```

Fencing tokens are stored as raw 8-byte big-endian `i64`, not JSON.

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
pub enum RepositoryError {
    RocksDb(#[from] rocksdb::Error),   // DB read/write failures
    Decode(String),                     // serde_json or byte-decode failures
}
```

No `??` double-unwrap needed (RocksDB calls return `Result<T, rocksdb::Error>` directly).

## Concurrency

All repo methods are called from `StateMachine::apply()` (single-writer, sequential) for
write operations. Read operations (`get_by_id`, `get`, `FencingTokenRepository::get`) can
be called concurrently from service layer. RocksDB is thread-safe for concurrent reads.

## Dependencies

- `error::RepositoryError`
- `schema::{Workflow, WorkflowStatus, Checkpoint, LeasedCheckpointValue}`
- `storage::state_machine::{CF_WORKFLOWS, CF_FENCING_TOKENS, CF_CHECKPOINTS}`
- `rocksdb::{DB, IteratorMode, Direction}`
- `moka::future::Cache`
