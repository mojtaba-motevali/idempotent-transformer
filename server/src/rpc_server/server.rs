use hiqlite::Client;

use tonic::{Request, Response, Status, transport::Server};
pub mod workflow_service {
    tonic::include_proto!("workflow_service");
}
use std::error::Error;
use std::io;

use crate::repositories::workflows::get_workflow;
use crate::rpc_server::server::workflow_service::{
    ReleaseCheckpointRequest, ReleaseCheckpointResponse, WorkflowStartRequest,
    WorkflowStartResponse, WorkflowStatusRequest, WorkflowStatusResponse,
};
use crate::services::checkpoint_service::{
    CheckpointInput, LeaseCheckpointInput, LeaseCheckpointReturnType, handle_checkpoint,
    handle_lease_checkpoint, release_checkpoint,
};
use crate::services::workflow_service::{
    CreateWorkflowInput, FinishWorkflowInput, create_workflow, finish_workflow,
};

use workflow_service::{
    CheckPointRequest, CheckPointResponse, CompleteWorkflowRequest, CompleteWorkflowResponse,
    LeaseCheckpointRequest, LeaseCheckpointResponse,
    lease_checkpoint_response::Response::RemainingLeaseTimeout,
    lease_checkpoint_response::Response::Value, workflow_service_impl_server::WorkflowServiceImpl,
    workflow_service_impl_server::WorkflowServiceImplServer,
};

fn to_status<T>(result: Result<T, Box<dyn Error + Send + Sync>>) -> Result<T, Status> {
    result.map_err(|e| {
        if let Some(io_err) = e.downcast_ref::<io::Error>() {
            match io_err.kind() {
                io::ErrorKind::Interrupted => Status::aborted(io_err.to_string()),
                io::ErrorKind::InvalidData => Status::aborted(io_err.to_string()),
                io::ErrorKind::InvalidInput => Status::aborted(io_err.to_string()),
                io::ErrorKind::Other => Status::internal(io_err.to_string()),
                _ => Status::internal(io_err.to_string()),
            }
        } else {
            // Not an io::Error — treat as unhandled/internal
            Status::internal("Something went wrong.")
        }
    })
}

// defining a struct for our service
pub struct WorkflowService {
    client: Client,
}

// implementing rpc for service defined in .proto
#[tonic::async_trait]
impl WorkflowServiceImpl for WorkflowService {
    async fn checkpoint(
        &self,
        request: Request<CheckPointRequest>,
    ) -> Result<Response<CheckPointResponse>, Status> {
        let data = request.into_inner();
        let result = to_status(
            handle_checkpoint(
                &self.client,
                CheckpointInput {
                    workflow_id: data.workflow_id,
                    fencing_token: data.fencing_token,
                    position_checksum: data.position_checksum,
                    value: data.value,
                    idempotency_checksum: data.idempotency_checksum,
                },
            )
            .await,
        )?;
        Ok(Response::new(CheckPointResponse {
            abort: result.abort,
        }))
    }

    async fn lease_checkpoint(
        &self,
        request: Request<LeaseCheckpointRequest>,
    ) -> Result<Response<LeaseCheckpointResponse>, Status> {
        let data = request.into_inner();
        let result = to_status(
            handle_lease_checkpoint(
                &self.client,
                LeaseCheckpointInput {
                    workflow_id: data.workflow_id,
                    fencing_token: data.fencing_token,
                    position_checksum: data.position_checksum,
                    lease_timeout: data.lease_timeout,
                    idempotency_checksum: data.idempotency_checksum,
                },
            )
            .await,
        )?;
        Ok(Response::new(LeaseCheckpointResponse {
            response: result.response.map(|r| match r {
                LeaseCheckpointReturnType::CheckpointValue(value) => Value(value),
                LeaseCheckpointReturnType::RemainingLeaseTimeout(remaining_lease_timeout) => {
                    RemainingLeaseTimeout(remaining_lease_timeout)
                }
            }),
        }))
    }

    async fn workflow_start(
        &self,
        request: Request<WorkflowStartRequest>,
    ) -> Result<Response<WorkflowStartResponse>, Status> {
        let data = request.into_inner();
        let result = to_status(
            create_workflow(
                &self.client,
                CreateWorkflowInput {
                    workflow_id: data.workflow_id,
                },
            )
            .await,
        )?;
        Ok(Response::new(WorkflowStartResponse {
            fencing_token: result.fencing_token,
        }))
    }

    async fn complete_workflow(
        &self,
        request: Request<CompleteWorkflowRequest>,
    ) -> Result<Response<CompleteWorkflowResponse>, Status> {
        let data = request.into_inner();
        to_status(
            finish_workflow(
                &self.client,
                FinishWorkflowInput {
                    workflow_id: data.workflow_id,
                    fencing_token: data.fencing_token,
                    expire_after: data.expire_after,
                },
            )
            .await,
        )?;
        Ok(Response::new(CompleteWorkflowResponse {}))
    }

    async fn workflow_status(
        &self,
        request: Request<WorkflowStatusRequest>,
    ) -> Result<Response<WorkflowStatusResponse>, Status> {
        let data = request.into_inner();
        let result = to_status(get_workflow(&self.client, &data.workflow_id).await)?;
        if let Some(workflow) = result {
            Ok(Response::new(WorkflowStatusResponse {
                workflow_id: workflow.id,
                status: workflow.status,
                expire_at: workflow.expire_at,
                completed_at: workflow.completed_at,
                created_at: workflow.created_at,
            }))
        } else {
            Err(Status::not_found("workflow_not_found"))
        }
    }

    async fn release_checkpoint(
        &self,
        request: Request<ReleaseCheckpointRequest>,
    ) -> Result<Response<ReleaseCheckpointResponse>, Status> {
        let data = request.into_inner();
        to_status(
            release_checkpoint(&self.client, &data.workflow_id, data.position_checksum).await,
        )?;
        Ok(Response::new(ReleaseCheckpointResponse {}))
    }
}

pub async fn start_server(
    rpc_addr: &str,
    client: &Client,
) -> Result<(), Box<dyn std::error::Error>> {
    // defining address for our service
    let addr = rpc_addr.parse().unwrap();
    println!("Server listening on {addr}");
    // adding our service to our server.
    Server::builder()
        .add_service(WorkflowServiceImplServer::new(WorkflowService {
            // It's cheap to clone because of inner Arc.
            client: client.clone(),
        }))
        .serve(addr)
        .await?;
    Ok(())
}
