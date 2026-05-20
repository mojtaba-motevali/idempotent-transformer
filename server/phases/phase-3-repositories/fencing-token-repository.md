# Phase 3 — FencingTokenRepository

## File

`src/repositories/workflows_fencing_tokens.rs` (full rewrite)

## Depends on

- `deadpool_sqlite::Pool` from `RaftNode::state_machine.pool` (Phase 2)
- `src/schema/workflow_fencing_token.rs` — `WorkflowFencingToken` (unchanged)

---

## Struct

```rust
use deadpool_sqlite::Pool;
use crate::error::RepositoryError;

#[derive(Clone)]
pub struct FencingTokenRepository {
    pool: Pool,
}

impl FencingTokenRepository {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }
}
```

---

## Methods

### `increment`

Atomically inserts or increments the token. Returns the new value.
Same semantics as the current hiqlite `ON CONFLICT DO UPDATE` upsert.

```rust
pub async fn increment(
    &self,
    workflow_id: &str,
) -> Result<i64, RepositoryError> {
    let conn = self.pool.get().await?;
    let wf_id = workflow_id.to_owned();

    let token = conn.interact(move |conn| {
        conn.query_row(
            "INSERT INTO WorkflowFencingTokens (workflow_id, fencing_token)
             VALUES (?1, 1)
             ON CONFLICT (workflow_id)
             DO UPDATE SET fencing_token = WorkflowFencingTokens.fencing_token + 1
             RETURNING fencing_token",
            rusqlite::params![wf_id],
            |row| row.get::<_, i64>(0),
        )
    })
    .await?
    .map_err(RepositoryError::Sqlite)?;
    Ok(token)
}
```

### `get`

Returns `None` if the workflow has never been started.

```rust
pub async fn get(
    &self,
    workflow_id: &str,
) -> Result<Option<i64>, RepositoryError> {
    let conn = self.pool.get().await?;
    let wf_id = workflow_id.to_owned();

    conn.interact(move |conn| {
        conn.query_row(
            "SELECT fencing_token FROM WorkflowFencingTokens WHERE workflow_id = ?1",
            rusqlite::params![wf_id],
            |row| row.get::<_, i64>(0),
        )
        .optional()
    })
    .await?
    .map_err(RepositoryError::Sqlite)
}
```

### `delete_for_completed`

Removes fencing tokens for expired, completed workflows.

```rust
pub async fn delete_for_completed(
    &self,
    before_ms: i64,
) -> Result<u64, RepositoryError> {
    let conn = self.pool.get().await?;

    let n = conn.interact(move |conn| {
        conn.execute(
            "DELETE FROM WorkflowFencingTokens
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

## Tracing

```rust
impl FencingTokenRepository {
    #[instrument(skip(self), fields(workflow_id), err)]
    pub async fn increment(...) -> Result<i64, RepositoryError> { ... }

    #[instrument(skip(self), fields(workflow_id), err)]
    pub async fn get(...) -> Result<Option<i64>, RepositoryError> { ... }

    #[instrument(skip(self), err)]
    pub async fn delete_for_completed(...) -> Result<u64, RepositoryError> { ... }
}
```

---

## Verification

```bash
cargo check -p idempotency-server
```
