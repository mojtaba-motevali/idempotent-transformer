use libsql::{Connection, params};
use tonic::{Request, Response, Status, transport::Server};
pub mod workflow_service {
    tonic::include_proto!("workflow_service");
}

use workflow_service::{
    CheckPointRequest, CheckPointResponse, Checkpoint, CompleteWorkflowRequest,
    CompleteWorkflowResponse, GetCheckpointsByWorkflowIdRequest,
    GetCheckpointsByWorkflowIdResponse, GetCheckpointsRequest, GetCheckpointsResponse,
    workflow_service_impl_server::WorkflowServiceImpl,
    workflow_service_impl_server::WorkflowServiceImplServer,
};

// defining a struct for our service
pub struct WorkflowService {
    connection: Connection,
}

// implementing rpc for service defined in .proto
#[tonic::async_trait]
impl WorkflowServiceImpl for WorkflowService {
    async fn checkpoint(
        &self,
        request: Request<CheckPointRequest>,
    ) -> Result<Response<CheckPointResponse>, Status> {
        let data = request.into_inner();
        let result = match self
            .connection
            .execute(
                "INSERT INTO checkpoints (workflow_id, execution_order, value) VALUES (?, ?, ?)",
                params![data.workflow_id, data.execution_order, data.value],
            )
            .await
        {
            Ok(_) => Ok(Response::new(CheckPointResponse {})),
            Err(e) => Err(Status::internal(e.to_string())),
        };
        result
    }

    async fn get_checkpoint(
        &self,
        request: Request<GetCheckpointsRequest>,
    ) -> Result<Response<GetCheckpointsResponse>, Status> {
        Ok(Response::new(GetCheckpointsResponse { value: vec![] }))
    }

    async fn get_checkpoints_by_workflow_id(
        &self,
        request: Request<GetCheckpointsByWorkflowIdRequest>,
    ) -> Result<Response<GetCheckpointsByWorkflowIdResponse>, Status> {
        Ok(Response::new(GetCheckpointsByWorkflowIdResponse {
            checkpoints: vec![Checkpoint {
                execution_order: 1,
                value: vec![],
            }],
        }))
    }

    async fn complete_workflow(
        &self,
        request: Request<CompleteWorkflowRequest>,
    ) -> Result<Response<CompleteWorkflowResponse>, Status> {
        Ok(Response::new(CompleteWorkflowResponse {}))
    }
}

pub async fn start_server(connection: Connection) -> Result<(), Box<dyn std::error::Error>> {
    // defining address for our service
    let addr = "[::1]:50051".parse().unwrap();
    println!("Server listening on {}", addr);
    // adding our service to our server.
    Server::builder()
        .add_service(WorkflowServiceImplServer::new(WorkflowService {
            connection,
        }))
        .serve(addr)
        .await?;
    Ok(())
}
