# Phase 3 — CheckpointRepository

## File

`src/repositories/checkpoints.rs` (full rewrite)

## Depends on

- `deadpool_sqlite::Pool` from `RaftNode::state_machine.pool` (Phase 2)
- `src/schema/checkpoint.rs` — `Checkpoint`, `CheckpointValue` (unchanged)

---

## Struct

```rust
use deadpool_sqlite::Pool;
use crate::{
    error::RepositoryError,
    schema::checkpoint::{Checkpoint, CheckpointValue},
};

#[derive(Clone)]
pub struct CheckpointRepository {
    pool: Pool,
}

impl CheckpointRepository {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }
}
```

---

## Methods

### `upsert`

Idempotent — same `ON CONFLICT DO UPDATE` semantics as the current hiqlite version.
`value` is `None` when called from `create_idempotency_key`; set when called from `checkpoint`.

```rust
pub async fn upsert(
    &self,
    workflow_id: &str,
    position: i64,
    idempotency_key: &str,
    value: Option<&[u8]>,
    created_at: i64,
) -> Result<(), RepositoryError> {
    let conn = self.pool.get().await?;
    let wf_id = workflow_id.to_owned();
    let ikey = idempotency_key.to_owned();
    let val = value.map(|v| v.to_vec());

    conn.interact(move |conn| {
        conn.execute(
            "INSERT INTO Checkpoints (workflow_id, position, idempotency_key, value, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT (workflow_id, position)
             DO UPDATE SET value = excluded.value",
            rusqlite::params![wf_id, position, ikey, val, created_at],
        )
    })
    .await?
    .map_err(RepositoryError::Sqlite)?;
    Ok(())
}
```

### `get`

```rust
pub async fn get(
    &self,
    workflow_id: &str,
    position: i64,
) -> Result<Option<Checkpoint>, RepositoryError> {
    let conn = self.pool.get().await?;
    let wf_id = workflow_id.to_owned();

    conn.interact(move |conn| {
        conn.query_row(
            "SELECT * FROM Checkpoints WHERE workflow_id = ?1 AND position = ?2",
            rusqlite::params![wf_id, position],
            Checkpoint::from_row,
        )
        .optional()
    })
    .await?
    .map_err(RepositoryError::Sqlite)
}
```

### `delete_for_completed`

Deletes checkpoints belonging to workflows that are completed and past their expiry.
Called by the cleanup cron job (via `WorkflowService::cleanup_expired`).

```rust
pub async fn delete_for_completed(
    &self,
    before_ms: i64,
) -> Result<u64, RepositoryError> {
    let conn = self.pool.get().await?;

    let n = conn.interact(move |conn| {
        conn.execute(
            "DELETE FROM Checkpoints
             WHERE workflow_id IN (
                 SELECT id FROM Workflows
                 WHERE status = 1 AND expire_at < ?1
             )",
            rusqlite::params![before_ms],
        )
    })
    .await?
    .map_err(RepositoryError::Sqlite)?;
    Ok(n as u64)
}
```

---

## `Checkpoint::from_row` helper

Add to `src/schema/checkpoint.rs`:

```rust
impl Checkpoint {
    pub fn from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            workflow_id:    row.get(0)?,
            position:       row.get(1)?,
            value:          row.get(2)?,
            idempotency_key: row.get(3)?,
            created_at:     row.get(4)?,
            lease_timeout:  0, // not stored in DB; set by LeaseRepository
        })
    }
}
```

---

## Tracing

```rust
impl CheckpointRepository {
    #[instrument(skip(self), fields(workflow_id, position), err)]
    pub async fn upsert(...) -> Result<(), RepositoryError> { ... }

    #[instrument(skip(self), fields(workflow_id, position), err)]
    pub async fn get(...) -> Result<Option<Checkpoint>, RepositoryError> { ... }

    #[instrument(skip(self), err)]
    pub async fn delete_for_completed(...) -> Result<u64, RepositoryError> { ... }
}
```

---

## Verification

```bash
cargo check -p idempotency-server
```
