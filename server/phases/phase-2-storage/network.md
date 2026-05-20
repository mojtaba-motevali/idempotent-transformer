# Phase 2 — Network (raw TCP + FlatBuffers)

## Files

| File | Action |
|------|--------|
| `flatbuffers/raft_network.fbs` | Create — FlatBuffers schema for all Raft messages |
| `src/storage/network.rs` | Create — `NetworkFactory` + `Network` (outbound, client side) |
| `src/storage/tcp_server.rs` | Create — `TcpRaftServer` (inbound, server side) |

Generated output from `flatc`:
`src/storage/raft_network_generated.rs` (via `include!` in `mod.rs`)

---

## Wire framing

Raw TCP is a byte stream — every message is length-prefixed:

```
┌─────────────────────┬──────────────────────────────────┐
│  4 bytes (BE u32)   │  N bytes                         │
│  payload length     │  FlatBuffers-encoded Envelope    │
└─────────────────────┴──────────────────────────────────┘
```

Shared framing helpers (`src/storage/framing.rs`):

```rust
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use crate::error::NetworkError;

pub async fn write_framed(stream: &mut TcpStream, data: &[u8]) -> Result<(), NetworkError> {
    let len = (data.len() as u32).to_be_bytes();
    stream.write_all(&len).await?;
    stream.write_all(data).await?;
    stream.flush().await?;
    Ok(())
}

pub async fn read_framed(stream: &mut TcpStream) -> Result<Vec<u8>, NetworkError> {
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).await?;
    let len = u32::from_be_bytes(len_buf) as usize;
    let mut buf = vec![0u8; len];
    stream.read_exact(&mut buf).await?;
    Ok(buf)
}
```

---

## FlatBuffers schema (`flatbuffers/raft_network.fbs`)

```fbs
namespace raft_net;

// ── Core Raft primitives ───────────────────────────────────────────────────

table LogId {
    leader_id: uint64;
    index: uint64;
}

table Vote {
    term: uint64;
    node_id: uint64;
    committed: bool;
}

// ── WorkflowCommand variants (carried inside log entries) ──────────────────

table CmdCreateOrGetWorkflow {
    workflow_id: string (required);
    name: string;
    created_at: int64;
}

table CmdIncrementFencingToken {
    workflow_id: string (required);
}

table CmdUpsertCheckpoint {
    workflow_id: string (required);
    position: int64;
    idempotency_key: string (required);
    value: [ubyte];
    created_at: int64;
}

table CmdCompleteWorkflow {
    workflow_id: string (required);
    expire_at: int64;
    completed_at: int64;
}

table CmdDeleteExpired {
    before_ms: int64;
}

union WorkflowCommand {
    CmdCreateOrGetWorkflow,
    CmdIncrementFencingToken,
    CmdUpsertCheckpoint,
    CmdCompleteWorkflow,
    CmdDeleteExpired,
}

// ── Log entry payload variants ─────────────────────────────────────────────

table BlankPayload {}

table NormalPayload {
    command: WorkflowCommand;
}

table MembershipPayload {
    // openraft Membership<u64, Node> is a complex nested type;
    // serialize as JSON to avoid duplicating its structure in FlatBuffers.
    membership_json: string (required);
}

union EntryPayload {
    BlankPayload,
    NormalPayload,
    MembershipPayload,
}

table Entry {
    log_id: LogId;
    payload: EntryPayload;
}

// ── AppendEntries ──────────────────────────────────────────────────────────

table AppendEntriesRequest {
    vote: Vote (required);
    prev_log_id: LogId;          // absent = no previous entry
    entries: [Entry];
    leader_commit: LogId;        // absent = nothing committed yet
}

table AppendEntriesResponse {
    vote: Vote (required);
    success: bool;
    conflict_log_id: LogId;      // set when success = false
}

// ── Vote ───────────────────────────────────────────────────────────────────

table VoteRequest {
    vote: Vote (required);
    last_log_id: LogId;          // absent = candidate has no entries
}

table VoteResponse {
    vote: Vote (required);
    vote_granted: bool;
}

// ── InstallSnapshot ────────────────────────────────────────────────────────

table SnapshotMeta {
    last_log_id: LogId;
    last_membership: string (required);  // JSON-encoded openraft Membership
    snapshot_id: string (required);
}

table InstallSnapshotRequest {
    vote: Vote (required);
    meta: SnapshotMeta (required);
    offset: uint64;
    data: [ubyte];
    done: bool;
}

table InstallSnapshotResponse {
    vote: Vote (required);
}

// ── Envelope (top-level wire type) ────────────────────────────────────────

union RaftMessage {
    AppendEntriesRequest,
    AppendEntriesResponse,
    VoteRequest,
    VoteResponse,
    InstallSnapshotRequest,
    InstallSnapshotResponse,
}

table Envelope {
    message: RaftMessage;
}

root_type Envelope;
```

---

## Code generation (`build.rs`)

Add to the existing `build.rs` alongside `tonic_build`:

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // existing proto build
    tonic_build::configure()
        .build_server(true)
        .compile(&["proto/workflow_service.proto"], &["proto"])?;

    // FlatBuffers schema for Raft network messages
    flatbuffers_build::BuilderOptions::new_with_files(["flatbuffers/raft_network.fbs"])
        .compile()?;

    Ok(())
}
```

Include the generated code in `src/storage/mod.rs`:

```rust
#[allow(warnings, clippy::all)]
mod raft_network_generated {
    include!(concat!(env!("OUT_DIR"), "/raft_network_generated.rs"));
}
pub use raft_network_generated::raft_net::*;
```

---

## Persistent connection (`src/storage/network.rs`)

One TCP connection per peer is kept alive for the lifetime of the node.
The `Mutex<Option<TcpStream>>` is held only for the duration of one
request-response round-trip, so pipelining is not supported (openraft does
not require it; it serialises RPCs to the same peer internally).

```rust
use std::sync::Arc;
use tokio::{net::TcpStream, sync::Mutex};
use dashmap::DashMap;
use openraft::network::{RaftNetwork, RaftNetworkFactory, RPCOption};
use crate::{
    error::NetworkError,
    storage::{
        type_config::{TypeConfig, NodeId, Node},
        framing::{read_framed, write_framed},
        raft_network_generated::raft_net::*,
    },
};

// ── Shared connection slot ─────────────────────────────────────────────────

struct PeerConn {
    addr: String,
    /// `tokio::sync::Mutex` is required because `send` holds the guard across `.await` points.
    stream: Mutex<Option<TcpStream>>,
}

impl PeerConn {
    fn new(addr: impl Into<String>) -> Arc<Self> {
        Arc::new(Self { addr: addr.into(), stream: Mutex::new(None) })
    }

    async fn send(&self, request: &[u8]) -> Result<Vec<u8>, NetworkError> {
        let mut guard = self.stream.lock().await;

        // Connect lazily (or reconnect after error)
        if guard.is_none() {
            *guard = Some(Self::connect(&self.addr).await?);
        }

        match Self::roundtrip(guard.as_mut().unwrap(), request).await {
            Ok(resp) => Ok(resp),
            Err(_e) => {
                // Broken pipe / peer reset — reconnect once and retry
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
        stream.set_nodelay(true)?;
        Ok(stream)
    }

    async fn roundtrip(stream: &mut TcpStream, req: &[u8]) -> Result<Vec<u8>, NetworkError> {
        write_framed(stream, req).await?;
        read_framed(stream).await
    }
}

// ── NetworkFactory ─────────────────────────────────────────────────────────

pub struct NetworkFactory {
    /// One PeerConn per remote node; shared across all Network handles to the same peer.
    pool: Arc<DashMap<NodeId, Arc<PeerConn>>>,
}

impl NetworkFactory {
    pub fn new() -> Self {
        Self { pool: Arc::new(DashMap::new()) }
    }
}

// ── Network (per-peer handle returned to openraft) ─────────────────────────

pub struct Network {
    conn: Arc<PeerConn>,
}

impl RaftNetworkFactory<TypeConfig> for NetworkFactory {
    type Network = Network;

    async fn new_client(&mut self, target: NodeId, node: &Node) -> Network {
        let conn = self.pool
            .entry(target)
            .or_insert_with(|| PeerConn::new(&node.addr_raft))
            .clone();
        Network { conn }
    }
}

#[async_trait::async_trait]
impl RaftNetwork<TypeConfig> for Network {

    async fn send_append_entries(
        &mut self,
        rpc: openraft::raft::AppendEntriesRequest<TypeConfig>,
        _opt: RPCOption,
    ) -> Result<openraft::raft::AppendEntriesResponse<NodeId>, openraft::error::RPCError<NodeId, Node, openraft::error::RaftError<NodeId>>> {
        let bytes = encode_append_entries_request(&rpc)
            .map_err(|e| openraft::error::RPCError::Network(openraft::error::NetworkError::new(&e)))?;
        let resp_bytes = self.conn.send(&bytes).await
            .map_err(|e| openraft::error::RPCError::Network(openraft::error::NetworkError::new(&e)))?;
        decode_append_entries_response(&resp_bytes)
            .map_err(|e| openraft::error::RPCError::Network(openraft::error::NetworkError::new(&e)))
    }

    async fn send_vote(
        &mut self,
        rpc: openraft::raft::VoteRequest<NodeId>,
        _opt: RPCOption,
    ) -> Result<openraft::raft::VoteResponse<NodeId>, openraft::error::RPCError<NodeId, Node, openraft::error::RaftError<NodeId>>> {
        let bytes = encode_vote_request(&rpc)
            .map_err(|e| openraft::error::RPCError::Network(openraft::error::NetworkError::new(&e)))?;
        let resp_bytes = self.conn.send(&bytes).await
            .map_err(|e| openraft::error::RPCError::Network(openraft::error::NetworkError::new(&e)))?;
        decode_vote_response(&resp_bytes)
            .map_err(|e| openraft::error::RPCError::Network(openraft::error::NetworkError::new(&e)))
    }

    async fn send_install_snapshot(
        &mut self,
        rpc: openraft::raft::InstallSnapshotRequest<TypeConfig>,
        _opt: RPCOption,
    ) -> Result<openraft::raft::InstallSnapshotResponse<NodeId>, openraft::error::RPCError<NodeId, Node, openraft::error::RaftError<NodeId>>> {
        let bytes = encode_install_snapshot_request(&rpc)
            .map_err(|e| openraft::error::RPCError::Network(openraft::error::NetworkError::new(&e)))?;
        let resp_bytes = self.conn.send(&bytes).await
            .map_err(|e| openraft::error::RPCError::Network(openraft::error::NetworkError::new(&e)))?;
        decode_install_snapshot_response(&resp_bytes)
            .map_err(|e| openraft::error::RPCError::Network(openraft::error::NetworkError::new(&e)))
    }
}
```

---

## Conversion helpers (`src/storage/network_codec.rs`)

These functions translate between openraft's Rust types and FlatBuffers-encoded bytes.
`encode_*` returns `Result<Vec<u8>, NetworkError>` — serialization is fallible if a string
field fails UTF-8 encoding. `decode_*` returns `Result<T, NetworkError>`.

```rust
use flatbuffers::{FlatBufferBuilder, WIPOffset};
use crate::{
    error::NetworkError,
    storage::raft_network_generated::raft_net::*,
};

pub fn encode_append_entries_request(
    rpc: &openraft::raft::AppendEntriesRequest<TypeConfig>,
) -> Result<Vec<u8>, NetworkError> {
    let mut b = FlatBufferBuilder::with_capacity(1024);

    let vote     = build_vote(&mut b, &rpc.vote);
    let prev_lid = rpc.prev_log_id.as_ref().map(|id| build_log_id(&mut b, id));
    let entries: Vec<_> = rpc.entries.iter()
        .map(|e| build_entry(&mut b, e))
        .collect::<Result<_, _>>()?;
    let entries_vec = b.create_vector(&entries);
    let leader_commit = rpc.leader_commit.as_ref().map(|id| build_log_id(&mut b, id));

    let req = AppendEntriesRequest::create(&mut b, &AppendEntriesRequestArgs {
        vote: Some(vote),
        prev_log_id: prev_lid,
        entries: Some(entries_vec),
        leader_commit,
    });

    let envelope = Envelope::create(&mut b, &EnvelopeArgs {
        message_type: RaftMessage::AppendEntriesRequest,
        message: Some(req.as_union_value()),
    });

    b.finish(envelope, None);
    Ok(b.finished_data().to_vec())
}

// decode_append_entries_response, encode_vote_request, decode_vote_response,
// encode_install_snapshot_request, decode_install_snapshot_response,
// build_vote, build_log_id, build_entry, build_command — implement similarly.
// All encode_* return Result<Vec<u8>, NetworkError>; all decode_* return Result<T, NetworkError>.

fn build_entry<'b>(
    b: &mut FlatBufferBuilder<'b>,
    entry: &openraft::Entry<TypeConfig>,
) -> Result<WIPOffset<Entry<'b>>, NetworkError> {
    let log_id = build_log_id(b, &entry.log_id);
    let (payload_type, payload) = match &entry.payload {
        openraft::EntryPayload::Blank => {
            let blank = BlankPayload::create(b, &BlankPayloadArgs {});
            (EntryPayload::BlankPayload, blank.as_union_value())
        }
        openraft::EntryPayload::Normal(cmd) => {
            let cmd_offset = build_workflow_command(b, cmd);
            let normal = NormalPayload::create(b, &NormalPayloadArgs {
                command_type: cmd_offset.0,
                command: Some(cmd_offset.1),
            });
            (EntryPayload::NormalPayload, normal.as_union_value())
        }
        openraft::EntryPayload::Membership(m) => {
            let json = serde_json::to_string(m)
                .map_err(|e| NetworkError::Serialize(e.to_string()))?;
            let json_off = b.create_string(&json);
            let membership = MembershipPayload::create(b, &MembershipPayloadArgs {
                membership_json: Some(json_off),
            });
            (EntryPayload::MembershipPayload, membership.as_union_value())
        }
    };

    Ok(Entry::create(b, &EntryArgs {
        log_id: Some(log_id),
        payload_type,
        payload: Some(payload),
    }))
}
```

---

## TCP server (`src/storage/tcp_server.rs`)

Accepts incoming connections from peer nodes, dispatches messages to the local Raft instance.

```rust
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tracing::{info, warn, instrument};
use crate::{
    error::NetworkError,
    storage::{
        type_config::TypeConfig,
        framing::{read_framed, write_framed},
        raft_network_generated::raft_net::*,
        network_codec::*,
    },
};

pub struct TcpRaftServer {
    pub raft: Arc<openraft::Raft<TypeConfig>>,
}

impl TcpRaftServer {
    pub fn new(raft: Arc<openraft::Raft<TypeConfig>>) -> Arc<Self> {
        Arc::new(Self { raft })
    }

    /// Binds and starts accepting connections. Run this in a spawned task.
    pub async fn listen(self: Arc<Self>, addr: &str) -> Result<(), NetworkError> {
        let listener = TcpListener::bind(addr).await?;
        info!(addr, "Raft TCP server listening");

        loop {
            let (stream, peer) = listener.accept().await?;
            stream.set_nodelay(true)?;
            let server = self.clone();
            tokio::spawn(async move {
                if let Err(e) = server.handle_conn(stream).await {
                    warn!(peer = %peer, err = %e, "Raft peer connection closed");
                }
            });
        }
    }

    /// Processes messages on a single accepted connection until it closes.
    async fn handle_conn(&self, mut stream: TcpStream) -> Result<(), NetworkError> {
        loop {
            let req_bytes = read_framed(&mut stream).await?;
            let resp_bytes = self.dispatch(&req_bytes).await?;
            write_framed(&mut stream, &resp_bytes).await?;
        }
    }

    #[instrument(skip(self, bytes), err)]
    async fn dispatch(&self, bytes: &[u8]) -> Result<Vec<u8>, NetworkError> {
        let envelope = flatbuffers::root::<Envelope>(bytes)
            .map_err(|e| NetworkError::Deserialize(e.to_string()))?;

        match envelope.message_type() {
            RaftMessage::AppendEntriesRequest => {
                let rpc = decode_append_entries_request(
                    envelope.message_as_append_entries_request().unwrap(),
                )?;
                let resp = self.raft.append_entries(rpc).await
                    .map_err(|e| NetworkError::Deserialize(e.to_string()))?;
                Ok(encode_append_entries_response(&resp))
            }
            RaftMessage::VoteRequest => {
                let rpc = decode_vote_request(
                    envelope.message_as_vote_request().unwrap(),
                )?;
                let resp = self.raft.vote(rpc).await
                    .map_err(|e| NetworkError::Deserialize(e.to_string()))?;
                Ok(encode_vote_response(&resp))
            }
            RaftMessage::InstallSnapshotRequest => {
                let rpc = decode_install_snapshot_request(
                    envelope.message_as_install_snapshot_request().unwrap(),
                )?;
                let resp = self.raft.install_snapshot(None, rpc).await
                    .map_err(|e| NetworkError::Deserialize(e.to_string()))?;
                Ok(encode_install_snapshot_response(&resp))
            }
            t => Err(NetworkError::InvalidMessageType(t as u8)),
        }
    }
}
```

---

## Module layout (`src/storage/mod.rs`)

```rust
pub mod command;
pub mod type_config;
pub mod log_store;
pub mod state_machine;
pub mod framing;
pub mod network;
pub mod network_codec;
pub mod tcp_server;

#[allow(warnings, clippy::all)]
mod raft_network_generated {
    include!(concat!(env!("OUT_DIR"), "/raft_network_generated.rs"));
}
pub use raft_network_generated::raft_net::*;
```

---

## Notes

- **Connection lifecycle**: `PeerConn::send` holds `Mutex<Option<TcpStream>>` for the full
  roundtrip. openraft does not send concurrent RPCs to the same peer, so this is safe and
  avoids head-of-line blocking.
- **Reconnect on error**: a single reconnect attempt is made before propagating the error
  to openraft. openraft will retry at its own cadence.
- **TCP_NODELAY**: set on every connection to avoid Nagle's algorithm delaying small Raft
  heartbeat messages.
- **Membership serialization**: openraft's `Membership<u64, Node>` is complex (maps of sets).
  The `MembershipPayload` table carries it as a JSON string to avoid duplicating its structure
  in the FlatBuffers schema.
- **No TLS**: omitted as in the original; add as a follow-up (wrap `TcpStream` with
  `tokio-rustls`).

---

## Verification

```bash
# flatc must be on PATH (install via package manager or https://github.com/google/flatbuffers)
flatc --version

cargo check -p idempotency-server
# Network must implement RaftNetwork<TypeConfig>
# NetworkFactory must implement RaftNetworkFactory<TypeConfig>
# TcpRaftServer::listen must compile
```
