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

/*
- session cookie PRESENT, VALID
  - inner service SUCCESS
    - session UNCHANGED
      -> session should be unmodified
      -> session expiry???
      -> should be no Set-Cookie header in response
    - session RENEWED
      -> session should be unmodified
      -> session expiry should be modified
      -> should be Set-Cookie header in response
    - session CHANGED
      -> session should be updated to given value
      -> session expiry???
      -> should be Set-Cookie header in response
    - session PURGED
      -> session should be absent
      -> should be Set-Cookie header in response to remove cookie
  - inner service ERROR
    -> should leave store unmodified
- session cookie PRESENT, INVALID
  -> should behave identically to ABSENT
- session cookie ABSENT
  - inner service SUCCESS
    - session UNCHANGED
      -> no session
      -> should be no Set-Cookie header in response
    - session RENEWED
      -> no session
      -> should be no Set-Cookie header in response
    - session CHANGED
      -> session should be created
      -> should be Set-Cookie header in response
    - session PURGED
      -> no session
      -> should be no Set-Cookie header in response
  - inner service ERROR
    -> should leave store unmodified
 */
