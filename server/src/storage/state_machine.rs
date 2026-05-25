use std::io::ErrorKind;
use std::sync::Arc;

use openraft::{
    EntryPayload, LogId, RaftSnapshotBuilder, RaftTypeConfig, SnapshotMeta, StorageError,
    StorageIOError, StoredMembership,
};
use openraft::storage::RaftStateMachine;
use rocksdb::{ColumnFamilyDescriptor, IteratorMode, Options, DB};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::repositories::{
    checkpoints::CheckpointRepository,
    workflows::WorkflowRepository,
    workflows_fencing_tokens::FencingTokenRepository,
};
use crate::schema::{checkpoint::Checkpoint, workflow::Workflow};
use crate::storage::command::{WorkflowCommand, WorkflowResponse};
use crate::storage::type_config::{Node, TypeConfig};

// ── Column family names ───────────────────────────────────────────────────────

pub const CF_WORKFLOWS: &str = "workflows";
pub const CF_FENCING_TOKENS: &str = "fencing_tokens";
pub const CF_CHECKPOINTS: &str = "checkpoints";
pub const CF_RAFT_META: &str = "raft_meta";

// ── Snapshot content ──────────────────────────────────────────────────────────

/// Full state machine snapshot: all data + Raft metadata.
#[derive(Serialize, Deserialize)]
struct SnapshotContent {
    workflows: Vec<Workflow>,
    fencing_tokens: Vec<(String, i64)>,
    checkpoints: Vec<Checkpoint>,
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Wraps any error into a state-machine `StorageError`.
fn sm_err(e: &dyn std::error::Error) -> StorageError<u64> {
    StorageError::IO {
        source: StorageIOError::write_state_machine(&std::io::Error::new(
            ErrorKind::Other,
            e.to_string(),
        )),
    }
}

/// Checkpoint key: `{workflow_id bytes} \x00 {position as 8-byte big-endian}`.
fn cp_key(workflow_id: &str, position: i64) -> Vec<u8> {
    let mut key = workflow_id.as_bytes().to_vec();
    key.push(0);
    key.extend_from_slice(&position.to_be_bytes());
    key
}

/// Reads all three data column families and returns a serialisable snapshot.
fn collect_snapshot_content(db: &DB) -> Result<SnapshotContent, StorageError<u64>> {
    let mut workflows = Vec::new();
    let cf = db.cf_handle(CF_WORKFLOWS).unwrap();
    for item in db.iterator_cf(cf, IteratorMode::Start) {
        let (_, v) = item.map_err(|e| sm_err(&e))?;
        let wf: Workflow = serde_json::from_slice(&v)
            .map_err(|e| sm_err(&e))?;
        workflows.push(wf);
    }

    let mut fencing_tokens = Vec::new();
    let cf = db.cf_handle(CF_FENCING_TOKENS).unwrap();
    for item in db.iterator_cf(cf, IteratorMode::Start) {
        let (k, v) = item.map_err(|e| sm_err(&e))?;
        let workflow_id = String::from_utf8(k.to_vec())
            .map_err(|e| sm_err(&e))?;
        let token = v.as_ref()
            .try_into()
            .map(i64::from_be_bytes)
            .map_err(|_| StorageError::IO {
                source: StorageIOError::write_state_machine(&std::io::Error::new(
                    ErrorKind::Other,
                    "fencing token: invalid byte length",
                )),
            })?;
        fencing_tokens.push((workflow_id, token));
    }

    let mut checkpoints = Vec::new();
    let cf = db.cf_handle(CF_CHECKPOINTS).unwrap();
    for item in db.iterator_cf(cf, IteratorMode::Start) {
        let (_, v) = item.map_err(|e| sm_err(&e))?;
        let cp: Checkpoint = serde_json::from_slice(&v)
            .map_err(|e| sm_err(&e))?;
        checkpoints.push(cp);
    }

    Ok(SnapshotContent { workflows, fencing_tokens, checkpoints })
}

/// Clears all entries from a column family.
fn clear_cf(db: &DB, cf_name: &str) -> Result<(), StorageError<u64>> {
    let cf = db.cf_handle(cf_name).unwrap();
    let keys: Vec<_> = db
        .iterator_cf(cf, IteratorMode::Start)
        .map(|item| item.map(|(k, _)| k))
        .collect::<Result<_, _>>()
        .map_err(|e| sm_err(&e))?;
    for k in keys {
        db.delete_cf(cf, &k).map_err(|e| sm_err(&e))?;
    }
    Ok(())
}

// ── DB open ───────────────────────────────────────────────────────────────────

pub fn open_state_db(path: &str) -> Result<Arc<DB>, rocksdb::Error> {
    let mut opts = Options::default();
    opts.create_if_missing(true);
    opts.create_missing_column_families(true);

    let cfs = vec![
        ColumnFamilyDescriptor::new(CF_WORKFLOWS, Options::default()),
        ColumnFamilyDescriptor::new(CF_FENCING_TOKENS, Options::default()),
        ColumnFamilyDescriptor::new(CF_CHECKPOINTS, Options::default()),
        ColumnFamilyDescriptor::new(CF_RAFT_META, Options::default()),
    ];

    let db = DB::open_cf_descriptors(&opts, path, cfs)?;
    Ok(Arc::new(db))
}

// ── StateMachine ──────────────────────────────────────────────────────────────

pub struct StateMachine {
    db: Arc<DB>,
    last_applied: Arc<Mutex<Option<LogId<u64>>>>,
    last_membership: Arc<Mutex<StoredMembership<u64, Node>>>,
    last_snapshot: Arc<Mutex<Option<(SnapshotMeta<u64, Node>, Vec<u8>)>>>,
}

impl StateMachine {
    pub fn new(db: Arc<DB>) -> anyhow::Result<Self> {
        let last_applied =
            Self::load_meta_json::<Option<LogId<u64>>>(&db, "last_applied")?.unwrap_or(None);
        let last_membership =
            Self::load_meta_json::<StoredMembership<u64, Node>>(&db, "last_membership")?
                .unwrap_or_default();

        Ok(Self {
            db,
            last_applied: Arc::new(Mutex::new(last_applied)),
            last_membership: Arc::new(Mutex::new(last_membership)),
            last_snapshot: Arc::new(Mutex::new(None)),
        })
    }

    fn meta_cf(db: &DB) -> &rocksdb::ColumnFamily {
        db.cf_handle(CF_RAFT_META).unwrap()
    }

    fn load_meta_json<T: serde::de::DeserializeOwned>(
        db: &DB,
        key: &str,
    ) -> anyhow::Result<Option<T>> {
        match db.get_cf(Self::meta_cf(db), key.as_bytes())? {
            Some(bytes) => Ok(Some(serde_json::from_slice(&bytes)?)),
            None => Ok(None),
        }
    }

    async fn put_meta(&self, key: &str, value: String) -> Result<(), StorageError<u64>> {
        let cf = Self::meta_cf(&self.db);
        self.db
            .put_cf(cf, key.as_bytes(), value.as_bytes())
            .map_err(|e| sm_err(&e))
    }

    async fn apply_command(
        &self,
        cmd: WorkflowCommand,
        wf: &WorkflowRepository,
        cp: &CheckpointRepository,
        ft: &FencingTokenRepository,
    ) -> Result<WorkflowResponse, StorageError<u64>> {
        match cmd {
            WorkflowCommand::CreateOrGetWorkflow { workflow_id, name, created_at } => {
                wf.create_or_get(&workflow_id, name.as_deref(), created_at)
                    .await
                    .map_err(|e| sm_err(&e))?;
                Ok(WorkflowResponse::Empty)
            }

            WorkflowCommand::IncrementFencingToken { workflow_id } => {
                let token = ft.increment(&workflow_id).await.map_err(|e| sm_err(&e))?;
                Ok(WorkflowResponse::FencingToken(token))
            }

            WorkflowCommand::UpsertCheckpoint {
                workflow_id,
                position,
                idempotency_key,
                value,
                created_at,
            } => {
                cp.upsert(&workflow_id, position, &idempotency_key, value.as_deref(), created_at)
                    .await
                    .map_err(|e| sm_err(&e))?;
                Ok(WorkflowResponse::Empty)
            }

            WorkflowCommand::CompleteWorkflow { workflow_id, expire_at, completed_at } => {
                wf.complete(&workflow_id, expire_at, completed_at)
                    .await
                    .map_err(|e| sm_err(&e))?;
                Ok(WorkflowResponse::Empty)
            }

            WorkflowCommand::DeleteExpired { before_ms } => {
                let (r1, r2, r3) = tokio::join!(
                    cp.delete_for_completed(before_ms),
                    ft.delete_for_completed(before_ms),
                    wf.delete_expired(before_ms),
                );
                if let Err(e) = r1 { tracing::warn!(err = %e, "checkpoint cleanup error"); }
                if let Err(e) = r2 { tracing::warn!(err = %e, "fencing token cleanup error"); }
                if let Err(e) = r3 { tracing::warn!(err = %e, "workflow cleanup error"); }
                Ok(WorkflowResponse::Empty)
            }
        }
    }
}

// ── SnapshotBuilder ───────────────────────────────────────────────────────────

pub struct StateSnapshotBuilder {
    db: Arc<DB>,
    last_applied: Option<LogId<u64>>,
    last_membership: StoredMembership<u64, Node>,
    last_snapshot: Arc<Mutex<Option<(SnapshotMeta<u64, Node>, Vec<u8>)>>>,
}

impl RaftSnapshotBuilder<TypeConfig> for StateSnapshotBuilder {
    async fn build_snapshot(
        &mut self,
    ) -> Result<openraft::Snapshot<TypeConfig>, StorageError<u64>> {
        let content = collect_snapshot_content(&self.db)?;
        let bytes = serde_json::to_vec(&content).map_err(|e| sm_err(&e))?;

        let snapshot_id = self
            .last_applied
            .map(|l| format!("snap-{}", l.index))
            .unwrap_or_else(|| "snap-0".to_string());

        let meta = SnapshotMeta {
            last_log_id: self.last_applied,
            last_membership: self.last_membership.clone(),
            snapshot_id,
        };

        *self.last_snapshot.lock().await = Some((meta.clone(), bytes.clone()));

        Ok(openraft::Snapshot {
            meta,
            snapshot: Box::new(std::io::Cursor::new(bytes)),
        })
    }
}

// ── RaftStateMachine impl ─────────────────────────────────────────────────────

impl RaftStateMachine<TypeConfig> for StateMachine {
    type SnapshotBuilder = StateSnapshotBuilder;

    async fn applied_state(
        &mut self,
    ) -> Result<(Option<LogId<u64>>, StoredMembership<u64, Node>), StorageError<u64>> {
        let last_applied = *self.last_applied.lock().await;
        let last_membership = self.last_membership.lock().await.clone();
        Ok((last_applied, last_membership))
    }

    async fn apply<I>(
        &mut self,
        entries: I,
    ) -> Result<Vec<WorkflowResponse>, StorageError<u64>>
    where
        I: IntoIterator<Item = openraft::Entry<TypeConfig>> + Send,
    {
        let wf = WorkflowRepository::new(self.db.clone());
        let cp = CheckpointRepository::new(self.db.clone());
        let ft = FencingTokenRepository::new(self.db.clone());

        let mut responses = Vec::new();
        let mut last_log_id: Option<LogId<u64>> = None;

        for entry in entries {
            let log_id = entry.log_id;

            let response = match entry.payload {
                EntryPayload::Normal(cmd) => self.apply_command(cmd, &wf, &cp, &ft).await?,
                EntryPayload::Membership(m) => {
                    let membership = StoredMembership::new(Some(log_id), m);
                    *self.last_membership.lock().await = membership.clone();
                    let json =
                        serde_json::to_string(&membership).map_err(|e| sm_err(&e))?;
                    self.put_meta("last_membership", json).await?;
                    WorkflowResponse::Empty
                }
                EntryPayload::Blank => WorkflowResponse::Empty,
            };

            *self.last_applied.lock().await = Some(log_id);
            last_log_id = Some(log_id);
            responses.push(response);
        }

        if let Some(lid) = last_log_id {
            let json = serde_json::to_string(&lid).map_err(|e| sm_err(&e))?;
            self.put_meta("last_applied", json).await?;
        }

        Ok(responses)
    }

    async fn get_snapshot_builder(&mut self) -> Self::SnapshotBuilder {
        let last_applied = *self.last_applied.lock().await;
        let last_membership = self.last_membership.lock().await.clone();
        StateSnapshotBuilder {
            db: self.db.clone(),
            last_applied,
            last_membership,
            last_snapshot: self.last_snapshot.clone(),
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

        // Clear all data CFs before restoring
        for cf_name in [CF_WORKFLOWS, CF_FENCING_TOKENS, CF_CHECKPOINTS] {
            clear_cf(&self.db, cf_name)?;
        }

        // Restore workflows
        let cf = self.db.cf_handle(CF_WORKFLOWS).unwrap();
        for wf in content.workflows {
            let v = serde_json::to_vec(&wf).map_err(|e| sm_err(&e))?;
            self.db.put_cf(cf, wf.id.as_bytes(), v).map_err(|e| sm_err(&e))?;
        }

        // Restore fencing tokens
        let cf = self.db.cf_handle(CF_FENCING_TOKENS).unwrap();
        for (workflow_id, token) in content.fencing_tokens {
            self.db
                .put_cf(cf, workflow_id.as_bytes(), token.to_be_bytes())
                .map_err(|e| sm_err(&e))?;
        }

        // Restore checkpoints
        let cf = self.db.cf_handle(CF_CHECKPOINTS).unwrap();
        for cp in content.checkpoints {
            let key = cp_key(&cp.workflow_id, cp.position);
            let v = serde_json::to_vec(&cp).map_err(|e| sm_err(&e))?;
            self.db.put_cf(cf, key, v).map_err(|e| sm_err(&e))?;
        }

        // Update in-memory state
        *self.last_applied.lock().await = meta.last_log_id;
        *self.last_membership.lock().await = meta.last_membership.clone();

        // Persist to RaftMeta
        let applied_json =
            serde_json::to_string(&meta.last_log_id).map_err(|e| sm_err(&e))?;
        self.put_meta("last_applied", applied_json).await?;
        let membership_json =
            serde_json::to_string(&meta.last_membership).map_err(|e| sm_err(&e))?;
        self.put_meta("last_membership", membership_json).await?;

        // Cache the installed snapshot
        *self.last_snapshot.lock().await = Some((meta.clone(), bytes));

        Ok(())
    }

    async fn get_current_snapshot(
        &mut self,
    ) -> Result<Option<openraft::Snapshot<TypeConfig>>, StorageError<u64>> {
        let guard = self.last_snapshot.lock().await;
        match guard.as_ref() {
            Some((meta, bytes)) => Ok(Some(openraft::Snapshot {
                meta: meta.clone(),
                snapshot: Box::new(std::io::Cursor::new(bytes.clone())),
            })),
            None => Ok(None),
        }
    }
}
