use std::sync::Arc;
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::error;
use crate::services::workflow_service::WorkflowService;

pub async fn clean_up_expired_workflows(
    workflow_svc: Arc<WorkflowService>,
) -> anyhow::Result<JobScheduler> {
    let scheduler = JobScheduler::new().await?;

    scheduler.add(Job::new_async("1/10 * * * * *", move |_, _| {
        let svc = workflow_svc.clone();
        Box::pin(async move {
            if let Err(e) = svc.cleanup_expired().await {
                error!(err = %e, "cleanup job failed");
            }
        })
    })?).await?;

    scheduler.start().await?;
    Ok(scheduler)
}
