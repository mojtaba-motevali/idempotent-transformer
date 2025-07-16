use chrono::Utc;
use hiqlite::Client;
use hiqlite_macros::params;

use tonic::{Request, Response, Status, transport::Server};
pub mod workflow_service {
    tonic::include_proto!("workflow_service");
}
use std::time::{SystemTime, UNIX_EPOCH};

use crate::schema::workflow::WorkflowStatus;
use crate::services::checkpoints::{
    count_checkpoints, create_checkpoint, get_checkpoint, get_checkpoints,
};
use crate::services::lease_checkpoint::{get_lease_timeout, lease_checkpoint};
use crate::services::workflows::{get_create_workflow_query, get_workflow};
use crate::services::workflows_fencing_tokens::get_workflow_fencing_token;

use workflow_service::{
    CheckPointRequest, CheckPointResponse, CompleteWorkflowRequest, CompleteWorkflowResponse,
    LeaseCheckpointRequest, LeaseCheckpointResponse, LeaseWorkflowRequest, LeaseWorkflowResponse,
    WorkflowCheckpoint,
    lease_checkpoint_response::Response::{AcquiredLeaseTimeout, LeaseTimeout, Value},
    workflow_service_impl_server::WorkflowServiceImpl,
    workflow_service_impl_server::WorkflowServiceImplServer,
};

/// Converts any error to a Status for easier use with the ? operator
fn to_status<T, E: std::fmt::Display>(result: Result<T, E>) -> Result<T, Status> {
    result.map_err(|e| Status::internal(e.to_string()))
}

fn return_error_if_true(condition: bool, status: Status) -> Result<(), Status> {
    if condition { Err(status) } else { Ok(()) }
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
        let (token, lease_timeout) = tokio::join!(
            get_workflow_fencing_token(&self.client, &data.workflow_id),
            get_lease_timeout(&self.client, &data.workflow_id, data.position_checksum)
        );
        let stored_fencing_token = to_status(token)?;
        let lease_timeout = to_status(lease_timeout)?;

        return_error_if_true(
            stored_fencing_token.is_none(),
            Status::aborted("fencing_token_not_found"),
        )?;

        if let Some(unwrapped_lease_timeout) = lease_timeout {
            if unwrapped_lease_timeout > Utc::now().timestamp_millis() {
                return_error_if_true(
                    stored_fencing_token.unwrap() > data.fencing_token,
                    Status::aborted("fencing_token_expired"),
                )?;
            }
        }

        to_status(
            create_checkpoint(
                &self.client,
                &data.workflow_id,
                data.value,
                data.position_checksum,
                &data.workflow_context_name,
                &data.checkpoint_context_name,
            )
            .await,
        )
        .map(|_| Response::new(CheckPointResponse {}))
    }

    async fn lease_checkpoint(
        &self,
        request: Request<LeaseCheckpointRequest>,
    ) -> Result<Response<LeaseCheckpointResponse>, Status> {
        let data = request.into_inner();
        let sent_fencing_token = data.fencing_token;

        // Lease checkpoint is only allowed if the workflow has a fencing token
        let found_fencing_token =
            to_status(get_workflow_fencing_token(&self.client, &data.workflow_id).await)?;
        return_error_if_true(
            found_fencing_token.is_none(),
            Status::aborted("fencing_token_not_found"),
        )?;
        // old fencing tokens can not be used to lease a checkpoint
        let stored_fencing_token = found_fencing_token.unwrap();
        return_error_if_true(
            stored_fencing_token > sent_fencing_token,
            Status::aborted("fencing_token_expired"),
        )?;

        let result = to_status(
            get_checkpoint(&self.client, &data.workflow_id, data.position_checksum).await,
        )?;

        // if checkpoint is already leased, then we need to return the value
        if result.is_some() {
            return Ok(Response::new(LeaseCheckpointResponse {
                response: Some(Value(result.unwrap().value)),
            }));
        }
        // if fencing token is the same, then we need to lease the checkpoint
        if sent_fencing_token == stored_fencing_token {
            // lease checkpoint
            let lease_timeout = to_status(
                lease_checkpoint(
                    &self.client,
                    &data.workflow_id,
                    data.position_checksum,
                    data.lease_timeout,
                )
                .await,
            )?;
            return Ok(Response::new(LeaseCheckpointResponse {
                response: Some(AcquiredLeaseTimeout(lease_timeout)),
            }));
        }
        // if fencing token is greater, then we need to get the checkpoint
        if sent_fencing_token > stored_fencing_token {
            // get lease timeout
            let lease_timeout = to_status(
                get_lease_timeout(&self.client, &data.workflow_id, data.position_checksum).await,
            )?;

            return_error_if_true(
                lease_timeout.is_none(),
                Status::aborted("lease_timeout_not_found"),
            )?;

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
        let mut checkpoint_fut = None;
        let mut fencing_token_fut = None;
        let mut checkpoints = vec![];
        let mut found_fencing_token = None;
        let workflow_fut = get_workflow(&self.client, &data.workflow_id);
        let mut total_workflow_context_based_checkpoints: i64 = 0;

        // prefetch checkpoints if requested
        if data.prefetch_checkpoints {
            checkpoint_fut = Some(get_checkpoints(&self.client, &data.workflow_id));
        }

        // prefetch fencing token if workflow is nested
        if data.is_nested {
            fencing_token_fut = Some(get_workflow_fencing_token(&self.client, &data.workflow_id));
        }

        // get checkpoints if requested
        if let Some(fut) = checkpoint_fut {
            checkpoints = to_status(fut.await)?;
            total_workflow_context_based_checkpoints = checkpoints.len() as i64;
        } else {
            total_workflow_context_based_checkpoints = to_status(
                count_checkpoints(&self.client, &data.workflow_id, &data.context_name).await,
            )?;
        }

        let workflow = to_status(workflow_fut.await)?;
        if let Some(fut) = fencing_token_fut {
            found_fencing_token = to_status(fut.await)?;
        };

        return_error_if_true(
            found_fencing_token.is_none(),
            Status::aborted("nested_workflow_fencing_token_conflict"),
        )?;

        let fencing_token = found_fencing_token.unwrap();

        let mut queries = vec![];

        if workflow.is_none() {
            queries.push(get_create_workflow_query(&data.workflow_id));
        }

        if !data.is_nested {
            queries.push((
                "INSERT INTO WorkflowFencingTokens (workflow_id, fencing_token) VALUES (?, ?) ON CONFLICT (workflow_id) DO UPDATE SET fencing_token = fencing_token + 1 RETURNING fencing_token",
                params![&data.workflow_id, 1 as i64],
            ));
        }

        to_status(self.client.txn(queries).await)?;

        Ok(Response::new(LeaseWorkflowResponse {
            checkpoints: checkpoints
                .into_iter()
                .map(|v| WorkflowCheckpoint {
                    position_checksum: v.position_checksum,
                    value: v.value,
                })
                .collect(),
            fencing_token: fencing_token,
            total_context_bound_checkpoints: total_workflow_context_based_checkpoints,
        }))
    }

    async fn complete_workflow(
        &self,
        request: Request<CompleteWorkflowRequest>,
    ) -> Result<Response<CompleteWorkflowResponse>, Status> {
        let data = request.into_inner();
        let token = to_status(get_workflow_fencing_token(&self.client, &data.workflow_id).await)?;
        return_error_if_true(token.is_none(), Status::aborted("fencing_token_not_found"))?;
        return_error_if_true(
            token.unwrap() < data.fencing_token,
            Status::aborted("fencing_token_expired"),
        )?;
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
