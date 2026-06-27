use clap::Parser;

#[derive(Parser)]
#[command(name = "idempotency-server")]
pub struct Args {
    #[arg(long, env = "NODE_ID")]
    pub node_id: u64,

    #[arg(long, env = "ADDR_RAFT")]
    pub addr_raft: String,

    #[arg(long, env = "RPC_ADDR")]
    pub rpc_addr: String,

    #[arg(long, env = "DATA_DIR")]
    pub data_dir: String,

    /// Cluster members, repeatable: --node "1 127.0.0.1:8100 127.0.0.1:8200"
    /// Or via env NODES as comma-separated: "1 host:api host:raft,2 host:api host:raft"
    #[arg(long = "node", env = "NODES", value_delimiter = ',')]
    pub nodes: Vec<String>,

    #[arg(long, env = "TOKIO_WORKER_THREADS", default_value_t = 4)]
    pub worker_threads: usize,

    /// Thread stack size in bytes
    #[arg(long, env = "TOKIO_WORKER_STACK_SIZE", default_value_t = 8_388_608)]
    pub stack_size: usize,
}
