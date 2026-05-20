# Phase 5 — WorkflowRpcHandler

## File

`src/rpc_server/server.rs` (full rewrite)

## Depends on

- `WorkflowService` (Phase 4)
- `CheckpointService` (Phase 4)

---

## Struct

```rust
use std::sync::Arc;
use tonic::{Request, Response, Status};
use crate::{
    error::{WorkflowError, CheckpointError},
    services::{
        workflow_service::WorkflowService,
        checkpoint_service::CheckpointService,
    },
};
use workflow_service_proto::workflow_service_impl_server::WorkflowServiceImpl;

pub struct WorkflowRpcHandler {
    workflow_service: Arc<WorkflowService>,
    checkpoint_service: Arc<CheckpointService>,
}

impl WorkflowRpcHandler {
    pub fn new(
        workflow_service: Arc<WorkflowService>,
        checkpoint_service: Arc<CheckpointService>,
    ) -> Self {
        Self { workflow_service, checkpoint_service }
    }
}
```

---

## Error mapping

Replace the old `to_status(Box<dyn Error>)` with two typed mappers — one per error domain.
Typed matching eliminates the fragile `downcast_ref` pattern and ensures every new variant
gets an explicit gRPC status code at compile time.

```rust
fn workflow_error_to_status(e: WorkflowError) -> Status {
    match e {
        WorkflowError::NotFound { id } =>
            Status::not_found(format!("workflow not found: {id}")),
        WorkflowError::FencingTokenNotFound { workflow_id } =>
            Status::not_found(format!("fencing token not found for workflow: {workflow_id}")),
        WorkflowError::FencingTokenExpired { stored, received } =>
            Status::aborted(format!("fencing token expired (stored: {stored}, received: {received})")),
        WorkflowError::Raft(msg) =>
            Status::internal(format!("raft error: {msg}")),
        WorkflowError::Repository(e) =>
            Status::internal(format!("storage error: {e}")),
    }
}

fn checkpoint_error_to_status(e: CheckpointError) -> Status {
    match e {
        CheckpointError::FencingTokenNotFound { workflow_id } =>
            Status::not_found(format!("fencing token not found for workflow: {workflow_id}")),
        CheckpointError::FencingTokenExpired =>
            Status::aborted("fencing token expired"),
        CheckpointError::NonDeterministicCheckpoint { position } =>
            Status::aborted(format!("non-deterministic checkpoint at position {position}")),
        CheckpointError::CheckpointValueNotFound { position } =>
            Status::not_found(format!("checkpoint value not found at position {position}")),
        CheckpointError::Repository(e) =>
            Status::internal(format!("storage error: {e}")),
    }
}
```

---

## RPC method implementations

Each method is pure delegation — no business logic here. All seven follow the same shape:

```rust
#[tonic::async_trait]
impl WorkflowServiceImpl for WorkflowRpcHandler {

    #[instrument(skip(self), fields(workflow_id = %req.get_ref().workflow_id), err)]
    async fn workflow_start(
        &self,
        req: Request<WorkflowStartRequest>,
    ) -> Result<Response<WorkflowStartResponse>, Status> {
        let r = req.into_inner();
        let out = self.workflow_service
            .create(&r.workflow_id, r.context_name.as_deref())
            .await
            .map_err(workflow_error_to_status)?;
        Ok(Response::new(WorkflowStartResponse { fencing_token: out.fencing_token }))
    }

    #[instrument(skip(self), fields(workflow_id = %req.get_ref().workflow_id, position = req.get_ref().position, fencing_token = req.get_ref().fencing_token), err)]
    async fn checkpoint(
        &self,
        req: Request<CheckPointRequest>,
    ) -> Result<Response<CheckPointResponse>, Status> {
        let r = req.into_inner();
        let out = self.checkpoint_service
            .checkpoint(CheckpointInput {
                workflow_id: r.workflow_id,
                value: r.value,
                fencing_token: r.fencing_token,
                position: r.position,
                idempotency_key: r.idempotency_key,
            })
            .await
            .map_err(checkpoint_error_to_status)?;
        Ok(Response::new(CheckPointResponse { abort: out.abort }))
    }

    #[instrument(skip(self), fields(workflow_id = %req.get_ref().workflow_id, position = req.get_ref().position, fencing_token = req.get_ref().fencing_token), err)]
    async fn lease_checkpoint(
        &self,
        req: Request<LeaseCheckpointRequest>,
    ) -> Result<Response<LeaseCheckpointResponse>, Status> {
        let r = req.into_inner();
        let out = self.checkpoint_service
            .lease(LeaseCheckpointInput {
                workflow_id: r.workflow_id,
                fencing_token: r.fencing_token,
                lease_timeout: r.lease_timeout,
                position: r.position,
                idempotency_key: r.idempotency_key,
            })
            .await
            .map_err(checkpoint_error_to_status)?;

        let response_oneof = match out {
            LeaseCheckpointOutput::CheckpointValue(v)       => Some(lease_checkpoint_response::Response::Value(v)),
            LeaseCheckpointOutput::RemainingLeaseTimeout(t) => Some(lease_checkpoint_response::Response::RemainingLeaseTimeout(t)),
            LeaseCheckpointOutput::Leased                   => None,
        };
        Ok(Response::new(LeaseCheckpointResponse { response: response_oneof }))
    }

    #[instrument(skip(self), fields(workflow_id = %req.get_ref().workflow_id, fencing_token = req.get_ref().fencing_token), err)]
    async fn complete_workflow(
        &self,
        req: Request<CompleteWorkflowRequest>,
    ) -> Result<Response<CompleteWorkflowResponse>, Status> {
        let r = req.into_inner();
        self.workflow_service
            .finish(FinishWorkflowInput {
                workflow_id: r.workflow_id,
                fencing_token: r.fencing_token,
                expire_after: r.expire_after,
            })
            .await
            .map_err(workflow_error_to_status)?;
        Ok(Response::new(CompleteWorkflowResponse {}))
    }

    #[instrument(skip(self), fields(workflow_id = %req.get_ref().workflow_id), err)]
    async fn workflow_status(
        &self,
        req: Request<WorkflowStatusRequest>,
    ) -> Result<Response<WorkflowStatusResponse>, Status> {
        let r = req.into_inner();
        let wf = self.workflow_service
            .status(&r.workflow_id).await
            .map_err(workflow_error_to_status)?;
        Ok(Response::new(WorkflowStatusResponse {
            workflow_id: wf.id,
            status: wf.status,
            expire_at: wf.expire_at,
            created_at: wf.created_at,
            completed_at: wf.completed_at,
        }))
    }

    #[instrument(skip(self), fields(workflow_id = %req.get_ref().workflow_id, position = req.get_ref().position), err)]
    async fn release_checkpoint(
        &self,
        req: Request<ReleaseCheckpointRequest>,
    ) -> Result<Response<ReleaseCheckpointResponse>, Status> {
        let r = req.into_inner();
        self.checkpoint_service
            .release(&r.workflow_id, r.position)
            .await
            .map_err(checkpoint_error_to_status)?;
        Ok(Response::new(ReleaseCheckpointResponse {}))
    }

    #[instrument(skip(self), fields(workflow_id = %req.get_ref().workflow_id, position = req.get_ref().position, fencing_token = req.get_ref().fencing_token), err)]
    async fn generate_idempotency_key(
        &self,
        req: Request<GenerateIdempotencyKeyRequest>,
    ) -> Result<Response<GenerateIdempotencyKeyResponse>, Status> {
        let r = req.into_inner();
        let out = self.checkpoint_service
            .create_idempotency_key(CreateDurableIdempotencyKeyInput {
                workflow_id: r.workflow_id,
                fencing_token: r.fencing_token,
                position: r.position,
            })
            .await
            .map_err(checkpoint_error_to_status)?;
        Ok(Response::new(GenerateIdempotencyKeyResponse { idempotency_key: out.idempotency_key }))
    }
}
```

---

## Starting the gRPC server

`start_server` exposes only the client-facing gRPC API.
Raft inter-node communication runs on a separate TCP port via `TcpRaftServer` (Phase 2)
and is started independently in `main.rs` (Phase 5 — `main-wiring.md`).

```rust
pub async fn start_server(
    addr: &str,
    handler: WorkflowRpcHandler,
) -> anyhow::Result<()> {
    info!(addr, "gRPC server listening");
    tonic::transport::Server::builder()
        .add_service(WorkflowServiceImplServer::new(handler))
        .serve(addr.parse()?)
        .await?;
    Ok(())
}
```

---

## Verification

```bash
cargo check -p idempotency-server
# WorkflowRpcHandler must implement WorkflowServiceImpl (tonic trait)
# All 7 methods must be present
```
