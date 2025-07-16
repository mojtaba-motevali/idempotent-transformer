use crate::database::server::Server;
use hiqlite::{Client, Error, Node, NodeConfig, start_node};
use hiqlite_macros::embed::*;
use std::fmt::Display;
use std::time::Duration;
use tokio::time;

// this way of logging makes our logs easier to see with all the raft logging enabled
fn log<S: Display>(s: S) {
    println!("\n\n>>> {s}\n");
}

#[derive(Embed)]
#[folder = "migrations"]
struct Migrations;

async fn node_config(node_id: u64, nodes: Vec<Server>) -> NodeConfig {
    let mut config = NodeConfig::from_toml("hiqlite.toml", None, None)
        .await
        .unwrap();
    config.node_id = node_id;
    config.nodes = nodes
        .iter()
        .map(|s| Node {
            id: s.id,
            addr_api: s.addr_api.clone(),
            addr_raft: s.addr_raft.clone(),
        })
        .collect();
    config.log_statements = true;
    config.tls_raft = None;
    config.tls_api = None;
    config
}

pub async fn get_client(
    args: Server,
    data_dir: String,
    nodes: Vec<Server>,
) -> Result<Client, Error> {
    let config = {
        let mut config = node_config(args.id, nodes).await;

        // to make this example work when starting all nodes on the same host,
        // we need to save into custom folders for each one
        config.data_dir = data_dir.into();
        config
    };
    // Start the Raft node itself and get a client
    // the auto_init setting will initialize the Raft cluster automatically and adds
    // all given Nodes as members, as soon as they are all up and running
    let client = start_node(config).await?;
    while client.is_healthy_db().await.is_err() {
        log("Waiting for the Cluster to become healthy");
        time::sleep(Duration::from_secs(1)).await;
    }

    Ok(client)
}

pub async fn init_tables(client: &Client) -> Result<(), Error> {
    client.migrate::<Migrations>().await?;
    Ok(())
}
