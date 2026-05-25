use openraft::raft::{
    AppendEntriesRequest, AppendEntriesResponse, InstallSnapshotRequest,
    InstallSnapshotResponse, VoteRequest, VoteResponse,
};

use crate::error::NetworkError;
use crate::storage::type_config::{NodeId, TypeConfig};

// ── Message-type discriminants ────────────────────────────────────────────────
pub const MSG_APPEND_ENTRIES_REQ: u8 = 1;
pub const MSG_APPEND_ENTRIES_RESP: u8 = 2;
pub const MSG_VOTE_REQ: u8 = 3;
pub const MSG_VOTE_RESP: u8 = 4;
pub const MSG_INSTALL_SNAPSHOT_REQ: u8 = 5;
pub const MSG_INSTALL_SNAPSHOT_RESP: u8 = 6;

// ── Helpers ───────────────────────────────────────────────────────────────────

fn encode<T: serde::Serialize>(discriminant: u8, value: &T) -> Result<Vec<u8>, NetworkError> {
    let json = serde_json::to_vec(value)
        .map_err(|e| NetworkError::Serialize(e.to_string()))?;
    let mut out = Vec::with_capacity(1 + json.len());
    out.push(discriminant);
    out.extend_from_slice(&json);
    Ok(out)
}

fn decode<T: serde::de::DeserializeOwned>(
    expected: u8,
    bytes: &[u8],
) -> Result<T, NetworkError> {
    if bytes.is_empty() {
        return Err(NetworkError::Deserialize("empty message".into()));
    }
    let discriminant = bytes[0];
    if discriminant != expected {
        return Err(NetworkError::InvalidMessageType(discriminant));
    }
    serde_json::from_slice(&bytes[1..]).map_err(|e| NetworkError::Deserialize(e.to_string()))
}

// ── Peek at the discriminant without consuming the bytes ──────────────────────

pub fn decode_message_type(bytes: &[u8]) -> Result<u8, NetworkError> {
    bytes
        .first()
        .copied()
        .ok_or_else(|| NetworkError::Deserialize("empty message".into()))
}

// ── Client-side encode/decode ─────────────────────────────────────────────────

pub fn encode_append_entries_request(
    rpc: &AppendEntriesRequest<TypeConfig>,
) -> Result<Vec<u8>, NetworkError> {
    encode(MSG_APPEND_ENTRIES_REQ, rpc)
}

pub fn decode_append_entries_response(
    bytes: &[u8],
) -> Result<AppendEntriesResponse<NodeId>, NetworkError> {
    decode(MSG_APPEND_ENTRIES_RESP, bytes)
}

pub fn encode_vote_request(rpc: &VoteRequest<NodeId>) -> Result<Vec<u8>, NetworkError> {
    encode(MSG_VOTE_REQ, rpc)
}

pub fn decode_vote_response(bytes: &[u8]) -> Result<VoteResponse<NodeId>, NetworkError> {
    decode(MSG_VOTE_RESP, bytes)
}

pub fn encode_install_snapshot_request(
    rpc: &InstallSnapshotRequest<TypeConfig>,
) -> Result<Vec<u8>, NetworkError> {
    encode(MSG_INSTALL_SNAPSHOT_REQ, rpc)
}

pub fn decode_install_snapshot_response(
    bytes: &[u8],
) -> Result<InstallSnapshotResponse<NodeId>, NetworkError> {
    decode(MSG_INSTALL_SNAPSHOT_RESP, bytes)
}

// ── Server-side decode/encode ─────────────────────────────────────────────────

pub fn decode_append_entries_request(
    bytes: &[u8],
) -> Result<AppendEntriesRequest<TypeConfig>, NetworkError> {
    decode(MSG_APPEND_ENTRIES_REQ, bytes)
}

pub fn decode_vote_request(bytes: &[u8]) -> Result<VoteRequest<NodeId>, NetworkError> {
    decode(MSG_VOTE_REQ, bytes)
}

pub fn decode_install_snapshot_request(
    bytes: &[u8],
) -> Result<InstallSnapshotRequest<TypeConfig>, NetworkError> {
    decode(MSG_INSTALL_SNAPSHOT_REQ, bytes)
}

pub fn encode_append_entries_response(
    resp: &AppendEntriesResponse<NodeId>,
) -> Result<Vec<u8>, NetworkError> {
    encode(MSG_APPEND_ENTRIES_RESP, resp)
}

pub fn encode_vote_response(resp: &VoteResponse<NodeId>) -> Result<Vec<u8>, NetworkError> {
    encode(MSG_VOTE_RESP, resp)
}

pub fn encode_install_snapshot_response(
    resp: &InstallSnapshotResponse<NodeId>,
) -> Result<Vec<u8>, NetworkError> {
    encode(MSG_INSTALL_SNAPSHOT_RESP, resp)
}
