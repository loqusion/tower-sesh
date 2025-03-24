use std::sync::{
    atomic::{AtomicUsize, Ordering::SeqCst},
    Arc,
};

use axum::{body::Body, response::IntoResponse, routing, Router};
use http::{header, Request, StatusCode};
use tower::ServiceExt;
use tower_sesh::{store::MemoryStore, Session, SessionLayer};
use tower_sesh_core::{
    store::{self},
    SessionKey,
};

mod support;
use support::{ErrStore, InvalidSessionKeyCookie};

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
    static HANDLER_RUN_COUNT: AtomicUsize = AtomicUsize::new(0);

    async fn handler(session: Session<()>) {
        assert!(session.get().is_none());
        HANDLER_RUN_COUNT.fetch_add(1, SeqCst);
    }

    let store = Arc::new(ErrStore::<()>::new(|| {
        store::Error::serde("`ErrStore` always returns an error")
    }));
    let app = Router::new()
        .route("/", routing::get(handler))
        .layer(SessionLayer::plain(store).cookie_name("id"));

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

#[test]
fn empty_session_if_session_key_parsing_fails() {
    #[tokio::main(flavor = "current_thread")]
    async fn check(
        invalid_session_key_cookie: InvalidSessionKeyCookie,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let invalid_session_key_cookie = invalid_session_key_cookie.0;

        async fn handler(session: Session<()>) -> impl IntoResponse {
            if session.get().is_some() {
                StatusCode::INTERNAL_SERVER_ERROR
            } else {
                StatusCode::OK
            }
        }

        let app = Router::new()
            .route("/", routing::get(handler))
            .layer(SessionLayer::plain(Arc::new(MemoryStore::<()>::new())).cookie_name("id"));

        let req = Request::builder()
            .uri("/")
            .header(header::COOKIE, format!("id={}", invalid_session_key_cookie))
            .body(Body::empty())?;
        let res = app.oneshot(req).await?;

        Ok(res.status().is_success())
    }

    quickcheck::quickcheck(check as fn(_) -> _);

    let result = quickcheck::Testable::result(
        &check(InvalidSessionKeyCookie("".to_string())),
        &mut quickcheck::Gen::new(0),
    );
    assert!(!result.is_failure(), "{result:?}");
}
