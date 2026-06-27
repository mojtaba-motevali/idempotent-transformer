//! rkyv data-transfer objects mirroring the openraft types that cross the
//! Raft network wire and live in the fjall log store.
//!
//! openraft's own types (`Entry`, `Vote`, `LogId`, the RPC request/response
//! structs, …) only implement `serde`, not rkyv's `Archive`/`Serialize`.  To
//! get rkyv's fast binary encoding we mirror each type with an owned DTO that
//! derives the rkyv traits and convert at the boundary via `From`.
//!
//! NodeId is `u64` and the only `Node` shape is [`Node`], so every conversion
//! here is concrete and lossless.

use std::collections::{BTreeMap, BTreeSet};

use openraft::raft::{
    AppendEntriesRequest, AppendEntriesResponse, InstallSnapshotRequest,
    InstallSnapshotResponse, VoteRequest, VoteResponse,
};
use openraft::{
    Entry, EntryPayload, LeaderId, LogId, Membership, SnapshotMeta, StoredMembership, Vote,
};

use crate::error::NetworkError;
use crate::storage::command::WorkflowCommand;
use crate::storage::type_config::{Node, NodeId, TypeConfig};

// ── DTO definitions ───────────────────────────────────────────────────────────

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct LeaderIdDto {
    pub term: u64,
    pub node_id: u64,
}

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct VoteDto {
    pub leader_id: LeaderIdDto,
    pub committed: bool,
}

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct LogIdDto {
    pub leader_id: LeaderIdDto,
    pub index: u64,
}

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct NodeDto {
    pub addr_raft: String,
}

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct MembershipDto {
    pub configs: Vec<Vec<u64>>,
    pub nodes: Vec<(u64, NodeDto)>,
}

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct StoredMembershipDto {
    pub log_id: Option<LogIdDto>,
    pub membership: MembershipDto,
}

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub enum WorkflowCommandDto {
    CreateOrGetWorkflow {
        workflow_id: String,
        name: Option<String>,
        created_at: i64,
    },
    IncrementFencingToken {
        workflow_id: String,
    },
    UpsertCheckpoint {
        workflow_id: String,
        position: i64,
        idempotency_key: String,
        value: Option<Vec<u8>>,
        created_at: i64,
    },
    CompleteWorkflow {
        workflow_id: String,
        expire_at: i64,
        completed_at: i64,
    },
    DeleteExpired {
        before_ms: i64,
    },
}

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub enum EntryPayloadDto {
    Blank,
    Normal(WorkflowCommandDto),
    Membership(MembershipDto),
}

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct EntryDto {
    pub log_id: LogIdDto,
    pub payload: EntryPayloadDto,
}

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct AppendEntriesRequestDto {
    pub vote: VoteDto,
    pub prev_log_id: Option<LogIdDto>,
    pub entries: Vec<EntryDto>,
    pub leader_commit: Option<LogIdDto>,
}

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub enum AppendEntriesResponseDto {
    Success,
    PartialSuccess(Option<LogIdDto>),
    Conflict,
    HigherVote(VoteDto),
}

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct VoteRequestDto {
    pub vote: VoteDto,
    pub last_log_id: Option<LogIdDto>,
}

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct VoteResponseDto {
    pub vote: VoteDto,
    pub vote_granted: bool,
    pub last_log_id: Option<LogIdDto>,
}

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct SnapshotMetaDto {
    pub last_log_id: Option<LogIdDto>,
    pub last_membership: StoredMembershipDto,
    pub snapshot_id: String,
}

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct InstallSnapshotRequestDto {
    pub vote: VoteDto,
    pub meta: SnapshotMetaDto,
    pub offset: u64,
    pub data: Vec<u8>,
    pub done: bool,
}

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct InstallSnapshotResponseDto {
    pub vote: VoteDto,
}

// ── Leaf conversions ──────────────────────────────────────────────────────────

impl From<&LeaderId<NodeId>> for LeaderIdDto {
    fn from(l: &LeaderId<NodeId>) -> Self {
        Self { term: l.term, node_id: l.node_id }
    }
}
impl From<&LeaderIdDto> for LeaderId<NodeId> {
    fn from(d: &LeaderIdDto) -> Self {
        LeaderId { term: d.term, node_id: d.node_id }
    }
}

impl From<&Vote<NodeId>> for VoteDto {
    fn from(v: &Vote<NodeId>) -> Self {
        Self { leader_id: (&v.leader_id).into(), committed: v.committed }
    }
}
impl From<&VoteDto> for Vote<NodeId> {
    fn from(d: &VoteDto) -> Self {
        Vote { leader_id: (&d.leader_id).into(), committed: d.committed }
    }
}

impl From<&LogId<NodeId>> for LogIdDto {
    fn from(l: &LogId<NodeId>) -> Self {
        Self { leader_id: (&l.leader_id).into(), index: l.index }
    }
}
impl From<&LogIdDto> for LogId<NodeId> {
    fn from(d: &LogIdDto) -> Self {
        LogId { leader_id: (&d.leader_id).into(), index: d.index }
    }
}

impl From<&Node> for NodeDto {
    fn from(n: &Node) -> Self {
        Self { addr_raft: n.addr_raft.clone() }
    }
}
impl From<&NodeDto> for Node {
    fn from(d: &NodeDto) -> Self {
        Node { addr_raft: d.addr_raft.clone() }
    }
}

impl From<&Membership<NodeId, Node>> for MembershipDto {
    fn from(m: &Membership<NodeId, Node>) -> Self {
        let configs = m
            .get_joint_config()
            .iter()
            .map(|set| set.iter().copied().collect::<Vec<u64>>())
            .collect();
        let nodes = m.nodes().map(|(id, n)| (*id, NodeDto::from(n))).collect();
        Self { configs, nodes }
    }
}
impl From<&MembershipDto> for Membership<NodeId, Node> {
    fn from(d: &MembershipDto) -> Self {
        let configs: Vec<BTreeSet<NodeId>> = d
            .configs
            .iter()
            .map(|c| c.iter().copied().collect())
            .collect();
        let nodes: BTreeMap<NodeId, Node> =
            d.nodes.iter().map(|(id, n)| (*id, Node::from(n))).collect();
        // `Membership::new` only ever *adds* default nodes for voter ids missing
        // from `nodes`; every voter is already present here, so this is lossless.
        Membership::new(configs, nodes)
    }
}

impl From<&StoredMembership<NodeId, Node>> for StoredMembershipDto {
    fn from(sm: &StoredMembership<NodeId, Node>) -> Self {
        Self {
            log_id: sm.log_id().as_ref().map(LogIdDto::from),
            membership: MembershipDto::from(sm.membership()),
        }
    }
}
impl From<&StoredMembershipDto> for StoredMembership<NodeId, Node> {
    fn from(d: &StoredMembershipDto) -> Self {
        StoredMembership::new(
            d.log_id.as_ref().map(LogId::from),
            Membership::from(&d.membership),
        )
    }
}

// ── WorkflowCommand ───────────────────────────────────────────────────────────

impl From<&WorkflowCommand> for WorkflowCommandDto {
    fn from(c: &WorkflowCommand) -> Self {
        match c {
            WorkflowCommand::CreateOrGetWorkflow { workflow_id, name, created_at } => {
                Self::CreateOrGetWorkflow {
                    workflow_id: workflow_id.clone(),
                    name: name.clone(),
                    created_at: *created_at,
                }
            }
            WorkflowCommand::IncrementFencingToken { workflow_id } => {
                Self::IncrementFencingToken { workflow_id: workflow_id.clone() }
            }
            WorkflowCommand::UpsertCheckpoint {
                workflow_id,
                position,
                idempotency_key,
                value,
                created_at,
            } => Self::UpsertCheckpoint {
                workflow_id: workflow_id.clone(),
                position: *position,
                idempotency_key: idempotency_key.clone(),
                value: value.clone(),
                created_at: *created_at,
            },
            WorkflowCommand::CompleteWorkflow { workflow_id, expire_at, completed_at } => {
                Self::CompleteWorkflow {
                    workflow_id: workflow_id.clone(),
                    expire_at: *expire_at,
                    completed_at: *completed_at,
                }
            }
            WorkflowCommand::DeleteExpired { before_ms } => {
                Self::DeleteExpired { before_ms: *before_ms }
            }
        }
    }
}
impl From<WorkflowCommandDto> for WorkflowCommand {
    fn from(d: WorkflowCommandDto) -> Self {
        match d {
            WorkflowCommandDto::CreateOrGetWorkflow { workflow_id, name, created_at } => {
                Self::CreateOrGetWorkflow { workflow_id, name, created_at }
            }
            WorkflowCommandDto::IncrementFencingToken { workflow_id } => {
                Self::IncrementFencingToken { workflow_id }
            }
            WorkflowCommandDto::UpsertCheckpoint {
                workflow_id,
                position,
                idempotency_key,
                value,
                created_at,
            } => Self::UpsertCheckpoint {
                workflow_id,
                position,
                idempotency_key,
                value,
                created_at,
            },
            WorkflowCommandDto::CompleteWorkflow { workflow_id, expire_at, completed_at } => {
                Self::CompleteWorkflow { workflow_id, expire_at, completed_at }
            }
            WorkflowCommandDto::DeleteExpired { before_ms } => {
                Self::DeleteExpired { before_ms }
            }
        }
    }
}

// ── Entry / EntryPayload ──────────────────────────────────────────────────────

fn entry_payload_to_dto(p: &EntryPayload<TypeConfig>) -> EntryPayloadDto {
    match p {
        EntryPayload::Blank => EntryPayloadDto::Blank,
        EntryPayload::Normal(cmd) => EntryPayloadDto::Normal(WorkflowCommandDto::from(cmd)),
        EntryPayload::Membership(m) => EntryPayloadDto::Membership(MembershipDto::from(m)),
    }
}
fn entry_payload_from_dto(d: EntryPayloadDto) -> EntryPayload<TypeConfig> {
    match d {
        EntryPayloadDto::Blank => EntryPayload::Blank,
        EntryPayloadDto::Normal(c) => EntryPayload::Normal(WorkflowCommand::from(c)),
        EntryPayloadDto::Membership(m) => EntryPayload::Membership(Membership::from(&m)),
    }
}

impl From<&Entry<TypeConfig>> for EntryDto {
    fn from(e: &Entry<TypeConfig>) -> Self {
        Self {
            log_id: LogIdDto::from(&e.log_id),
            payload: entry_payload_to_dto(&e.payload),
        }
    }
}
impl From<EntryDto> for Entry<TypeConfig> {
    fn from(d: EntryDto) -> Self {
        Entry {
            log_id: LogId::from(&d.log_id),
            payload: entry_payload_from_dto(d.payload),
        }
    }
}

// ── Raft RPC messages ─────────────────────────────────────────────────────────

impl From<&AppendEntriesRequest<TypeConfig>> for AppendEntriesRequestDto {
    fn from(r: &AppendEntriesRequest<TypeConfig>) -> Self {
        Self {
            vote: VoteDto::from(&r.vote),
            prev_log_id: r.prev_log_id.as_ref().map(LogIdDto::from),
            entries: r.entries.iter().map(EntryDto::from).collect(),
            leader_commit: r.leader_commit.as_ref().map(LogIdDto::from),
        }
    }
}
impl From<AppendEntriesRequestDto> for AppendEntriesRequest<TypeConfig> {
    fn from(d: AppendEntriesRequestDto) -> Self {
        AppendEntriesRequest {
            vote: Vote::from(&d.vote),
            prev_log_id: d.prev_log_id.as_ref().map(LogId::from),
            entries: d.entries.into_iter().map(Entry::from).collect(),
            leader_commit: d.leader_commit.as_ref().map(LogId::from),
        }
    }
}

impl From<&AppendEntriesResponse<NodeId>> for AppendEntriesResponseDto {
    fn from(r: &AppendEntriesResponse<NodeId>) -> Self {
        match r {
            AppendEntriesResponse::Success => Self::Success,
            AppendEntriesResponse::PartialSuccess(o) => {
                Self::PartialSuccess(o.as_ref().map(LogIdDto::from))
            }
            AppendEntriesResponse::Conflict => Self::Conflict,
            AppendEntriesResponse::HigherVote(v) => Self::HigherVote(VoteDto::from(v)),
        }
    }
}
impl From<AppendEntriesResponseDto> for AppendEntriesResponse<NodeId> {
    fn from(d: AppendEntriesResponseDto) -> Self {
        match d {
            AppendEntriesResponseDto::Success => Self::Success,
            AppendEntriesResponseDto::PartialSuccess(o) => {
                Self::PartialSuccess(o.as_ref().map(LogId::from))
            }
            AppendEntriesResponseDto::Conflict => Self::Conflict,
            AppendEntriesResponseDto::HigherVote(v) => Self::HigherVote(Vote::from(&v)),
        }
    }
}

impl From<&VoteRequest<NodeId>> for VoteRequestDto {
    fn from(r: &VoteRequest<NodeId>) -> Self {
        Self {
            vote: VoteDto::from(&r.vote),
            last_log_id: r.last_log_id.as_ref().map(LogIdDto::from),
        }
    }
}
impl From<VoteRequestDto> for VoteRequest<NodeId> {
    fn from(d: VoteRequestDto) -> Self {
        VoteRequest {
            vote: Vote::from(&d.vote),
            last_log_id: d.last_log_id.as_ref().map(LogId::from),
        }
    }
}

impl From<&VoteResponse<NodeId>> for VoteResponseDto {
    fn from(r: &VoteResponse<NodeId>) -> Self {
        Self {
            vote: VoteDto::from(&r.vote),
            vote_granted: r.vote_granted,
            last_log_id: r.last_log_id.as_ref().map(LogIdDto::from),
        }
    }
}
impl From<VoteResponseDto> for VoteResponse<NodeId> {
    fn from(d: VoteResponseDto) -> Self {
        VoteResponse {
            vote: Vote::from(&d.vote),
            vote_granted: d.vote_granted,
            last_log_id: d.last_log_id.as_ref().map(LogId::from),
        }
    }
}

impl From<&SnapshotMeta<NodeId, Node>> for SnapshotMetaDto {
    fn from(m: &SnapshotMeta<NodeId, Node>) -> Self {
        Self {
            last_log_id: m.last_log_id.as_ref().map(LogIdDto::from),
            last_membership: StoredMembershipDto::from(&m.last_membership),
            snapshot_id: m.snapshot_id.clone(),
        }
    }
}
impl From<&SnapshotMetaDto> for SnapshotMeta<NodeId, Node> {
    fn from(d: &SnapshotMetaDto) -> Self {
        SnapshotMeta {
            last_log_id: d.last_log_id.as_ref().map(LogId::from),
            last_membership: StoredMembership::from(&d.last_membership),
            snapshot_id: d.snapshot_id.clone(),
        }
    }
}

impl From<&InstallSnapshotRequest<TypeConfig>> for InstallSnapshotRequestDto {
    fn from(r: &InstallSnapshotRequest<TypeConfig>) -> Self {
        Self {
            vote: VoteDto::from(&r.vote),
            meta: SnapshotMetaDto::from(&r.meta),
            offset: r.offset,
            data: r.data.clone(),
            done: r.done,
        }
    }
}
impl From<InstallSnapshotRequestDto> for InstallSnapshotRequest<TypeConfig> {
    fn from(d: InstallSnapshotRequestDto) -> Self {
        InstallSnapshotRequest {
            vote: Vote::from(&d.vote),
            meta: SnapshotMeta::from(&d.meta),
            offset: d.offset,
            data: d.data,
            done: d.done,
        }
    }
}

impl From<&InstallSnapshotResponse<NodeId>> for InstallSnapshotResponseDto {
    fn from(r: &InstallSnapshotResponse<NodeId>) -> Self {
        Self { vote: VoteDto::from(&r.vote) }
    }
}
impl From<InstallSnapshotResponseDto> for InstallSnapshotResponse<NodeId> {
    fn from(d: InstallSnapshotResponseDto) -> Self {
        InstallSnapshotResponse { vote: Vote::from(&d.vote) }
    }
}

// ── rkyv encode / decode helpers ──────────────────────────────────────────────

/// Serialize a DTO to a rkyv byte buffer.
pub fn encode_rkyv<T>(value: &T) -> Result<Vec<u8>, NetworkError>
where
    T: for<'a> rkyv::Serialize<
            rkyv::api::high::HighSerializer<
                rkyv::util::AlignedVec,
                rkyv::ser::allocator::ArenaHandle<'a>,
                rkyv::rancor::Error,
            >,
        >,
{
    rkyv::to_bytes::<rkyv::rancor::Error>(value)
        .map(|b| b.to_vec())
        .map_err(|e| NetworkError::Serialize(e.to_string()))
}

/// Copy a (possibly unaligned) byte slice into an `AlignedVec` so that
/// `rkyv::from_bytes` can validate and access it. fjall values and TCP frames
/// arrive as `&[u8]` with no alignment guarantee; rkyv needs the buffer aligned
/// to the archived type.
pub fn aligned(bytes: &[u8]) -> rkyv::util::AlignedVec {
    let mut buf = rkyv::util::AlignedVec::new();
    buf.extend_from_slice(bytes);
    buf
}
