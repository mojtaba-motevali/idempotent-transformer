use std::error::Error;

use chrono::Utc;
use hiqlite::Client;

use crate::{database::db::Cache, schema::leased_checkpoint::LeasedCheckpointValue};

fn generate_leased_checkpoint_key(position_checksum: i64) -> String {
    position_checksum.to_string()
}

pub async fn lease_checkpoint(
    client: &Client,
    position_checksum: i64,
    lease_timeout: i64,
) -> Result<LeasedCheckpointValue, Box<dyn Error + Send + Sync>> {
    let created_at = Utc::now().timestamp_millis();
    let key = generate_leased_checkpoint_key(position_checksum);
    let minimum_cache_ttl = 30;
    client
        .put(
            Cache::One,
            key.clone(),
            &LeasedCheckpointValue {
                lease_timeout: lease_timeout,
                created_at,
            },
            Some((lease_timeout / 1000).max(minimum_cache_ttl)),
        )
        .await?;

    Ok(LeasedCheckpointValue {
        lease_timeout: lease_timeout,
        created_at: created_at,
    })
}

pub async fn remove_leased_checkpoint(
    client: &Client,
    position_checksum: i64,
) -> Result<Option<LeasedCheckpointValue>, Box<dyn Error + Send + Sync>> {
    let key = generate_leased_checkpoint_key(position_checksum);
    let result: Option<LeasedCheckpointValue> = client.get(Cache::One, key.clone()).await?;
    client.delete(Cache::One, key).await?;
    Ok(result)
}

pub async fn get_leased_checkpoint(
    client: &Client,
    position_checksum: i64,
) -> Result<Option<LeasedCheckpointValue>, Box<dyn Error + Send + Sync>> {
    let key = generate_leased_checkpoint_key(position_checksum);
    let leased_checkpoint: Option<LeasedCheckpointValue> = client.get(Cache::One, key).await?;
    Ok(leased_checkpoint)
}
