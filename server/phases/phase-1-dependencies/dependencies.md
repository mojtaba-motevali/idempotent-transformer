# Phase 1: Dependencies & Configuration

## Files to modify

- `Cargo.toml`
- `hiqlite.toml` → delete; replace with `openraft.toml` (or env vars)

---

## Cargo.toml changes

### Remove

```toml
hiqlite = { version = "0.10", features = ["full", "jemalloc"] }
hiqlite-macros = "0.10.0"
```

### Add

```toml
# Raft consensus
openraft = { version = "0.9", features = ["serde"] }

# Raft log store
rocksdb = "0.22"

# State machine / repository storage
rusqlite = { version = "0.31", features = ["bundled", "chrono"] }
deadpool-sqlite = "0.9"

# Lease cache (TTL, async, no manual sweep)
moka = { version = "0.12", features = ["future"] }

# Per-position in-process locks (replaces client.lock())
dashmap = "6"

# FlatBuffers runtime (Raft network serialization)
flatbuffers = "24"

# JSON — only for openraft Membership serialization inside MembershipPayload
serde_json = "1"

# Async trait support (required by RaftNetwork impl)
async-trait = "0.1"
```

### Also add

```toml
# Typed error definitions at service / repository layers (m06-error-handling skill)
thiserror = "1"

# Ergonomic error handling at the application boundary (RPC handler, main.rs)
anyhow = "1"

# Faster Mutex/RwLock for sync (non-async) contexts (coding-guidelines skill)
parking_lot = "0.12"
```

### Also add to `[build-dependencies]`

```toml
[build-dependencies]
tonic-build = "0.9"        # existing
flatbuffers-build = "0.4"  # compiles .fbs schema → Rust via flatc
```

### Also add (CLI argument parsing)

```toml
clap = { version = "4", features = ["derive"] }
```

### Remove

```toml
dotenvy = "0.15.7"   # replaced by clap — args come from CLI, not .env
```

Keep existing deps unchanged:
- `serde`, `chrono`, `tokio`, `prost`, `tonic`, `tracing`, `tracing-subscriber`,
  `tokio-cron-scheduler`, `strum`, `uuid`

### `build.rs` update

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // existing: gRPC for client-facing API
    tonic_build::configure()
        .build_server(true)
        .compile(&["proto/workflow_service.proto"], &["proto"])?;

    // new: FlatBuffers schema for Raft inter-node TCP messages
    flatbuffers_build::BuilderOptions::new_with_files(["flatbuffers/raft_network.fbs"])
        .compile()?;

    Ok(())
}
```

> `flatbuffers-build` requires `flatc` on `PATH`. Install via OS package manager
> (`apt install flatbuffers-compiler`, `brew install flatbuffers`) or download from
> https://github.com/google/flatbuffers/releases. Add to CI as a prerequisite.

---

## openraft TypeConfig

Create `src/storage/type_config.rs` — referenced by all subsequent phases:

```rust
use openraft::declare_raft_types;
use serde::{Deserialize, Serialize};

pub type NodeId = u64;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub addr_raft: String,
    pub addr_api: String,
}

declare_raft_types!(
    pub TypeConfig:
        D         = WorkflowCommand,
        R         = WorkflowResponse,
        NodeId    = NodeId,
        Node      = Node,
        Entry     = openraft::Entry<TypeConfig>,
        SnapshotData = Vec<u8>,
);
```

Create `src/storage/command.rs`:

```rust
use serde::{Deserialize, Serialize};

/// Every write operation that must go through Raft consensus.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowCommand {
    CreateOrGetWorkflow {
        workflow_id: String,
        name: Option<String>,
        created_at: i64,
    },
    IncrementFencingToken {
        workflow_id: String,
    },
    UpsertCheckpoint {
        workflow_id: String,
        position: i64,
        idempotency_key: String,
        value: Option<Vec<u8>>,
        created_at: i64,
    },
    CompleteWorkflow {
        workflow_id: String,
        expire_at: i64,
        completed_at: i64,
    },
    DeleteExpired {
        before_ms: i64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowResponse {
    Workflow { id: String, fencing_token: i64 },
    FencingToken(i64),
    Empty,
}
```

---

## Shared error types (`src/error.rs`)

Create this file once in Phase 1. All subsequent phases import from it.
Follows the **m06-error-handling** and **m13-domain-error** skill patterns:
typed errors with `thiserror` in library/service code; `anyhow` only at the RPC boundary.

```rust
use thiserror::Error;

// ── Repository layer ───────────────────────────────────────────────────────

/// All database and connection-pool failures. Returned by repository methods.
#[derive(Error, Debug)]
pub enum RepositoryError {
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("connection pool error: {0}")]
    Pool(#[from] deadpool_sqlite::PoolError),

    #[error("interact error: {0}")]
    Interact(#[from] deadpool_sqlite::InteractError),
}

// ── Service layer ──────────────────────────────────────────────────────────

/// Domain failures for workflow lifecycle operations.
#[derive(Error, Debug)]
pub enum WorkflowError {
    #[error("workflow not found: {id}")]
    NotFound { id: String },

    #[error("fencing token not found for workflow: {workflow_id}")]
    FencingTokenNotFound { workflow_id: String },

    #[error("fencing token expired (stored: {stored}, received: {received})")]
    FencingTokenExpired { stored: i64, received: i64 },

    #[error("raft error: {0}")]
    Raft(String),

    #[error(transparent)]
    Repository(#[from] RepositoryError),
}

/// Domain failures for checkpoint and lease operations.
#[derive(Error, Debug)]
pub enum CheckpointError {
    #[error("fencing token not found for workflow: {workflow_id}")]
    FencingTokenNotFound { workflow_id: String },

    #[error("fencing token expired")]
    FencingTokenExpired,

    #[error("non-deterministic checkpoint: position {position} already has a different idempotency key")]
    NonDeterministicCheckpoint { position: i64 },

    #[error("checkpoint value not found at position {position}")]
    CheckpointValueNotFound { position: i64 },

    #[error(transparent)]
    Repository(#[from] RepositoryError),
}

// ── Network layer ──────────────────────────────────────────────────────────

/// TCP transport and FlatBuffers serialization failures.
#[derive(Error, Debug)]
pub enum NetworkError {
    #[error("TCP connect failed to {addr}: {source}")]
    Connect { addr: String, #[source] source: std::io::Error },

    #[error("TCP I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("FlatBuffers serialize error: {0}")]
    Serialize(String),

    #[error("FlatBuffers deserialize error: {0}")]
    Deserialize(String),

    #[error("unexpected message type: {0:#x}")]
    InvalidMessageType(u8),
}
```

> **Why not `Box<dyn Error>` in service/repo code?**
> Per m06 skill: `Box<dyn Error>` loses type information, preventing callers from
> matching on error variants. The RPC handler needs to distinguish
> `FencingTokenExpired` (→ `Status::aborted`) from `NotFound` (→ `Status::not_found`).
> `Box<dyn Error>` would require fragile string matching or `downcast_ref`.

---

## Cluster configuration

Delete `hiqlite.toml`. Cluster membership is passed as CLI flags (parsed by `clap` in `main.rs`):

| CLI flag | Type | Purpose |
|----------|------|---------|
| `--node-id` | `u64` | This node's id |
| `--addr-raft` | `String` | Raft TCP address for this node (e.g. `0.0.0.0:9001`) |
| `--addr-api` | `String` | API address for this node |
| `--node` | `String` (repeatable) | One cluster member: `"id addr_api addr_raft"` |
| `--data-dir` | `String` | Root dir for RocksDB and SQLite files |
| `--worker-threads` | `usize` | Tokio worker thread count (default `4`) |
| `--stack-size` | `usize` | Thread stack size in bytes (default `8388608`) |

Example invocation:

```bash
idempotency-server \
  --node-id 1 \
  --addr-api 127.0.0.1:8100 \
  --addr-raft 127.0.0.1:8200 \
  --rpc-addr 127.0.0.1:3000 \
  --node "1 127.0.0.1:8100 127.0.0.1:8200" \
  --node "2 127.0.0.1:8101 127.0.0.1:8201" \
  --node "3 127.0.0.1:8102 127.0.0.1:8202" \
  --data-dir /var/lib/idempotency
```

No config file needed — `src/database/db.rs` (Phase 2) receives the parsed values directly from `main.rs`.

---

## Verification

```bash
cargo check   # must compile with zero hiqlite references
```
