use chrono::Utc;
use hiqlite::{Client, Lock};
use hiqlite_macros::params;

use std::time::Duration;
use tonic::{Request, Response, Status, transport::Server};
pub mod workflow_service {
    tonic::include_proto!("workflow_service");
}

use crate::schema::leased_checkpoint::LeasedCheckpointValue;
use crate::schema::workflow::WorkflowStatus;
use crate::services::checkpoints::{create_checkpoint, get_checkpoint, get_checkpoints};
use crate::services::lease_checkpoint::{
    get_leased_checkpoint, lease_checkpoint, release_checkpoint,
};
use crate::services::workflows::create_or_get_workflow;
use crate::services::workflows_fencing_tokens::{
    get_workflow_fencing_token, increment_workflow_fencing_token,
};

use workflow_service::{
    CheckPointRequest, CheckPointResponse, CompleteWorkflowRequest, CompleteWorkflowResponse,
    LeaseCheckpointRequest, LeaseCheckpointResponse, LeaseWorkflowRequest, LeaseWorkflowResponse,
    lease_checkpoint_response::Response::Value, workflow_service_impl_server::WorkflowServiceImpl,
    workflow_service_impl_server::WorkflowServiceImplServer,
};

fn has_expired(leased_checkpoint: &LeasedCheckpointValue) -> bool {
    let now = chrono::Utc::now().timestamp_millis();
    let expires_at = leased_checkpoint
        .created_at
        .saturating_add(leased_checkpoint.lease_timeout);
    now <= expires_at
}

/// Converts any error to a Status for easier use with the ? operator
fn to_status<T, E: std::fmt::Display>(
    result: Result<T, E>,
    lock: Option<Lock>,
) -> Result<T, Status> {
    if result.is_err() {
        if let Some(lock) = lock {
            drop(lock);
        }
    }
    result.map_err(|e| Status::internal(e.to_string()))
}

fn return_error_if_true(condition: bool, status: Status, lock: Option<Lock>) -> Result<(), Status> {
    if condition {
        if let Some(lock) = lock {
            drop(lock);
        }
        Err(status)
    } else {
        Ok(())
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
        println!("Checkpoint {:?}", data);
        let (token, leased_checkpoint_result) = tokio::join!(
            get_workflow_fencing_token(&self.client, &data.workflow_id),
            release_checkpoint(&self.client, &data.workflow_id, data.position_checksum)
        );
        let stored_fencing_token = to_status(token, None)?;
        let leased_checkpoint = to_status(leased_checkpoint_result, None)?;

        return_error_if_true(
            stored_fencing_token.is_none(),
            Status::aborted("fencing_token_not_found"),
            None,
        )?;

        let mut abort = false;
        let is_fencing_token_expired = stored_fencing_token.unwrap() > data.fencing_token;

        // Reject the result if the lease timeout is expired for the worker lease used to belong to.
        // if lease timeout is expired, then we need to check if the fencing token is expired
        if !has_expired(&leased_checkpoint) {
            return_error_if_true(
                is_fencing_token_expired,
                Status::aborted("fencing_token_expired"),
                None,
            )?;
        }

        if is_fencing_token_expired {
            abort = true;
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
            None,
        )?;

        return Ok(Response::new(CheckPointResponse { abort: abort }));
    }

    async fn lease_checkpoint(
        &self,
        request: Request<LeaseCheckpointRequest>,
    ) -> Result<Response<LeaseCheckpointResponse>, Status> {
        // A lock key can be any String to provide the most flexibility.
        // It behaves the same as any other lock - it will be released on drop and as long as it
        // exists, other locks will have to wait.
        //
        // In the current implementation, distributed locks have an internal timeout of 10 seconds.
        // When this time expires, a lock will be considered "dead" because of network issues, just
        // in case it has not been possible to release the lock properly. This prevents deadlocks
        // just because some client or server crashed.

        let data = request.into_inner();

        let result = to_status(
            get_checkpoint(&self.client, &data.workflow_id, data.position_checksum).await,
            None,
        )?;

        // if checkpoint is already leased, then we need to return the value
        if result.is_some() {
            return Ok(Response::new(LeaseCheckpointResponse {
                response: Some(Value(result.unwrap().value)),
            }));
        }

        let lock_key = data.workflow_id.clone() + "_" + &data.position_checksum.to_string();

        let lock = match self.client.lock(lock_key).await {
            Ok(lock) => lock,
            Err(err) => {
                println!("Lock acquisition failed: {:?}", err);
                return Err(Status::internal(err.to_string()));
            }
        };
        // At least 40 milliseconds is required for lock being held, otherwise thread panics under high load.
        tokio::time::sleep(Duration::from_millis(40)).await;

        let sent_fencing_token = data.fencing_token;

        let leased_checkpoint_result = to_status(
            get_leased_checkpoint(&self.client, &data.workflow_id, data.position_checksum).await,
            None,
        )?;
        if let Some(leased_checkpoint) = leased_checkpoint_result {
            return_error_if_true(
                has_expired(&leased_checkpoint),
                Status::aborted("checkpoint_leased_by_other_worker"),
                None,
            )?;
        }
        // Lease checkpoint is only allowed if the workflow has a fencing token
        let found_fencing_token =
            match get_workflow_fencing_token(&self.client, &data.workflow_id).await {
                Ok(token) => token,
                Err(err) => {
                    drop(lock);
                    return Err(Status::internal(err.to_string()));
                }
            };

        return_error_if_true(
            found_fencing_token.is_none(),
            Status::aborted("fencing_token_not_found"),
            None,
        )?;
        // old fencing tokens can not be used to lease a checkpoint
        let stored_fencing_token = found_fencing_token.unwrap();

        return_error_if_true(
            stored_fencing_token > sent_fencing_token,
            Status::aborted("fencing_token_expired"),
            None,
        )?;

        // if fencing token is the same, then we need to lease the checkpoint
        if sent_fencing_token == stored_fencing_token {
            to_status(
                lease_checkpoint(
                    &self.client,
                    &data.workflow_id,
                    data.position_checksum,
                    data.lease_timeout,
                )
                .await,
                None,
            )?;
            return Ok(Response::new(LeaseCheckpointResponse { response: None }));
        }
        drop(lock);
        Err(Status::internal("unexpected state"))
    }

    async fn lease_workflow(
        &self,
        request: Request<LeaseWorkflowRequest>,
    ) -> Result<Response<LeaseWorkflowResponse>, Status> {
        let data = request.into_inner();
        let (workflow_id_result, checkpoints, found_fencing_token) = tokio::join!(
            create_or_get_workflow(&self.client, &data.workflow_id, WorkflowStatus::Running),
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
        to_status(workflow_id_result, None)?;

        let checkpoints = to_status(checkpoints, None)?;
        let found_fencing_token = to_status(found_fencing_token, None)?;

        let mut fencing_token = found_fencing_token.unwrap_or(-1);

        if data.is_nested && fencing_token == -1 {
            return_error_if_true(
                found_fencing_token.is_none(),
                Status::aborted("nested_workflow_fencing_token_conflict"),
                None,
            )?;
        }
        if fencing_token == -1 {
            fencing_token = to_status(
                increment_workflow_fencing_token(&self.client, &data.workflow_id, 1).await,
                None,
            )?;
        }

        let hash_map = checkpoints
            .into_iter()
            .map(|value| (value.position_checksum.to_string(), value.value))
            .collect();

        Ok(Response::new(LeaseWorkflowResponse {
            checkpoints: hash_map,
            fencing_token: fencing_token,
        }))
    }

    async fn complete_workflow(
        &self,
        request: Request<CompleteWorkflowRequest>,
    ) -> Result<Response<CompleteWorkflowResponse>, Status> {
        let data = request.into_inner();
        let token = to_status(
            get_workflow_fencing_token(&self.client, &data.workflow_id).await,
            None,
        )?;
        return_error_if_true(
            token.is_none(),
            Status::aborted("fencing_token_not_found"),
            None,
        )?;
        return_error_if_true(
            token.unwrap() < data.fencing_token,
            Status::aborted("fencing_token_expired"),
            None,
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
