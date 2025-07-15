use std::error::Error;

use hiqlite::{Client, Row};
use hiqlite_macros::params;

use crate::schema::workflow_fencing_token::WorkflowFencingToken;

pub async fn create_workflow_fencing_token(
    client: &Client,
    workflow_id: &str,
) -> Result<WorkflowFencingToken, Box<dyn Error>> {
    let mut result: Row<'_> = match client.execute_returning_one(
        "INSERT INTO WorkflowFencingTokens (workflow_id, fencing_token) VALUES (?, ?) ON CONFLICT (workflow_id) DO UPDATE SET fencing_token = fencing_token + 1 RETURNING fencing_token",
         params![workflow_id, 1 as i64]).await {
        Ok(result) => result,
        Err(e) => return Err(Box::new(e)),
    };
    Ok(WorkflowFencingToken {
        workflow_id: workflow_id.to_string(),
        fencing_token: result.get::<i64>("fencing_token"),
    })
}

pub async fn get_workflow_fencing_token(
    client: &Client,
    workflow_id: &str,
) -> Result<Option<i64>, Box<dyn Error>> {
    let fencing_token = client
        .query_as_optional::<i64, _>(
            "SELECT fencing_token FROM WorkflowFencingTokens WHERE workflow_id = $1",
            params![workflow_id],
        )
        .await?;
    Ok(fencing_token)
}
