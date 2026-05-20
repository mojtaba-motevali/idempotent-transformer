# Phase 3 — WorkflowRepository

## File

`src/repositories/workflows.rs` (full rewrite)

## Depends on

- `deadpool_sqlite::Pool` from `RaftNode::state_machine.pool` (Phase 2)
- `src/schema/workflow.rs` — `Workflow`, `WorkflowStatus` (unchanged)

---

## Struct

```rust
use deadpool_sqlite::Pool;
use crate::{
    error::RepositoryError,
    schema::workflow::{Workflow, WorkflowStatus},
};

#[derive(Clone)]
pub struct WorkflowRepository {
    pool: Pool,
}

impl WorkflowRepository {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }
}
```

---

## Methods

### `create_or_get`

Idempotent upsert — same semantics as the current hiqlite version.
`RepositoryError` has `From` impls for `PoolError`, `InteractError`, and `rusqlite::Error`,
so the double-`?` (`await??`) propagates through all three layers automatically.

```rust
pub async fn create_or_get(
    &self,
    workflow_id: &str,
    name: Option<&str>,
    created_at: i64,
) -> Result<Workflow, RepositoryError> {
    let conn = self.pool.get().await?;
    let wf_id = workflow_id.to_owned();
    let wf_name = name.map(str::to_owned);
    let status = WorkflowStatus::Running as i64;

    conn.interact(move |conn| {
        conn.query_row(
            "INSERT INTO Workflows (id, status, name, created_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT (id) DO UPDATE SET name = excluded.name
             RETURNING *",
            rusqlite::params![wf_id, status, wf_name, created_at],
            Workflow::from_row,
        )
    })
    .await?
    .map_err(RepositoryError::Sqlite)
}
```

### `get_by_id`

```rust
pub async fn get_by_id(
    &self,
    workflow_id: &str,
) -> Result<Option<Workflow>, RepositoryError> {
    let conn = self.pool.get().await?;
    let wf_id = workflow_id.to_owned();

    conn.interact(move |conn| {
        conn.query_row(
            "SELECT * FROM Workflows WHERE id = ?1",
            rusqlite::params![wf_id],
            Workflow::from_row,
        )
        .optional()
    })
    .await?
    .map_err(RepositoryError::Sqlite)
}
```

### `complete`

```rust
pub async fn complete(
    &self,
    workflow_id: &str,
    expire_at: i64,
    completed_at: i64,
) -> Result<(), RepositoryError> {
    let conn = self.pool.get().await?;
    let wf_id = workflow_id.to_owned();
    let status = WorkflowStatus::Completed as i64;

    conn.interact(move |conn| {
        conn.execute(
            "UPDATE Workflows SET status = ?1, expire_at = ?2, completed_at = ?3
             WHERE id = ?4",
            rusqlite::params![status, expire_at, completed_at, wf_id],
        )
    })
    .await?
    .map_err(RepositoryError::Sqlite)?;
    Ok(())
}
```

### `delete_expired`

```rust
pub async fn delete_expired(
    &self,
    before_ms: i64,
) -> Result<u64, RepositoryError> {
    let conn = self.pool.get().await?;
    let status = WorkflowStatus::Completed as i64;

    let n = conn.interact(move |conn| {
        conn.execute(
            "DELETE FROM Workflows WHERE expire_at < ?1 AND status = ?2",
            rusqlite::params![before_ms, status],
        )
    })
    .await?
    .map_err(RepositoryError::Sqlite)?;
    Ok(n as u64)
}
```

---

## `Workflow::from_row` helper

Add to `src/schema/workflow.rs`:

```rust
impl Workflow {
    pub fn from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            status: row.get(1)?,
            name: row.get(2)?,
            expire_at: row.get(3)?,
            completed_at: row.get(4)?,
            created_at: row.get(5)?,
        })
    }
}
```

---

## Tracing

All methods carry `#[instrument]` so failures are logged with structured context.
`err` records the `RepositoryError` variant in the span automatically.

```rust
use tracing::instrument;

impl WorkflowRepository {
    #[instrument(skip(self), fields(workflow_id), err)]
    pub async fn create_or_get(...) -> Result<Workflow, RepositoryError> { ... }

    #[instrument(skip(self), fields(workflow_id), err)]
    pub async fn get_by_id(...) -> Result<Option<Workflow>, RepositoryError> { ... }

    #[instrument(skip(self), fields(workflow_id), err)]
    pub async fn complete(...) -> Result<(), RepositoryError> { ... }

    #[instrument(skip(self), err)]
    pub async fn delete_expired(...) -> Result<u64, RepositoryError> { ... }
}
```

---

## Verification

```bash
cargo check -p idempotency-server
# WorkflowRepository must compile with no hiqlite imports
```
