# Implementation Phases

> For full codebase context, read `../CONTEXT.md` first.

## Status

| Phase | Name | Status | Key output |
|-------|------|--------|------------|
| [1](./phase-1-dependencies/) | Dependencies & Config | **DONE** | hiqlite removed, openraft+rocksdb+rusqlite added, error types defined |
| [2](./phase-2-storage/) | Storage Layer | **DONE** | `LogStore`, `StateMachine`, `Network`, `RaftNode` compiling |
| [3](./phase-3-repositories/) | Repository Layer | **DONE** | All 4 repository structs compiling against rusqlite |
| [4](./phase-4-services/) | Service Layer | **DONE** | `WorkflowService`, `CheckpointService` compiling |
| [5](./phase-5-rpc-wiring/) | RPC & Wiring | **DONE** | `WorkflowRpcHandler` + `main.rs` wiring compiling end-to-end |
| [6](./phase-6-tracing/) | Tracing & Logging | **DONE** | All layers instrumented; no `println!`/`eprintln!` remaining |
| [7](./phase-7-testing/) | Testing & Validation | **TODO** | Unit, integration, BDD, benchmark, failover tests |

`cargo check` is clean (zero errors).

## Dependency graph

```
Phase 1 (Cargo.toml, error.rs, type_config.rs, command.rs)
    ↓
Phase 2 (LogStore, StateMachine, NetworkFactory, RaftNode, TcpRaftServer)
    ↓
Phase 3 (WorkflowRepository, CheckpointRepository,
         FencingTokenRepository, LeaseRepository)
    ↓
Phase 4 (WorkflowService, CheckpointService)
    ↓
Phase 5 (WorkflowRpcHandler, main.rs wiring, cli.rs)
    ↓
Phase 6 (tracing — done alongside 3–5, verified here)
    ↓
Phase 7 (integration tests)
```

## Shared types (defined in Phase 1/2, imported everywhere)

| Type | File |
|------|------|
| `TypeConfig` | `src/storage/type_config.rs` |
| `WorkflowCommand` / `WorkflowResponse` | `src/storage/command.rs` |
| `NodeId = u64` | `src/storage/type_config.rs` |
| `RepositoryError`, `WorkflowError`, `CheckpointError`, `NetworkError` | `src/error.rs` |
