use std::error::Error;

use hiqlite::Client;
use hiqlite_macros::params;

pub async fn lease_checkpoint(
    client: &Client,
    workflow_id: &str,
    checkpoint_position: i64,
    lease_timeout: i64,
) -> Result<i64, Box<dyn Error>> {
    let mut lease_timeout = client
        .execute_returning_one(
            "INSERT INTO CheckpointLeases (workflow_id, checkpoint_position, lease_timeout) VALUES ($1, $2, $3) ON CONFLICT (workflow_id, checkpoint_position) DO UPDATE SET lease_timeout = $3 RETURNING lease_timeout",
            params![workflow_id, checkpoint_position, lease_timeout],
        )
        .await?;
    Ok(lease_timeout.get::<i64>("lease_timeout"))
}

pub async fn get_lease_timeout(
    client: &Client,
    workflow_id: &str,
    checkpoint_position: i64,
) -> Result<Option<i64>, Box<dyn Error>> {
    let lease_timeout = client
        .query_as_optional::<i64, _>(
            "SELECT lease_timeout FROM CheckpointLeases WHERE workflow_id = $1 AND checkpoint_position = $2",
            params![workflow_id, checkpoint_position],
        )
        .await?;
    Ok(lease_timeout)
}
