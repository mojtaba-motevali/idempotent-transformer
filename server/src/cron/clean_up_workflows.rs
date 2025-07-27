use std::error::Error;

use hiqlite::Client;
use tokio_cron_scheduler::{Job, JobScheduler};

use crate::services::workflow_service::handle_workflow_cleanup;

pub async fn clean_up_expired_workflows(client: &Client) -> Result<JobScheduler, Box<dyn Error>> {
    let scheduler = JobScheduler::new().await?;
    let cloned_client = client.clone();

    scheduler
        .add(Job::new_async("1/10 * * * * *", move |_uuid, _l| {
            let job_client = cloned_client.clone();
            Box::pin(async move {
                if let Err(e) = handle_workflow_cleanup(&job_client).await {
                    eprintln!("Error during scheduled delete: {e}");
                }
            })
        })?)
        .await?;

    scheduler.start().await?;
    Ok(scheduler)
}
