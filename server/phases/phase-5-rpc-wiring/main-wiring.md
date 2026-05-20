# Phase 5 — main.rs Wiring

## File

`src/main.rs` (significant rewrite)

## Depends on

All previous phases.

---

## New init sequence

Replace the hiqlite init block with the following construction order.
Each step must succeed before the next begins.

```rust
// 1. CLI arguments — parsed before the async runtime starts
#[derive(clap::Parser)]
#[command(name = "idempotency-server")]
struct Args {
    #[arg(long)]
    node_id: u64,

    #[arg(long)]
    addr_api: String,

    #[arg(long)]
    addr_raft: String,

    #[arg(long)]
    rpc_addr: String,

    #[arg(long)]
    data_dir: String,

    /// Cluster members, repeatable: --node "1 127.0.0.1:8100 127.0.0.1:8200"
    #[arg(long = "node")]
    nodes: Vec<String>,

    #[arg(long, default_value_t = 4)]
    worker_threads: usize,

    /// Thread stack size in bytes
    #[arg(long, default_value_t = 8_388_608)]
    stack_size: usize,
}

let args = Args::parse();
let nodes: Vec<Server> = args.nodes.iter()
    .map(|s| Server::parse(s).unwrap())
    .collect();

// 2. Tokio runtime — built from CLI args before block_on
// (see fn main() below)

// 3. Tracing subscriber (before any await points)
tracing_subscriber::fmt()
    .with_target(true)
    .with_level(true)
    .with_env_filter(
        EnvFilter::try_from_env("RUST_LOG")
            .unwrap_or_else(|_| EnvFilter::new("info")),
    )
    .init();

info!(node_id = args.node_id, rpc_addr = %args.rpc_addr, "starting server");

// 4. Start Raft node (opens RocksDB + SQLite pool inside)
let peers: Vec<(NodeId, Node)> = nodes.iter()
    .map(|s| (s.id, Node { addr_raft: s.addr_raft.clone(), addr_api: s.addr_api.clone() }))
    .collect();
let raft_node = RaftNode::start(args.node_id, peers, &args.data_dir).await?;

// 4. Repositories — all share the same SQLite pool from the state machine
let pool = raft_node.state_machine.pool.clone();
let workflow_repo       = Arc::new(WorkflowRepository::new(pool.clone()));
let checkpoint_repo     = Arc::new(CheckpointRepository::new(pool.clone()));
let fencing_token_repo  = Arc::new(FencingTokenRepository::new(pool.clone()));
let lease_repo          = Arc::new(LeaseRepository::new());

// 5. Services
let raft = Arc::new(raft_node.raft.clone());
let workflow_svc = Arc::new(WorkflowService::new(
    workflow_repo.clone(),
    fencing_token_repo.clone(),
    checkpoint_repo.clone(), // needed for cleanup_expired
    raft.clone(),
    node_id,
));
let checkpoint_svc = Arc::new(CheckpointService::new(
    checkpoint_repo,
    lease_repo,
    fencing_token_repo,
));

// 6. RPC handlers
let workflow_handler = WorkflowRpcHandler::new(workflow_svc.clone(), checkpoint_svc);

// 7. Raft TCP server — listens on ADDR_RAFT (separate port from gRPC)
let tcp_raft = TcpRaftServer::new(raft.clone());
let tcp_raft_fut = {
    let addr_raft = args.addr_raft.clone();
    tokio::spawn(async move {
        if let Err(e) = tcp_raft.listen(&addr_raft).await {
            error!(err = %e, "Raft TCP server error");
        }
    })
};

// 8. Cron scheduler (leader-only cleanup)
let scheduler = clean_up_expired_workflows(workflow_svc.clone()).await?;

// 9. gRPC server (client-facing; only WorkflowServiceImpl, no Raft service)
let server_fut = start_server(&args.rpc_addr, workflow_handler);
```

### `fn main()` — Tokio runtime construction

```rust
fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(args.worker_threads)
        .thread_stack_size(args.stack_size)
        .enable_all()
        .build()?
        .block_on(run(args))
}
```

`run(args)` is the async function containing the full init sequence above.

---

## Graceful shutdown

```rust
tokio::select! {
    res = server_fut => {
        match res {
            Ok(()) => error!("gRPC server stopped unexpectedly"),
            Err(e) => error!(err = %e, "gRPC server error"),
        }
    }
    res = tcp_raft_fut => {
        if let Err(e) = res { error!(err = %e, "Raft TCP server task panicked"); }
    }
    _ = tokio::signal::ctrl_c() => info!(signal = "SIGINT", "shutting down"),
    _ = sigterm()               => info!(signal = "SIGTERM", "shutting down"),
}

if let Err(e) = scheduler.shutdown().await {
    error!(err = %e, "scheduler shutdown error");
}
// RaftNode / pool drop naturally via Arc
```

---

## Cron job update

`src/cron/clean_up_workflows.rs` changes from accepting a `hiqlite::Client` to accepting
`Arc<WorkflowService>`:

```rust
pub async fn clean_up_expired_workflows(
    workflow_svc: Arc<WorkflowService>,
) -> anyhow::Result<JobScheduler> {
    let scheduler = JobScheduler::new().await?;

    scheduler.add(Job::new_async("1/10 * * * * *", move |_, _| {
        let svc = workflow_svc.clone();
        Box::pin(async move {
            if let Err(e) = svc.cleanup_expired().await {
                error!(err = %e, "cleanup job failed");
            }
        })
    })?).await?;

    scheduler.start().await?;
    Ok(scheduler)
}
```

---

## Removed from main.rs

- `get_client()` call
- `client.wait_until_healthy_cache()` / `wait_until_healthy_db()`
- `init_tables()` call (migrations now run inside `StateMachine::new()`)
- `println!("Starting server")` → replaced with `info!()`
- All hiqlite imports

---

## Verification

```bash
cargo build -p idempotency-server
# Must compile end-to-end with no hiqlite references anywhere in the binary
grep -r "hiqlite" src/ && echo "FAIL: hiqlite still present" || echo "OK"
```
