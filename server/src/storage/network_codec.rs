use openraft::raft::{
    AppendEntriesRequest, AppendEntriesResponse, InstallSnapshotRequest,
    InstallSnapshotResponse, VoteRequest, VoteResponse,
};

use crate::error::NetworkError;
use crate::storage::rkyv_dto::{
    aligned, encode_rkyv, AppendEntriesRequestDto, AppendEntriesResponseDto,
    InstallSnapshotRequestDto, InstallSnapshotResponseDto, VoteRequestDto, VoteResponseDto,
};
use crate::storage::type_config::{NodeId, TypeConfig};

// ── Message-type discriminants ────────────────────────────────────────────────
pub const MSG_APPEND_ENTRIES_REQ: u8 = 1;
pub const MSG_APPEND_ENTRIES_RESP: u8 = 2;
pub const MSG_VOTE_REQ: u8 = 3;
pub const MSG_VOTE_RESP: u8 = 4;
pub const MSG_INSTALL_SNAPSHOT_REQ: u8 = 5;
pub const MSG_INSTALL_SNAPSHOT_RESP: u8 = 6;

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Prepend the 1-byte message-type discriminant to an rkyv body.
fn with_discriminant(discriminant: u8, body: Vec<u8>) -> Vec<u8> {
    let mut out = Vec::with_capacity(1 + body.len());
    out.push(discriminant);
    out.extend_from_slice(&body);
    out
}

/// Validate the leading discriminant and return the rkyv body (`&[u8]`).
fn split_discriminant(expected: u8, bytes: &[u8]) -> Result<&[u8], NetworkError> {
    match bytes.first() {
        None => Err(NetworkError::Deserialize("empty message".into())),
        Some(&d) if d != expected => Err(NetworkError::InvalidMessageType(d)),
        Some(_) => Ok(&bytes[1..]),
    }
}

/// Decode an rkyv DTO from a discriminant-tagged frame.
macro_rules! decode_dto {
    ($ty:ty, $expected:expr, $bytes:expr) => {{
        let body = split_discriminant($expected, $bytes)?;
        rkyv::from_bytes::<$ty, rkyv::rancor::Error>(&aligned(body))
            .map_err(|e| NetworkError::Deserialize(e.to_string()))?
    }};
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
    let body = encode_rkyv(&AppendEntriesRequestDto::from(rpc))?;
    Ok(with_discriminant(MSG_APPEND_ENTRIES_REQ, body))
}

pub fn decode_append_entries_response(
    bytes: &[u8],
) -> Result<AppendEntriesResponse<NodeId>, NetworkError> {
    let dto: AppendEntriesResponseDto =
        decode_dto!(AppendEntriesResponseDto, MSG_APPEND_ENTRIES_RESP, bytes);
    Ok(dto.into())
}

pub fn encode_vote_request(rpc: &VoteRequest<NodeId>) -> Result<Vec<u8>, NetworkError> {
    let body = encode_rkyv(&VoteRequestDto::from(rpc))?;
    Ok(with_discriminant(MSG_VOTE_REQ, body))
}

pub fn decode_vote_response(bytes: &[u8]) -> Result<VoteResponse<NodeId>, NetworkError> {
    let dto: VoteResponseDto = decode_dto!(VoteResponseDto, MSG_VOTE_RESP, bytes);
    Ok(dto.into())
}

pub fn encode_install_snapshot_request(
    rpc: &InstallSnapshotRequest<TypeConfig>,
) -> Result<Vec<u8>, NetworkError> {
    let body = encode_rkyv(&InstallSnapshotRequestDto::from(rpc))?;
    Ok(with_discriminant(MSG_INSTALL_SNAPSHOT_REQ, body))
}

pub fn decode_install_snapshot_response(
    bytes: &[u8],
) -> Result<InstallSnapshotResponse<NodeId>, NetworkError> {
    let dto: InstallSnapshotResponseDto =
        decode_dto!(InstallSnapshotResponseDto, MSG_INSTALL_SNAPSHOT_RESP, bytes);
    Ok(dto.into())
}

// ── Server-side decode/encode ─────────────────────────────────────────────────

pub fn decode_append_entries_request(
    bytes: &[u8],
) -> Result<AppendEntriesRequest<TypeConfig>, NetworkError> {
    let dto: AppendEntriesRequestDto =
        decode_dto!(AppendEntriesRequestDto, MSG_APPEND_ENTRIES_REQ, bytes);
    Ok(dto.into())
}

pub fn decode_vote_request(bytes: &[u8]) -> Result<VoteRequest<NodeId>, NetworkError> {
    let dto: VoteRequestDto = decode_dto!(VoteRequestDto, MSG_VOTE_REQ, bytes);
    Ok(dto.into())
}

pub fn decode_install_snapshot_request(
    bytes: &[u8],
) -> Result<InstallSnapshotRequest<TypeConfig>, NetworkError> {
    let dto: InstallSnapshotRequestDto =
        decode_dto!(InstallSnapshotRequestDto, MSG_INSTALL_SNAPSHOT_REQ, bytes);
    Ok(dto.into())
}

pub fn encode_append_entries_response(
    resp: &AppendEntriesResponse<NodeId>,
) -> Result<Vec<u8>, NetworkError> {
    let body = encode_rkyv(&AppendEntriesResponseDto::from(resp))?;
    Ok(with_discriminant(MSG_APPEND_ENTRIES_RESP, body))
}

pub fn encode_vote_response(resp: &VoteResponse<NodeId>) -> Result<Vec<u8>, NetworkError> {
    let body = encode_rkyv(&VoteResponseDto::from(resp))?;
    Ok(with_discriminant(MSG_VOTE_RESP, body))
}

pub fn encode_install_snapshot_response(
    resp: &InstallSnapshotResponse<NodeId>,
) -> Result<Vec<u8>, NetworkError> {
    let body = encode_rkyv(&InstallSnapshotResponseDto::from(resp))?;
    Ok(with_discriminant(MSG_INSTALL_SNAPSHOT_RESP, body))
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use openraft::{
        Entry, EntryPayload, LeaderId, LogId, Membership, SnapshotMeta, StoredMembership, Vote,
    };
    use serde::Serialize;

    use super::*;
    use crate::storage::command::WorkflowCommand;
    use crate::storage::type_config::Node;

    /// rkyv encode/decode is lossy-free iff the openraft type serializes
    /// identically before and after the round-trip.  Comparing serde_json
    /// strings sidesteps the missing `PartialEq` on `WorkflowCommand`.
    fn assert_json_eq<T: Serialize>(a: &T, b: &T) {
        assert_eq!(
            serde_json::to_string(a).unwrap(),
            serde_json::to_string(b).unwrap()
        );
    }

    fn log_id(term: u64, node: u64, index: u64) -> LogId<NodeId> {
        LogId::new(LeaderId::new(term, node), index)
    }

    fn entry() -> Entry<TypeConfig> {
        Entry {
            log_id: log_id(2, 1, 7),
            payload: EntryPayload::Normal(WorkflowCommand::UpsertCheckpoint {
                workflow_id: "wf".into(),
                position: 3,
                idempotency_key: "key".into(),
                value: Some(vec![1, 2, 3]),
                created_at: 42,
            }),
        }
    }

    fn membership() -> StoredMembership<NodeId, Node> {
        let m = Membership::new(
            vec![BTreeSet::from([1u64, 2u64])],
            BTreeMap::from([
                (1u64, Node { addr_raft: "a:1".into() }),
                (2u64, Node { addr_raft: "b:2".into() }),
            ]),
        );
        StoredMembership::new(Some(log_id(1, 1, 0)), m)
    }

    #[test]
    fn append_entries_request_roundtrips() {
        let rpc = AppendEntriesRequest {
            vote: Vote::new_committed(3, 1),
            prev_log_id: Some(log_id(1, 1, 0)),
            entries: vec![entry(), entry()],
            leader_commit: Some(log_id(2, 1, 5)),
        };
        let bytes = encode_append_entries_request(&rpc).unwrap();
        let back = decode_append_entries_request(&bytes).unwrap();
        assert_json_eq(&rpc, &back);
    }

    #[test]
    fn append_entries_response_roundtrips() {
        for resp in [
            AppendEntriesResponse::Success,
            AppendEntriesResponse::PartialSuccess(Some(log_id(1, 1, 2))),
            AppendEntriesResponse::Conflict,
            AppendEntriesResponse::HigherVote(Vote::new_committed(9, 2)),
        ] {
            let bytes = encode_append_entries_response(&resp).unwrap();
            let back = decode_append_entries_response(&bytes).unwrap();
            assert_json_eq(&resp, &back);
        }
    }

    #[test]
    fn vote_request_response_roundtrips() {
        let req = VoteRequest {
            vote: Vote::new(4, 1),
            last_log_id: Some(log_id(3, 1, 8)),
        };
        let bytes = encode_vote_request(&req).unwrap();
        assert_json_eq(&req, &decode_vote_request(&bytes).unwrap());

        let resp = VoteResponse {
            vote: Vote::new_committed(4, 1),
            vote_granted: true,
            last_log_id: Some(log_id(3, 1, 8)),
        };
        let bytes = encode_vote_response(&resp).unwrap();
        assert_json_eq(&resp, &decode_vote_response(&bytes).unwrap());
    }

    #[test]
    fn install_snapshot_roundtrips() {
        let req = InstallSnapshotRequest {
            vote: Vote::new_committed(5, 1),
            meta: SnapshotMeta {
                last_log_id: Some(log_id(5, 1, 100)),
                last_membership: membership(),
                snapshot_id: "snap-1".into(),
            },
            offset: 0,
            data: vec![9, 8, 7, 6],
            done: true,
        };
        let bytes = encode_install_snapshot_request(&req).unwrap();
        assert_json_eq(&req, &decode_install_snapshot_request(&bytes).unwrap());

        let resp = InstallSnapshotResponse { vote: Vote::new_committed(5, 1) };
        let bytes = encode_install_snapshot_response(&resp).unwrap();
        assert_json_eq(&resp, &decode_install_snapshot_response(&bytes).unwrap());
    }

    #[test]
    fn wrong_discriminant_is_rejected() {
        let bytes = encode_vote_request(&VoteRequest {
            vote: Vote::new(1, 1),
            last_log_id: None,
        })
        .unwrap();
        // Decoding as the wrong message type must fail, not mis-parse.
        assert!(decode_append_entries_request(&bytes).is_err());
    }
}
