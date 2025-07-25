use database::db::{get_client, init_tables};
use database::server::Server;
use std::env::var;

use rpc_server::server::start_server;
use std::error::Error;
use tokio::signal;
use tracing::error;
use tracing_subscriber::EnvFilter;

use crate::cron::clean_up_workflows::clean_up_expired_workflows;

mod cron;
mod database;
mod helpers;
mod repositories;
mod rpc_server;
mod schema;
mod services;

// Replace #[tokio::main] with custom runtime configuration
fn main() -> Result<(), Box<dyn Error>> {
    dotenvy::dotenv().ok();
    let server_node = Server {
        id: var("NODE_ID")
            .expect("NODE_ID is not set")
            .parse::<u64>()
            .unwrap(),
        addr_api: var("ADDR_API").expect("ADDR_API is not set").to_string(),
        addr_raft: var("ADDR_RAFT").expect("ADDR_RAFT is not set").to_string(),
    };

    // Configure Tokio runtime with custom thread pool
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(8) // Number of worker threads (default: # of CPUs)
        .max_blocking_threads(512) // Max threads for blocking operations (default: 512)
        .thread_name("idempotency-worker")
        .thread_stack_size(2 * 1024 * 1024) // 2MB stack size per thread
        .enable_all()
        .build()?;

    rt.block_on(async_main(server_node))
}

async fn async_main(server_node: Server) -> Result<(), Box<dyn Error>> {
    let rpc_addr = var("RPC_ADDR").expect("RPC_ADDR is not set").to_string();
    let nodes = var("NODES")
        .expect("NODES is not set")
        .split(",")
        .map(|s| Server::parse(s).unwrap())
        .collect::<Vec<Server>>();
    let data_dir = var("DATA_DIR").expect("DATA_DIR is not set").to_string();

    tracing_subscriber::fmt()
        .with_target(true)
        .with_level(true)
        .with_env_filter(EnvFilter::from("info"))
        .init();

    let client = get_client(server_node.clone(), data_dir, nodes)
        .await
        .map_err(|e| Box::new(e) as Box<dyn Error>)?;
    let mut shutdown_handle = client.shutdown_handle()?;

    init_tables(&client)
        .await
        .map_err(|e| Box::new(e) as Box<dyn Error>)?;
    let mut scheduler = None;
    if server_node.id == 1 {
        // Start the cleanup workflow scheduler
        scheduler = Some(clean_up_expired_workflows(&client).await?);
    }

    // Helper function for graceful shutdown
    let shutdown_gracefully = |scheduler: Option<tokio_cron_scheduler::JobScheduler>| async move {
        if let Some(mut sched) = scheduler {
            if let Err(e) = sched.shutdown().await {
                error!("Error shutting down scheduler: {}", e);
            }
        }
        if let Err(e) = shutdown_handle.wait().await {
            error!("Error during database shutdown: {}", e);
        }
    };

    let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate()).unwrap();
    let mut sigint = signal::unix::signal(signal::unix::SignalKind::interrupt()).unwrap();

    tokio::select! {
        result = start_server(&rpc_addr, &client) => {
            match result {
                Ok(_) => {
                    error!("Server stopped unexpectedly");
                    shutdown_gracefully(scheduler).await;
                    Ok(())
                }
                Err(e) => {
                    error!("Error starting server: {}", e);
                    shutdown_gracefully(scheduler).await;
                    Err(e)
                }
            }
        }
        _ = signal::ctrl_c() => {
            error!("Received Ctrl+C, initiating graceful shutdown...");
            shutdown_gracefully(scheduler).await;
            Ok(())
        }
        _ = sigterm.recv() => {
            error!("Received SIGTERM, initiating graceful shutdown...");
            shutdown_gracefully(scheduler).await;
            Ok(())
        }
        _ = sigint.recv() => {
            error!("Received SIGINT, initiating graceful shutdown...");
            shutdown_gracefully(scheduler).await;
            Ok(())
        }
    }
}
