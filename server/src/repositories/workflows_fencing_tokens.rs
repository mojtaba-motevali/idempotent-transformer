use std::error::Error;

use hiqlite::Client;
use hiqlite_macros::params;

pub async fn get_workflow_fencing_token(
    client: &Client,
    workflow_id: &str,
) -> Result<Option<i64>, Box<dyn Error + Send + Sync>> {
    let fencing_token = client
        .query_as_optional::<i64, _>(
            "SELECT fencing_token FROM WorkflowFencingTokens WHERE workflow_id = $1",
            params![workflow_id],
        )
        .await?;
    Ok(fencing_token)
}

pub async fn increment_workflow_fencing_token(
    client: &Client,
    workflow_id: &str,
    initial_fencing_token: i64,
) -> Result<i64, Box<dyn Error + Send + Sync>> {
    let mut result = client.execute_returning_one(
        "INSERT INTO WorkflowFencingTokens (workflow_id, fencing_token) VALUES ($1, $2) RETURNING fencing_token",
        params![workflow_id, initial_fencing_token],
    ).await?;
    Ok(result.get::<i64>("fencing_token"))
}

pub async fn delete_expired_workflow_fencing_tokens(
    client: &Client,
    current_timestamp: i64,
    status: i8,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    client
        .execute(
            "DELETE FROM WorkflowFencingTokens WHERE workflow_id IN (SELECT id FROM Workflows WHERE expire_at < $1 AND status = $2)",
            params![current_timestamp, status ],
        )
        .await?;
    Ok(())
}
