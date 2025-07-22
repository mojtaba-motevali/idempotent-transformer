use chrono::Utc;
use hiqlite::Client;
use hiqlite_macros::params;
use std::error::Error;

use crate::helpers::common::return_error_if_true;
use crate::repositories::workflows::create_or_get_workflow;
use crate::repositories::workflows_fencing_tokens::{
    get_workflow_fencing_token, increment_workflow_fencing_token,
};
use crate::schema::workflow::WorkflowStatus;

pub struct CreateWorkflowInput {
    pub workflow_id: String,
}

pub struct CreateWorkflowOutput {
    pub fencing_token: i64,
}

pub async fn create_workflow(
    client: &Client,
    data: CreateWorkflowInput,
) -> Result<CreateWorkflowOutput, Box<dyn Error + Send + Sync>> {
    let (workflow_id_result, found_fencing_token) = tokio::join!(
        create_or_get_workflow(client, &data.workflow_id, WorkflowStatus::Running),
        get_workflow_fencing_token(client, &data.workflow_id)
    );
    workflow_id_result?;

    let found_fencing_token = found_fencing_token?;

    let mut fencing_token = found_fencing_token.unwrap_or(-1);

    if fencing_token == -1 {
        fencing_token = increment_workflow_fencing_token(client, &data.workflow_id, 1).await?;
    }

    Ok(CreateWorkflowOutput { fencing_token })
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
