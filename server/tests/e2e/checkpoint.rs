use idempotency_server::rpc_server::server::workflow_service::lease_checkpoint_response;

use crate::helper::{new_id, ServerFixture};

/// Full idempotency round-trip: lease → commit checkpoint → re-lease returns
/// the cached value rather than acquiring a fresh lease.
#[tokio::test]
async fn stores_and_lease_retrieves_value() {
    let mut fx = ServerFixture::start().await;
    let id = new_id();
    let ikey = new_id();
    let payload = b"result payload".to_vec();

    let token = fx.workflow_start(&id).await;

    // First lease: no checkpoint exists yet → Leased (no response body).
    let l1 = fx.lease_checkpoint(&id, token, 5_000, 0, &ikey).await;
    assert!(l1.response.is_none(), "fresh lease should carry no value");

    // Commit the checkpoint value.
    let abort = fx.checkpoint(&id, payload.clone(), token, 0, &ikey).await;
    assert!(!abort, "valid token checkpoint must not abort");

    // Second lease on the same position → must return the committed value.
    let l2 = fx.lease_checkpoint(&id, token, 5_000, 0, &ikey).await;
    match l2.response {
        Some(lease_checkpoint_response::Response::Value(v)) => {
            assert_eq!(v, payload, "re-lease must return the committed payload");
        }
        other => panic!("expected CheckpointValue, got {other:?}"),
    }
}

/// A checkpoint that arrives with a superseded fencing token must be rejected
/// (`abort = true`) so the stale worker knows to stop.
#[tokio::test]
async fn stale_token_aborts() {
    let mut fx = ServerFixture::start().await;
    let id = new_id();

    let stale_token = fx.workflow_start(&id).await;
    fx.workflow_start(&id).await; // increments the token, making stale_token outdated

    let abort = fx.checkpoint(&id, b"stale".to_vec(), stale_token, 0, &new_id()).await;
    assert!(abort, "stale fencing token must cause abort=true");
}
