use std::error::Error;

use chrono::Utc;
use hiqlite::Client;
use hiqlite_macros::params;

use crate::schema::leased_checkpoint::LeasedCheckpointValue;

pub async fn lease_checkpoint(
    client: &Client,
    workflow_id: &str,
    position_checksum: i64,
    lease_timeout: i64,
) -> Result<LeasedCheckpointValue, Box<dyn Error + Send + Sync>> {
    let mut leased_checkpoint = client
        .execute_returning_one(
            "INSERT INTO CheckpointLeases 
            (workflow_id, position_checksum, lease_timeout, created_at) 
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (workflow_id, position_checksum) DO UPDATE SET lease_timeout = CheckpointLeases.lease_timeout 
            RETURNING lease_timeout, created_at",
            params![
                workflow_id,
                position_checksum,
                lease_timeout,
                Utc::now().timestamp_millis()
            ],
        )
        .await?;
    Ok(LeasedCheckpointValue {
        lease_timeout: leased_checkpoint.get::<i64>("lease_timeout"),
        created_at: leased_checkpoint.get::<i64>("created_at"),
    })
}

pub async fn remove_leased_checkpoint(
    client: &Client,
    workflow_id: &str,
    position_checksum: i64,
) -> Result<LeasedCheckpointValue, Box<dyn Error + Send + Sync>> {
    let mut leased_checkpoint = client
        .execute_returning_one(
            "DELETE FROM CheckpointLeases WHERE workflow_id = $1 AND position_checksum = $2 RETURNING lease_timeout, created_at",
            params![workflow_id, position_checksum],
        )
        .await?;
    Ok(LeasedCheckpointValue {
        lease_timeout: leased_checkpoint.get::<i64>("lease_timeout"),
        created_at: leased_checkpoint.get::<i64>("created_at"),
    })
}

pub async fn get_leased_checkpoint(
    client: &Client,
    workflow_id: &str,
    position_checksum: i64,
) -> Result<Option<LeasedCheckpointValue>, Box<dyn Error + Send + Sync>> {
    let leased_checkpoint = client
        .query_as_optional::<LeasedCheckpointValue, _>(
            "SELECT lease_timeout, created_at FROM CheckpointLeases WHERE workflow_id = $1 AND position_checksum = $2",
            params![workflow_id, position_checksum],
        )
        .await?;
    Ok(leased_checkpoint)
}

pub async fn delete_expired_leased_checkpoints(
    client: &Client,
    current_timestamp: i64,
    status: i8,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    client
        .execute(
            "DELETE FROM CheckpointLeases WHERE workflow_id IN (SELECT id FROM Workflows WHERE expire_at < $1 AND status = $2)",
            params![current_timestamp, status ],
        )
        .await?;
    Ok(())
}
