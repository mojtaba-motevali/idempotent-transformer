use chrono::Utc;
use std::error::Error;

use hiqlite::Client;
use hiqlite_macros::params;

use crate::schema::checkpoint::CheckpointValue;

pub async fn get_checkpoints(
    client: &Client,
    workflow_id: &str,
) -> Result<Vec<CheckpointValue>, Box<dyn Error>> {
    let checkpoints = client
        .query_map::<CheckpointValue, _>(
            "SELECT position, value FROM Checkpoints WHERE workflow_id = $1 ORDER BY position ASC",
            params![workflow_id],
        )
        .await?;
    Ok(checkpoints)
}

pub async fn get_checkpoint(
    client: &Client,
    workflow_id: &str,
    id: i64,
) -> Result<Option<CheckpointValue>, Box<dyn Error>> {
    let checkpoint = client
        .query_as_optional::<CheckpointValue, _>(
            "SELECT position, value FROM Checkpoints WHERE workflow_id = $1 AND position = $2",
            params![workflow_id, id],
        )
        .await?;
    Ok(checkpoint)
}

pub async fn create_checkpoint(
    client: &Client,
    workflow_id: &str,
    value: Vec<u8>,
    position: i64,
) -> Result<(), Box<dyn Error>> {
    client
        .execute(
            "INSERT INTO Checkpoints (workflow_id, position, value, created_at) VALUES ($1, $2, $3, $4)",
            params![workflow_id, position, value, Utc::now().timestamp_millis()],
        )
        .await?;
    Ok(())
}
