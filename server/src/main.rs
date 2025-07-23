use database::db::{get_client, init_tables};
use database::server::Server;
use std::env::var;

use rpc_server::server::start_server;
use std::error::Error;
use tokio::signal;
use tokio::sync::oneshot;
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

async fn graceful_shutdown(client: &hiqlite::Client) -> Result<(), Box<dyn Error>> {
    error!("Initiating graceful shutdown...");
    // This is very important:
    // You MUST do a graceful shutdown when your application exits. This will make sure all
    // lock files are cleaned up and will make your next start faster. If the node starts up
    // without cleanup lock files, it will delete the DB and re-create it from the latest
    // snapshot + logs to really make sure it is 100% consistent.
    // You can set features for `hiqlite` which enable auto-healing (without it will panic on
    // start), but you should always try to do a shutdown.
    //
    // You have 2 options:
    // - register an automatic shutdown handle with the DbClient like shown above
    // - trigger the shutdown manually at the end of your application
    //   This makes sense when you already have structures implemented that catch shutdown signals,
    //   for instance if you `.await` and API being terminated.
    //   Then you can do a `client.shutdown().await?`
    let mut shutdown_handle = client
        .shutdown_handle()
        .map_err(|e| Box::new(e) as Box<dyn Error>)?;
    shutdown_handle
        .wait()
        .await
        .map_err(|e| Box::new(e) as Box<dyn Error>)?;
    Ok(())
}

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
    init_tables(&client)
        .await
        .map_err(|e| Box::new(e) as Box<dyn Error>)?;

    // Create a channel for graceful shutdown
    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

    if server_node.id == 1 {
        // Start the cleanup workflow scheduler
        if let Err(e) = clean_up_expired_workflows(&client).await {
            error!("Error during clean up workflows: {}", e);
            graceful_shutdown(&client).await?;
            return Err(e);
        }
    }

    let client_clone = client.clone();

    // Spawn a task to handle shutdown signals
    let shutdown_handle = tokio::spawn(async move {
        let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate()).unwrap();
        let mut sigint = signal::unix::signal(signal::unix::SignalKind::interrupt()).unwrap();

        tokio::select! {
            _ = signal::ctrl_c() => {
                error!("Received Ctrl+C, initiating graceful shutdown...");
            }
            _ = sigterm.recv() => {
                error!("Received SIGTERM, initiating graceful shutdown...");
            }
            _ = sigint.recv() => {
                error!("Received SIGINT, initiating graceful shutdown...");
            }
        }

        // Perform graceful shutdown
        if let Err(e) = graceful_shutdown(&client_clone).await {
            error!("Error during graceful shutdown: {}", e);
        }

        // Signal the main task to stop
        let _ = shutdown_tx.send(());
    });

    // Start the server with shutdown handling
    let server_result = tokio::select! {
        result = start_server(&rpc_addr, &client) => result,
        _ = shutdown_rx => {
            error!("Server shutdown requested");
            Ok(())
        }
    };

    // Wait for shutdown to complete
    let _ = shutdown_handle.await;

    match server_result {
        Ok(_) => Ok(()),
        Err(e) => {
            error!("Error starting server: {}", e);
            graceful_shutdown(&client).await?;
            Err(e)
        }
    }
}
