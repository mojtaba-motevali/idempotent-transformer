# Phase 3 — LeaseRepository

## File

`src/repositories/lease_checkpoint.rs` (full rewrite)

## Depends on

- `moka::future::Cache` (Phase 1 dependency)
- `src/schema/leased_checkpoint.rs` — `LeasedCheckpointValue` (unchanged)

---

## Struct

Replaces `hiqlite`'s `Cache::One` (in-memory, Raft-replicated) with a local-only
`moka` async cache. Leases are ephemeral coordination signals — they do not need
cross-node replication; each node tracks only the leases it issued.

```rust
use std::time::Duration;
use moka::future::Cache;
use crate::schema::leased_checkpoint::LeasedCheckpointValue;

#[derive(Clone)]
pub struct LeaseRepository {
    cache: Cache<String, LeasedCheckpointValue>,
}

impl LeaseRepository {
    pub fn new() -> Self {
        Self {
            cache: Cache::builder()
                // hard upper bound — individual TTLs set per entry via time_to_live
                .max_capacity(100_000)
                .build(),
        }
    }

    fn key(workflow_id: &str, position: i64) -> String {
        format!("{workflow_id}:{position}")
    }
}
```

---

## Methods

### `acquire`

Stores a new lease with TTL. Returns the stored value.

```rust
pub async fn acquire(
    &self,
    workflow_id: &str,
    position: i64,
    lease_timeout_ms: i64,
) -> LeasedCheckpointValue {
    let created_at = chrono::Utc::now().timestamp_millis();
    let value = LeasedCheckpointValue { lease_timeout: lease_timeout_ms, created_at };
    let ttl = Duration::from_millis(lease_timeout_ms.max(30_000) as u64);

    self.cache
        .insert_with_ttl(Self::key(workflow_id, position), value.clone(), ttl)
        .await;
    value
}
```

> `Cache::insert_with_ttl` requires the `moka` `future` feature with per-entry TTL enabled.
> Enable it in `Cargo.toml`:
> ```toml
> moka = { version = "0.12", features = ["future"] }
> ```
> And build the cache with `.time_to_live(Duration::MAX)` disabled — use only per-entry TTL.

If `moka` per-entry TTL is not available in the chosen version, fall back to storing
`(LeasedCheckpointValue, Instant::now() + ttl)` and checking expiry in `get`.

### `get`

```rust
pub async fn get(
    &self,
    workflow_id: &str,
    position: i64,
) -> Option<LeasedCheckpointValue> {
    self.cache.get(&Self::key(workflow_id, position)).await
}
```

### `release`

```rust
pub async fn release(&self, workflow_id: &str, position: i64) {
    self.cache.invalidate(&Self::key(workflow_id, position)).await;
}
```

---

## Why no Raft replication for leases

Leases are a liveness mechanism — a worker holds a lease to signal "I am processing this
position." If the node crashes, the lease disappears naturally when TTL expires. Replicating
leases through Raft would add write amplification for a value that is intentionally ephemeral.
The fencing token (Raft-replicated) is the authoritative correctness guard; the lease is only
a performance optimisation to avoid redundant work.

---

## Tracing

```rust
impl LeaseRepository {
    #[instrument(skip(self), fields(workflow_id, position))]
    pub async fn acquire(...) { ... }

    #[instrument(skip(self), fields(workflow_id, position))]
    pub async fn get(...) { ... }

    #[instrument(skip(self), fields(workflow_id, position))]
    pub async fn release(...) { ... }
}
```

---

## Verification

```bash
cargo check -p idempotency-server
# No hiqlite imports; LeaseRepository::clone must be cheap (Cache is Arc-backed)
```
