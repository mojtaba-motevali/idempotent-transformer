# cron/ — Periodic jobs

## Files

### `clean_up_workflows.rs`

```rust
pub async fn clean_up_expired_workflows(
    workflow_svc: Arc<WorkflowService>,
) -> anyhow::Result<JobScheduler>
```

Starts a `tokio-cron-scheduler` job that fires every 10 seconds (`"1/10 * * * * *"`).
On each tick it calls `workflow_svc.cleanup_expired()`.

`cleanup_expired()` is **leader-only** — it checks `WorkflowService::is_leader()` and
returns immediately on followers. This prevents duplicate deletes across cluster nodes.

Errors are logged at `ERROR` level via `error!(err = %e, "cleanup job failed")`.
The scheduler handle is returned to `main.rs` so it can be shut down on SIGTERM/SIGINT.

## Dependencies

- `services::workflow_service::WorkflowService` — the only dependency
- `tokio-cron-scheduler` crate — provides `JobScheduler` and `Job`
- `tracing::error!` — structured error logging
