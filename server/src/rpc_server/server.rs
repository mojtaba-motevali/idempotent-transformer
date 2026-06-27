pub mod workflow_service {
    tonic::include_proto!("workflow_service");
}

use std::sync::Arc;

use tonic::{Request, Response, Status};
use tracing::instrument;

use workflow_service::{
    lease_checkpoint_response,
    workflow_service_impl_server::{WorkflowServiceImpl, WorkflowServiceImplServer},
    CheckPointRequest, CheckPointResponse, CompleteWorkflowRequest, CompleteWorkflowResponse,
    GenerateIdempotencyKeyRequest, GenerateIdempotencyKeyResponse, LeaseCheckpointRequest,
    LeaseCheckpointResponse, ReleaseCheckpointRequest, ReleaseCheckpointResponse,
    WorkflowStartRequest, WorkflowStartResponse, WorkflowStatusRequest, WorkflowStatusResponse,
};

use crate::error::{CheckpointError, WorkflowError};
use crate::services::checkpoint_service::{
    CheckpointInput, CheckpointService, CreateDurableIdempotencyKeyInput, LeaseCheckpointInput,
    LeaseCheckpointOutput,
};
use crate::services::workflow_service::{FinishWorkflowInput, WorkflowService};

// ── Error mappers ────────────────────────────────────────────────────────────

fn workflow_err(e: WorkflowError) -> Status {
    match e {
        WorkflowError::NotFound { id } => {
            Status::not_found(format!("workflow not found: {id}"))
        }
        WorkflowError::FencingTokenNotFound { workflow_id } => {
            Status::not_found(format!("fencing token not found for workflow: {workflow_id}"))
        }
        WorkflowError::FencingTokenExpired { stored, received } => Status::aborted(format!(
            "fencing token expired (stored: {stored}, received: {received})"
        )),
        WorkflowError::Raft(msg) => Status::internal(format!("raft error: {msg}")),
        WorkflowError::Repository(e) => Status::internal(format!("storage error: {e}")),
    }
}

fn checkpoint_err(e: CheckpointError) -> Status {
    match e {
        CheckpointError::FencingTokenNotFound { workflow_id } => {
            Status::not_found(format!("fencing token not found for workflow: {workflow_id}"))
        }
        CheckpointError::FencingTokenExpired => Status::aborted("fencing token expired"),
        CheckpointError::NonDeterministicCheckpoint { position } => {
            Status::aborted(format!("non-deterministic checkpoint at position {position}"))
        }
        CheckpointError::CheckpointValueNotFound { position } => {
            Status::not_found(format!("checkpoint value not found at position {position}"))
        }
        CheckpointError::Repository(e) => Status::internal(format!("storage error: {e}")),
    }
}

// ── Handler struct ───────────────────────────────────────────────────────────

pub struct WorkflowRpcHandler {
    workflow_service: Arc<WorkflowService>,
    checkpoint_service: Arc<CheckpointService>,
}

impl WorkflowRpcHandler {
    pub fn new(
        workflow_service: Arc<WorkflowService>,
        checkpoint_service: Arc<CheckpointService>,
    ) -> Self {
        Self {
            workflow_service,
            checkpoint_service,
        }
    }
}

// ── RPC implementations ──────────────────────────────────────────────────────

#[tonic::async_trait]
impl WorkflowServiceImpl for WorkflowRpcHandler {
    #[instrument(skip(self), fields(workflow_id = %req.get_ref().workflow_id), err)]
    async fn workflow_start(
        &self,
        req: Request<WorkflowStartRequest>,
    ) -> Result<Response<WorkflowStartResponse>, Status> {
        let data = req.into_inner();
        let out = self
            .workflow_service
            .create(&data.workflow_id, data.context_name.as_deref())
            .await
            .map_err(workflow_err)?;

        Ok(Response::new(WorkflowStartResponse {
            fencing_token: out.fencing_token,
        }))
    }

    #[instrument(skip(self), fields(workflow_id = %req.get_ref().workflow_id, position = req.get_ref().position, fencing_token = req.get_ref().fencing_token), err)]
    async fn checkpoint(
        &self,
        req: Request<CheckPointRequest>,
    ) -> Result<Response<CheckPointResponse>, Status> {
        let data = req.into_inner();
        let out = self
            .checkpoint_service
            .checkpoint(CheckpointInput {
                workflow_id: data.workflow_id,
                value: data.value,
                fencing_token: data.fencing_token,
                position: data.position,
                idempotency_key: data.idempotency_key,
            })
            .await
            .map_err(checkpoint_err)?;

        Ok(Response::new(CheckPointResponse { abort: out.abort }))
    }

    #[instrument(skip(self), fields(workflow_id = %req.get_ref().workflow_id, position = req.get_ref().position, fencing_token = req.get_ref().fencing_token), err)]
    async fn lease_checkpoint(
        &self,
        req: Request<LeaseCheckpointRequest>,
    ) -> Result<Response<LeaseCheckpointResponse>, Status> {
        let data = req.into_inner();
        let out = self
            .checkpoint_service
            .lease(LeaseCheckpointInput {
                workflow_id: data.workflow_id,
                fencing_token: data.fencing_token,
                lease_timeout: data.lease_timeout,
                position: data.position,
                idempotency_key: data.idempotency_key,
            })
            .await
            .map_err(checkpoint_err)?;

        let response = match out {
            LeaseCheckpointOutput::CheckpointValue(v) => {
                Some(lease_checkpoint_response::Response::Value(v))
            }
            LeaseCheckpointOutput::RemainingLeaseTimeout(t) => {
                Some(lease_checkpoint_response::Response::RemainingLeaseTimeout(t))
            }
            LeaseCheckpointOutput::Leased => None,
        };

        Ok(Response::new(LeaseCheckpointResponse { response }))
    }

    #[instrument(skip(self), fields(workflow_id = %req.get_ref().workflow_id, fencing_token = req.get_ref().fencing_token), err)]
    async fn complete_workflow(
        &self,
        req: Request<CompleteWorkflowRequest>,
    ) -> Result<Response<CompleteWorkflowResponse>, Status> {
        let data = req.into_inner();
        self.workflow_service
            .finish(FinishWorkflowInput {
                workflow_id: data.workflow_id,
                fencing_token: data.fencing_token,
                expire_after: data.expire_after,
            })
            .await
            .map_err(workflow_err)?;

        Ok(Response::new(CompleteWorkflowResponse {}))
    }

    #[instrument(skip(self), fields(workflow_id = %req.get_ref().workflow_id), err)]
    async fn workflow_status(
        &self,
        req: Request<WorkflowStatusRequest>,
    ) -> Result<Response<WorkflowStatusResponse>, Status> {
        let data = req.into_inner();
        let workflow = self
            .workflow_service
            .status(&data.workflow_id)
            .await
            .map_err(workflow_err)?;

        Ok(Response::new(WorkflowStatusResponse {
            workflow_id: workflow.id,
            status: workflow.status,
            expire_at: workflow.expire_at,
            completed_at: workflow.completed_at,
            created_at: workflow.created_at,
        }))
    }

    #[instrument(skip(self), fields(workflow_id = %req.get_ref().workflow_id, position = req.get_ref().position), err)]
    async fn release_checkpoint(
        &self,
        req: Request<ReleaseCheckpointRequest>,
    ) -> Result<Response<ReleaseCheckpointResponse>, Status> {
        let data = req.into_inner();
        self.checkpoint_service
            .release(&data.workflow_id, data.position)
            .await
            .map_err(checkpoint_err)?;

        Ok(Response::new(ReleaseCheckpointResponse {}))
    }

    #[instrument(skip(self), fields(workflow_id = %req.get_ref().workflow_id, position = req.get_ref().position, fencing_token = req.get_ref().fencing_token), err)]
    async fn generate_idempotency_key(
        &self,
        req: Request<GenerateIdempotencyKeyRequest>,
    ) -> Result<Response<GenerateIdempotencyKeyResponse>, Status> {
        let data = req.into_inner();
        let out = self
            .checkpoint_service
            .create_idempotency_key(CreateDurableIdempotencyKeyInput {
                workflow_id: data.workflow_id,
                fencing_token: data.fencing_token,
                position: data.position,
            })
            .await
            .map_err(checkpoint_err)?;

        Ok(Response::new(GenerateIdempotencyKeyResponse {
            idempotency_key: out.idempotency_key,
        }))
    }
}

// ── Server startup ───────────────────────────────────────────────────────────

pub async fn start_server(addr: &str, handler: WorkflowRpcHandler) -> anyhow::Result<()> {
    tracing::info!(addr, "gRPC server listening");
    tonic::transport::Server::builder()
        .add_service(WorkflowServiceImplServer::new(handler))
        .serve(addr.parse()?)
        .await?;
    Ok(())
}
