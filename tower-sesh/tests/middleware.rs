use std::sync::Arc;

use axum::{body::Body, routing, Router};
use http::Request;
use tower::{ServiceBuilder, ServiceExt};
use tower_sesh::{store::MemoryStore, SessionLayer};

#[tokio::test]
#[should_panic = "called more than once!"]
async fn multiple_session_layers() {
    let session_layer = SessionLayer::plain(Arc::new(MemoryStore::<()>::new()));
    let app = Router::new().route("/", routing::get(|| async {})).layer(
        ServiceBuilder::new()
            .layer(session_layer.clone())
            .layer(session_layer),
    );
    let req = Request::builder().uri("/").body(Body::empty()).unwrap();
    let _res = app.oneshot(req).await.unwrap();
}

#[test]
#[should_panic = "invalid `cookie_name` value"]
fn invalid_cookie_name() {
    SessionLayer::plain(Arc::new(MemoryStore::<()>::new())).cookie_name("\n");
}
