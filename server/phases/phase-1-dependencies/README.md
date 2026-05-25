# Phase 1 — Dependencies & Configuration

**Status: DONE**

## What was done

- Removed `hiqlite`, `hiqlite-macros`, `dotenvy` from `Cargo.toml`
- Added: `openraft 0.9 (serde)`, `rocksdb 0.22`, `rusqlite 0.32 (bundled)`,
  `deadpool-sqlite 0.9`, `moka 0.12 (future)`, `dashmap 6`, `flatbuffers 24`,
  `serde_json 1`, `async-trait 0.1`, `thiserror 1`, `anyhow 1`, `parking_lot 0.12`,
  `clap 4 (derive)`
- `build.rs`: compiles proto (tonic-build) + optional FlatBuffers (falls back to empty module if `flatc` absent)
- Deleted `hiqlite.toml` — cluster config is now CLI flags via `clap`
- Created `src/storage/type_config.rs` — `TypeConfig`, `NodeId`, `Node`
- Created `src/storage/command.rs` — `WorkflowCommand`, `WorkflowResponse`
- Created `src/error.rs` — `RepositoryError`, `WorkflowError`, `CheckpointError`, `NetworkError`
- Created `src/cli.rs` — `Args` struct (node_id, addr_raft, rpc_addr, data_dir, nodes, worker_threads, stack_size)

## Key files

| File | Role |
|------|------|
| `Cargo.toml` | Dependency manifest |
| `build.rs` | Proto + FlatBuffers codegen |
| `src/error.rs` | All typed error enums |
| `src/cli.rs` | CLI argument struct |
| `src/storage/type_config.rs` | openraft type parameters |
| `src/storage/command.rs` | Raft command/response enums |

## Error hierarchy

```
anyhow::Error          ← application boundary only (main, start_server, RaftNode::start)
WorkflowError          ← workflow service methods
  └─ RepositoryError   ← via #[from]
CheckpointError        ← checkpoint service methods
  └─ RepositoryError   ← via #[from]
NetworkError           ← TCP network layer
RepositoryError        ← all repository methods
  ├─ Sqlite(rusqlite::Error)
  ├─ Pool(deadpool_sqlite::PoolError)
  └─ Interact(deadpool_sqlite::InteractError)
```

## Verification

```bash
cargo check   # zero errors, zero hiqlite references
grep -r "hiqlite" src/ Cargo.toml  # must return nothing
```

## Detail

See `dependencies.md` for the full Cargo.toml diff and error type definitions.
