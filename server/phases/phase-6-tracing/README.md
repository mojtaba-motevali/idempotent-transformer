# Phase 6 — Tracing & Logging

**Status: DONE**

## What was done

All items verified complete as of last `cargo check`.

### Subscriber setup (`src/main.rs`)

`EnvFilter::try_from_env("RUST_LOG").unwrap_or_else(|_| EnvFilter::new("info"))` — runtime-configurable via `RUST_LOG`. Includes `with_target(true)` and `with_level(true)`.

### No raw print statements

```bash
grep -rn "println!\|eprintln!" src/   # returns nothing
```

### `#[instrument]` coverage

| Layer | Files | Fields captured |
|-------|-------|----------------|
| RPC handler (7 methods) | `src/rpc_server/server.rs` | `workflow_id`, `position`, `fencing_token` per RPC |
| WorkflowService (4 methods) | `src/services/workflow_service.rs` | `workflow_id`, `fencing_token` |
| CheckpointService (4 methods) | `src/services/checkpoint_service.rs` | `workflow_id`, `position`, `fencing_token` |
| WorkflowRepository (4 methods) | `src/repositories/workflows.rs` | `workflow_id` |
| CheckpointRepository (3 methods) | `src/repositories/checkpoints.rs` | `workflow_id`, `position` |
| FencingTokenRepository (3 methods) | `src/repositories/workflows_fencing_tokens.rs` | `workflow_id` |
| LeaseRepository (3 methods) | `src/repositories/lease_checkpoint.rs` | `workflow_id`, `position` |

All `Result`-returning methods have `err` attribute → automatic `ERROR` event on `Err`.

## Log level reference

| Layer | Level |
|-------|-------|
| RPC handler | `INFO` (span duration = request latency) |
| Service — normal path | `DEBUG` |
| Service — abort/error path | `DEBUG` (plus automatic `ERROR` from `err`) |
| Repository | `TRACE` (SQL hidden at INFO/DEBUG) |
| Raft events | `INFO` via openraft's own tracing |
| Cron cleanup | `DEBUG` |
| Shutdown signals | `INFO` (not `ERROR`) |

## Runtime configuration

```bash
RUST_LOG=info                   # production (RPC lifecycle + errors only)
RUST_LOG=debug                  # debugging (service decisions visible)
RUST_LOG=trace                  # full SQL visibility
RUST_LOG=info,openraft=debug    # Raft election debugging without noisy app logs
```

## Verification commands

```bash
grep -rn "println!\|eprintln!" src/          # must be empty
RUST_LOG=debug cargo run 2>&1 | grep workflow_id   # must show fields
RUST_LOG=trace cargo run 2>&1 | grep "SELECT\|INSERT"  # SQL visible
```

## Detail doc

See `tracing-logging.md` for the full `#[instrument]` attribute list per method.
