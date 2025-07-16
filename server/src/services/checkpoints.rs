use chrono::Utc;
use std::error::Error;

use hiqlite::Client;
use hiqlite_macros::params;

use crate::schema::checkpoint::CheckpointValue;

pub async fn get_checkpoints(
    client: &Client,
    workflow_id: &str,
) -> Result<Vec<CheckpointValue>, Box<dyn Error + Send + Sync>> {
    let checkpoints = client
        .query_map::<CheckpointValue, _>(
            "SELECT position, value FROM Checkpoints WHERE workflow_id = $1 ORDER BY position ASC",
            params![workflow_id],
        )
        .await?;
    Ok(checkpoints)
}

pub async fn count_checkpoints(
    client: &Client,
    workflow_id: &str,
    workflow_context_name: &str,
) -> Result<i64, Box<dyn Error + Send + Sync>> {
    let checkpoints = client
        .query_as_one::<i64, _>(
            "SELECT COUNT(*) FROM Checkpoints WHERE workflow_id = $1 AND workflow_context_name = $2",
            params![workflow_id, workflow_context_name],
        )
        .await?;
    Ok(checkpoints)
}

pub async fn get_checkpoint(
    client: &Client,
    workflow_id: &str,
    position_checksum: i64,
) -> Result<Option<CheckpointValue>, Box<dyn Error + Send + Sync>> {
    let checkpoint = client
        .query_as_optional::<CheckpointValue, _>(
            "SELECT position_checksum, value FROM Checkpoints WHERE workflow_id = $1 AND position_checksum = $2",
            params![workflow_id, position_checksum],
        )
        .await?;
    Ok(checkpoint)
}

pub async fn create_checkpoint(
    client: &Client,
    workflow_id: &str,
    value: Vec<u8>,
    position_checksum: i64,
    workflow_context_name: &str,
    checkpoint_context_name: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    client
        .execute(
            "INSERT INTO Checkpoints (workflow_id, position_checksum, value, workflow_context_name, checkpoint_context_name, created_at) VALUES ($1, $2, $3, $4, $5, $6)",
            params![workflow_id, position_checksum, value, workflow_context_name, checkpoint_context_name, Utc::now().timestamp_millis()],
        )
        .await?;
    Ok(())
}
