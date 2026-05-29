use crate::helper::{new_id, ServerFixture};

/// `generate_idempotency_key` must return a valid UUID string that can be
/// passed to an external side-effecting call to make it idempotent.
#[tokio::test]
async fn returns_valid_uuid() {
    let mut fx = ServerFixture::start().await;
    let id = new_id();

    let token = fx.workflow_start(&id).await;
    let key = fx.generate_idempotency_key(&id, token, 0).await;

    assert!(!key.is_empty(), "idempotency key must be non-empty");
    uuid::Uuid::parse_str(&key).expect("idempotency key must be a valid UUID");
}
