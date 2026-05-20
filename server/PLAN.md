# Plan: Replace Hiqlite with openraft + Multi-Writer Storage

> **Status:** In Progress  
> **Branch:** `feat/remove-hiqilite`  
> **Date:** 2026-05-19

---

## Problem

The workflow engine uses **Hiqlite** (SQLite + Raft) to store and replicate state across
cluster nodes. SQLite is a **single-writer** database: only one thread can write to the
WAL at a time. Every Raft log append, checkpoint upsert, fencing token increment, and lease
operation serializes behind this writer. Under concurrent load — many workflows with
simultaneous `checkpoint` / `workflow_start` calls — write latency spikes because all writes
queue behind the SQLite lock.

Additionally, Hiqlite bundles both the Raft consensus layer and the state-machine storage into
one opaque crate, making it hard to tune or replace either independently.

---

## Goal

Replace Hiqlite with **openraft** (for the Raft consensus layer) and an embeddable
**multi-writer database** (for both the Raft log store and the state machine). Refactor
the architecture to have zero-cost abstractions across layers so storage backends are
swappable without runtime overhead.

---

## Non-Goals

- Changing the gRPC API surface (proto file stays the same)
- Adding new workflow features
- Changing the fencing token / idempotency protocol

---

## Context

### Current architecture

```
Client (gRPC)
    ↓
src/rpc_server/server.rs        WorkflowService — 7 RPC endpoints
    ↓
src/services/
    workflow_service.rs         create_workflow, finish_workflow, leader-only cleanup
    checkpoint_service.rs       handle_checkpoint, handle_lease_checkpoint,
                                create_durable_idempotency_key
    ↓
src/repositories/
    workflows.rs                Workflows table CRUD
    checkpoints.rs              Checkpoints table CRUD
    lease_checkpoint.rs         Cache::One — in-memory TTL leases
    workflows_fencing_tokens.rs WorkflowFencingTokens table
    ↓
hiqlite::Client                 SQLite + Raft consensus  ← replace this layer
```

### Module → file map

| File | Role |
|------|------|
| `src/main.rs` | Runtime setup, env loading, graceful shutdown |
| `src/database/db.rs` | Hiqlite client init, Cache enum, `init_tables()`, WAL pragmas |
| `src/database/server.rs` | `Server` struct — node id, addr_api, addr_raft |
| `src/rpc_server/server.rs` | Tonic gRPC impl — 7 RPCs, `to_status()` error mapper |
| `src/services/workflow_service.rs` | Workflow lifecycle + leader-only cron cleanup |
| `src/services/checkpoint_service.rs` | Checkpoint, lease, idempotency key logic |
| `src/repositories/workflows.rs` | SQL: insert/select/delete Workflows |
| `src/repositories/checkpoints.rs` | SQL: upsert/select/delete Checkpoints |
| `src/repositories/lease_checkpoint.rs` | Cache: put/get/delete leases (TTL-keyed) |
| `src/repositories/workflows_fencing_tokens.rs` | SQL: atomic increment fencing token |
| `src/schema/workflow.rs` | `Workflow` struct, `WorkflowStatus` enum |
| `src/schema/checkpoint.rs` | `Checkpoint`, `CheckpointValue` structs |
| `src/schema/leased_checkpoint.rs` | `LeasedCheckpointValue`, `LeasedCheckpoint` |
| `src/schema/workflow_fencing_token.rs` | `WorkflowFencingToken` struct |
| `src/cron/clean_up_workflows.rs` | 10-second cron job (leader-only cleanup) |
| `src/helpers/common.rs` | `return_error_if_true()` helper |
| `migrations/1_init.sql` | 3 tables + 1 index |
| `proto/workflow_service.proto` | 7-endpoint gRPC service definition |
| `hiqlite.toml` | Raft cluster config (node addresses, WAL interval 50 ms) |

### SQL schema (migrations/1_init.sql — must be preserved semantically)

```sql
Workflows              (id PK, status, name, expire_at, completed_at, created_at)
WorkflowFencingTokens  (workflow_id PK, fencing_token)
Checkpoints            (workflow_id, position, value BYTEA, idempotency_key, created_at)
  UNIQUE INDEX on (workflow_id, position)
```

Leased checkpoints: in-memory only (`Cache::One` with TTL) — not persisted to disk.

### gRPC API (proto unchanged)

| RPC | Key inputs | Key output |
|-----|-----------|------------|
| `workflow_start` | workflow_id, context_name? | fencing_token |
| `generate_idempotency_key` | workflow_id, fencing_token, position | idempotency_key |
| `checkpoint` | workflow_id, value, fencing_token, position, idempotency_key | abort (bool) |
| `lease_checkpoint` | workflow_id, fencing_token, lease_timeout, position, idempotency_key | value OR remaining_lease_timeout |
| `complete_workflow` | workflow_id, fencing_token, expire_after | — |
| `workflow_status` | workflow_id | status, timestamps |
| `release_checkpoint` | workflow_id, position | — |

### Concurrency hotspots where SQLite single-writer hurts

All of these currently go through the single SQLite write lock via Raft consensus:

| Call site | Trigger |
|-----------|---------|
| `increment_workflow_fencing_token()` | every `workflow_start` |
| `create_or_get_workflow()` | every `workflow_start` (parallel with above) |
| `upsert_checkpoint()` | every `checkpoint` RPC |
| lease cache writes | every `checkpoint` RPC (Raft-replicated through hiqlite) |
| `delete_expired_*()` | every 10 s on leader |

### Files to replace (hiqlite touch points)

- `src/database/db.rs` — `start_node_with_cache`, `Client`, `Cache` enum, `CacheIndex`
- `src/repositories/workflows.rs` — `client.query_as_optional`, `client.execute`, `client.execute_returning_one`
- `src/repositories/checkpoints.rs` — same
- `src/repositories/workflows_fencing_tokens.rs` — same
- `src/repositories/lease_checkpoint.rs` — `client.put/get/delete` on `Cache::One`
- `src/services/workflow_service.rs` — `client.is_leader_db()`
- `src/services/checkpoint_service.rs` — `client.lock()`
- `Cargo.toml` — `hiqlite`, `hiqlite-macros`
- `hiqlite.toml` — entire file

### Current tracing & logging gaps

`tracing` and `tracing_subscriber` are already in `Cargo.toml` but are barely used:

| Location | Problem |
|----------|---------|
| `src/main.rs:73` | `EnvFilter::from("info")` — level hardcoded, cannot be changed at runtime |
| `src/main.rs:105–115` | Shutdown signals logged as `error!` — SIGTERM/SIGINT are normal events, should be `info!` |
| `src/main.rs:43` | `println!("Starting server")` — not captured by the tracing subscriber |
| `src/rpc_server/server.rs:203` | `println!("Server listening on {addr}")` — same |
| `src/services/workflow_service.rs:91,102` | `println!("Deleting expired workflows")` — not structured |
| `src/cron/clean_up_workflows.rs:17` | `eprintln!("Error during scheduled delete: {e}")` — goes to stderr, bypasses tracing |
| All RPC handlers | No spans → no correlation between a log line and the request that produced it |
| All service/repository functions | No `#[instrument]` → no call-graph visibility in traces |
| No structured fields | `workflow_id`, `fencing_token`, `position` never appear as fields in any log line |

---

## Design

### Storage backend recommendation

| Option | Multi-writer | Rust native | Notes |
|--------|-------------|-------------|-------|
| **RocksDB** (`rocksdb` crate) | Yes (column-family level) | No (C++ FFI) | Industry standard for Raft log; used by TiKV, CockroachDB |
| **fjall** | Yes (multiple keyspaces) | Yes | Pure Rust LSM, actively maintained |
| **redb** | No (MVCC, single writer per txn) | Yes | Still single writer |
| **sled** | No | Yes | Deprecated upstream |

**Recommended split:**

- **Raft log store** → RocksDB (append-only, high-frequency, multi-writer via column families)
- **State machine** → SQLite via `rusqlite` + `deadpool-sqlite` connection pool (keeps existing schema and migrations intact; WAL mode with a pool eliminates the single-connection bottleneck for reads; writes go through the Raft state machine apply path which is single-threaded by design)

If RocksDB C++ FFI build pain is too high, swap in **fjall** as a pure-Rust alternative for the log store.

### openraft integration points

```rust
// src/storage/log_store.rs
struct LogStore(Arc<rocksdb::DB>);
// impl openraft::storage::RaftLogStore<TypeConfig>

// src/storage/state_machine.rs
struct StateMachine(deadpool_sqlite::Pool);
// impl openraft::storage::RaftStateMachine<TypeConfig>

// src/storage/network.rs
struct Network;
// impl openraft::network::RaftNetwork<TypeConfig>

// src/database/db.rs (new)
struct RaftNode {
    raft: openraft::Raft<TypeConfig>,
    log_store: Arc<LogStore>,
    state_machine: Arc<StateMachine>,
}
```

### Struct-based abstraction layers

Every layer is a **struct** whose capabilities are exposed as `impl` methods. No free functions
in repositories or services. Each struct owns its dependencies (pool, cache, raft handle) and
is constructed once in `main.rs` and shared via `Arc`.

#### Repository structs (`src/repositories/`)

```rust
// workflows.rs
pub struct WorkflowRepository {
    pool: deadpool_sqlite::Pool,
}
impl WorkflowRepository {
    pub fn new(pool: deadpool_sqlite::Pool) -> Self { ... }
    pub async fn create_or_get(&self, input: CreateWorkflowInput) -> Result<Workflow, Error> { ... }
    pub async fn get_by_id(&self, id: &str) -> Result<Option<Workflow>, Error> { ... }
    pub async fn complete(&self, id: &str, expire_at: i64) -> Result<(), Error> { ... }
    pub async fn delete_expired(&self) -> Result<u64, Error> { ... }
}

// checkpoints.rs
pub struct CheckpointRepository {
    pool: deadpool_sqlite::Pool,
}
impl CheckpointRepository {
    pub fn new(pool: deadpool_sqlite::Pool) -> Self { ... }
    pub async fn upsert(&self, input: UpsertCheckpointInput) -> Result<(), Error> { ... }
    pub async fn get(&self, workflow_id: &str, position: i64) -> Result<Option<Checkpoint>, Error> { ... }
    pub async fn delete_for_completed(&self) -> Result<u64, Error> { ... }
}

// workflows_fencing_tokens.rs
pub struct FencingTokenRepository {
    pool: deadpool_sqlite::Pool,
}
impl FencingTokenRepository {
    pub fn new(pool: deadpool_sqlite::Pool) -> Self { ... }
    pub async fn increment(&self, workflow_id: &str) -> Result<i64, Error> { ... }
    pub async fn get(&self, workflow_id: &str) -> Result<Option<i64>, Error> { ... }
    pub async fn delete_for_completed(&self) -> Result<u64, Error> { ... }
}

// lease_checkpoint.rs
pub struct LeaseRepository {
    cache: moka::future::Cache<String, LeasedCheckpointValue>,
}
impl LeaseRepository {
    pub fn new() -> Self { ... }
    pub async fn acquire(&self, workflow_id: &str, position: i64, lease_timeout_ms: i64)
        -> Result<LeasedCheckpointValue, Error> { ... }
    pub async fn get(&self, workflow_id: &str, position: i64)
        -> Option<LeasedCheckpointValue> { ... }
    pub async fn release(&self, workflow_id: &str, position: i64) { ... }
}
```

All four repository structs wrap the same `deadpool_sqlite::Pool` clone (cheap `Arc` clone).
`LeaseRepository` wraps a `moka` async cache (TTL-based, no manual sweep needed).

#### Service structs (`src/services/`)

```rust
// workflow_service.rs
pub struct WorkflowService {
    workflows: Arc<WorkflowRepository>,
    fencing_tokens: Arc<FencingTokenRepository>,
    raft: Arc<openraft::Raft<TypeConfig>>,
    local_node_id: NodeId,
}
impl WorkflowService {
    pub fn new(...) -> Self { ... }
    pub async fn create(&self, input: CreateWorkflowInput) -> Result<CreateWorkflowOutput, Error> { ... }
    pub async fn finish(&self, input: FinishWorkflowInput) -> Result<FinishWorkflowOutput, Error> { ... }
    pub async fn status(&self, workflow_id: &str) -> Result<Workflow, Error> { ... }
    pub async fn cleanup_expired(&self) -> Result<(), Error> { ... }
    // replaces client.is_leader_db()
    pub async fn is_leader(&self) -> bool {
        self.raft.current_leader().await == Some(self.local_node_id)
    }
}

// checkpoint_service.rs
pub struct CheckpointService {
    checkpoints: Arc<CheckpointRepository>,
    leases: Arc<LeaseRepository>,
    fencing_tokens: Arc<FencingTokenRepository>,
    // replaces client.lock() — per-position in-process mutex
    locks: Arc<DashMap<String, Arc<tokio::sync::Mutex<()>>>>,
}
impl CheckpointService {
    pub fn new(...) -> Self { ... }
    pub async fn checkpoint(&self, input: CheckpointInput) -> Result<CheckpointOutput, Error> { ... }
    pub async fn lease(&self, input: LeaseCheckpointInput) -> Result<LeaseCheckpointOutput, Error> { ... }
    pub async fn release(&self, workflow_id: &str, position: i64) -> Result<(), Error> { ... }
    pub async fn create_idempotency_key(&self, input: CreateDurableIdempotencyKeyInput)
        -> Result<CreateDurableIdempotencyKeyOutput, Error> { ... }
    // internal helper — acquires or creates the per-position Mutex
    async fn lock_position(&self, workflow_id: &str, position: i64) -> Arc<tokio::sync::Mutex<()>> { ... }
}
```

#### RPC handler struct (`src/rpc_server/server.rs`)

```rust
pub struct WorkflowRpcHandler {
    workflow_service: Arc<WorkflowService>,
    checkpoint_service: Arc<CheckpointService>,
}
// impl WorkflowServiceImpl (tonic-generated trait)
// Each RPC method delegates to the corresponding service method — no business logic here
```

#### Wiring in `main.rs`

```rust
let pool = deadpool_sqlite::Pool::builder(manager).build()?;

let workflow_repo    = Arc::new(WorkflowRepository::new(pool.clone()));
let checkpoint_repo  = Arc::new(CheckpointRepository::new(pool.clone()));
let fencing_repo     = Arc::new(FencingTokenRepository::new(pool.clone()));
let lease_repo       = Arc::new(LeaseRepository::new());

let workflow_svc    = Arc::new(WorkflowService::new(workflow_repo, fencing_repo.clone(), raft.clone(), node_id));
let checkpoint_svc  = Arc::new(CheckpointService::new(checkpoint_repo, lease_repo, fencing_repo));

let handler = WorkflowRpcHandler::new(workflow_svc.clone(), checkpoint_svc);
```

State machine `apply()` receives a command enum and calls the same repository methods
directly — no duplication of logic.

### Lease cache replacement

`Cache::One` (hiqlite) → `LeaseRepository` backed by a `moka::future::Cache` with per-entry TTL.
`moka` handles expiry automatically; no background sweep task needed.

### Tracing & logging

#### Fix subscriber setup (`src/main.rs`)

```rust
// before — hardcoded level
tracing_subscriber::fmt()
    .with_env_filter(EnvFilter::from("info"))
    .init();

// after — level driven by RUST_LOG env var, defaulting to info
tracing_subscriber::fmt()
    .with_target(true)
    .with_level(true)
    .with_env_filter(
        EnvFilter::try_from_env("RUST_LOG")
            .unwrap_or_else(|_| EnvFilter::new("info")),
    )
    .init();
```

#### Replace `println!` / `eprintln!` with structured macros

| File | Current | Replacement | Level |
|------|---------|-------------|-------|
| `src/main.rs:43` | `println!("Starting server")` | `info!("starting server", node_id, rpc_addr)` | `INFO` |
| `src/main.rs:105–115` | `error!("Received Ctrl+C …")` | `info!("received signal, shutting down", signal)` | `INFO` |
| `src/rpc_server/server.rs:203` | `println!("Server listening on {addr}")` | `info!(addr, "gRPC server listening")` | `INFO` |
| `src/services/workflow_service.rs:91,102` | `println!("Deleting …")` | `debug!("running expired workflow cleanup")` | `DEBUG` |
| `src/cron/clean_up_workflows.rs:17` | `eprintln!("Error …")` | `error!(err = %e, "cleanup job failed")` | `ERROR` |

#### Instrument RPC handlers (`src/rpc_server/server.rs`)

Each handler gets an `#[instrument]` span with the key business fields so every log line
emitted inside the handler automatically carries them:

```rust
#[instrument(skip(self), fields(workflow_id = %req.workflow_id))]
async fn workflow_start(&self, req: Request<WorkflowStartRequest>) -> ... { ... }

#[instrument(skip(self), fields(workflow_id = %req.workflow_id, position = req.position, fencing_token = req.fencing_token))]
async fn checkpoint(&self, req: Request<CheckPointRequest>) -> ... { ... }

#[instrument(skip(self), fields(workflow_id = %req.workflow_id, position = req.position))]
async fn lease_checkpoint(&self, req: Request<LeaseCheckpointRequest>) -> ... { ... }
```

Apply the same pattern to all 7 RPC methods.

#### Instrument service & repository methods

Because all logic lives in `impl` methods, `skip(self)` hides the pool/cache internals
while key domain fields are captured explicitly:

```rust
// services/checkpoint_service.rs
impl CheckpointService {
    #[instrument(skip(self), fields(workflow_id, position, fencing_token), err)]
    pub async fn checkpoint(&self, input: CheckpointInput) -> Result<CheckpointOutput, Error> { ... }

    #[instrument(skip(self), fields(workflow_id, position), err)]
    pub async fn lease(&self, input: LeaseCheckpointInput) -> Result<LeaseCheckpointOutput, Error> { ... }
}

// repositories/workflows.rs
impl WorkflowRepository {
    #[instrument(skip(self), fields(workflow_id = %input.workflow_id), err)]
    pub async fn create_or_get(&self, input: CreateWorkflowInput) -> Result<Workflow, Error> { ... }
}

// repositories/checkpoints.rs
impl CheckpointRepository {
    #[instrument(skip(self), fields(workflow_id, position), err)]
    pub async fn upsert(&self, input: UpsertCheckpointInput) -> Result<(), Error> { ... }
}
```

`err` on `#[instrument]` automatically emits an `ERROR` event when the method returns `Err`.

#### Log level strategy

| Layer | Level | What to log |
|-------|-------|-------------|
| RPC handlers | `INFO` | Request received, response sent (latency via span duration) |
| Services | `DEBUG` | Key decisions: fencing token expired, lease found/not found |
| Repositories | `TRACE` | SQL statements (gated so they never appear in production) |
| Raft events | `INFO` | Leader election, node join/leave |
| Cleanup cron | `DEBUG` | Number of rows deleted per run |
| Errors | `ERROR` | Any `Err` propagated to the RPC boundary |
| Shutdown signals | `INFO` | SIGTERM / SIGINT / Ctrl+C (not `error!`) |

#### openraft tracing integration

openraft emits its own `tracing` spans. Ensure the subscriber captures them by including
`openraft=info` (or `openraft=debug` when debugging elections) in `RUST_LOG`.

#### Optional: distributed tracing across nodes

If cross-node trace correlation is needed later, add `tracing-opentelemetry` +
`opentelemetry-otlp` (exports to Jaeger / Tempo). Each RPC span would carry a trace-id
that propagates through the Raft network calls, making it possible to visualise the full
path of a `checkpoint` call across leader and follower nodes. This is an opt-in addition
— the `#[instrument]` annotations above are the prerequisite.

---

## Tasks

- [ ] 1. Add `openraft`, `rocksdb`, `rusqlite`, `deadpool-sqlite`, `moka`, `dashmap` to `Cargo.toml`; remove `hiqlite`, `hiqlite-macros`
- [ ] 2. Implement `struct LogStore(Arc<rocksdb::DB>)` with `impl RaftLogStore` (`src/storage/log_store.rs`)
- [ ] 3. Implement `struct StateMachine(deadpool_sqlite::Pool)` with `impl RaftStateMachine` (`src/storage/state_machine.rs`); `apply()` delegates to repository methods
- [ ] 4. Implement `struct Network` with `impl RaftNetwork` for inter-node gRPC (`src/storage/network.rs`)
- [ ] 5. Implement `struct RaftNode` with init logic in `src/database/db.rs`; expose `Arc<RaftNode>` to the rest of the app
- [ ] 6. Implement `struct WorkflowRepository` with all methods in `src/repositories/workflows.rs`
- [ ] 7. Implement `struct CheckpointRepository` with all methods in `src/repositories/checkpoints.rs`
- [ ] 8. Implement `struct FencingTokenRepository` with all methods in `src/repositories/workflows_fencing_tokens.rs`
- [ ] 9. Implement `struct LeaseRepository` backed by `moka::future::Cache` in `src/repositories/lease_checkpoint.rs`
- [ ] 10. Implement `struct WorkflowService` with methods (including `is_leader()`) in `src/services/workflow_service.rs`; inject `WorkflowRepository`, `FencingTokenRepository`, `Arc<Raft>`
- [ ] 11. Implement `struct CheckpointService` with methods (including `lock_position()`) in `src/services/checkpoint_service.rs`; inject `CheckpointRepository`, `LeaseRepository`, `FencingTokenRepository`
- [ ] 12. Rewrite `struct WorkflowRpcHandler` in `src/rpc_server/server.rs` to hold `Arc<WorkflowService>` + `Arc<CheckpointService>`; each RPC method delegates to the corresponding service method
- [ ] 13. Update `src/main.rs`: construct pool → repositories → services → handler in dependency order; remove `hiqlite` init
- [ ] 14. Replace `hiqlite.toml` with openraft cluster config (TOML or env-based)
- [ ] 15. Fix tracing subscriber: replace hardcoded `EnvFilter::from("info")` with `EnvFilter::try_from_env("RUST_LOG")` in `src/main.rs`
- [ ] 16. Replace all `println!` / `eprintln!` with structured `tracing` macros; fix shutdown signal log level from `error!` to `info!`
- [ ] 17. Add `#[instrument(skip(self), ..., err)]` to all 7 RPC handler methods, all `WorkflowService` methods, all `CheckpointService` methods, and all repository methods
- [ ] 18. Run full integration test suite; validate idempotency protocol end-to-end

---

## Testing

| Scenario | Command / File |
|----------|---------------|
| Unit tests | `cargo test` |
| Integration (3-node cluster) | `docker-compose up` then BDD test suite |
| Idempotency correctness | Existing BDD tests must pass unchanged |
| Concurrent write throughput | Benchmark `checkpoint` RPC under N parallel workers |
| Leader failover | Kill leader mid-workflow; verify takeover with no duplicate execution |
| Tracing output | `RUST_LOG=debug cargo run` — confirm `workflow_id`/`position` fields appear in every relevant log line; no `println!` output visible |
| Log levels | `RUST_LOG=trace` — confirm SQL statements visible; `RUST_LOG=info` — confirm only RPC lifecycle and errors visible |

---

## Risks & Open Questions

- **Risk:** openraft `RaftStorage` traits have a non-trivial API surface.  
  **Mitigation:** Start from openraft's official `rocksstore` example.

- **Risk:** RocksDB C++ FFI requires cmake; adds CI complexity.  
  **Mitigation:** Evaluate `fjall` as a drop-in pure-Rust alternative for the log store.

- **Open question:** Should the SQLite state machine be Raft-replicated (each node applies the same commands) or should each node maintain a local SQLite and rebuild from the Raft log on restart?  
  Local-only = simpler code, but requires full log replay after crashes unless snapshots are implemented.

- **Risk:** Distributed lock semantics differ; `lease_checkpoint` race conditions must be re-validated after replacing `client.lock()`.

---

## Out-of-scope follow-ups

- TLS for Raft inter-node communication (currently disabled in hiqlite.toml)
- Prometheus metrics for Raft log throughput and apply latency (prerequisite: `#[instrument]` spans from task 16)
- RocksDB compaction / snapshot strategy to bound log size
- Distributed trace propagation across nodes via `tracing-opentelemetry` + OTLP exporter (Jaeger / Tempo)
