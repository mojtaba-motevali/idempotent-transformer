# Phase 6 — Tracing & Logging

## Files to modify

| File | Change |
|------|--------|
| `src/main.rs` | Fix subscriber setup |
| `src/main.rs` | Replace `println!`, fix signal log levels |
| `src/rpc_server/server.rs` | Add `#[instrument]` to all 7 RPC methods |
| `src/services/workflow_service.rs` | Add `#[instrument]` to all methods |
| `src/services/checkpoint_service.rs` | Add `#[instrument]` to all methods |
| `src/repositories/workflows.rs` | Add `#[instrument]` to all methods |
| `src/repositories/checkpoints.rs` | Add `#[instrument]` to all methods |
| `src/repositories/workflows_fencing_tokens.rs` | Add `#[instrument]` to all methods |
| `src/repositories/lease_checkpoint.rs` | Add `#[instrument]` to all methods |
| `src/cron/clean_up_workflows.rs` | Replace `eprintln!` |

Note: if `#[instrument]` was added to individual structs during Phases 3–5 (as shown in those
phase documents), this phase is just a verification and gap-fill pass.

---

## 1. Subscriber setup (`src/main.rs`)

```rust
// REMOVE
tracing_subscriber::fmt()
    .with_env_filter(EnvFilter::from("info"))
    .init();

// ADD
tracing_subscriber::fmt()
    .with_target(true)
    .with_level(true)
    .with_env_filter(
        EnvFilter::try_from_env("RUST_LOG")
            .unwrap_or_else(|_| EnvFilter::new("info")),
    )
    .init();
```

---

## 2. Replace `println!` / `eprintln!`

| Location | Before | After |
|----------|--------|-------|
| `main.rs:43` | `println!("Starting server")` | `info!(node_id, rpc_addr = %rpc_addr, "starting server")` |
| `main.rs:105` | `error!("Received Ctrl+C …")` | `info!(signal = "SIGINT", "received signal, shutting down")` |
| `main.rs:110` | `error!("Received SIGTERM …")` | `info!(signal = "SIGTERM", "received signal, shutting down")` |
| `main.rs:115` | `error!("Received SIGINT …")` | `info!(signal = "SIGINT", "received signal, shutting down")` |
| `rpc_server/server.rs:203` | `println!("Server listening on {addr}")` | `info!(addr, "gRPC server listening")` |
| `services/workflow_service.rs:91` | `println!("Deleting expired workflows")` | `debug!("running expired workflow cleanup")` |
| `services/workflow_service.rs:102` | `println!("Deleted expired workflows")` | `debug!(deleted, "expired workflows removed")` |
| `cron/clean_up_workflows.rs:17` | `eprintln!("Error during scheduled delete: {e}")` | `error!(err = %e, "cleanup job failed")` |

---

## 3. `#[instrument]` on RPC handler methods

All 7 methods in `src/rpc_server/server.rs`. Each skips `self` and captures key domain fields:

```rust
// workflow_start
#[instrument(skip(self), fields(workflow_id = %req.get_ref().workflow_id), err)]

// checkpoint
#[instrument(skip(self), fields(
    workflow_id   = %req.get_ref().workflow_id,
    position      = req.get_ref().position,
    fencing_token = req.get_ref().fencing_token,
), err)]

// lease_checkpoint
#[instrument(skip(self), fields(
    workflow_id   = %req.get_ref().workflow_id,
    position      = req.get_ref().position,
    fencing_token = req.get_ref().fencing_token,
), err)]

// complete_workflow
#[instrument(skip(self), fields(
    workflow_id   = %req.get_ref().workflow_id,
    fencing_token = req.get_ref().fencing_token,
), err)]

// workflow_status
#[instrument(skip(self), fields(workflow_id = %req.get_ref().workflow_id), err)]

// release_checkpoint
#[instrument(skip(self), fields(
    workflow_id = %req.get_ref().workflow_id,
    position    = req.get_ref().position,
), err)]

// generate_idempotency_key
#[instrument(skip(self), fields(
    workflow_id   = %req.get_ref().workflow_id,
    position      = req.get_ref().position,
    fencing_token = req.get_ref().fencing_token,
), err)]
```

---

## 4. `#[instrument]` on service methods

`src/services/workflow_service.rs`:

```rust
#[instrument(skip(self), fields(workflow_id), err)]    // create
#[instrument(skip(self), fields(workflow_id, fencing_token), err)]  // finish
#[instrument(skip(self), fields(workflow_id), err)]    // status
#[instrument(skip(self), err)]                         // cleanup_expired
```

`src/services/checkpoint_service.rs`:

```rust
#[instrument(skip(self), fields(workflow_id, position, fencing_token), err)]  // checkpoint
#[instrument(skip(self), fields(workflow_id, position, fencing_token), err)]  // lease
#[instrument(skip(self), fields(workflow_id, position), err)]                  // release
#[instrument(skip(self), fields(workflow_id, position, fencing_token), err)]  // create_idempotency_key
```

---

## 5. `#[instrument]` on repository methods

All repository methods use `skip(self)` to hide the pool internals. Fields mirror what the
calling service already captured, so they appear in child spans automatically.

`WorkflowRepository`: `workflow_id` on `create_or_get`, `get_by_id`, `complete`; none on `delete_expired`.  
`CheckpointRepository`: `workflow_id, position` on `upsert`, `get`; none on `delete_for_completed`.  
`FencingTokenRepository`: `workflow_id` on `increment`, `get`; none on `delete_for_completed`.  
`LeaseRepository`: `workflow_id, position` on all methods.

Add `err` to all methods that return `Result`.

---

## 6. openraft span integration

openraft uses `tracing` internally. Control its verbosity via `RUST_LOG`:

```bash
# production — only leader elections and membership changes
RUST_LOG=info,openraft=info

# debugging Raft elections
RUST_LOG=debug,openraft=debug

# silent Raft internals
RUST_LOG=debug,openraft=warn
```

---

## 7. Log level reference

| Layer | Level | Example event |
|-------|-------|--------------|
| RPC handler | `INFO` | request received, response sent (span duration = latency) |
| Service — normal path | `DEBUG` | "lease acquired", "fencing token valid" |
| Service — abort path | `DEBUG` | "checkpoint aborted — fencing token expired" |
| Repository | `TRACE` | individual SQL executions |
| Raft | `INFO` | leader elected, membership change |
| Cron cleanup | `DEBUG` | rows deleted count |
| Errors | `ERROR` | any propagated `Err` (via `err` on `#[instrument]`) |
| Shutdown signals | `INFO` | SIGTERM / SIGINT / Ctrl+C |

---

## Verification

```bash
# No raw print statements remaining
grep -rn "println!\|eprintln!" src/ && echo "FAIL" || echo "OK"

# Key fields appear in output
RUST_LOG=debug cargo run 2>&1 | grep "workflow_id"

# SQL visible at TRACE
RUST_LOG=trace cargo run 2>&1 | grep "SELECT\|INSERT\|UPDATE\|DELETE"

# Clean at INFO
RUST_LOG=info cargo run 2>&1 | grep -v "workflow_id\|position"
```
