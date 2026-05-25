use std::collections::BTreeMap;
use std::sync::Arc;

use anyhow::Context as _;
use openraft::{Config, Raft};

use crate::storage::{
    log_store::LogStore,
    network::NetworkFactory,
    state_machine::StateMachine,
    type_config::{Node, NodeId, TypeConfig},
};

pub struct RaftNode {
    pub raft: Raft<TypeConfig>,
    pub node_id: NodeId,
    /// Shared SQLite pool — passed to repository structs in main.rs.
    pub pool: deadpool_sqlite::Pool,
}

impl RaftNode {
    pub async fn start(
        node_id: NodeId,
        peers: Vec<(NodeId, Node)>,
        data_dir: &str,
    ) -> anyhow::Result<Arc<Self>> {
        let log_path = format!("{data_dir}/raft-log");
        let log_store = LogStore::open(&log_path)
            .context("failed to open RocksDB log store")?;

        let sqlite_path = format!("{data_dir}/state.db");
        let pool = deadpool_sqlite::Config::new(&sqlite_path)
            .create_pool(deadpool_sqlite::Runtime::Tokio1)
            .context("failed to build SQLite pool")?;

        let state_machine = StateMachine::new(pool.clone())
            .await
            .context("failed to initialise state machine")?;

        let config = Arc::new(
            Config::default()
                .validate()
                .expect("invalid openraft config"),
        );

        let network = NetworkFactory::new();

        let raft = Raft::new(node_id, config, network, log_store, state_machine)
            .await
            .context("failed to start openraft")?;

        if peers.is_empty() {
            let members = BTreeMap::from([(
                node_id,
                Node {
                    addr_raft: String::new(),
                    addr_api: String::new(),
                },
            )]);
            let _ = raft.initialize(members).await;
        } else {
            let members: BTreeMap<_, _> = peers.into_iter().collect();
            let _ = raft.initialize(members).await;
        }

        Ok(Arc::new(Self { raft, node_id, pool }))
    }

    pub async fn is_leader(&self) -> bool {
        self.raft.current_leader().await == Some(self.node_id)
    }
}
