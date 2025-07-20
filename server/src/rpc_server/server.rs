use hiqlite::Client;

use tonic::{Request, Response, Status, transport::Server};
pub mod workflow_service {
    tonic::include_proto!("workflow_service");
}
use std::error::Error;
use std::io;

use crate::services::checkpoint_service::{
    CheckpointInput, LeaseCheckpointInput, handle_checkpoint, handle_lease_checkpoint,
};
use crate::services::workflow_service::{
    CreateWorkflowInput, FinishWorkflowInput, create_workflow, finish_workflow,
};

use workflow_service::{
    CheckPointRequest, CheckPointResponse, CompleteWorkflowRequest, CompleteWorkflowResponse,
    LeaseCheckpointRequest, LeaseCheckpointResponse, LeaseWorkflowRequest, LeaseWorkflowResponse,
    lease_checkpoint_response::Response::Value, workflow_service_impl_server::WorkflowServiceImpl,
    workflow_service_impl_server::WorkflowServiceImplServer,
};

fn to_status<T>(result: Result<T, Box<dyn Error + Send + Sync>>) -> Result<T, Status> {
    result.map_err(|e| {
        if let Some(io_err) = e.downcast_ref::<io::Error>() {
            match io_err.kind() {
                io::ErrorKind::Interrupted => Status::aborted("interrupted"),
                io::ErrorKind::InvalidData => Status::aborted("invalid_data"),
                io::ErrorKind::InvalidInput => Status::aborted("invalid_input"),
                io::ErrorKind::Other => Status::internal("Something went wrong."),
                _ => Status::internal(io_err.to_string()),
            }
        } else {
            // Not an io::Error — treat as unhandled/internal
            Status::internal(e.to_string())
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
                    workflow_context_name: data.workflow_context_name,
                    checkpoint_context_name: data.checkpoint_context_name,
                    value: data.value,
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
                },
            )
            .await,
        )?;
        Ok(Response::new(LeaseCheckpointResponse {
            response: result.response.map(Value),
        }))
    }

    async fn lease_workflow(
        &self,
        request: Request<LeaseWorkflowRequest>,
    ) -> Result<Response<LeaseWorkflowResponse>, Status> {
        let data = request.into_inner();
        let result = to_status(
            create_workflow(
                &self.client,
                CreateWorkflowInput {
                    workflow_id: data.workflow_id,
                    is_nested: data.is_nested,
                    prefetch_checkpoints: data.prefetch_checkpoints,
                },
            )
            .await,
        )?;
        Ok(Response::new(LeaseWorkflowResponse {
            checkpoints: result.checkpoints,
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
