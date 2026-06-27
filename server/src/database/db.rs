use std::collections::BTreeMap;
use std::sync::Arc;

use anyhow::Context as _;
use openraft::{Config, Raft};

use crate::storage::{
    log_store::LogStore,
    network::NetworkFactory,
    state_machine::{SharedStore, StateMachine},
    type_config::{Node, NodeId, TypeConfig},
};

pub struct RaftNode {
    pub raft:  Raft<TypeConfig>,
    /// In-memory application state — shared with repository structs.
    pub store: SharedStore,
}

impl RaftNode {
    pub async fn start(
        node_id: NodeId,
        peers: Vec<(NodeId, Node)>,
        data_dir: &str,
    ) -> anyhow::Result<Arc<Self>> {
        let log_store = LogStore::open(&format!("{data_dir}/raft-log"))
            .context("failed to open fjall log store")?;

        let (state_machine, store) =
            StateMachine::new(&format!("{data_dir}/state"))
                .context("failed to initialise in-memory state machine")?;

        let config = Arc::new(
            Config {
                // Heartbeat every 500 ms.  Election only triggers after 10–20×
                // that interval, giving plenty of margin for fsync latency and
                // OS scheduling jitter.
                heartbeat_interval:    500,
                election_timeout_min:  5_000,
                election_timeout_max: 10_000,
                ..Default::default()
            }
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
                Node { addr_raft: String::new() },
            )]);
            let _ = raft.initialize(members).await;
        } else {
            let members: BTreeMap<_, _> = peers.into_iter().collect();
            let _ = raft.initialize(members).await;
        }

        Ok(Arc::new(Self { raft, store }))
    }
}
