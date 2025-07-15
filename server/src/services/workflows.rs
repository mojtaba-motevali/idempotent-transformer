use std::error::Error;

use chrono::Utc;
use hiqlite::{Client, Param};
use hiqlite_macros::params;

use crate::schema::workflow::{Workflow, WorkflowStatus};

pub async fn get_workflow(
    client: &Client,
    workflow_id: &str,
) -> Result<Option<Workflow>, Box<dyn Error>> {
    println!("get_workflow: {:?}", workflow_id);
    let result = client
        .query_as_optional::<Workflow, _>(
            "SELECT * FROM Workflows WHERE id = $1",
            params![workflow_id],
        )
        .await?;
    println!("result: {:?}", result);
    Ok(result)
}

pub fn get_create_workflow_query(workflow_id: &str) -> (&'static str, Vec<Param>) {
    (
        "INSERT INTO Workflows (id, status, created_at) VALUES (?, ?, ?)",
        params![
            workflow_id,
            WorkflowStatus::Running as i64,
            Utc::now().timestamp_millis()
        ],
    )
}
