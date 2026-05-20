# Implementation Phases

Each phase must be completed and compiling before the next begins.
Phases 3–5 depend on Phase 2 storage types; Phase 4 depends on Phase 3 repository structs.

| Phase | Name | Tasks | Key output |
|-------|------|-------|------------|
| [1](./phase-1-dependencies/) | Dependencies & Config | 1, 14 | `Cargo.toml` updated, openraft config in place |
| [2](./phase-2-storage/) | Storage Layer | 2, 3, 4, 5 | `LogStore`, `StateMachine`, `Network`, `RaftNode` structs compiling |
| [3](./phase-3-repositories/) | Repository Layer | 6, 7, 8, 9 | All four repository structs compiling against rusqlite |
| [4](./phase-4-services/) | Service Layer | 10, 11 | `WorkflowService`, `CheckpointService` structs compiling |
| [5](./phase-5-rpc-wiring/) | RPC & Wiring | 12, 13 | `WorkflowRpcHandler` + `main.rs` wiring compiling end-to-end |
| [6](./phase-6-tracing/) | Tracing & Logging | 15, 16, 17 | All layers instrumented; no `println!`/`eprintln!` remaining |
| [7](./phase-7-testing/) | Testing & Validation | 18 | Full BDD suite green |

## Dependency graph

```
Phase 1 (Cargo.toml, config)
    ↓
Phase 2 (LogStore, StateMachine, Network, RaftNode)
    ↓
Phase 3 (WorkflowRepository, CheckpointRepository,
         FencingTokenRepository, LeaseRepository)
    ↓
Phase 4 (WorkflowService, CheckpointService)
    ↓
Phase 5 (WorkflowRpcHandler, main.rs wiring)
    ↓
Phase 6 (tracing & logging — can be done per-layer alongside 3–5)
    ↓
Phase 7 (integration tests)
```

## Shared types

The following types are defined once and imported by all phases:

| Type | File | Used by |
|------|------|---------|
| `TypeConfig` | `src/storage/type_config.rs` | Phase 2, 4, 5 |
| `WorkflowCommand` | `src/storage/command.rs` | Phase 2, 3, 4 |
| `WorkflowResponse` | `src/storage/command.rs` | Phase 2 |
| `NodeId = u64` | `src/storage/type_config.rs` | All |

Define these in Phase 2 before starting Phase 3.
