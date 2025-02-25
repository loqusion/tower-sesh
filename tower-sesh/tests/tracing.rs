use std::{fmt, marker::PhantomData, sync::Arc, time::Duration};

use axum::{async_trait, body::Body, response::IntoResponse, routing, Router};
use http::Request;
use tokio::sync::mpsc;
use tower::ServiceExt;
use tower_sesh::{session::SessionRejection, store::MemoryStore, Session, SessionLayer};
use tower_sesh_core::{
    store::{self, SessionStoreImpl},
    Record, SessionKey, SessionStore, Ttl,
};
use tracing::Level;
use tracing_mock::{expect, subscriber};

#[tokio::test]
async fn no_parent_span_in_handler() {
    let handler_span = expect::span().named("test_handler");
    let handler_new_span = handler_span
        .clone()
        .with_ancestry(expect::is_contextual_root());

    let (subscriber, handle) = subscriber::mock()
        .with_filter(|meta| meta.file() == Some(file!()))
        .new_span(handler_new_span)
        .enter(&handler_span)
        .event(expect::event())
        .exit(&handler_span)
        .run_with_handle();

    #[tracing::instrument(name = "test_handler")]
    async fn handler() -> impl IntoResponse {
        tracing::error!("an error message");
    }

    let app = Router::new()
        .route("/", routing::get(handler))
        .layer(SessionLayer::plain(Arc::new(MemoryStore::<()>::new())));

    {
        let _guard = tracing::subscriber::set_default(subscriber);
        let req = Request::builder().uri("/").body(Body::empty()).unwrap();
        app.oneshot(req).await.unwrap();
    }

    handle.assert_finished();
}

#[tokio::test]
async fn session_sync_error() {
    let (subscriber, handle) =
        subscriber::mock()
            .with_filter(|meta| meta.target() == "tower_sesh::middleware")
            .event(expect::event().with_fields(
                expect::field("err").with_value(&debug_value(ErrStore::<()>::MESSAGE)),
            ))
            .run_with_handle();

    async fn handler(session: Session<()>) -> impl IntoResponse {
        session.insert(());
    }

    let app = Router::new()
        .route("/", routing::get(handler))
        .layer(SessionLayer::plain(Arc::new(ErrStore::<()>::new())));

    {
        let _guard = tracing::subscriber::set_default(subscriber);
        let req = Request::builder().uri("/").body(Body::empty()).unwrap();
        app.oneshot(req).await.unwrap();
    }

    handle.assert_finished();
}

#[tokio::test]
async fn extractor_rejection() {
    let (subscriber, handle) = subscriber::mock()
        .with_filter(|meta| meta.target() == "tower_sesh::rejection")
        .event(
            expect::event().at_level(Level::TRACE).with_fields(
                expect::field("rejection_type")
                    .with_value(&std::any::type_name::<SessionRejection>())
                    .and(expect::field("message").with_value(&debug_value("rejecting request"))),
            ),
        )
        .run_with_handle();

    async fn handler(_session: Session<()>) {
        unimplemented!()
    }

    let app = Router::new()
        .route("/", routing::get(handler))
        .layer(SessionLayer::plain(Arc::new(ErrStore::<()>::new())).cookie_name("id"));

    {
        let _guard = tracing::subscriber::set_default(subscriber);
        let req = Request::builder()
            .uri("/")
            .header(
                "Cookie",
                format!("id={}", SessionKey::try_from(1).unwrap().encode()),
            )
            .body(Body::empty())
            .unwrap();
        app.oneshot(req).await.unwrap();
    }

    handle.assert_finished();
}

#[tokio::test]
async fn session_load_error() {
    let (subscriber, handle) =
        subscriber::mock()
            .with_filter(|meta| meta.target().starts_with("tower_sesh::session"))
            .event(expect::event().at_level(Level::ERROR).with_fields(
                expect::field("err").with_value(&debug_value(ErrStore::<()>::MESSAGE)),
            ))
            .run_with_handle();

    async fn handler(_session: Session<()>) {
        unimplemented!()
    }

    let app = Router::new()
        .route("/", routing::get(handler))
        .layer(SessionLayer::plain(Arc::new(ErrStore::<()>::new())).cookie_name("id"));

    {
        let _guard = tracing::subscriber::set_default(subscriber);
        let req = Request::builder()
            .uri("/")
            .header(
                "Cookie",
                format!("id={}", SessionKey::try_from(1).unwrap().encode()),
            )
            .body(Body::empty())
            .unwrap();
        app.oneshot(req).await.unwrap();
    }

    handle.assert_finished();
}

#[tokio::test]
async fn use_session_after_taken() {
    use axum::extract::State;

    let (subscriber, handle) = subscriber::mock()
        .with_filter(|meta| meta.target() == "tower_sesh::session")
        .event(expect::event().at_level(Level::ERROR).with_fields(
            expect::field("message").with_value(&debug_value(
                "called `Session` method after it was synchronized to store",
            )),
        ))
        .run_with_handle();

    let (tx, mut rx) = mpsc::channel(1);

    #[derive(Clone)]
    struct AppState {
        pub tx: mpsc::Sender<tokio::task::JoinHandle<()>>,
    }

    async fn handler(State(AppState { tx }): State<AppState>, session: Session<()>) {
        let join_handle = tokio::spawn(async move {
            // Sleep so that sync has a chance to run
            tokio::time::sleep(Duration::from_millis(1)).await;
            let _ = session.get();
        });
        let _ = tx.send(join_handle).await;
    }

    let app = Router::new()
        .route("/", routing::get(handler))
        .layer(SessionLayer::plain(Arc::new(MemoryStore::<()>::new())))
        .with_state(AppState { tx });

    {
        let _guard = tracing::subscriber::set_default(subscriber);
        let req = Request::builder().uri("/").body(Body::empty()).unwrap();
        app.oneshot(req).await.unwrap();

        let join_handle = rx.try_recv().unwrap();
        // If `tokio-mock` assertions fail, this will panic
        join_handle.await.unwrap();
    }

    handle.assert_finished();
}

fn debug_value(message: impl Into<String>) -> tracing::field::DebugValue<Box<dyn fmt::Debug>> {
    struct Message {
        message: String,
    }

    impl fmt::Debug for Message {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str(&self.message)
        }
    }

    tracing::field::debug(Box::new(Message {
        message: message.into(),
    }))
}

struct ErrStore<T = ()>(PhantomData<fn() -> T>);

impl<T> ErrStore<T> {
    const MESSAGE: &str = "`ErrStore` always returns an error";

    fn new() -> Self {
        ErrStore(PhantomData)
    }
}

impl<T> SessionStore<T> for ErrStore<T> where T: Send + Sync + 'static {}
#[async_trait]
impl<T> SessionStoreImpl<T> for ErrStore<T>
where
    T: Send + Sync + 'static,
{
    async fn create(&self, _data: &T, _ttl: Ttl) -> Result<SessionKey, store::Error> {
        Err(store::Error::message(Self::MESSAGE))
    }

    async fn load(&self, _session_key: &SessionKey) -> Result<Option<Record<T>>, store::Error> {
        Err(store::Error::message(Self::MESSAGE))
    }

    async fn update(
        &self,
        _session_key: &SessionKey,
        _data: &T,
        _ttl: Ttl,
    ) -> Result<(), store::Error> {
        Err(store::Error::message(Self::MESSAGE))
    }

    async fn update_ttl(&self, _session_key: &SessionKey, _ttl: Ttl) -> Result<(), store::Error> {
        Err(store::Error::message(Self::MESSAGE))
    }

    async fn delete(&self, _session_key: &SessionKey) -> Result<(), store::Error> {
        Err(store::Error::message(Self::MESSAGE))
    }
}

#[tokio::test]
async fn sandbox() {
    subscriber_init();

    #[tracing::instrument]
    async fn handler() -> impl IntoResponse {
        tracing::error!("error!");
    }

    let app = Router::new()
        .route("/", routing::get(handler))
        .layer(SessionLayer::plain(Arc::new(MemoryStore::<()>::new())));

    let req = Request::builder().uri("/").body(Body::empty()).unwrap();
    app.oneshot(req).await.unwrap();

    // uncomment to view tracing messages
    // panic!();
}

fn subscriber_init() {
    use std::sync::Once;
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

    static TRACING: Once = Once::new();
    TRACING.call_once(|| {
        let log_layer = tracing_subscriber::fmt::layer().pretty().with_test_writer();

        let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "TRACE".into());

        tracing_subscriber::registry()
            .with(log_layer)
            .with(env_filter)
            .init();
    });
}
