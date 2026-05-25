use clap::Parser;
use std::sync::Arc;
use tracing::error;
use tracing_subscriber::EnvFilter;

use crate::cli::Args;
use crate::database::{db::RaftNode, server::Server};
use crate::repositories::{
    checkpoints::CheckpointRepository,
    lease_checkpoint::LeaseRepository,
    workflows::WorkflowRepository,
    workflows_fencing_tokens::FencingTokenRepository,
};
use crate::rpc_server::server::{start_server, WorkflowRpcHandler};
use crate::services::{
    checkpoint_service::CheckpointService,
    workflow_service::WorkflowService,
};
use crate::storage::{tcp_server::TcpRaftServer, type_config::Node};
use crate::cron::clean_up_workflows::clean_up_expired_workflows;

mod cli;
mod cron;
mod database;
mod error;
mod helpers;
mod repositories;
mod rpc_server;
mod schema;
mod services;
mod storage;

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(args.worker_threads)
        .thread_stack_size(args.stack_size)
        .enable_all()
        .build()?
        .block_on(run(args))
}

async fn run(args: Args) -> anyhow::Result<()> {
    // Parse peers
    let peers: Vec<(u64, Node)> = args
        .nodes
        .iter()
        .map(|s| {
            let srv = Server::parse(s).unwrap();
            (
                srv.id,
                Node {
                    addr_raft: srv.addr_raft,
                    addr_api: srv.addr_api,
                },
            )
        })
        .collect();

    // Tracing subscriber
    tracing_subscriber::fmt()
        .with_target(true)
        .with_level(true)
        .with_env_filter(
            EnvFilter::try_from_env("RUST_LOG")
                .unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    tracing::info!(node_id = args.node_id, rpc_addr = %args.rpc_addr, "starting server");

    // 1. Start Raft node
    let raft_node = RaftNode::start(args.node_id, peers, &args.data_dir).await?;
    let pool = raft_node.pool.clone();
    let raft = Arc::new(raft_node.raft.clone());

    // 2. Repositories
    let workflow_repo = Arc::new(WorkflowRepository::new(pool.clone()));
    let checkpoint_repo = Arc::new(CheckpointRepository::new(pool.clone()));
    let fencing_token_repo = Arc::new(FencingTokenRepository::new(pool.clone()));
    let lease_repo = Arc::new(LeaseRepository::new());

    // 3. Services
    let workflow_svc = Arc::new(WorkflowService::new(
        workflow_repo,
        fencing_token_repo.clone(),
        checkpoint_repo.clone(),
        raft.clone(),
        args.node_id,
    ));
    let checkpoint_svc = Arc::new(CheckpointService::new(
        checkpoint_repo,
        lease_repo,
        fencing_token_repo,
    ));

    // 4. RPC handler
    let handler = WorkflowRpcHandler::new(workflow_svc.clone(), checkpoint_svc);

    // 5. Raft TCP server (separate port from gRPC)
    let tcp_raft = TcpRaftServer::new(raft.clone());
    let addr_raft = args.addr_raft.clone();
    let tcp_raft_fut = tokio::spawn(async move {
        if let Err(e) = tcp_raft.listen(&addr_raft).await {
            error!(err = %e, "Raft TCP server error");
        }
    });

    // 6. Cron
    let mut scheduler = clean_up_expired_workflows(workflow_svc).await?;

    // 7. gRPC + shutdown
    let mut sigterm =
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;

    tokio::select! {
        res = start_server(args.rpc_addr.as_str(), handler) => {
            match res {
                Ok(()) => error!("gRPC server stopped unexpectedly"),
                Err(e) => error!(err = %e, "gRPC server error"),
            }
        }
        res = tcp_raft_fut => {
            if let Err(e) = res { error!(err = %e, "Raft TCP task panicked"); }
        }
        _ = tokio::signal::ctrl_c() => tracing::info!(signal = "SIGINT", "shutting down"),
        _ = sigterm.recv() => tracing::info!(signal = "SIGTERM", "shutting down"),
    }

    if let Err(e) = scheduler.shutdown().await {
        error!(err = %e, "scheduler shutdown error");
    }
    Ok(())
}
