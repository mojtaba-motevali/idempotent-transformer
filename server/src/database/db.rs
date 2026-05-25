use std::collections::BTreeMap;
use std::sync::Arc;

use anyhow::Context as _;
use openraft::{Config, Raft};
use rocksdb::DB;

use crate::storage::{
    log_store::LogStore,
    network::NetworkFactory,
    state_machine::{open_state_db, StateMachine},
    type_config::{Node, NodeId, TypeConfig},
};

pub struct RaftNode {
    pub raft: Raft<TypeConfig>,
    pub node_id: NodeId,
    /// State machine DB — shared with repository structs in main.rs.
    pub db: Arc<DB>,
}

impl RaftNode {
    pub async fn start(
        node_id: NodeId,
        peers: Vec<(NodeId, Node)>,
        data_dir: &str,
    ) -> anyhow::Result<Arc<Self>> {
        let log_store = LogStore::open(&format!("{data_dir}/raft-log"))
            .context("failed to open RocksDB log store")?;

        let db = open_state_db(&format!("{data_dir}/state"))
            .context("failed to open RocksDB state machine")?;

        let state_machine = StateMachine::new(db.clone())
            .context("failed to initialise state machine")?;

        let config = Arc::new(
            Config::default()
                .validate()
                .expect("invalid openraft config"),
        );

        let raft = Raft::new(
            node_id,
            config,
            NetworkFactory::new(),
            log_store,
            state_machine,
        )
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

        Ok(Arc::new(Self { raft, node_id, db }))
    }

    pub async fn is_leader(&self) -> bool {
        self.raft.current_leader().await == Some(self.node_id)
    }
}
