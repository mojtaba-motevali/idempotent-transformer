use chrono::Utc;
use hiqlite::Client;
use hiqlite_macros::params;
use std::error::Error;

use crate::helpers::common::return_error_if_true;
use crate::repositories::checkpoints::delete_expired_checkpoints;
use crate::repositories::workflows::{create_or_get_workflow, delete_expired_workflows};
use crate::repositories::workflows_fencing_tokens::{
    delete_expired_workflow_fencing_tokens, get_workflow_fencing_token,
    increment_workflow_fencing_token,
};
use crate::schema::workflow::WorkflowStatus;

pub struct CreateWorkflowInput {
    pub workflow_id: String,
    pub name: Option<String>,
}

pub struct CreateWorkflowOutput {
    pub fencing_token: i64,
}

pub async fn create_workflow(
    client: &Client,
    data: CreateWorkflowInput,
) -> Result<CreateWorkflowOutput, Box<dyn Error + Send + Sync>> {
    let (_, fencing_token) = tokio::join!(
        create_or_get_workflow(
            client,
            &data.workflow_id,
            WorkflowStatus::Running,
            data.name
        ),
        increment_workflow_fencing_token(client, &data.workflow_id, 1),
    );

    Ok(CreateWorkflowOutput {
        fencing_token: fencing_token?,
    })
}

pub struct FinishWorkflowInput {
    pub workflow_id: String,
    pub fencing_token: i64,
    pub expire_after: i64,
}

pub struct FinishWorkflowOutput {}

pub async fn finish_workflow(
    client: &Client,
    data: FinishWorkflowInput,
) -> Result<FinishWorkflowOutput, Box<dyn Error + Send + Sync>> {
    let token = get_workflow_fencing_token(client, &data.workflow_id).await?;
    return_error_if_true(
        token.is_none(),
        Box::new(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "fencing_token_not_found",
        )),
    )?;
    return_error_if_true(
        token.unwrap() < data.fencing_token,
        Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "fencing_token_expired",
        )),
    )?;
    let expire_at = Utc::now().timestamp_millis() + data.expire_after;

    client
        .execute(
            "UPDATE Workflows SET expire_at = $1, status = $2, completed_at = $3  WHERE id = $4",
            params![
                expire_at,
                WorkflowStatus::Completed as i64,
                Utc::now().timestamp_millis(),
                data.workflow_id
            ],
        )
        .await?;

    Ok(FinishWorkflowOutput {})
}

pub async fn handle_workflow_cleanup(client: &Client) -> Result<(), Box<dyn Error + Send + Sync>> {
    if !client.is_leader_db().await {
        return Ok(());
    }
    println!("Deleting expired workflows");

    let current_timestamp = Utc::now().timestamp_millis();
    let status = WorkflowStatus::Completed as i8;
    let (fencing_tokens, checkpoints) = tokio::join!(
        delete_expired_workflow_fencing_tokens(client, current_timestamp, status),
        delete_expired_checkpoints(client, current_timestamp, status),
    );
    fencing_tokens?;
    checkpoints?;
    delete_expired_workflows(client, current_timestamp, status).await?;
    println!("Deleted expired workflows");
    Ok(())
}
