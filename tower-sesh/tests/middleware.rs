use std::{collections::HashSet, sync::Arc};

use axum::{body::Body, response::IntoResponse, routing, Router};
use cookie::Cookie;
use http::{header, HeaderValue, Request};
use tower::{ServiceBuilder, ServiceExt};
use tower_sesh::{store::MemoryStore, Session, SessionLayer};

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

#[tokio::test]
async fn preserves_existing_set_cookie() {
    async fn handler(session: Session<()>) -> impl IntoResponse {
        session.insert(());

        axum::response::Response::builder()
            .header(
                header::SET_COOKIE,
                Cookie::new("hello", "world")
                    .encoded()
                    .to_string()
                    .parse::<HeaderValue>()
                    .unwrap(),
            )
            .body(Body::empty())
            .unwrap()
    }

    let app = Router::new()
        .route("/", routing::get(handler))
        .layer(SessionLayer::plain(Arc::new(MemoryStore::<()>::new())).cookie_name("id"));

    let req = Request::builder().uri("/").body(Body::empty()).unwrap();
    let res = app.oneshot(req).await.unwrap();

    let mut names = HashSet::new();
    for value in res.headers().get_all(header::SET_COOKIE) {
        let cookie = Cookie::parse_encoded(value.to_str().unwrap()).unwrap();
        names.insert(cookie.name().to_owned());
    }

    assert!(names.contains("hello"));
}
