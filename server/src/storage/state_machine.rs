use std::collections::{BTreeMap, HashMap, hash_map::Entry as HEntry};
use std::io::ErrorKind;
use std::path::PathBuf;
use std::sync::Arc;

use openraft::{
    EntryPayload, LogId, RaftSnapshotBuilder, RaftTypeConfig, SnapshotMeta, StorageError,
    StorageIOError, StoredMembership,
};
use openraft::storage::RaftStateMachine;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use crate::schema::{
    checkpoint::Checkpoint,
    workflow::{Workflow, WorkflowStatus},
};
use crate::storage::command::{WorkflowCommand, WorkflowResponse};
use crate::storage::type_config::{Node, TypeConfig};

// ── Application data store ─────────────────────────────────────────────────

/// All application state held in RAM.
///
/// Key layout:
///   workflows:      workflow_id  → Workflow
///   fencing_tokens: workflow_id  → monotonic i64
///   checkpoints:    (workflow_id, position) → Checkpoint  (BTreeMap for ordered prefix scan)
pub struct Store {
    pub(crate) workflows:      HashMap<String, Workflow>,
    pub(crate) fencing_tokens: HashMap<String, i64>,
    pub(crate) checkpoints:    BTreeMap<(String, i64), Checkpoint>,
}

impl Store {
    fn new() -> Self {
        Self {
            workflows:      HashMap::new(),
            fencing_tokens: HashMap::new(),
            checkpoints:    BTreeMap::new(),
        }
    }
}

/// Arc-wrapped, parking_lot-protected store shared between the state machine
/// and the repository layer.
pub type SharedStore = Arc<Mutex<Store>>;

// ── Snapshot content ───────────────────────────────────────────────────────

/// JSON-serializable representation of the full state machine snapshot.
/// Raft metadata is embedded so the file is self-contained on restart.
#[derive(Serialize, Deserialize)]
struct SnapshotContent {
    workflows:       Vec<Workflow>,
    fencing_tokens:  Vec<(String, i64)>,
    checkpoints:     Vec<Checkpoint>,
    last_applied:    Option<LogId<u64>>,
    last_membership: StoredMembership<u64, Node>,
}

// ── Error helper ───────────────────────────────────────────────────────────

fn sm_err(e: &dyn std::error::Error) -> StorageError<u64> {
    StorageError::IO {
        source: StorageIOError::write_state_machine(&std::io::Error::new(
            ErrorKind::Other,
            e.to_string(),
        )),
    }
}

// ── Command application ────────────────────────────────────────────────────

/// Applies one Raft command to the in-memory store.  Pure and synchronous —
/// the caller holds a `parking_lot::MutexGuard<Store>` for the duration.
fn apply_command(store: &mut Store, cmd: WorkflowCommand) -> WorkflowResponse {
    match cmd {
        WorkflowCommand::CreateOrGetWorkflow { workflow_id, name, created_at } => {
            match store.workflows.entry(workflow_id) {
                HEntry::Occupied(mut o) => {
                    if name.is_some() {
                        o.get_mut().name = name;
                    }
                }
                HEntry::Vacant(v) => {
                    let id = v.key().clone();
                    v.insert(Workflow {
                        id,
                        status: WorkflowStatus::Running as i64,
                        name,
                        expire_at: None,
                        completed_at: None,
                        created_at,
                    });
                }
            }
            WorkflowResponse::Empty
        }

        WorkflowCommand::IncrementFencingToken { workflow_id } => {
            let token = store
                .fencing_tokens
                .entry(workflow_id)
                .and_modify(|t| *t += 1)
                .or_insert(1);
            WorkflowResponse::FencingToken(*token)
        }

        WorkflowCommand::UpsertCheckpoint {
            workflow_id,
            position,
            idempotency_key,
            value,
            created_at,
        } => {
            store.checkpoints.insert(
                (workflow_id.clone(), position),
                Checkpoint { workflow_id, position, value, idempotency_key, created_at },
            );
            WorkflowResponse::Empty
        }

        WorkflowCommand::CompleteWorkflow { workflow_id, expire_at, completed_at } => {
            if let Some(wf) = store.workflows.get_mut(&workflow_id) {
                wf.status = WorkflowStatus::Completed as i64;
                wf.expire_at = Some(expire_at);
                wf.completed_at = Some(completed_at);
            }
            WorkflowResponse::Empty
        }

        WorkflowCommand::DeleteExpired { before_ms } => {
            let expired: Vec<String> = store
                .workflows
                .values()
                .filter(|wf| {
                    wf.status == WorkflowStatus::Completed as i64
                        && wf.expire_at.map_or(false, |e| e < before_ms)
                })
                .map(|wf| wf.id.clone())
                .collect();

            for id in &expired {
                store.workflows.remove(id);
                store.fencing_tokens.remove(id);
            }

            let expired_set: std::collections::HashSet<&str> =
                expired.iter().map(String::as_str).collect();
            store
                .checkpoints
                .retain(|(wf_id, _), _| !expired_set.contains(wf_id.as_str()));

            WorkflowResponse::Empty
        }
    }
}

// ── SnapshotBuilder ────────────────────────────────────────────────────────

/// Holds a consistent point-in-time copy of the store captured at
/// `get_snapshot_builder()` time.  `build_snapshot()` serializes it and
/// writes the file — no store lock needed.
pub struct StateSnapshotBuilder {
    snapshot_workflows:      Vec<Workflow>,
    snapshot_fencing_tokens: Vec<(String, i64)>,
    snapshot_checkpoints:    Vec<Checkpoint>,
    last_applied:            Option<LogId<u64>>,
    last_membership:         StoredMembership<u64, Node>,
    /// Shared with `StateMachine` so `get_current_snapshot()` sees the result.
    last_snapshot: Arc<Mutex<Option<(SnapshotMeta<u64, Node>, Vec<u8>)>>>,
    snapshot_path: PathBuf,
}

impl RaftSnapshotBuilder<TypeConfig> for StateSnapshotBuilder {
    async fn build_snapshot(
        &mut self,
    ) -> Result<openraft::Snapshot<TypeConfig>, StorageError<u64>> {
        let content = SnapshotContent {
            workflows:       self.snapshot_workflows.clone(),
            fencing_tokens:  self.snapshot_fencing_tokens.clone(),
            checkpoints:     self.snapshot_checkpoints.clone(),
            last_applied:    self.last_applied,
            last_membership: self.last_membership.clone(),
        };

        let bytes = serde_json::to_vec(&content).map_err(|e| sm_err(&e))?;

        let snapshot_id = self
            .last_applied
            .map(|l| format!("snap-{}", l.index))
            .unwrap_or_else(|| "snap-0".to_string());

        let meta = SnapshotMeta {
            last_log_id:     self.last_applied,
            last_membership: self.last_membership.clone(),
            snapshot_id,
        };

        // Persist to disk.  After this returns successfully, openraft calls
        // LogStore::purge(meta.last_log_id) to vacuum committed log entries.
        std::fs::write(&self.snapshot_path, &bytes).map_err(|e| sm_err(&e))?;

        *self.last_snapshot.lock() = Some((meta.clone(), bytes.clone()));

        Ok(openraft::Snapshot {
            meta,
            snapshot: Box::new(std::io::Cursor::new(bytes)),
        })
    }
}

// ── StateMachine ───────────────────────────────────────────────────────────

pub struct StateMachine {
    store:           SharedStore,
    /// Updated after every applied entry — inline write, no await while held.
    last_applied:    Mutex<Option<LogId<u64>>>,
    /// Updated for Membership entries — inline write, no await while held.
    last_membership: Mutex<StoredMembership<u64, Node>>,
    /// Shared with `StateSnapshotBuilder` via `Arc`.
    last_snapshot:   Arc<Mutex<Option<(SnapshotMeta<u64, Node>, Vec<u8>)>>>,
    snapshot_path:   PathBuf,
}

impl StateMachine {
    /// Initialises the state machine, restoring from `{data_dir}/snapshot.bin`
    /// if present.  Returns `(StateMachine, SharedStore)` — the store handle
    /// is what repositories receive.
    pub fn new(data_dir: &str) -> anyhow::Result<(Self, SharedStore)> {
        let snapshot_path = PathBuf::from(data_dir).join("snapshot.bin");
        std::fs::create_dir_all(data_dir)?;

        let mut store = Store::new();
        let mut last_applied: Option<LogId<u64>> = None;
        let mut last_membership = StoredMembership::default();
        let mut cached_snapshot: Option<(SnapshotMeta<u64, Node>, Vec<u8>)> = None;

        if snapshot_path.exists() {
            match std::fs::read(&snapshot_path) {
                Ok(bytes) => match serde_json::from_slice::<SnapshotContent>(&bytes) {
                    Ok(content) => {
                        for wf in content.workflows {
                            store.workflows.insert(wf.id.clone(), wf);
                        }
                        for (id, token) in content.fencing_tokens {
                            store.fencing_tokens.insert(id, token);
                        }
                        for cp in content.checkpoints {
                            store
                                .checkpoints
                                .insert((cp.workflow_id.clone(), cp.position), cp);
                        }
                        last_applied   = content.last_applied;
                        last_membership = content.last_membership;

                        let snapshot_id = last_applied
                            .map(|l| format!("snap-{}", l.index))
                            .unwrap_or_else(|| "snap-0".to_string());
                        let meta = SnapshotMeta {
                            last_log_id:     last_applied,
                            last_membership: last_membership.clone(),
                            snapshot_id,
                        };
                        cached_snapshot = Some((meta, bytes));
                        tracing::info!(
                            last_applied = ?last_applied,
                            "state machine restored from snapshot"
                        );
                    }
                    Err(e) => {
                        tracing::warn!(
                            err = %e,
                            path = %snapshot_path.display(),
                            "snapshot file is unreadable, starting fresh"
                        );
                    }
                },
                Err(e) => {
                    tracing::warn!(
                        err = %e,
                        path = %snapshot_path.display(),
                        "could not read snapshot file, starting fresh"
                    );
                }
            }
        }

        let shared_store = Arc::new(Mutex::new(store));

        let sm = Self {
            store:           shared_store.clone(),
            last_applied:    Mutex::new(last_applied),
            last_membership: Mutex::new(last_membership),
            last_snapshot:   Arc::new(Mutex::new(cached_snapshot)),
            snapshot_path,
        };

        Ok((sm, shared_store))
    }
}

// ── RaftStateMachine impl ──────────────────────────────────────────────────

impl RaftStateMachine<TypeConfig> for StateMachine {
    type SnapshotBuilder = StateSnapshotBuilder;

    async fn applied_state(
        &mut self,
    ) -> Result<(Option<LogId<u64>>, StoredMembership<u64, Node>), StorageError<u64>> {
        Ok((*self.last_applied.lock(), self.last_membership.lock().clone()))
    }

    async fn apply<I>(
        &mut self,
        entries: I,
    ) -> Result<Vec<WorkflowResponse>, StorageError<u64>>
    where
        I: IntoIterator<Item = openraft::Entry<TypeConfig>> + Send,
    {
        let mut responses = Vec::new();

        for entry in entries {
            let log_id = entry.log_id;

            let response = match entry.payload {
                EntryPayload::Normal(cmd) => {
                    // Lock is held only for the synchronous in-memory mutation.
                    // No await inside → parking_lot::Mutex is appropriate.
                    let mut store = self.store.lock();
                    apply_command(&mut store, cmd)
                } // lock released here

                EntryPayload::Membership(m) => {
                    *self.last_membership.lock() = StoredMembership::new(Some(log_id), m);
                    WorkflowResponse::Empty
                }

                EntryPayload::Blank => WorkflowResponse::Empty,
            };

            *self.last_applied.lock() = Some(log_id);
            responses.push(response);
        }

        Ok(responses)
    }

    async fn get_snapshot_builder(&mut self) -> Self::SnapshotBuilder {
        // Clone the store under a brief lock, then read metadata separately.
        // No await is possible here, so parking_lot is safe.
        let (snapshot_workflows, snapshot_fencing_tokens, snapshot_checkpoints) = {
            let store = self.store.lock();
            (
                store.workflows.values().cloned().collect(),
                store.fencing_tokens.iter().map(|(k, &v)| (k.clone(), v)).collect(),
                store.checkpoints.values().cloned().collect(),
            )
        }; // store lock released before reading the other mutexes

        StateSnapshotBuilder {
            snapshot_workflows,
            snapshot_fencing_tokens,
            snapshot_checkpoints,
            last_applied:    *self.last_applied.lock(),
            last_membership: self.last_membership.lock().clone(),
            last_snapshot:   self.last_snapshot.clone(),
            snapshot_path:   self.snapshot_path.clone(),
        }
    }

    async fn begin_receiving_snapshot(
        &mut self,
    ) -> Result<Box<<TypeConfig as RaftTypeConfig>::SnapshotData>, StorageError<u64>> {
        Ok(Box::new(std::io::Cursor::new(Vec::new())))
    }

    async fn install_snapshot(
        &mut self,
        meta: &SnapshotMeta<u64, Node>,
        snapshot: Box<<TypeConfig as RaftTypeConfig>::SnapshotData>,
    ) -> Result<(), StorageError<u64>> {
        let bytes = snapshot.into_inner();
        let content: SnapshotContent =
            serde_json::from_slice(&bytes).map_err(|e| sm_err(&e))?;

        // Replace store contents atomically under a single lock acquisition.
        {
            let mut store = self.store.lock();
            store.workflows.clear();
            store.fencing_tokens.clear();
            store.checkpoints.clear();

            for wf in content.workflows {
                store.workflows.insert(wf.id.clone(), wf);
            }
            for (id, token) in content.fencing_tokens {
                store.fencing_tokens.insert(id, token);
            }
            for cp in content.checkpoints {
                store
                    .checkpoints
                    .insert((cp.workflow_id.clone(), cp.position), cp);
            }
        } // store lock released

        *self.last_applied.lock()    = meta.last_log_id;
        *self.last_membership.lock() = meta.last_membership.clone();

        // Persist so this snapshot is available on the next restart.
        std::fs::write(&self.snapshot_path, &bytes).map_err(|e| sm_err(&e))?;

        *self.last_snapshot.lock() = Some((meta.clone(), bytes));

        Ok(())
    }

    async fn get_current_snapshot(
        &mut self,
    ) -> Result<Option<openraft::Snapshot<TypeConfig>>, StorageError<u64>> {
        let guard = self.last_snapshot.lock();
        Ok(guard.as_ref().map(|(meta, bytes)| openraft::Snapshot {
            meta:     meta.clone(),
            snapshot: Box::new(std::io::Cursor::new(bytes.clone())),
        }))
    }
}
