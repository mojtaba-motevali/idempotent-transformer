# database/ — Raft node initialisation

## Files

### `db.rs` — `RaftNode`

```rust
pub struct RaftNode {
    pub raft: Raft<TypeConfig>,   // openraft handle
    pub node_id: NodeId,
    pub db: Arc<rocksdb::DB>,    // state machine DB — shared with repositories in main.rs
}
```

**`start(node_id, peers, data_dir) -> anyhow::Result<Arc<Self>>`**

Sequence:
1. `LogStore::open("{data_dir}/raft-log")` — RocksDB with CFs `logs` + `meta`
2. `open_state_db("{data_dir}/state")` — RocksDB with CFs `workflows` + `fencing_tokens` + `checkpoints` + `raft_meta`
3. `StateMachine::new(db.clone())` — loads persisted `last_applied` / `last_membership`
4. `Raft::new(node_id, config, NetworkFactory::new(), log_store, state_machine)`
5. `raft.initialize(members)` — single-node bootstrap if `peers` is empty; multi-node otherwise (first call wins, `AlreadyInitialized` silently ignored)
6. Returns `Arc<RaftNode>`

**`is_leader() -> bool`**: `raft.current_leader().await == Some(self.node_id)`

`db` is cloned (cheap `Arc` clone) into repository structs in `main.rs`.
`StateMachine` also holds a clone of the same `Arc<DB>`.

### `server.rs` — `Server`

```rust
pub struct Server { pub id: u64, pub addr_api: String, pub addr_raft: String }

impl Server {
    pub fn parse(s: &str) -> Result<Self, Box<dyn Error>>
    // Parses "id addr_api addr_raft" (3 whitespace-separated fields)
}
```

Used in `main.rs` to parse each `--node "id addr_api addr_raft"` CLI argument.

## Dependencies

- `storage::log_store::LogStore`
- `storage::state_machine::{open_state_db, StateMachine}`
- `storage::network::NetworkFactory`
- `storage::type_config::{TypeConfig, NodeId, Node}`
- `openraft::{Config, Raft}`
- `rocksdb::DB`
- `anyhow::Context` — `.context("...")` on error propagation
