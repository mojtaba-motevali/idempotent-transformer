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
