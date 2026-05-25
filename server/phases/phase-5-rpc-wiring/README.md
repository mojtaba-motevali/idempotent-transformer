# Phase 5 — RPC & Wiring

**Status: DONE**

## What was done

gRPC handler and `main.rs` wiring complete and compiling end-to-end.

## WorkflowRpcHandler (`src/rpc_server/server.rs`)

```rust
pub struct WorkflowRpcHandler {
    workflow_service: Arc<WorkflowService>,
    checkpoint_service: Arc<CheckpointService>,
}
```

Implements the tonic-generated `WorkflowServiceImpl` trait. All 7 RPC methods
delegate to the corresponding service method with no business logic in the handler.

### Error mappers (typed, not Box<dyn Error>)

```rust
fn workflow_err(e: WorkflowError) -> Status {
    match e {
        WorkflowError::NotFound { .. }             => Status::not_found(e.to_string()),
        WorkflowError::FencingTokenExpired { .. }  => Status::aborted(e.to_string()),
        WorkflowError::FencingTokenNotFound { .. } => Status::not_found(e.to_string()),
        WorkflowError::Raft(_)                     => Status::internal(e.to_string()),
        WorkflowError::Repository(_)               => Status::internal(e.to_string()),
    }
}

fn checkpoint_err(e: CheckpointError) -> Status {
    match e {
        CheckpointError::FencingTokenExpired       => Status::aborted(e.to_string()),
        CheckpointError::FencingTokenNotFound { .. }=> Status::not_found(e.to_string()),
        CheckpointError::NonDeterministicCheckpoint{ .. } => Status::failed_precondition(...),
        CheckpointError::CheckpointValueNotFound{ .. }    => Status::not_found(...),
        CheckpointError::Repository(_)             => Status::internal(e.to_string()),
    }
}
```

### 7 RPCs

| RPC method | Service call |
|------------|-------------|
| `workflow_start` | `workflow_svc.create(...)` |
| `generate_idempotency_key` | `checkpoint_svc.create_idempotency_key(...)` |
| `checkpoint` | `checkpoint_svc.checkpoint(...)` |
| `lease_checkpoint` | `checkpoint_svc.lease(...)` |
| `complete_workflow` | `workflow_svc.finish(...)` |
| `workflow_status` | `workflow_svc.status(...)` |
| `release_checkpoint` | `checkpoint_svc.release(...)` |

## `start_server` (`src/rpc_server/server.rs`)

```rust
pub async fn start_server(addr: &str, handler: WorkflowRpcHandler) -> anyhow::Result<()>
```

Binds the tonic server to `addr` and serves until shutdown.

## `main.rs` wiring order

```
1. Parse Args (cli::Args::parse())
2. Parse peers (Server::parse per --node flag)
3. Init tracing subscriber (RUST_LOG env var, default "info")
4. RaftNode::start(node_id, peers, data_dir)  → RaftNode { raft, pool }
5. Clone pool → WorkflowRepository, CheckpointRepository, FencingTokenRepository
6. LeaseRepository::new()
7. WorkflowService::new(...)
8. CheckpointService::new(...)
9. WorkflowRpcHandler::new(workflow_svc, checkpoint_svc)
10. TcpRaftServer::new(raft) → tokio::spawn(tcp_raft.listen(addr_raft))
11. clean_up_expired_workflows(workflow_svc) → JobScheduler
12. tokio::select!: gRPC server | TCP Raft task | SIGINT | SIGTERM
13. scheduler.shutdown()
```

## `cli.rs`

```rust
// src/cli.rs
pub struct Args {
    pub node_id: u64,
    pub addr_raft: String,
    pub rpc_addr: String,
    pub data_dir: String,
    pub nodes: Vec<String>,          // "id addr_api addr_raft" per --node flag
    pub worker_threads: usize,       // default 4
    pub stack_size: usize,           // default 8 MiB
}
```

Moved out of `main.rs` into its own file to keep `main.rs` focused on wiring.

## Verification

```bash
cargo build   # full binary must link cleanly
```

## Detail docs

- `rpc-handler.md` — full RPC method signatures with instrumentation attributes
- `main-wiring.md` — step-by-step init sequence with code
