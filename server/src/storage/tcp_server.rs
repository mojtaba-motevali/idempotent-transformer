use std::sync::Arc;

use tokio::net::{TcpListener, TcpStream};
use tracing::{info, warn};

use crate::{
    error::NetworkError,
    storage::{
        framing::{read_framed, write_framed},
        network_codec::{
            decode_append_entries_request, decode_install_snapshot_request, decode_message_type,
            decode_vote_request, encode_append_entries_response, encode_install_snapshot_response,
            encode_vote_response, MSG_APPEND_ENTRIES_REQ, MSG_INSTALL_SNAPSHOT_REQ, MSG_VOTE_REQ,
        },
        type_config::TypeConfig,
    },
};

pub struct TcpRaftServer {
    pub raft: Arc<openraft::Raft<TypeConfig>>,
}

impl TcpRaftServer {
    pub fn new(raft: Arc<openraft::Raft<TypeConfig>>) -> Arc<Self> {
        Arc::new(Self { raft })
    }

    /// Bind `addr` and accept Raft TCP connections. Run this in a spawned task.
    pub async fn listen(self: Arc<Self>, addr: &str) -> Result<(), NetworkError> {
        let listener = TcpListener::bind(addr).await?;
        info!(addr, "Raft TCP server listening");

        loop {
            let (stream, peer_addr) = listener.accept().await?;
            stream.set_nodelay(true).map_err(NetworkError::Io)?;

            let server = Arc::clone(&self);
            tokio::spawn(async move {
                if let Err(e) = server.handle_conn(stream).await {
                    warn!(peer = %peer_addr, err = %e, "Raft peer connection closed with error");
                }
            });
        }
    }

    /// Process all messages on a single accepted connection until it closes.
    async fn handle_conn(&self, mut stream: TcpStream) -> Result<(), NetworkError> {
        loop {
            let (req_id, req_bytes) = read_framed(&mut stream).await?;
            let resp_bytes = self.dispatch(&req_bytes).await?;
            // Echo req_id so the client's reader task can route the response.
            write_framed(&mut stream, req_id, &resp_bytes).await?;
        }
    }

    /// Decode the message, call the local Raft instance, encode the response.
    async fn dispatch(&self, bytes: &[u8]) -> Result<Vec<u8>, NetworkError> {
        let msg_type = decode_message_type(bytes)?;

        match msg_type {
            MSG_APPEND_ENTRIES_REQ => {
                let rpc = decode_append_entries_request(bytes)?;
                let resp = self
                    .raft
                    .append_entries(rpc)
                    .await
                    .map_err(|e| NetworkError::Serialize(e.to_string()))?;
                encode_append_entries_response(&resp)
            }
            MSG_VOTE_REQ => {
                let rpc = decode_vote_request(bytes)?;
                let resp = self
                    .raft
                    .vote(rpc)
                    .await
                    .map_err(|e| NetworkError::Serialize(e.to_string()))?;
                encode_vote_response(&resp)
            }
            MSG_INSTALL_SNAPSHOT_REQ => {
                let rpc = decode_install_snapshot_request(bytes)?;
                let resp = self
                    .raft
                    .install_snapshot(rpc)
                    .await
                    .map_err(|e| NetworkError::Serialize(e.to_string()))?;
                encode_install_snapshot_response(&resp)
            }
            other => Err(NetworkError::InvalidMessageType(other)),
        }
    }
}
