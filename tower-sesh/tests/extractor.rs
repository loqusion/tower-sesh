use std::sync::{
    atomic::{AtomicUsize, Ordering::SeqCst},
    Arc,
};

use axum::{body::Body, routing, Router};
use http::{header, Request, StatusCode};
use tower::ServiceExt;
use tower_sesh::{Session, SessionLayer};
use tower_sesh_core::{
    store::{self},
    SessionKey,
};

mod support;
use support::ErrStore;

#[tokio::test]
#[should_panic = "missing request extension"]
async fn session_extractor_without_layer() {
    let app = Router::new()
        .route("/", routing::get(|| async {}))
        .layer(axum::middleware::from_extractor::<Session<()>>());
    let req = Request::builder().uri("/").body(Body::empty()).unwrap();
    let _res = app.oneshot(req).await.unwrap();
}

fn serde_err_store<T>() -> Arc<ErrStore<T>> {
    Arc::new(ErrStore::new(|| {
        store::Error::serde("`ErrStore` always returns an error")
    }))
}

#[tokio::test]
async fn ignores_deserialization_error() {
    static HANDLER_RUN_COUNT: AtomicUsize = AtomicUsize::new(0);

    async fn handler(session: Session<()>) {
        assert!(session.get().is_none());
        HANDLER_RUN_COUNT.fetch_add(1, SeqCst);
    }

    let app = Router::new()
        .route("/", routing::get(handler))
        .layer(SessionLayer::plain(serde_err_store::<()>()).cookie_name("id"));

    let req = Request::builder()
        .uri("/")
        .header(
            header::COOKIE,
            format!("id={}", SessionKey::try_from(1).unwrap().encode()),
        )
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(HANDLER_RUN_COUNT.load(SeqCst), 1);
}
