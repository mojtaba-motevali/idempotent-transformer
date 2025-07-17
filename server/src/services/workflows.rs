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
    })
}
