use std::error::Error;

use chrono::Utc;
use hiqlite::Client;
use hiqlite_macros::params;

use crate::schema::workflow::{Workflow, WorkflowStatus};

pub async fn create_or_get_workflow(
    client: &Client,
    workflow_id: &str,
    status: WorkflowStatus,
) -> Result<Workflow, Box<dyn Error + Send + Sync>> {
    let mut result = client.execute_returning_one(
        "INSERT INTO Workflows (id, status, created_at) VALUES ($1, $2, $3) ON CONFLICT (id) DO UPDATE SET status = Workflows.status RETURNING *",
        params![
            workflow_id,
            status as i64,
            Utc::now().timestamp_millis()
        ],
    ).await?;
    Ok(Workflow {
        id: result.get::<String>("id"),
        status: result.get::<i64>("status"),
        created_at: result.get::<i64>("created_at"),
        expire_at: result.get::<Option<i64>>("expire_at"),
        completed_at: result.get::<Option<i64>>("completed_at"),
    })
}

pub async fn get_workflow(
    client: &Client,
    workflow_id: &str,
) -> Result<Option<Workflow>, Box<dyn Error + Send + Sync>> {
    let result = client
        .query_as_optional::<Workflow, _>(
            "SELECT * FROM Workflows WHERE id = $1",
            params![workflow_id],
        )
        .await?;
    Ok(result)
}

pub async fn delete_expired_workflows(
    client: &Client,
    current_timestamp: i64,
    status: i8,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    client
        .execute(
            "DELETE FROM Workflows WHERE expire_at < $1 AND status = $2",
            params![current_timestamp, status],
        )
        .await?;
    Ok(())
}
