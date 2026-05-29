use crate::helper::{new_id, ServerFixture};

/// After `complete_workflow`, `workflow_status` must report status=Completed
/// and non-null `completed_at` / `expire_at` timestamps.
#[tokio::test]
async fn reflects_completed_state() {
    let mut fx = ServerFixture::start().await;
    let id = new_id();

    let token = fx.workflow_start(&id).await;
    fx.complete_workflow(&id, token, 60_000).await;

    let status = fx.workflow_status(&id).await.unwrap();
    assert_eq!(status.status, 1, "status should be Completed (1)");
    assert!(status.completed_at.is_some(), "completed_at must be set after completion");
    assert!(status.expire_at.is_some(), "expire_at must be set after completion");
}

/// Querying the status of an unknown workflow must return a NOT_FOUND gRPC error.
#[tokio::test]
async fn unknown_workflow_returns_not_found() {
    let mut fx = ServerFixture::start().await;

    let err = fx.workflow_status(&new_id()).await.unwrap_err();
    assert_eq!(
        err.code(),
        tonic::Code::NotFound,
        "unknown workflow must return NOT_FOUND, got {:?}",
        err.code()
    );
}
