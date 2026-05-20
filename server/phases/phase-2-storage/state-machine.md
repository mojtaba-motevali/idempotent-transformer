# Phase 2 — StateMachine

## File

`src/storage/state_machine.rs`

## Purpose

Implements `openraft::storage::RaftStateMachine<TypeConfig>` on top of a
`deadpool_sqlite::Pool`. The state machine receives committed `WorkflowCommand` entries
from the Raft leader and applies them to SQLite. It delegates every write to the
repository structs defined in Phase 3 — no SQL is written here directly.

---

## Struct

```rust
use std::sync::Arc;
use tokio::sync::Mutex;
use deadpool_sqlite::Pool;
use openraft::{LogId, SnapshotMeta, StoredMembership};
use crate::storage::type_config::TypeConfig;

pub struct StateMachine {
    pub pool: Pool,
    // last applied log id — persisted in a dedicated SQLite table
    last_applied: Arc<Mutex<Option<LogId<u64>>>>,
    last_membership: Arc<Mutex<StoredMembership<u64, crate::storage::type_config::Node>>>,
}

impl StateMachine {
    /// Called from `RaftNode::start()` — `anyhow::Error` is appropriate (init-only path).
    pub async fn new(pool: Pool) -> anyhow::Result<Self> {
        // Run migrations on init (reuses existing migrations/1_init.sql)
        // Also create a "raft_meta" table to persist last_applied
        Ok(Self {
            pool,
            last_applied: Arc::new(Mutex::new(None)),
            last_membership: Arc::new(Mutex::new(StoredMembership::default())),
        })
    }
}
```

Additional SQLite table needed (add to migrations or apply inline):

```sql
CREATE TABLE IF NOT EXISTS RaftMeta (
    key   TEXT NOT NULL PRIMARY KEY,
    value TEXT NOT NULL
);
-- stores last_applied log id and last membership as JSON
```

---

## RaftStateMachine implementation

| openraft method | What it does |
|----------------|--------------|
| `applied_state()` | Returns `(last_applied, last_membership)` from in-memory fields (loaded from `RaftMeta` on startup) |
| `apply(entries)` | Iterates entries; dispatches each `WorkflowCommand` variant to the correct repository method; updates `last_applied` |
| `get_snapshot()` | Serializes full DB state to `Vec<u8>` (simple approach: dump all rows as JSON) |
| `install_snapshot(meta, data)` | Deserializes and bulk-inserts; resets `last_applied` to snapshot meta |
| `begin_receiving_snapshot()` | Returns a `Box<dyn AsyncWrite>` (use `tokio::io::sink()` buffered into memory) |

### apply dispatch (skeleton)

```rust
async fn apply<I>(&mut self, entries: I) -> Result<Vec<WorkflowResponse>, StorageError<u64>>
where
    I: IntoIterator<Item = Entry<TypeConfig>> + Send,
{
    let conn = self.pool.get().await?;
    let mut responses = Vec::new();

    for entry in entries {
        let response = match entry.payload {
            EntryPayload::Normal(cmd) => match cmd {
                WorkflowCommand::CreateOrGetWorkflow { .. } => {
                    // delegate to WorkflowRepository::create_or_get (Phase 3)
                    WorkflowResponse::Empty
                }
                WorkflowCommand::IncrementFencingToken { workflow_id } => {
                    // delegate to FencingTokenRepository::increment (Phase 3)
                    WorkflowResponse::FencingToken(0) // replace with actual result
                }
                WorkflowCommand::UpsertCheckpoint { .. } => {
                    // delegate to CheckpointRepository::upsert (Phase 3)
                    WorkflowResponse::Empty
                }
                WorkflowCommand::CompleteWorkflow { .. } => WorkflowResponse::Empty,
                WorkflowCommand::DeleteExpired { .. } => WorkflowResponse::Empty,
            },
            EntryPayload::Membership(m) => {
                *self.last_membership.lock().await =
                    StoredMembership::new(Some(entry.log_id), m);
                WorkflowResponse::Empty
            }
            EntryPayload::Blank => WorkflowResponse::Empty,
        };

        *self.last_applied.lock().await = Some(entry.log_id);
        responses.push(response);
    }

    // persist last_applied to RaftMeta table
    Ok(responses)
}
```

---

## Notes

- The `pool` field is `pub` so repository structs (Phase 3) can share the same pool —
  pass `state_machine.pool.clone()` when constructing each repository.
- Snapshots: for MVP, serialise all rows to JSON. A proper incremental snapshot (WAL-based)
  is an out-of-scope follow-up.
- `apply` is called serially by openraft — no concurrent apply calls, so the single-writer
  SQLite constraint is respected here.

---

## Verification

```bash
cargo check -p idempotency-server
# StateMachine must implement RaftStateMachine<TypeConfig>
```
