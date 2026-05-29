use std::{sync::{
    Arc, atomic::{AtomicU64, Ordering}
}};

use dashmap::DashMap;
use openraft::{
    error::{InstallSnapshotError, NetworkError as OpenRaftNetworkError, RPCError, RaftError},
    network::{RPCOption, RaftNetwork, RaftNetworkFactory},
    raft::{
        AppendEntriesRequest, AppendEntriesResponse, InstallSnapshotRequest,
        InstallSnapshotResponse, VoteRequest, VoteResponse,
    },
};
use parking_lot::RwLock;
use tokio::{
    net::{
        TcpStream, tcp::{OwnedReadHalf, OwnedWriteHalf}
    },
    sync::{ mpsc::{self, UnboundedSender}, oneshot},
};

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

// ── openraft error bridge ─────────────────────────────────────────────────────

fn to_rpc_err<E: std::error::Error>(e: NetworkError) -> RPCError<NodeId, Node, E> {
    RPCError::Network(OpenRaftNetworkError::new(&e))
}

// ── Pending-response map ──────────────────────────────────────────────────────

type PendingMap = Arc<DashMap<u64, oneshot::Sender<Result<Vec<u8>, NetworkError>>>>;

fn drain_pending(pending: &PendingMap, reason: &str) {
    let keys: Vec<u64> = pending.iter().map(|e| *e.key()).collect();
    for key in keys {
        if let Some((_, tx)) = pending.remove(&key) {
            let _ = tx.send(Err(NetworkError::ConnectionLost(reason.to_owned())));
        }
    }
}

// ── Background tasks ──────────────────────────────────────────────────────────

/// Reads frames from the socket and routes each response to the matching waiter.
/// On any read error all pending waiters are failed.
async fn run_reader(mut read_half: OwnedReadHalf, pending: PendingMap) {
    loop {
        match read_framed(&mut read_half).await {
            Ok((req_id, payload)) => {
                if let Some((_, tx)) = pending.remove(&req_id) {
                    let _ = tx.send(Ok(payload));
                }
            }
            Err(e) => {
                drain_pending(&pending, &e.to_string());
                break;
            }
        }
    }
}

/// Drains the write channel and sends each frame to the socket.
/// On write error the failed request and all remaining pending waiters are failed.
async fn run_writer(
    mut write_half: OwnedWriteHalf,
    mut rx: mpsc::UnboundedReceiver<(u64, Vec<u8>)>,
    pending: PendingMap,
) {
    while let Some((req_id, payload)) = rx.recv().await {
        if let Err(e) = write_framed(&mut write_half, req_id, &payload).await {
            if let Some((_, tx)) = pending.remove(&req_id) {
                let _ = tx.send(Err(NetworkError::ConnectionLost(e.to_string())));
            }
            // Drain anything that raced in before the reader notices the broken pipe.
            drain_pending(&pending, "write error");
            break;
        }
    }
}

// ── PeerConn ──────────────────────────────────────────────────────────────────

struct ConnState {
    /// Channel to the background writer task.  `None` or closed = disconnected.
    write_tx: Option<mpsc::UnboundedSender<(u64, Vec<u8>)>>,
}

/// Persistent connection to one Raft peer.
///
/// The `Mutex` is held only during (re)connection setup — never for the
/// duration of a roundtrip.  Concurrent sends proceed in parallel: each
/// registers a oneshot waiter, drops the lock, writes its frame to the
/// channel, then awaits its own response independently.
struct PeerConn {
    addr: String,
    state: RwLock<ConnState>,
    pending: PendingMap,
    next_id: AtomicU64,
}

impl PeerConn {
    fn new(addr: impl Into<String>) -> Arc<Self> {
        Arc::new(Self {
            addr: addr.into(),
            state:  RwLock::new( ConnState { write_tx: None }),
            pending: Arc::new(DashMap::new()),
            next_id: AtomicU64::new(0),
        })
    }

    /// Establish a new TCP connection and spawn reader + writer tasks.
    /// Caller must hold the `state` lock.
    async fn connect(&self) -> Result<UnboundedSender<(u64,Vec<u8>)>, NetworkError> {
        let stream =
            TcpStream::connect(&self.addr)
                .await
                .map_err(|e| NetworkError::Connect {
                    addr: self.addr.clone(),
                    source: e,
                })?;
        stream.set_nodelay(true).map_err(NetworkError::Io)?;

        let (read_half, write_half) = stream.into_split();
        let (tx, rx) = mpsc::unbounded_channel::<(u64, Vec<u8>)>();

        let pending = self.pending.clone();
        tokio::spawn(run_reader(read_half, pending.clone()));
        tokio::spawn(run_writer(write_half, rx, pending));

        Ok(tx)
    }

    /// Send `payload` to the peer and return the response payload.
    ///
    /// The mutex is released before waiting for the response, so concurrent
    /// callers do not block each other.
    async fn send(&self, payload: Vec<u8>) -> Result<Vec<u8>, NetworkError> {
        let req_id = self.next_id.fetch_add(1, Ordering::Relaxed);

        // Register the waiter before sending so we cannot miss a fast response.
        let (tx, rx) = oneshot::channel();
        self.pending.insert(req_id, tx);

        // Hold the lock only for (re)connection, then release it.
        let write_tx = {
            let alive = self.state.read()
                .write_tx
                .as_ref()
                .map(|t| !t.is_closed())
                .unwrap_or(false);
            if !alive {
                let tx = match self.connect().await {
                    Ok(tx) => tx,
                    Err(e) => {
                        self.pending.remove(&req_id);
                    return Err(e);
                    }
                    
                };
                self.state.write().write_tx = Some(tx);
            }
            let guard = self.state.read(); 
            guard.write_tx.clone().unwrap()
        }; // ← mutex released here

        if write_tx.send((req_id, payload)).is_err() {
            self.pending.remove(&req_id);
            return Err(NetworkError::ConnectionLost("write channel closed".into()));
        }

        // Await the response with no lock held.
        rx.await.unwrap_or_else(|_| {
            Err(NetworkError::ConnectionLost("response channel dropped".into()))
        })
    }
}

// ── NetworkFactory ────────────────────────────────────────────────────────────

pub struct NetworkFactory {
    pool: Arc<DashMap<NodeId, Arc<PeerConn>>>,
}

impl NetworkFactory {
    pub fn new() -> Self {
        Self { pool: Arc::new(DashMap::new()) }
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
    ) -> Result<AppendEntriesResponse<NodeId>, RPCError<NodeId, Node, RaftError<NodeId>>> {
        let bytes = encode_append_entries_request(&rpc).map_err(to_rpc_err)?;
        let resp = self.conn.send(bytes).await.map_err(to_rpc_err)?;
        decode_append_entries_response(&resp).map_err(to_rpc_err)
    }

    async fn vote(
        &mut self,
        rpc: VoteRequest<NodeId>,
        _option: RPCOption,
    ) -> Result<VoteResponse<NodeId>, RPCError<NodeId, Node, RaftError<NodeId>>> {
        let bytes = encode_vote_request(&rpc).map_err(to_rpc_err)?;
        let resp = self.conn.send(bytes).await.map_err(to_rpc_err)?;
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
        let resp = self.conn.send(bytes).await.map_err(to_rpc_err)?;
        decode_install_snapshot_response(&resp).map_err(to_rpc_err)
    }
}
