use chrono::Utc;
use std::error::Error;

use hiqlite::Client;
use hiqlite_macros::params;

use crate::schema::checkpoint::CheckpointValue;

pub async fn get_checkpoint(
    client: &Client,
    workflow_id: &str,
    position_checksum: i64,
) -> Result<Option<CheckpointValue>, Box<dyn Error + Send + Sync>> {
    let checkpoint = client
        .query_as_optional::<CheckpointValue, _>(
            "SELECT position_checksum, idempotency_checksum, value FROM Checkpoints WHERE workflow_id = $1 AND position_checksum = $2",
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
    idempotency_checksum: i64,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    client
        .execute(
            "INSERT INTO Checkpoints (workflow_id, position_checksum, idempotency_checksum, value, created_at) VALUES ($1, $2, $3, $4, $5) ON CONFLICT (workflow_id, position_checksum) DO UPDATE SET created_at = $5",
            params![workflow_id, position_checksum, idempotency_checksum, value, Utc::now().timestamp_millis()],
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
