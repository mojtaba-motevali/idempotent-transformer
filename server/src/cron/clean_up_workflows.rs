use std::error::Error;

use hiqlite::Client;
use tokio_cron_scheduler::{Job, JobScheduler};

use crate::services::workflows::delete_expired_workflows;

pub async fn clean_up_expired_workflows(client: &Client) -> Result<(), Box<dyn Error>> {
    println!("Starting cleanup workflow scheduler");
    let scheduler = JobScheduler::new().await?;
    let cloned_client = client.clone();
    println!("Starting cleanup workflow scheduler 2");

    scheduler
        .add(Job::new_async("1/10 * * * * *", move |_uuid, _l| {
            let job_client = cloned_client.clone();
            println!("Deleting expired workflows");
            Box::pin(async move {
                if let Err(e) = delete_expired_workflows(&job_client).await {
                    eprintln!("Error during scheduled delete: {:?}", e);
                } else {
                    println!("Deleted expired workflows");
                }
            })
        })?)
        .await?;

    scheduler.start().await?;
    Ok(())
}
