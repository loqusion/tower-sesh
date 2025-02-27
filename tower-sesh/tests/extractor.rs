use std::sync::{
    atomic::{AtomicBool, Ordering},
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

mod common;
use common::ErrStore;

#[tokio::test]
#[should_panic = "missing request extension"]
async fn session_extractor_without_layer() {
    let app = Router::new()
        .route("/", routing::get(|| async {}))
        .layer(axum::middleware::from_extractor::<Session<()>>());
    let req = Request::builder().uri("/").body(Body::empty()).unwrap();
    let _res = app.oneshot(req).await.unwrap();
}

#[tokio::test]
async fn ignores_deserialization_error() {
    let did_handler = Arc::new(AtomicBool::new(false));

    let did_handler_clone = Arc::clone(&did_handler);
    let handler = |session: Session<()>| async move {
        did_handler_clone.store(true, Ordering::SeqCst);
        assert!(session.get().is_none());
    };

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
    assert!(did_handler.load(Ordering::SeqCst));
}

fn serde_err_store<T>() -> Arc<ErrStore<T>> {
    Arc::new(ErrStore::new(|| {
        store::Error::serde("`ErrStore` always returns an error")
    }))
}
