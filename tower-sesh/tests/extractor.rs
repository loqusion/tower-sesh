use axum::{body::Body, routing, Router};
use http::Request;
use tower::ServiceExt;
use tower_sesh::Session;

#[tokio::test]
#[should_panic = "missing request extension"]
async fn session_extractor_without_layer() {
    let app = Router::new()
        .route("/", routing::get(|| async {}))
        .layer(axum::middleware::from_extractor::<Session<()>>());
    let req = Request::builder().uri("/").body(Body::empty()).unwrap();
    let _res = app.oneshot(req).await.unwrap();
}
