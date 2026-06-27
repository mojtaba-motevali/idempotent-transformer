# Idempotency Server — Rust gRPC Engine

Distributed, Raft-replicated idempotency server. Clients call 7 gRPC RPCs to start workflows, checkpoint intermediate state, and mark completion.

## Key dependencies

| Crate | Role |
|-------|------|
| `openraft 0.9` | Raft consensus (storage-v2 API) |
| `fjall 2` | Raft log store (on-disk, LSM-tree) |
| `tonic 0.9` | gRPC server + generated client stubs |
| `moka 0.12` | In-memory TTL lease cache |
| `dashmap 6` | Per-checkpoint-position async locks |
| `thiserror` / `anyhow` | Typed domain errors / application boundary errors |
| `clap 4` | CLI argument parsing |

## Source layout

```
src/
  cli.rs                     Args struct (clap) — parsed by main.rs
  main.rs                    Tokio runtime entry: RaftNode → repos → services → gRPC handler
  error.rs                   RepositoryError, WorkflowError, CheckpointError, NetworkError

  database/
    db.rs                    RaftNode::start() — opens fjall + initialises openraft
    server.rs                Server::parse() — "id addr_api addr_raft" string parser

  storage/
    type_config.rs           TypeConfig, NodeId=u64, Node (openraft type parameters)
    command.rs               WorkflowCommand enum, WorkflowResponse enum (Raft payloads)
    log_store.rs             LogStore: RaftLogStorage impl (fjall, partitions: logs + meta)
    state_machine.rs         StateMachine: RaftStateMachine impl (pure in-memory HashMap/BTreeMap)
                               → apply() dispatches WorkflowCommand to repositories
    network.rs               NetworkFactory + PeerConn: TCP connections to Raft peers
    network_codec.rs         Raft message framing: 1-byte type tag + rkyv body
    rkyv_dto.rs              rkyv DTOs mirroring openraft wire/log types + From conversions
    framing.rs               Length-prefixed TCP framing utilities
    tcp_server.rs            TcpRaftServer: accepts incoming Raft peer connections

  repositories/
    workflows.rs             WorkflowRepository: create_or_get, get_by_id, complete, delete_expired
    checkpoints.rs           CheckpointRepository: upsert, get, delete_for_completed
    workflows_fencing_tokens.rs FencingTokenRepository: increment, get, delete_for_completed
    lease_checkpoint.rs      LeaseRepository: acquire, get, release (moka TTL cache)

  services/
    workflow_service.rs      WorkflowService: create, finish, status, cleanup_expired, is_leader
    checkpoint_service.rs    CheckpointService: checkpoint, lease, release, create_idempotency_key

  rpc_server/
    server.rs                WorkflowRpcHandler: 7 gRPC RPCs + typed error → tonic::Status mappers

  schema/
    workflow.rs              Workflow struct, WorkflowStatus enum
    checkpoint.rs            Checkpoint, CheckpointValue
    leased_checkpoint.rs     LeasedCheckpointValue, LeasedCheckpoint
    workflow_fencing_token.rs WorkflowFencingToken

  cron/
    clean_up_workflows.rs    10 s leader-only cron: sends WorkflowCommand::DeleteExpired via Raft

proto/
  workflow_service.proto     gRPC service definition (7 RPCs)

tests/e2e/
  helper.rs                  ServerFixture — starts a full in-process single-node cluster on a random port
  main.rs                    Test module root
  checkpoint.rs / idempotency_key.rs / workflow_start.rs / workflow_status.rs  — test files
```

## Storage architecture

- **Raft log** (`fjall`): durable on-disk LSM-tree, partitions `logs` + `meta`
- **State machine** (in-memory): `HashMap`/`BTreeMap` held in `Arc<Mutex<Store>>`; snapshots persist to disk for recovery
- **Leases** (`moka`): in-memory TTL cache, NOT Raft-replicated — lost on restart

## Data flow

**Write path:**
```
gRPC client → WorkflowRpcHandler → WorkflowService/CheckpointService
  → raft.client_write(WorkflowCommand)   ← replicated via Raft
  → StateMachine::apply()
  → Repository methods (mutate in-memory Store)
```

**Read path:**
```
gRPC client → WorkflowRpcHandler → Service → Repository::get*()
  → reads in-memory Store directly (no Raft round-trip)
```

## gRPC API

| RPC | Key inputs | Key output |
|-----|-----------|------------|
| `workflow_start` | workflow_id, context_name? | fencing_token |
| `generate_idempotency_key` | workflow_id, fencing_token, position | idempotency_key |
| `checkpoint` | workflow_id, value, fencing_token, position, idempotency_key | abort (bool) |
| `lease_checkpoint` | workflow_id, fencing_token, lease_timeout, position, idempotency_key | value OR remaining_timeout |
| `complete_workflow` | workflow_id, fencing_token, expire_after | — |
| `workflow_status` | workflow_id | status, timestamps |
| `release_checkpoint` | workflow_id, position | — |

## Build & run

```bash
cargo build                    # debug
cargo build --release          # release (LTO + strip)
cargo check                    # fast type-check only

# Single node (dev)
cargo run -- --node-id 1 --addr-raft 0.0.0.0:9001 --rpc-addr 0.0.0.0:50051 \
  --data-dir /tmp/idempotency \
  --node "1 127.0.0.1:50051 127.0.0.1:9001"

RUST_LOG=info,openraft=info    # production logging
RUST_LOG=debug                 # verbose logging
```

## Tests

```bash
# E2E tests (in-process, no Docker required)
cargo test --test e2e

# All tests
cargo test
```

E2E tests use `ServerFixture` (`tests/e2e/helper.rs`) which spins up a full single-node Raft cluster in-process on a random port. No external processes or Docker needed.

## Error handling layers

```
main.rs / start_server   → anyhow::Result
RPC handler              → typed mappers → tonic::Status
Service layer            → WorkflowError / CheckpointError  (thiserror)
Repository layer         → RepositoryError  (thiserror)
Storage layer            → openraft::StorageError<u64>
Network layer            → NetworkError  (thiserror)
```
