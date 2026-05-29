# src/ — Root module

Entry point and cross-cutting concerns. See `../CONTEXT.md` for the full project overview.

## Files

### `main.rs`
Binary entry point. Builds the Tokio runtime, then wires every component in order:

```
1. cli::Args::parse()
2. Parse --node flags → Vec<(NodeId, Node)> via database::server::Server::parse()
3. Init tracing subscriber (RUST_LOG env var, default "info")
4. database::db::RaftNode::start(node_id, peers, data_dir) → RaftNode { raft, store }
5. Clone store → WorkflowRepository, CheckpointRepository, FencingTokenRepository
6. LeaseRepository::new()
7. WorkflowService::new(workflows, fencing_tokens, checkpoints, raft, node_id)
8. CheckpointService::new(checkpoints, leases, fencing_tokens)
9. WorkflowRpcHandler::new(workflow_svc, checkpoint_svc)
10. TcpRaftServer::new(raft) → tokio::spawn(tcp_raft.listen(addr_raft))
11. clean_up_expired_workflows(workflow_svc) → JobScheduler
12. tokio::select!: gRPC start_server | TCP Raft task | SIGINT | SIGTERM
13. scheduler.shutdown()
```

No business logic. All initialisation errors bubble up as `anyhow::Result<()>`.

### `cli.rs`
CLI argument struct parsed by clap.

```rust
pub struct Args {
    pub node_id: u64,
    pub addr_raft: String,   // Raft TCP address for this node, e.g. "0.0.0.0:9001"
    pub rpc_addr: String,    // gRPC address for clients, e.g. "0.0.0.0:50051"
    pub data_dir: String,    // root dir for fjall log + state machine snapshot
    pub nodes: Vec<String>,  // repeatable: "id addr_api addr_raft"
    pub worker_threads: usize, // default 4
    pub stack_size: usize,     // default 8 MiB
}
```

### `error.rs`
All typed error enums for the project. Uses `thiserror`.

```rust
pub enum RepositoryError { /* empty — in-memory ops are infallible */ }
pub enum WorkflowError   { NotFound { id }, FencingTokenNotFound { workflow_id }, FencingTokenExpired { stored, received }, Raft(String), Repository(#[from] RepositoryError) }
pub enum CheckpointError { FencingTokenNotFound { workflow_id }, FencingTokenExpired, NonDeterministicCheckpoint { position }, CheckpointValueNotFound { position }, Repository(#[from] RepositoryError) }
pub enum NetworkError    { Connect { addr, source }, Io(io::Error), Serialize(String), Deserialize(String), InvalidMessageType(u8), ConnectionLost(String) }
```

**Rule**: `anyhow` is used only at `main.rs`, `start_server`, and `RaftNode::start` (application boundary). All other code uses typed errors.

## Sub-modules

| Directory | Purpose |
|-----------|---------|
| `cron/` | Periodic cleanup job |
| `database/` | Raft node initialisation and peer parsing |
| `repositories/` | Data access layer (SharedStore + moka) |
| `rpc_server/` | gRPC handler and server startup |
| `schema/` | Plain data structs used as in-memory store values |
| `services/` | Business logic layer |
| `storage/` | openraft storage impl (LogStore, StateMachine, Network) |
