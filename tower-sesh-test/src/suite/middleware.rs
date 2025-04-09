use std::sync::Arc;
use tower::util::ServiceExt;

use axum::{body::Body, http::Request, response::IntoResponse, routing, Router};
use rand::SeedableRng;
use tower_sesh::SessionLayer;
use tower_sesh_core::{store::SessionStoreRng, SessionStore};

use crate::support::{SessionData, TestRng};

async fn test_thing(mut store: impl SessionStore<SessionData> + SessionStoreRng<TestRng>) {
    let rng = TestRng::seed_from_u64(2123027923);
    store.rng(rng);
    let store = Arc::new(store);

    async fn handler() -> impl IntoResponse {
        ""
    }

    let app = Router::new()
        .route("/", routing::get(handler))
        .layer(SessionLayer::plain(store.clone()).cookie_name("id"));

    let req = Request::builder().uri("/").body(Body::empty()).unwrap();
    let res = app.oneshot(req).await.unwrap();
}
