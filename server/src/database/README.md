# database/ — Raft node initialisation

## Files

### `db.rs` — `RaftNode`

```rust
pub struct RaftNode {
    pub raft:  Raft<TypeConfig>,
    pub store: SharedStore,   // in-memory application state — shared with repositories
}
```

**`start(node_id, peers, data_dir) -> anyhow::Result<Arc<Self>>`**

Sequence:
1. `LogStore::open("{data_dir}/raft-log")` — fjall keyspace with partitions `logs` + `meta`
2. `StateMachine::new("{data_dir}/state")` — initialises in-memory store, restoring from `snapshot.bin` if present; returns `(StateMachine, SharedStore)`
3. `Raft::new(node_id, config, NetworkFactory::new(), log_store, state_machine)`
4. `raft.initialize(members)` — single-node bootstrap if `peers` is empty; multi-node otherwise (first call wins, `AlreadyInitialized` silently ignored)
5. Returns `Arc<RaftNode>`

`store` is cloned (cheap `Arc` clone) into repository structs in `main.rs`.
`StateMachine` also holds the same `SharedStore`.

### `server.rs` — `Server`

```rust
pub struct Server { pub id: u64, pub addr_api: String, pub addr_raft: String }

impl Server {
    pub fn parse(s: &str) -> Result<Self, Box<dyn Error>>
    // Parses "id addr_api addr_raft" (3 whitespace-separated fields)
}
```

Used in `main.rs` to parse each `--node "id addr_api addr_raft"` CLI argument.
Only `addr_raft` is forwarded into the `Node` struct; `addr_api` is parsed but unused by the server.

## Dependencies

- `storage::log_store::LogStore`
- `storage::state_machine::{StateMachine, SharedStore}`
- `storage::network::NetworkFactory`
- `storage::type_config::{TypeConfig, NodeId, Node}`
- `openraft::{Config, Raft}`
- `fjall` — log store persistence
- `anyhow::Context` — `.context("...")` on error propagation
