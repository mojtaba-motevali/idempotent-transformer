use chrono::Utc;
use std::error::Error;

use hiqlite::Client;
use hiqlite_macros::params;

use crate::schema::checkpoint::CheckpointValue;

pub async fn get_checkpoint(
    client: &Client,
    workflow_id: &str,
    position: i64,
) -> Result<Option<CheckpointValue>, Box<dyn Error + Send + Sync>> {
    let checkpoint = client
        .query_as_optional::<CheckpointValue, _>(
            "SELECT position, idempotency_key, value FROM Checkpoints WHERE workflow_id = $1 AND position = $2",
            params![workflow_id, position],
        )
        .await?;
    Ok(checkpoint)
}

pub async fn create_checkpoint(
    client: &Client,
    workflow_id: &str,
    value: Option<Vec<u8>>,
    position: i64,
    idempotency_key: String,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    client
        .execute(
            "INSERT INTO Checkpoints (workflow_id, position, idempotency_key, value, created_at) VALUES ($1, $2, $3, $4, $5) ON CONFLICT (workflow_id, position) DO UPDATE SET created_at = $5",
            params![workflow_id, position, idempotency_key, value, Utc::now().timestamp_millis()],
        )
        .await?;
    Ok(())
}

pub async fn delete_expired_checkpoints(
    client: &Client,
    current_timestamp: i64,
    status: i8,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    client
        .execute(
            "DELETE FROM Checkpoints WHERE workflow_id IN (SELECT id FROM Workflows WHERE expire_at < $1 AND status = $2)",
            params![current_timestamp, status ],
        )
        .await?;
    Ok(())
}
