use std::{
    sync::{
        atomic::{AtomicUsize, Ordering::SeqCst},
        Arc,
    },
    time::Duration,
};

use axum::{body::Body, response::IntoResponse, routing, Router};
use cookie::{Cookie, CookieJar};
use http::{header, HeaderValue, Request, Response};
use tower::{ServiceBuilder, ServiceExt};
use tower_sesh::{store::MemoryStore, Session, SessionLayer};
use tower_sesh_core::{store::SessionStoreImpl, SessionKey, Ttl};

fn jar_from_response<B>(
    res: &Response<B>,
) -> Result<CookieJar, Box<dyn std::error::Error + Send + Sync + 'static>> {
    res.headers()
        .get_all(header::SET_COOKIE)
        .into_iter()
        .try_fold(CookieJar::new(), |mut jar, header_value| {
            let s = header_value.to_str()?;
            let cookie = Cookie::parse_encoded(s)?;
            jar.add(cookie.into_owned());
            Ok(jar)
        })
}

fn header_value_from_cookie_key_values<'a, S>(
    key_value_iter: impl Iterator<Item = &'a (S, S)>,
) -> Result<HeaderValue, header::InvalidHeaderValue>
where
    &'a S: Into<String> + 'a,
{
    let value = key_value_iter
        .map::<(String, String), _>(|(k, v)| (k.into(), v.into()))
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join("; ");
    HeaderValue::try_from(value)
}

#[tokio::test]
async fn option_cookie_name() {
    async fn handler(session: Session<()>) {
        session.insert(());
    }

    let session_layer = SessionLayer::plain(MemoryStore::<()>::new().into()).cookie_name("hello");
    let app = Router::new()
        .route("/", routing::get(handler))
        .layer(session_layer);
    let req = Request::builder().uri("/").body(Body::empty()).unwrap();
    let res = app.oneshot(req).await.unwrap();

    let jar = jar_from_response(&res).unwrap();
    assert!(jar.get("hello").is_some());
    assert!(jar.iter().collect::<Vec<_>>().len() == 1);
}

#[tokio::test]
async fn option_domain() {
    async fn handler(session: Session<()>) {
        session.insert(());
    }

    let session_layer = SessionLayer::plain(MemoryStore::<()>::new().into())
        .cookie_name("id")
        .domain("doc.rust-lang.org");
    let app = Router::new()
        .route("/", routing::get(handler))
        .layer(session_layer);
    let req = Request::builder().uri("/").body(Body::empty()).unwrap();
    let res = app.oneshot(req).await.unwrap();

    let jar = jar_from_response(&res).unwrap();
    assert!(jar.iter().collect::<Vec<_>>().len() == 1);
    let cookie = jar.get("id").unwrap();
    assert_eq!(cookie.domain(), Some("doc.rust-lang.org"));
}

#[tokio::test]
async fn option_http_only() {
    async fn handler(session: Session<()>) {
        session.insert(());
    }

    let session_layer = SessionLayer::plain(MemoryStore::<()>::new().into())
        .cookie_name("id")
        .http_only(true);
    let app = Router::new()
        .route("/", routing::get(handler))
        .layer(session_layer);
    let req = Request::builder().uri("/").body(Body::empty()).unwrap();
    let res = app.oneshot(req).await.unwrap();

    let jar = jar_from_response(&res).unwrap();
    assert!(jar.iter().collect::<Vec<_>>().len() == 1);
    let cookie = jar.get("id").unwrap();
    assert_eq!(cookie.http_only(), Some(true));

    let session_layer = SessionLayer::plain(MemoryStore::<()>::new().into())
        .cookie_name("id")
        .http_only(false);
    let app = Router::new()
        .route("/", routing::get(handler))
        .layer(session_layer);
    let req = Request::builder().uri("/").body(Body::empty()).unwrap();
    let res = app.oneshot(req).await.unwrap();

    let jar = jar_from_response(&res).unwrap();
    assert!(jar.iter().collect::<Vec<_>>().len() == 1);
    let cookie = jar.get("id").unwrap();
    assert!(!cookie.http_only().unwrap_or(false));
}

#[tokio::test]
async fn option_path() {
    async fn handler(session: Session<()>) {
        session.insert(());
    }

    let session_layer = SessionLayer::plain(MemoryStore::<()>::new().into())
        .cookie_name("id")
        .path("/std");
    let app = Router::new()
        .route("/", routing::get(handler))
        .layer(session_layer);
    let req = Request::builder().uri("/").body(Body::empty()).unwrap();
    let res = app.oneshot(req).await.unwrap();

    let jar = jar_from_response(&res).unwrap();
    assert!(jar.iter().collect::<Vec<_>>().len() == 1);
    let cookie = jar.get("id").unwrap();
    assert_eq!(cookie.path(), Some("/std"));
}

#[tokio::test]
async fn option_same_site() {
    use tower_sesh::middleware::SameSite;

    async fn handler(session: Session<()>) {
        session.insert(());
    }

    for (same_site, expected) in [
        (SameSite::Strict, cookie::SameSite::Strict),
        (SameSite::Lax, cookie::SameSite::Lax),
        (SameSite::None, cookie::SameSite::None),
    ] {
        let session_layer = SessionLayer::plain(MemoryStore::<()>::new().into())
            .cookie_name("id")
            .same_site(same_site);
        let app = Router::new()
            .route("/", routing::get(handler))
            .layer(session_layer);
        let req = Request::builder().uri("/").body(Body::empty()).unwrap();
        let res = app.oneshot(req).await.unwrap();

        let jar = jar_from_response(&res).unwrap();
        assert!(jar.iter().collect::<Vec<_>>().len() == 1);
        let cookie = jar.get("id").unwrap();
        assert_eq!(cookie.same_site(), Some(expected));
    }
}

#[tokio::test]
async fn option_secure() {
    async fn handler(session: Session<()>) {
        session.insert(());
    }

    let session_layer = SessionLayer::plain(MemoryStore::<()>::new().into())
        .cookie_name("id")
        .secure(true);
    let app = Router::new()
        .route("/", routing::get(handler))
        .layer(session_layer);
    let req = Request::builder().uri("/").body(Body::empty()).unwrap();
    let res = app.oneshot(req).await.unwrap();

    let jar = jar_from_response(&res).unwrap();
    assert!(jar.iter().collect::<Vec<_>>().len() == 1);
    let cookie = jar.get("id").unwrap();
    assert_eq!(cookie.secure(), Some(true));

    let session_layer = SessionLayer::plain(MemoryStore::<()>::new().into())
        .cookie_name("id")
        .secure(false);
    let app = Router::new()
        .route("/", routing::get(handler))
        .layer(session_layer);
    let req = Request::builder().uri("/").body(Body::empty()).unwrap();
    let res = app.oneshot(req).await.unwrap();

    let jar = jar_from_response(&res).unwrap();
    assert!(jar.iter().collect::<Vec<_>>().len() == 1);
    let cookie = jar.get("id").unwrap();
    assert!(!cookie.secure().unwrap_or(false));
}

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

#[test]
#[should_panic = "not implemented"]
fn plain_to_private() {
    SessionLayer::plain(Arc::new(MemoryStore::<()>::new())).private();
}

#[test]
#[should_panic = "not implemented"]
fn plain_to_signed() {
    SessionLayer::plain(Arc::new(MemoryStore::<()>::new())).signed();
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

    let jar = jar_from_response(&res).unwrap();
    assert_eq!(jar.get("hello").unwrap().value(), "world");
    assert!(jar.get("id").is_some());
}

#[tokio::test]
async fn extracts_cookie_from_many_in_single_header() {
    static HANDLER_RUN_COUNT: AtomicUsize = AtomicUsize::new(0);

    async fn handler(session: Session<()>) -> impl IntoResponse {
        assert!(session.get().is_some());
        HANDLER_RUN_COUNT.fetch_add(1, SeqCst);
    }

    let store = Arc::new(MemoryStore::<()>::new());
    let key = SessionKey::try_from(1).unwrap();
    store.update(&key, &(), ttl()).await.unwrap();

    let app = Router::new()
        .route("/", routing::get(handler))
        .layer(SessionLayer::plain(store).cookie_name("id"));

    let sample_cookies = [
        ("hello".to_owned(), "world".to_owned()),
        ("foo".to_owned(), "bar".to_owned()),
    ];
    let session_cookie = ("id".to_owned(), key.encode());

    for cookie_header in [
        [&session_cookie, &sample_cookies[0], &sample_cookies[1]],
        [&sample_cookies[0], &session_cookie, &sample_cookies[1]],
        [&sample_cookies[0], &sample_cookies[1], &session_cookie],
    ]
    .into_iter()
    .map(IntoIterator::into_iter)
    .map(header_value_from_cookie_key_values)
    .collect::<Result<Vec<_>, _>>()
    .unwrap()
    {
        let req = Request::builder()
            .uri("/")
            .header(header::COOKIE, cookie_header)
            .body(Body::empty())
            .unwrap();
        app.clone().oneshot(req).await.unwrap();
    }

    assert_eq!(HANDLER_RUN_COUNT.load(SeqCst), 3);
}

#[tokio::test]
async fn extracts_cookie_from_many_headers() {
    static HANDLER_RUN_COUNT: AtomicUsize = AtomicUsize::new(0);

    async fn handler(session: Session<()>) -> impl IntoResponse {
        assert!(session.get().is_some());
        HANDLER_RUN_COUNT.fetch_add(1, SeqCst);
    }

    let store = Arc::new(MemoryStore::<()>::new());

    let app = Router::new()
        .route("/", routing::get(handler))
        .layer(SessionLayer::plain(Arc::clone(&store)).cookie_name("id"));

    let key = SessionKey::try_from(1).unwrap();
    store.update(&key, &(), ttl()).await.unwrap();

    let sample_cookies = [
        ("hello".to_owned(), "world".to_owned()),
        ("foo".to_owned(), "bar".to_owned()),
    ];
    let session_cookie = ("id".to_owned(), key.encode());

    for header_values in [
        [&session_cookie, &sample_cookies[0], &sample_cookies[1]],
        [&sample_cookies[0], &session_cookie, &sample_cookies[1]],
        [&sample_cookies[0], &sample_cookies[1], &session_cookie],
    ]
    .into_iter()
    .map(|[(k1, v1), (k2, v2), (k3, v3)]| {
        (
            format!("{k1}={v1}"),
            format!("{k2}={v2}"),
            format!("{k3}={v3}"),
        )
    }) {
        let req = Request::builder()
            .uri("/")
            .header(header::COOKIE, header_values.0)
            .header(header::COOKIE, header_values.1)
            .header(header::COOKIE, header_values.2)
            .body(Body::empty())
            .unwrap();
        app.clone().oneshot(req).await.unwrap();
    }

    assert_eq!(HANDLER_RUN_COUNT.load(SeqCst), 3);
}

fn ttl() -> Ttl {
    let now = Ttl::now_local().unwrap();
    now + Duration::from_secs(10 * 60)
}
