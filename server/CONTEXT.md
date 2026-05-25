# Codebase Context

> Quick-load reference for a fresh session. Read this instead of PLAN.md when you need to understand the current state.

## What this project is

A distributed **idempotent workflow engine** — a gRPC server that lets callers execute
multi-step workflows exactly once, even under retries and concurrent workers. It uses:

- **openraft 0.9** for Raft consensus (log replication across a 3-node cluster)
- **RocksDB (log store)** — Raft log entries, vote, committed state (`{data_dir}/raft-log`)
- **RocksDB (state machine)** — workflow/checkpoint/fencing data (`{data_dir}/state`)
- **Tonic** for the gRPC API
- **moka** for in-memory TTL lease cache
- **DashMap + tokio::sync::Mutex** for per-position distributed locks

## Migration status: DONE (Phases 1–6 complete)

The project was migrated from **Hiqlite** (SQLite + opaque Raft bundle) to
openraft + RocksDB log + SQLite state machine. All code compiles (`cargo check` clean).
Phase 7 (integration tests) is the only remaining work.

## Source file map

```
src/
  cli.rs                           Args struct (clap) — parsed by main.rs
  main.rs                          Tokio runtime, wiring: RaftNode → repos → services → handler
  error.rs                         RepositoryError, WorkflowError, CheckpointError, NetworkError

  database/
    db.rs                          RaftNode::start() — opens RocksDB+SQLite, inits openraft
    server.rs                      Server::parse() — parses "id addr_api addr_raft" strings

  storage/
    type_config.rs                 TypeConfig, NodeId, Node (openraft type parameters)
    command.rs                     WorkflowCommand enum, WorkflowResponse enum
    log_store.rs                   LogStore: impl RaftLogStorage (RocksDB, CFs: logs+meta)
    state_machine.rs               StateMachine: impl RaftStateMachine (SQLite pool)
                                     → apply() dispatches WorkflowCommand to repositories
                                     → persists last_applied/last_membership to RaftMeta table
    network.rs                     NetworkFactory + PeerConn: TCP connections to Raft peers
    network_codec.rs               Encode/decode Raft messages: 1-byte type + JSON body
    framing.rs                     Length-prefixed TCP framing
    tcp_server.rs                  TcpRaftServer: accepts incoming Raft peer connections

  repositories/
    workflows.rs                   WorkflowRepository: create_or_get, get_by_id, complete, delete_expired
    checkpoints.rs                 CheckpointRepository: upsert, get, delete_for_completed
    workflows_fencing_tokens.rs    FencingTokenRepository: increment, get, delete_for_completed
    lease_checkpoint.rs            LeaseRepository: acquire, get, release (moka TTL cache)

  services/
    workflow_service.rs            WorkflowService: create, finish, status, cleanup_expired, is_leader
    checkpoint_service.rs          CheckpointService: checkpoint, lease, release, create_idempotency_key

  rpc_server/
    server.rs                      WorkflowRpcHandler: 7 gRPC RPCs, typed error mappers
                                   start_server() → anyhow::Result<()>

  schema/
    workflow.rs                    Workflow struct, WorkflowStatus enum
    checkpoint.rs                  Checkpoint, CheckpointValue
    leased_checkpoint.rs           LeasedCheckpointValue, LeasedCheckpoint
    workflow_fencing_token.rs      WorkflowFencingToken

  cron/
    clean_up_workflows.rs          10s cron (leader-only): deletes expired workflows via Raft

  helpers/
    common.rs                      return_error_if_true() [UNUSED — kept for compat]

migrations/
  1_init.sql                       Workflows, WorkflowFencingTokens, Checkpoints tables + index

proto/
  workflow_service.proto           7-endpoint gRPC definition (UNCHANGED from hiqlite version)

flatbuffers/
  raft_network.fbs                 FlatBuffers schema (codegen skipped if flatc absent)
```

## Key types

| Type | Location | Role |
|------|----------|------|
| `TypeConfig` | `src/storage/type_config.rs` | openraft type parameter bundle |
| `NodeId = u64` | `src/storage/type_config.rs` | Raft node identity |
| `WorkflowCommand` | `src/storage/command.rs` | All Raft-replicated write operations |
| `WorkflowResponse` | `src/storage/command.rs` | Reply from state machine apply() |
| `RaftNode` | `src/database/db.rs` | Owns `Raft<TypeConfig>` + SQLite pool |
| `RepositoryError` | `src/error.rs` | DB/pool failures (thiserror) |
| `WorkflowError` | `src/error.rs` | Workflow domain failures (thiserror) |
| `CheckpointError` | `src/error.rs` | Checkpoint/lease domain failures (thiserror) |
| `NetworkError` | `src/error.rs` | TCP/serialization failures (thiserror) |

## Data flow: write path

```
gRPC client
  → WorkflowRpcHandler (src/rpc_server/server.rs)
  → WorkflowService / CheckpointService (src/services/)
  → raft.client_write(WorkflowCommand)          ← goes through Raft log
  → StateMachine::apply() (src/storage/state_machine.rs)
  → WorkflowRepository / CheckpointRepository / FencingTokenRepository (src/repositories/)
  → SQLite via deadpool-sqlite
```

## Data flow: read path

```
gRPC client
  → WorkflowRpcHandler
  → WorkflowService::status()  or  CheckpointService::lease()
  → Repository::get_by_id() / LeaseRepository::get()
  → SQLite directly (no Raft round-trip)
```

## Concurrency design

- **Fencing tokens**: atomic SQL upsert per workflow, Raft-replicated → monotonic
- **Checkpoints**: idempotent on replay (`ON CONFLICT DO UPDATE`), Raft-replicated
- **Leases**: `moka::future::Cache` with per-entry TTL, NOT Raft-replicated (in-memory only)
- **Per-position lock**: `DashMap<String, Arc<tokio::sync::Mutex<()>>>` in CheckpointService
- **Leader check**: `raft.current_leader().await == Some(local_node_id)`
- **Cleanup cron**: leader-only, 10 s interval, sends `WorkflowCommand::DeleteExpired` via Raft

## Error handling pattern

```
Application boundary (main.rs, start_server)  → anyhow::Result
RPC handler (server.rs)                        → typed error mappers → tonic::Status
Service layer                                  → WorkflowError / CheckpointError (thiserror)
Repository layer                               → RepositoryError (thiserror)
Storage layer (state_machine, log_store)       → openraft::StorageError<u64>
Network layer                                  → NetworkError (thiserror)
```

## gRPC API (7 RPCs — proto unchanged)

| RPC | Key inputs | Key output |
|-----|-----------|------------|
| `workflow_start` | workflow_id, context_name? | fencing_token |
| `generate_idempotency_key` | workflow_id, fencing_token, position | idempotency_key |
| `checkpoint` | workflow_id, value, fencing_token, position, idempotency_key | abort (bool) |
| `lease_checkpoint` | workflow_id, fencing_token, lease_timeout, position, idempotency_key | value OR remaining_timeout |
| `complete_workflow` | workflow_id, fencing_token, expire_after | — |
| `workflow_status` | workflow_id | status, timestamps |
| `release_checkpoint` | workflow_id, position | — |

## What still needs to be done

**Phase 7 — Testing** (see `phases/phase-7-testing/`):
- Unit tests per repository and service method
- Integration tests (single-node RaftNode in test harness)
- BDD tests (3-node docker-compose cluster)
- Throughput benchmark vs hiqlite baseline
- Leader failover test

## RocksDB schema (state machine DB at `{data_dir}/state`)

| Column family | Key | Value |
|---------------|-----|-------|
| `workflows` | `workflow_id` UTF-8 bytes | serde_json `Workflow` |
| `fencing_tokens` | `workflow_id` UTF-8 bytes | 8-byte big-endian `i64` |
| `checkpoints` | `{workflow_id}\x00{position 8-byte BE}` | serde_json `Checkpoint` |
| `raft_meta` | key string bytes | serde_json (last_applied, last_membership) |

Checkpoint key design: null-byte separator allows prefix scanning all checkpoints for a workflow.

Note: `migrations/1_init.sql` is kept as documentation but is no longer executed.

## Cluster startup

```bash
cargo build --release
./target/release/idempotency-server \
  --node-id 1 --addr-raft 0.0.0.0:9001 --rpc-addr 0.0.0.0:50051 \
  --data-dir /var/lib/idempotency \
  --node "1 127.0.0.1:50051 127.0.0.1:9001" \
  --node "2 127.0.0.1:50052 127.0.0.1:9002" \
  --node "3 127.0.0.1:50053 127.0.0.1:9003"

RUST_LOG=info,openraft=info  # production
RUST_LOG=debug               # debugging
```
