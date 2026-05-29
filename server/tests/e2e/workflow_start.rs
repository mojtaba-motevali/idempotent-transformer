use crate::helper::{new_id, ServerFixture};

/// A fresh `workflow_start` must return a fencing token ≥ 1.
#[tokio::test]
async fn returns_fencing_token() {
    let mut fx = ServerFixture::start().await;
    let token = fx.workflow_start(&new_id()).await;
    assert!(token >= 1, "fencing_token must be ≥ 1, got {token}");
}

/// Calling `workflow_start` twice on the same ID must strictly increment the
/// fencing token — this is the guard against stale concurrent workers.
#[tokio::test]
async fn twice_increments_token() {
    let mut fx = ServerFixture::start().await;
    let id = new_id();

    let t1 = fx.workflow_start(&id).await;
    let t2 = fx.workflow_start(&id).await;

    assert!(t2 > t1, "second token ({t2}) must exceed first ({t1})");
}
