use std::sync::Arc;

use dashmap::DashMap;
use openraft::{
    error::{InstallSnapshotError, NetworkError as OpenRaftNetworkError, RPCError, RaftError},
    network::{RPCOption, RaftNetwork, RaftNetworkFactory},
    raft::{AppendEntriesRequest, AppendEntriesResponse, InstallSnapshotRequest, InstallSnapshotResponse, VoteRequest, VoteResponse},
};
use tokio::{net::TcpStream, sync::Mutex};

use crate::{
    error::NetworkError,
    storage::{
        framing::{read_framed, write_framed},
        network_codec::{
            decode_append_entries_response, decode_install_snapshot_response,
            decode_vote_response, encode_append_entries_request,
            encode_install_snapshot_request, encode_vote_request,
        },
        type_config::{Node, NodeId, TypeConfig},
    },
};

// ── Helper: map our NetworkError into openraft's RPCError ─────────────────────

fn to_rpc_err<E>(
    e: NetworkError,
) -> RPCError<NodeId, Node, E>
where
    E: std::error::Error,
{
    RPCError::Network(OpenRaftNetworkError::new(&e))
}

// ── Persistent per-peer connection ────────────────────────────────────────────

struct PeerConn {
    addr: String,
    /// tokio::sync::Mutex needed because `send` holds the guard across `.await` points.
    stream: Mutex<Option<TcpStream>>,
}

impl PeerConn {
    fn new(addr: impl Into<String>) -> Arc<Self> {
        Arc::new(Self {
            addr: addr.into(),
            stream: Mutex::new(None),
        })
    }

    /// Send a framed request and return the framed response.
    /// Reconnects once on IO failure.
    async fn send(&self, request: &[u8]) -> Result<Vec<u8>, NetworkError> {
        let mut guard = self.stream.lock().await;

        // Lazy connect
        if guard.is_none() {
            *guard = Some(Self::connect(&self.addr).await?);
        }

        match Self::roundtrip(guard.as_mut().unwrap(), request).await {
            Ok(resp) => Ok(resp),
            Err(_) => {
                // Reconnect once on any IO error, then retry
                *guard = None;
                let mut fresh = Self::connect(&self.addr).await?;
                let resp = Self::roundtrip(&mut fresh, request).await?;
                *guard = Some(fresh);
                Ok(resp)
            }
        }
    }

    async fn connect(addr: &str) -> Result<TcpStream, NetworkError> {
        let stream = TcpStream::connect(addr).await.map_err(|e| NetworkError::Connect {
            addr: addr.to_owned(),
            source: e,
        })?;
        // Disable Nagle to avoid delayed Raft heartbeats.
        stream
            .set_nodelay(true)
            .map_err(NetworkError::Io)?;
        Ok(stream)
    }

    async fn roundtrip(
        stream: &mut TcpStream,
        req: &[u8],
    ) -> Result<Vec<u8>, NetworkError> {
        write_framed(stream, req).await?;
        read_framed(stream).await
    }
}

// ── NetworkFactory ─────────────────────────────────────────────────────────────

/// One `PeerConn` per remote node; shared across all `Network` handles to the same peer.
pub struct NetworkFactory {
    pool: Arc<DashMap<NodeId, Arc<PeerConn>>>,
}

impl NetworkFactory {
    pub fn new() -> Self {
        Self {
            pool: Arc::new(DashMap::new()),
        }
    }
}

impl Default for NetworkFactory {
    fn default() -> Self {
        Self::new()
    }
}

// ── Network (per-peer handle returned to openraft) ────────────────────────────

pub struct Network {
    conn: Arc<PeerConn>,
}

impl RaftNetworkFactory<TypeConfig> for NetworkFactory {
    type Network = Network;

    async fn new_client(&mut self, target: NodeId, node: &Node) -> Network {
        let conn = self
            .pool
            .entry(target)
            .or_insert_with(|| PeerConn::new(&node.addr_raft))
            .clone();
        Network { conn }
    }
}

impl RaftNetwork<TypeConfig> for Network {
    async fn append_entries(
        &mut self,
        rpc: AppendEntriesRequest<TypeConfig>,
        _option: RPCOption,
    ) -> Result<
        AppendEntriesResponse<NodeId>,
        RPCError<NodeId, Node, RaftError<NodeId>>,
    > {
        let bytes = encode_append_entries_request(&rpc).map_err(to_rpc_err)?;
        let resp = self.conn.send(&bytes).await.map_err(to_rpc_err)?;
        decode_append_entries_response(&resp).map_err(to_rpc_err)
    }

    async fn vote(
        &mut self,
        rpc: VoteRequest<NodeId>,
        _option: RPCOption,
    ) -> Result<
        VoteResponse<NodeId>,
        RPCError<NodeId, Node, RaftError<NodeId>>,
    > {
        let bytes = encode_vote_request(&rpc).map_err(to_rpc_err)?;
        let resp = self.conn.send(&bytes).await.map_err(to_rpc_err)?;
        decode_vote_response(&resp).map_err(to_rpc_err)
    }

    async fn install_snapshot(
        &mut self,
        rpc: InstallSnapshotRequest<TypeConfig>,
        _option: RPCOption,
    ) -> Result<
        InstallSnapshotResponse<NodeId>,
        RPCError<NodeId, Node, RaftError<NodeId, InstallSnapshotError>>,
    > {
        let bytes = encode_install_snapshot_request(&rpc).map_err(to_rpc_err)?;
        let resp = self.conn.send(&bytes).await.map_err(to_rpc_err)?;
        decode_install_snapshot_response(&resp).map_err(to_rpc_err)
    }
}
