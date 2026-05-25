# Phase 2 — Storage Layer

**Status: DONE**

## What was done

All four storage components implemented and compiling.

## Components

### LogStore (`src/storage/log_store.rs`)
- `struct LogStore { db: Arc<rocksdb::DB> }`
- Two RocksDB column families: `logs` (u64 BE key → JSON entry), `meta` (vote, committed, purged)
- Implements `RaftLogStorage<TypeConfig>` + `RaftLogReader<TypeConfig>`
- `LogStoreReader` exposes a cloned `Arc<DB>` for concurrent reads

### StateMachine (`src/storage/state_machine.rs`)
- `struct StateMachine { pool: deadpool_sqlite::Pool, last_applied: Arc<Mutex<Option<LogId<u64>>>>, last_membership: Arc<Mutex<StoredMembership<u64, Node>>> }`
- `new()`: runs `migrations/1_init.sql` + creates `RaftMeta` table; loads persisted `last_applied` + `last_membership`
- `apply()`: dispatches `WorkflowCommand` variants to repository methods; persists `last_applied` after each batch
- `NoopSnapshotBuilder` — snapshots not implemented (log replay on restart instead)
- Uses `tokio::sync::Mutex` (not `std::sync::Mutex`) — guards held across `.await`

### Network (`src/storage/network.rs`, `network_codec.rs`, `framing.rs`, `tcp_server.rs`)
- `NetworkFactory` implements `RaftNetworkFactory<TypeConfig>`
- `PeerConn`: persistent TCP connection per peer, lazy connect, auto-reconnect on error
  - `tokio::sync::Mutex<Option<TcpStream>>` — async-safe guard
- Wire format: 1-byte type discriminant + JSON body (FlatBuffers schema exists, codegen optional)
- `TcpRaftServer`: accepts incoming peer connections, dispatches to openraft

### RaftNode (`src/database/db.rs`)
- `struct RaftNode { raft: Raft<TypeConfig>, node_id: NodeId, pool: deadpool_sqlite::Pool }`
- `start(node_id, peers, data_dir) -> anyhow::Result<Self>`:
  1. Open RocksDB at `{data_dir}/raft-log`
  2. Open SQLite pool at `{data_dir}/state.db`
  3. Init `StateMachine` (runs migrations)
  4. Start `openraft::Raft`
  5. Bootstrap (single-node dev) or join cluster (multi-node)
- `is_leader() -> bool`: `raft.current_leader().await == Some(self.node_id)`
- `pool` is cloned into repository structs in `main.rs`

## Key design decisions

- **Mutex type**: Always `tokio::sync::Mutex` where a guard crosses `.await`. `parking_lot::Mutex` only for sync contexts.
- **`??` pattern**: `interact(|conn| {...})?.await?.map_err(RepositoryError::Sqlite)` — the first `?` unwraps `InteractError`, the second `?` unwraps `rusqlite::Error`.
- **`sm_err()` helper**: Wraps any `std::error::Error` into `StorageError::IO { source: StorageIOError::write_state_machine(...) }`.
- **last_applied persistence**: Prevents re-applying entries on restart. Written to `RaftMeta` table after every `apply()` batch.

## Verification

```bash
cargo check -p idempotency-server
# RaftNode::start must compile; Arc<RaftNode> must be Send + Sync
```

## Detail docs

- `log-store.md` — RocksDB column family layout, entry encoding
- `state-machine.md` — apply() dispatch, RaftMeta persistence, migration sequence
- `network.md` — PeerConn lifecycle, TCP framing, codec discriminant table
- `raft-node.md` — start() sequence, bootstrap logic
