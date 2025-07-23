use std::error::Error;

use hiqlite::Client;
use tokio_cron_scheduler::{Job, JobScheduler};

use crate::services::workflow_service::handle_workflow_cleanup;

pub async fn clean_up_expired_workflows(client: &Client) -> Result<(), Box<dyn Error>> {
    let scheduler = JobScheduler::new().await?;
    let cloned_client = client.clone();

    scheduler
        .add(Job::new_async("1/10 * * * * *", move |_uuid, _l| {
            let job_client = cloned_client.clone();
            println!("Deleting expired workflows");
            Box::pin(async move {
                if let Err(e) = handle_workflow_cleanup(&job_client).await {
                    eprintln!("Error during scheduled delete: {e}");
                } else {
                    println!("Deleted expired workflows");
                }
            })
        })?)
        .await?;

    scheduler.start().await?;
    Ok(())
}
