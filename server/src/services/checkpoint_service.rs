use std::error::Error;

use hiqlite::Client;

use crate::helpers::common::return_error_if_true;
use crate::repositories::lease_checkpoint::{get_leased_checkpoint, lease_checkpoint};
use crate::repositories::{
    checkpoints::{create_checkpoint, get_checkpoint},
    lease_checkpoint::remove_leased_checkpoint,
    workflows_fencing_tokens::get_workflow_fencing_token,
};
use crate::schema::leased_checkpoint::LeasedCheckpointValue;

pub struct CheckpointInput {
    pub workflow_id: String,
    pub fencing_token: i64,
    pub position_checksum: i64,
    pub value: Vec<u8>,
    pub idempotency_checksum: i64,
}

pub struct CheckpointOutput {
    pub abort: bool,
}

fn has_expired(leased_checkpoint: &LeasedCheckpointValue) -> bool {
    let now = chrono::Utc::now().timestamp_millis();
    let expires_at = leased_checkpoint
        .created_at
        .saturating_add(leased_checkpoint.lease_timeout);
    now <= expires_at
}

pub async fn handle_checkpoint(
    client: &Client,
    data: CheckpointInput,
) -> Result<CheckpointOutput, Box<dyn Error + Send + Sync>> {
    let (internal_fencing_token, leased_checkpoint) = tokio::join!(
        get_workflow_fencing_token(client, &data.workflow_id),
        remove_leased_checkpoint(client, &data.workflow_id, data.position_checksum)
    );

    let stored_fencing_token = internal_fencing_token?;
    let leased_checkpoint = leased_checkpoint?;

    return_error_if_true(
        stored_fencing_token.is_none(),
        Box::new(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "fencing_token_not_found",
        )),
    )?;

    let mut abort = false;
    let is_fencing_token_expired = stored_fencing_token.unwrap() > data.fencing_token;

    // Reject the result if the lease timeout is expired for the worker lease used to belong to.
    // if lease timeout is expired, then we need to check if the fencing token is expired
    if !has_expired(&leased_checkpoint) {
        return_error_if_true(
            is_fencing_token_expired,
            Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "fencing_token_expired",
            )),
        )?;
    }

    if is_fencing_token_expired {
        abort = true;
    }
    create_checkpoint(
        client,
        &data.workflow_id,
        data.value,
        data.position_checksum,
        data.idempotency_checksum,
    )
    .await?;

    Ok(CheckpointOutput { abort })
}

pub struct LeaseCheckpointInput {
    pub workflow_id: String,
    pub fencing_token: i64,
    pub position_checksum: i64,
    pub lease_timeout: i64,
    pub idempotency_checksum: i64,
}

pub struct LeaseCheckpointOutput {
    pub response: Option<Vec<u8>>,
}

pub async fn handle_lease_checkpoint(
    client: &Client,
    data: LeaseCheckpointInput,
) -> Result<LeaseCheckpointOutput, Box<dyn Error + Send + Sync>> {
    // A lock key can be any String to provide the most flexibility.
    // It behaves the same as any other lock - it will be released on drop and as long as it
    // exists, other locks will have to wait.
    //
    // In the current implementation, distributed locks have an internal timeout of 10 seconds.
    // When this time expires, a lock will be considered "dead" because of network issues, just
    // in case it has not been possible to release the lock properly. This prevents deadlocks
    // just because some client or server crashed.

    let result = get_checkpoint(client, &data.workflow_id, data.position_checksum).await?;

    // if checkpoint is already leased, then we need to return the value
    if let Some(checkpoint) = result {
        return_error_if_true(
            checkpoint.idempotency_checksum != data.idempotency_checksum,
            Box::new(std::io::Error::new(
                std::io::ErrorKind::Interrupted,
                "non_deterministic_checkpoint_found",
            )),
        )?;
        return Ok(LeaseCheckpointOutput {
            response: Some(checkpoint.value),
        });
    }

    let lock_key = data.workflow_id.clone() + "_" + &data.position_checksum.to_string();
    // lock will automatically be released when the lock is dropped
    let lock = match client.lock(lock_key).await {
        Ok(lock) => lock,
        Err(err) => {
            println!("Lock acquisition failed: {err}");
            return Err(Box::new(std::io::Error::other(err.to_string())));
        }
    };
    // At least 40 milliseconds is required for lock being held, otherwise thread panics under high load.
    // tokio::time::sleep(std::time::Duration::from_millis(40)).await;

    let sent_fencing_token = data.fencing_token;
    let (leased_checkpoint_result, workflow_fencing_token) = tokio::join!(
        get_leased_checkpoint(client, &data.workflow_id, data.position_checksum),
        get_workflow_fencing_token(client, &data.workflow_id),
    );
    let leased_checkpoint_result = leased_checkpoint_result?;
    let workflow_fencing_token = workflow_fencing_token?;
    if let Some(leased_checkpoint) = leased_checkpoint_result {
        return_error_if_true(
            has_expired(&leased_checkpoint),
            Box::new(std::io::Error::new(
                std::io::ErrorKind::Interrupted,
                "checkpoint_leased_by_other_worker",
            )),
        )?;
    }
    // Lease checkpoint is only allowed if the workflow has a fencing token
    let found_fencing_token = workflow_fencing_token;

    return_error_if_true(
        found_fencing_token.is_none(),
        Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "fencing_token_not_found",
        )),
    )?;
    // old fencing tokens can not be used to lease a checkpoint
    let stored_fencing_token = found_fencing_token.unwrap();

    return_error_if_true(
        stored_fencing_token > sent_fencing_token,
        Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "fencing_token_expired",
        )),
    )?;

    // if fencing token is the same, then we need to lease the checkpoint
    if sent_fencing_token == stored_fencing_token {
        lease_checkpoint(
            client,
            &data.workflow_id,
            data.position_checksum,
            data.lease_timeout,
        )
        .await?;
        return Ok(LeaseCheckpointOutput { response: None });
    }
    drop(lock);
    Err(Box::new(std::io::Error::other("unexpected state")))
}

pub async fn release_checkpoint(
    client: &Client,
    workflow_id: &str,
    position_checksum: i64,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    remove_leased_checkpoint(client, workflow_id, position_checksum).await?;
    Ok(())
}
