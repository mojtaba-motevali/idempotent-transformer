# Phase 2 — RaftNode

## File

`src/database/db.rs` (full rewrite)

## Purpose

Replaces the `get_client()` + `init_tables()` functions with a `RaftNode` struct that owns
the openraft handle, log store, and state machine. The rest of the application receives an
`Arc<RaftNode>` and interacts with it through methods only.

---

## Struct

```rust
use std::sync::Arc;
use anyhow::Context as _;
use openraft::{Config, Raft};
use crate::storage::{
    log_store::LogStore,
    state_machine::StateMachine,
    network::NetworkFactory,
    type_config::{TypeConfig, NodeId, Node},
};

pub struct RaftNode {
    pub raft: Raft<TypeConfig>,
    pub node_id: NodeId,
    /// Exposed so repository structs (Phase 3) can share the same SQLite pool.
    pub state_machine: Arc<StateMachine>,
}

impl RaftNode {
    /// Initialises RocksDB, SQLite pool, openraft, and runs DB migrations.
    /// Returns `anyhow::Error` — this is init code called only from `main.rs`.
    pub async fn start(
        node_id: NodeId,
        peers: Vec<(NodeId, Node)>,
        data_dir: &str,
    ) -> anyhow::Result<Arc<Self>> {
        // 1. Open RocksDB for the log store
        let log_path = format!("{data_dir}/raft-log");
        let log_store = Arc::new(LogStore::open(&log_path)?);

        // 2. Open SQLite pool for the state machine
        let sqlite_path = format!("{data_dir}/state.db");
        let manager = deadpool_sqlite::Manager::new(
            &sqlite_path,
            deadpool_sqlite::Runtime::Tokio1,
        );
        let pool = deadpool_sqlite::Pool::builder(manager)
            .max_size(16)
            .build()?;
        let state_machine = Arc::new(StateMachine::new(pool).await?);

        // 3. Configure and start openraft
        let config = Arc::new(
            Config::default()
                .validate()
                .expect("invalid openraft config"),
        );
        let network = NetworkFactory;
        let raft = Raft::new(
            node_id,
            config,
            network,
            log_store.clone(),
            state_machine.clone(),
        )
        .await?;

        // 4. Bootstrap single-node or join cluster
        if peers.is_empty() {
            // single-node dev/test mode
            let members = std::collections::BTreeMap::from([(
                node_id,
                Node { addr_raft: String::new(), addr_api: String::new() },
            )]);
            raft.initialize(members).await?;
        } else {
            // multi-node: peers drives cluster membership
            // Each node attempts initialize(); only the first to win matters.
            let members: std::collections::BTreeMap<_, _> = peers.into_iter().collect();
            let _ = raft.initialize(members).await; // ignore AlreadyInitialized
        }

        Ok(Arc::new(Self { raft, node_id, state_machine }))
    }

    /// True if this node is currently the Raft leader.
    pub async fn is_leader(&self) -> bool {
        self.raft.current_leader().await == Some(self.node_id)
    }
}
```

---

## Migrations

Run existing `migrations/1_init.sql` via `rusqlite` directly inside `StateMachine::new()`:

```rust
let conn = pool.get().await?;
conn.interact(|conn| {
    conn.execute_batch(include_str!("../../migrations/1_init.sql"))
}).await??;
```

Also run the `RaftMeta` table DDL inline (see `state-machine.md`).

---

## Environment variables read here

| Var | Usage |
|-----|-------|
| `NODE_ID` | `node_id: u64` |
| `NODES` | parsed into `Vec<(NodeId, Node)>` peers |
| `DATA_DIR` | root dir for RocksDB + SQLite |

Parsing already exists in `src/database/server.rs` — reuse `Server::parse()`.

---

## What is removed

- `start_node_with_cache::<Cache>()` call
- `Cache` enum and `CacheIndex` impl
- `client.wait_until_healthy_cache()` / `wait_until_healthy_db()`
- `client.migrate::<Migrations>()`
- `PRAGMA journal_mode = WAL` / `PRAGMA synchronous = NORMAL`
  (set these on each SQLite connection in the pool builder instead)

---

## Verification

```bash
cargo check -p idempotency-server
# RaftNode::start must compile; Arc<RaftNode> must be Send + Sync
```
