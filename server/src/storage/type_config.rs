use openraft::declare_raft_types;
use serde::{Deserialize, Serialize};

use crate::storage::command::{WorkflowCommand, WorkflowResponse};

pub type NodeId = u64;

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Node {
    pub addr_raft: String,
}

declare_raft_types!(
    pub TypeConfig:
        D            = WorkflowCommand,
        R            = WorkflowResponse,
        NodeId       = NodeId,
        Node         = Node,
        Entry        = openraft::Entry<TypeConfig>,
        SnapshotData = std::io::Cursor<Vec<u8>>,
);
