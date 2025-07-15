use hiqlite::Client;
use hiqlite_macros::params;

use tonic::{Request, Response, Status, transport::Server};
pub mod workflow_service {
    tonic::include_proto!("workflow_service");
}
use std::time::{SystemTime, UNIX_EPOCH};

use crate::schema::workflow::WorkflowStatus;
use crate::services::checkpoints::{create_checkpoint, get_checkpoint, get_checkpoints};
use crate::services::lease_checkpoint::{get_lease_timeout, lease_checkpoint};
use crate::services::workflows::{get_create_workflow_query, get_workflow};
use crate::services::workflows_fencing_tokens::get_workflow_fencing_token;

use workflow_service::{
    CheckPointRequest, CheckPointResponse, Checkpoint, CompleteWorkflowRequest,
    CompleteWorkflowResponse, LeaseCheckpointRequest, LeaseCheckpointResponse,
    LeaseWorkflowRequest, LeaseWorkflowResponse,
    lease_checkpoint_response::Response::{AcquiredLeaseTimeout, LeaseTimeout, Value},
    workflow_service_impl_server::WorkflowServiceImpl,
    workflow_service_impl_server::WorkflowServiceImplServer,
};

async fn check_fencing_token(
    client: &Client,
    workflow_id: &str,
    fencing_token: i64,
) -> Result<(), Status> {
    match get_workflow_fencing_token(client, workflow_id).await {
        Ok(token) => {
            if token.is_some() && token.unwrap() >= fencing_token {
                Ok(())
            } else {
                Err(Status::aborted("fencing_token_expired"))
            }
        }
        Err(e) => Err(Status::internal(e.to_string())),
    }
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
        check_fencing_token(&self.client, &data.workflow_id, data.fencing_token).await?;
        let result =
            match create_checkpoint(&self.client, &data.workflow_id, data.value, data.position)
                .await
            {
                Ok(_) => Ok(Response::new(CheckPointResponse {})),
                Err(e) => Err(Status::internal(e.to_string())),
            };
        result
    }

    async fn lease_checkpoint(
        &self,
        request: Request<LeaseCheckpointRequest>,
    ) -> Result<Response<LeaseCheckpointResponse>, Status> {
        let data = request.into_inner();
        let sent_fencing_token = data.fencing_token;
        let found_fencing_token =
            match get_workflow_fencing_token(&self.client, &data.workflow_id).await {
                Ok(token) => token,
                Err(e) => return Err(Status::internal(e.to_string())),
            };
        if found_fencing_token.is_none() {
            return Err(Status::aborted("fencing_token_not_found"));
        }
        let stored_fencing_token = found_fencing_token.unwrap();

        // old fencing tokens can not be used to lease a checkpoint
        if stored_fencing_token > sent_fencing_token {
            return Err(Status::aborted("fencing_token_expired"));
        }

        let result = match get_checkpoint(&self.client, &data.workflow_id, data.position).await {
            Ok(value) => value,
            Err(e) => return Err(Status::internal(e.to_string())),
        };
        // if checkpoint is already leased, then we need to return the value
        if result.is_some() {
            return Ok(Response::new(LeaseCheckpointResponse {
                response: Some(Value(result.unwrap().value)),
            }));
        }
        // if fencing token is the same, then we need to lease the checkpoint
        if sent_fencing_token == stored_fencing_token {
            // lease checkpoint
            let lease_timeout = match lease_checkpoint(
                &self.client,
                &data.workflow_id,
                data.position,
                data.lease_timeout,
            )
            .await
            {
                Ok(lease_timeout) => lease_timeout,
                Err(e) => return Err(Status::internal(e.to_string())),
            };
            return Ok(Response::new(LeaseCheckpointResponse {
                response: Some(AcquiredLeaseTimeout(lease_timeout)),
            }));
        }
        // if fencing token is greater, then we need to get the checkpoint
        if sent_fencing_token > stored_fencing_token {
            // get lease timeout
            let lease_timeout =
                match get_lease_timeout(&self.client, &data.workflow_id, data.position).await {
                    Ok(lease_timeout) => lease_timeout,
                    Err(e) => return Err(Status::internal(e.to_string())),
                };
            if lease_timeout.is_none() {
                return Err(Status::aborted("lease_timeout_not_found"));
            }

            return Ok(Response::new(LeaseCheckpointResponse {
                response: Some(LeaseTimeout(lease_timeout.unwrap())),
            }));
        }
        println!("data: {:?}", data);
        Err(Status::internal("unexpected state"))
    }

    async fn lease_workflow(
        &self,
        request: Request<LeaseWorkflowRequest>,
    ) -> Result<Response<LeaseWorkflowResponse>, Status> {
        let data = request.into_inner();
        let mut fut1 = None;
        if data.prefetch_checkpoints {
            fut1 = Some(get_checkpoints(&self.client, &data.workflow_id));
        }
        let fut2 = get_workflow(&self.client, &data.workflow_id);
        let mut checkpoints = vec![];
        if let Some(fut) = fut1 {
            checkpoints = match fut.await {
                Ok(value) => value,
                Err(e) => {
                    println!("error: {:?}", e);
                    return Err(Status::internal(e.to_string()));
                }
            };
        }
        let workflow = match fut2.await {
            Ok(value) => value,
            Err(e) => {
                println!("error: {:?}", e);
                return Err(Status::internal(e.to_string()));
            }
        };
        let mut queries = vec![];
        if workflow.is_none() {
            let query = get_create_workflow_query(&data.workflow_id);
            queries.push(query);
        }
        if !data.is_nested {
            queries.push((
                "INSERT INTO WorkflowFencingTokens (workflow_id, fencing_token) VALUES (?, ?) ON CONFLICT (workflow_id) DO UPDATE SET fencing_token = fencing_token + 1 RETURNING fencing_token",
                params![&data.workflow_id, 1 as i64],
            ));
        } else {
            queries.push((
                "SELECT fencing_token FROM WorkflowFencingTokens WHERE workflow_id = ?",
                params![&data.workflow_id],
            ));
        }

        match self.client.txn(queries).await {
            Ok(txn) => txn,
            Err(e) => {
                println!("error: {:?}", e);
                return Err(Status::internal(e.to_string()));
            }
        };
        let fencing_token = match get_workflow_fencing_token(&self.client, &data.workflow_id).await
        {
            Ok(token) => {
                if token.is_some() {
                    token.unwrap()
                } else {
                    return Err(Status::aborted("unable_to_create_fencing_token"));
                }
            }
            Err(e) => {
                println!("error: {:?}", e);
                return Err(Status::internal(e.to_string()));
            }
        };
        Ok(Response::new(LeaseWorkflowResponse {
            checkpoints: checkpoints
                .into_iter()
                .map(|v| Checkpoint {
                    position: v.position,
                    value: v.value,
                })
                .collect(),
            fencing_token: fencing_token,
        }))
    }

    async fn complete_workflow(
        &self,
        request: Request<CompleteWorkflowRequest>,
    ) -> Result<Response<CompleteWorkflowResponse>, Status> {
        let data = request.into_inner();
        check_fencing_token(&self.client, &data.workflow_id, data.fencing_token).await?;

        let result = match self
            .client
            .execute(
                "UPDATE Workflows SET expire_at = $1, status = $2 WHERE id = $3",
                params![
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .expect("clock went backwards")
                        .as_secs() as i64,
                    WorkflowStatus::Completed as i64,
                    data.workflow_id
                ],
            )
            .await
        {
            Ok(_) => Ok(Response::new(CompleteWorkflowResponse {})),
            Err(e) => Err(Status::internal(e.to_string())),
        };
        result
    }
}

pub async fn start_server(
    rpc_addr: &str,
    client: &Client,
) -> Result<(), Box<dyn std::error::Error>> {
    // defining address for our service
    let addr = rpc_addr.parse().unwrap();
    println!("Server listening on {}", addr);
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
