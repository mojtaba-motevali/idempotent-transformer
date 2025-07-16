use chrono::{TimeZone, Utc};
use hiqlite::Client;
use hiqlite_macros::params;

use tonic::{Request, Response, Status, transport::Server};
pub mod workflow_service {
    tonic::include_proto!("workflow_service");
}

use crate::schema::checkpoint::CheckpointValue;
use crate::schema::workflow::{Workflow, WorkflowStatus};
use crate::services::checkpoints::{
    count_checkpoints, create_checkpoint, get_checkpoint, get_checkpoints,
};
use crate::services::lease_checkpoint::{get_leased_checkpoint, lease_checkpoint};
use crate::services::workflows::{get_create_workflow_query, get_workflow};
use crate::services::workflows_fencing_tokens::get_workflow_fencing_token;

use workflow_service::{
    CheckPointRequest, CheckPointResponse, CompleteWorkflowRequest, CompleteWorkflowResponse,
    LeaseCheckpointRequest, LeaseCheckpointResponse, LeaseWorkflowRequest, LeaseWorkflowResponse,
    lease_checkpoint_response::Response::Value, workflow_service_impl_server::WorkflowServiceImpl,
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
        let (token, leased_checkpoint_result) = tokio::join!(
            get_workflow_fencing_token(&self.client, &data.workflow_id),
            get_leased_checkpoint(&self.client, &data.workflow_id, data.position_checksum)
        );
        let stored_fencing_token = to_status(token)?;
        let leased_checkpoint = to_status(leased_checkpoint_result)?;

        return_error_if_true(
            stored_fencing_token.is_none(),
            Status::aborted("fencing_token_not_found"),
        )?;

        let mut abort = false;

        // Reject the result if the lease timeout is expired for the worker lease used to belong to.
        if let Some(unwrapped_lease_timeout) = leased_checkpoint {
            // if lease timeout is expired, then we need to check if the fencing token is expired
            if unwrapped_lease_timeout.lease_timeout + unwrapped_lease_timeout.created_at
                < Utc::now().timestamp_millis()
            {
                return_error_if_true(
                    stored_fencing_token.unwrap() > data.fencing_token,
                    Status::aborted("fencing_token_expired"),
                )?;
                abort = true;
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
        .map(|_| Response::new(CheckPointResponse { abort: abort }))
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
            to_status(
                lease_checkpoint(
                    &self.client,
                    &data.workflow_id,
                    data.position_checksum,
                    data.lease_timeout,
                )
                .await,
            )?;
            return Ok(Response::new(LeaseCheckpointResponse { response: None }));
        }
        // if fencing token is greater, then we need to get the checkpoint
        if sent_fencing_token > stored_fencing_token {
            // get lease timeout
            let leased_checkpoint = to_status(
                get_leased_checkpoint(&self.client, &data.workflow_id, data.position_checksum)
                    .await,
            )?;

            return_error_if_true(
                leased_checkpoint.is_none(),
                Status::aborted("lease_timeout_not_found"),
            )?;

            let leased_checkpoint = leased_checkpoint.unwrap();

            return_error_if_true(
                (leased_checkpoint.lease_timeout + leased_checkpoint.created_at)
                    < Utc::now().timestamp_millis(),
                Status::aborted("checkpoint_leased_by_other_workflow"),
            )?;
        }
        println!("data: {:?}", data);
        Err(Status::internal("unexpected state"))
    }

    async fn lease_workflow(
        &self,
        request: Request<LeaseWorkflowRequest>,
    ) -> Result<Response<LeaseWorkflowResponse>, Status> {
        let data = request.into_inner();
        println!("data: {:?}", data);
        let (workflow, checkpoints, found_fencing_token) = tokio::join!(
            get_workflow(&self.client, &data.workflow_id),
            async {
                if data.prefetch_checkpoints {
                    get_checkpoints(&self.client, &data.workflow_id).await
                } else {
                    Ok(vec![])
                }
            },
            async {
                if data.is_nested {
                    get_workflow_fencing_token(&self.client, &data.workflow_id).await
                } else {
                    Ok(None)
                }
            }
        );

        let workflow = to_status(workflow)?;
        let checkpoints = to_status(checkpoints)?;
        let found_fencing_token = to_status(found_fencing_token)?;
        let mut total_workflow_context_based_checkpoints: i64 = 0;

        // Handle checkpoints
        if data.prefetch_checkpoints {
            total_workflow_context_based_checkpoints = checkpoints.len() as i64;
        } else {
            total_workflow_context_based_checkpoints = to_status(
                count_checkpoints(&self.client, &data.workflow_id, &data.context_name).await,
            )?;
        }

        let mut queries = vec![];

        if workflow.is_none() {
            queries.push(get_create_workflow_query(&data.workflow_id));
        }

        if !data.is_nested {
            queries.push((
                "INSERT INTO WorkflowFencingTokens (workflow_id, fencing_token) VALUES (?, ?) ON CONFLICT (workflow_id) DO UPDATE SET fencing_token = fencing_token + 1 RETURNING fencing_token",
                params![&data.workflow_id, 1 as i64],
            ));
        } else {
            // Handle fencing token for nested workflows
            return_error_if_true(
                found_fencing_token.is_none(),
                Status::aborted("nested_workflow_fencing_token_conflict"),
            )?;
        }

        to_status(self.client.txn(queries).await)?;

        let fencing_token = if data.is_nested {
            found_fencing_token.unwrap_or(0)
        } else {
            to_status(get_workflow_fencing_token(&self.client, &data.workflow_id).await)?.unwrap()
        };
        let hash_map = checkpoints
            .into_iter()
            .map(|value| (value.position_checksum.to_string(), value.value))
            .collect();
        println!("hash_map: {:?}", hash_map);
        Ok(Response::new(LeaseWorkflowResponse {
            checkpoints: hash_map,
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
                    Utc::now()
                        .checked_add_signed(chrono::Duration::milliseconds(data.expire_after)),
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
