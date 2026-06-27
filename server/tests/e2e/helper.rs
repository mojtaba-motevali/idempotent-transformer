use std::sync::Arc;

use tempfile::TempDir;
use tonic::transport::Channel;

use idempotency_server::{
    database::db::RaftNode,
    repositories::{
        checkpoints::CheckpointRepository,
        lease_checkpoint::LeaseRepository,
        workflows::WorkflowRepository,
        workflows_fencing_tokens::FencingTokenRepository,
    },
    rpc_server::server::{
        workflow_service::{
            workflow_service_impl_client::WorkflowServiceImplClient,
            workflow_service_impl_server::WorkflowServiceImplServer,
            CheckPointRequest, CompleteWorkflowRequest, GenerateIdempotencyKeyRequest,
            LeaseCheckpointRequest, LeaseCheckpointResponse, WorkflowStartRequest,
            WorkflowStatusRequest, WorkflowStatusResponse,
        },
        WorkflowRpcHandler,
    },
    services::{checkpoint_service::CheckpointService, workflow_service::WorkflowService},
};

// ── Fixture ───────────────────────────────────────────────────────────────────

pub struct ServerFixture {
    _temp_dir: TempDir,
    _shutdown_tx: tokio::sync::oneshot::Sender<()>,
    client: WorkflowServiceImplClient<Channel>,
}

impl ServerFixture {
    /// Starts a full in-process single-node Raft cluster and gRPC server on a
    /// random port.  The cluster shuts down when the fixture is dropped.
    pub async fn start() -> Self {
        let temp_dir = tempfile::tempdir().unwrap();
        let data_dir = temp_dir.path().to_str().unwrap().to_string();

        let probe = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr: std::net::SocketAddr = probe.local_addr().unwrap();
        drop(probe);

        let raft_node = RaftNode::start(1, vec![], &data_dir).await.unwrap();
        let store = raft_node.store.clone();
        let raft = Arc::new(raft_node.raft.clone());

        let workflow_repo = Arc::new(WorkflowRepository::new(store.clone()));
        let checkpoint_repo = Arc::new(CheckpointRepository::new(store.clone()));
        let fencing_token_repo = Arc::new(FencingTokenRepository::new(store.clone()));
        let lease_repo = Arc::new(LeaseRepository::new());

        let workflow_svc = Arc::new(WorkflowService::new(
            workflow_repo,
            fencing_token_repo.clone(),
            checkpoint_repo.clone(),
            raft.clone(),
            1u64,
        ));
        let checkpoint_svc = Arc::new(CheckpointService::new(
            checkpoint_repo,
            lease_repo,
            fencing_token_repo,
        ));

        let handler = WorkflowRpcHandler::new(workflow_svc, checkpoint_svc);

        for _ in 0..100 {
            if raft.current_leader().await == Some(1) {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        assert_eq!(
            raft.current_leader().await,
            Some(1),
            "Raft single-node did not elect a leader within 10 s"
        );

        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
        tokio::spawn(async move {
            tonic::transport::Server::builder()
                .add_service(WorkflowServiceImplServer::new(handler))
                .serve_with_shutdown(addr, async move { let _ = shutdown_rx.await; })
                .await
                .unwrap();
        });

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let channel = Channel::from_shared(format!("http://{addr}"))
            .unwrap()
            .connect()
            .await
            .unwrap();

        Self {
            _temp_dir: temp_dir,
            _shutdown_tx: shutdown_tx,
            client: WorkflowServiceImplClient::new(channel),
        }
    }

    // ── RPC methods ───────────────────────────────────────────────────────────

    /// Starts (or re-starts) a workflow and returns the new fencing token.
    pub async fn workflow_start(&mut self, workflow_id: &str) -> i64 {
        self.client
            .workflow_start(WorkflowStartRequest {
                workflow_id:  workflow_id.to_owned(),
                context_name: None,
            })
            .await
            .unwrap()
            .into_inner()
            .fencing_token
    }

    /// Commits a checkpoint value.  Returns `true` when the fencing token is
    /// stale and the caller should abort, `false` on success.
    pub async fn checkpoint(
        &mut self,
        workflow_id:     &str,
        value:           Vec<u8>,
        fencing_token:   i64,
        position:        i64,
        idempotency_key: &str,
    ) -> bool {
        self.client
            .checkpoint(CheckPointRequest {
                workflow_id:     workflow_id.to_owned(),
                value,
                fencing_token,
                position,
                idempotency_key: idempotency_key.to_owned(),
                task_name:       None,
            })
            .await
            .unwrap()
            .into_inner()
            .abort
    }

    /// Attempts to acquire a lease on a checkpoint slot.
    /// Returns the raw response — callers can inspect `response` to determine
    /// whether the slot was leased, already committed, or still held.
    pub async fn lease_checkpoint(
        &mut self,
        workflow_id:     &str,
        fencing_token:   i64,
        lease_timeout:   i64,
        position:        i64,
        idempotency_key: &str,
    ) -> LeaseCheckpointResponse {
        self.client
            .lease_checkpoint(LeaseCheckpointRequest {
                workflow_id:     workflow_id.to_owned(),
                fencing_token,
                lease_timeout,
                position,
                idempotency_key: idempotency_key.to_owned(),
            })
            .await
            .unwrap()
            .into_inner()
    }

    /// Marks a workflow as completed with the given TTL.
    pub async fn complete_workflow(
        &mut self,
        workflow_id:   &str,
        fencing_token: i64,
        expire_after:  i64,
    ) {
        self.client
            .complete_workflow(CompleteWorkflowRequest {
                workflow_id:   workflow_id.to_owned(),
                fencing_token,
                expire_after,
            })
            .await
            .unwrap();
    }

    /// Returns the workflow status, or the gRPC `Status` on error (e.g. `NOT_FOUND`).
    pub async fn workflow_status(
        &mut self,
        workflow_id: &str,
    ) -> Result<WorkflowStatusResponse, tonic::Status> {
        self.client
            .workflow_status(WorkflowStatusRequest {
                workflow_id: workflow_id.to_owned(),
            })
            .await
            .map(|r| r.into_inner())
            .map_err(|s| s)
    }

    /// Generates a durable idempotency key at the given checkpoint position.
    /// Returns the key string (a UUID).
    pub async fn generate_idempotency_key(
        &mut self,
        workflow_id:   &str,
        fencing_token: i64,
        position:      i64,
    ) -> String {
        self.client
            .generate_idempotency_key(GenerateIdempotencyKeyRequest {
                workflow_id:   workflow_id.to_owned(),
                fencing_token,
                position,
            })
            .await
            .unwrap()
            .into_inner()
            .idempotency_key
    }
}

// ── Utilities ─────────────────────────────────────────────────────────────────

/// Returns a random UUID string — convenient for generating unique workflow IDs.
pub fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}
