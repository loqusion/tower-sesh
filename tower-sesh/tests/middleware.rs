use std::sync::{
    atomic::{AtomicUsize, Ordering::SeqCst},
    Arc,
};

use axum::{body::Body, response::IntoResponse, routing, Router};
use cookie::{Cookie, CookieJar};
use http::{header, HeaderValue, Method, Request, Response, StatusCode};
use rand::SeedableRng;
use tower::{ServiceBuilder, ServiceExt};
use tower_sesh::{store::MemoryStore, Session, SessionLayer};
use tower_sesh_core::{
    store::{SessionStoreImpl, SessionStoreRng},
    SessionKey,
};
use tower_sesh_test::{support::SessionData, TestRng};

mod support;
use support::{ttl, ArbitraryKey, ArbitrarySessionKey};

fn jar_from_response<B>(
    res: &Response<B>,
) -> Result<CookieJar, Box<dyn std::error::Error + 'static>> {
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

#[cfg_attr(miri, ignore)]
#[test]
fn private_or_signed_cookie_no_load() {
    #[tokio::main(flavor = "current_thread")]
    async fn check(
        ArbitrarySessionKey(session_key): ArbitrarySessionKey,
        ArbitraryKey(key): ArbitraryKey,
        is_private: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        async fn handler(session: Session<()>) -> impl IntoResponse {
            if session.get().is_none() {
                StatusCode::OK
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }

        let store = MemoryStore::<()>::new();
        store.update(&session_key, &(), ttl()).await?;

        let session_layer = SessionLayer::new(store.into(), key).cookie_name("id");
        let app = Router::new().route("/", routing::get(handler));
        let app = if is_private {
            app.layer(session_layer.private())
        } else {
            app.layer(session_layer.signed())
        };

        macro_rules! try_request {
            ($request:expr) => {{
                let req: Request<_> = $request;
                let res = app.clone().oneshot(req).await?;
                if !res.status().is_success() {
                    return Err("assertion in handler failed".into());
                }
            }};
        }

        try_request!(Request::builder().uri("/").body(Body::empty())?);
        try_request!(Request::builder()
            .uri("/")
            .header(header::COOKIE, "id=!?!?!?!?!?!?")
            .body(Body::empty())?);
        try_request!(Request::builder()
            .uri("/")
            .header(header::COOKIE, format!("id={}", session_key.encode()))
            .body(Body::empty())?);

        Ok(())
    }

    quickcheck::quickcheck(check as fn(_, _, _) -> _);
}

#[cfg_attr(miri, ignore)]
#[test]
fn private_or_signed_cookie_create_and_load() {
    #[tokio::main(flavor = "current_thread")]
    async fn check(
        ArbitraryKey(key): ArbitraryKey,
        seed: u64,
        is_private: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        async fn session_create(session: Session<SessionData>) -> impl IntoResponse {
            assert!(session.get().is_none());
            session.insert(SessionData::sample());
        }

        async fn session_load(session: Session<SessionData>) -> impl IntoResponse {
            if session.get().is_some() {
                StatusCode::OK
            } else {
                StatusCode::UNAUTHORIZED
            }
        }

        let rng = TestRng::seed_from_u64(seed);
        let mut store = MemoryStore::<SessionData>::new();
        store.rng(rng);

        let session_layer = SessionLayer::new(store.into(), key).cookie_name("id");
        let app = Router::new()
            .route("/create", routing::post(session_create))
            .route("/load", routing::get(session_load));
        let app = if is_private {
            app.layer(session_layer.private())
        } else {
            app.layer(session_layer.signed())
        };

        let req = Request::builder()
            .uri("/create")
            .method(Method::POST)
            .body(Body::empty())?;
        let res = app.clone().oneshot(req).await?;
        let jar = jar_from_response(&res)?;

        let req = Request::builder()
            .uri("/load")
            .method(Method::GET)
            .header(
                header::COOKIE,
                format!("id={}", jar.get("id").unwrap().value()),
            )
            .body(Body::empty())?;
        let res = app.clone().oneshot(req).await?;
        if !res.status().is_success() {
            return Err("assertion in handler failed".into());
        }

        Ok(())
    }

    quickcheck::quickcheck(check as fn(_, _, _) -> _);
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

#[test]
#[should_panic = "invalid `cookie_name` value"]
fn invalid_cookie_name() {
    SessionLayer::plain(Arc::new(MemoryStore::<()>::new())).cookie_name("\n");
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

#[tokio::test]
async fn preserves_existing_set_cookie() {
    async fn handler(session: Session<()>) -> impl IntoResponse {
        session.insert(());

        [(header::SET_COOKIE, "hello=world")]
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
