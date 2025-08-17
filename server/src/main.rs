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

async fn shutdown_gracefully(
    mut tokio_cron_scheduler: tokio_cron_scheduler::JobScheduler,
    shutdown_handle: impl std::future::Future<Output = Result<(), hiqlite::Error>>,
) {
    if let Err(e) = shutdown_handle.await {
        error!("Error during database shutdown: {}", e);
    }
    if let Err(e) = tokio_cron_scheduler.shutdown().await {
        error!("Error shutting down scheduler: {}", e);
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    dotenvy::dotenv().ok();
    let total_threads = var("TOKIO_WORKER_THREADS")
        .expect("TOKIO_WORKER_THREADS is not set")
        .parse::<usize>()
        .unwrap();
    let stack_size = var("TOKIO_WORKER_STACK_SIZE")
        .expect("TOKIO_WORKER_STACK_SIZE is not set")
        .parse::<usize>()
        .unwrap();
    println!("Starting server");
    // Configure Tokio runtime with custom stack size and thread count
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(total_threads)
        .thread_stack_size(stack_size)
        .enable_all()
        .build()
        .unwrap();

    runtime.block_on(async {
        let server_node = Server {
            id: var("NODE_ID")
                .expect("NODE_ID is not set")
                .parse::<u64>()
                .unwrap(),
            addr_api: var("ADDR_API").expect("ADDR_API is not set").to_string(),
            addr_raft: var("ADDR_RAFT").expect("ADDR_RAFT is not set").to_string(),
        };

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
        let scheduler = clean_up_expired_workflows(&client).await?;

        let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate()).unwrap();
        let mut sigint = signal::unix::signal(signal::unix::SignalKind::interrupt()).unwrap();

        tokio::select! {
            result = start_server(&rpc_addr, &client) => {
                match result {
                    Ok(_) => {
                        error!("Server stopped unexpectedly");
                        shutdown_gracefully(scheduler, shutdown_handle.wait()).await;
                        Ok(())
                    }
                    Err(e) => {
                        error!("Error starting server: {}", e);
                        shutdown_gracefully(scheduler, shutdown_handle.wait()).await;
                        Err(e)
                    }
                }
            }
            _ = signal::ctrl_c() => {
                error!("Received Ctrl+C, initiating graceful shutdown...");
                shutdown_gracefully(scheduler, shutdown_handle.wait()).await;
                Ok(())
            }
            _ = sigterm.recv() => {
                error!("Received SIGTERM, initiating graceful shutdown...");
                shutdown_gracefully(scheduler, shutdown_handle.wait()).await;
                Ok(())
            }
            _ = sigint.recv() => {
                error!("Received SIGINT, initiating graceful shutdown...");
                shutdown_gracefully(scheduler, shutdown_handle.wait()).await;
                Ok(())
            }
        }
    })
}
