use std::{
    borrow::Cow,
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{ready, Context, Poll},
};

use cookie::CookieJar;
use http::{Request, Response};
use pin_project_lite::pin_project;
use tower::{Layer, Service};
use tower_cookies::{Cookie, Cookies};

use crate::{
    config::{CookieConfiguration, CookieContentSecurity},
    cookie::CookieJarExt,
    session::{Session, SessionKey},
    store::SessionStore,
    util::ErrorExt,
};

/// The default cookie name used by [`SessionManagerLayer`] to store a session
/// id.
pub const DEFAULT_COOKIE_NAME: &str = "session_key";

/// A layer that provides [`Session`] as a request extension.
///
/// # Example
///
/// ```rust
/// use std::sync::Arc;
/// use tower::ServiceBuilder;
/// use tower_cookies::CookieManagerLayer;
/// use tower_sesh::{store::MemoryStore, SessionManagerLayer};
///
/// let session_store = MemoryStore::new();
/// let middleware = ServiceBuilder::new()
///     .layer(CookieManagerLayer::new())
///     .layer(SessionManagerLayer::new(Arc::new(session_store)));
/// ```
#[derive(Debug)]
pub struct SessionManagerLayer<Store: SessionStore, C: CookieController = PlaintextCookie> {
    session_store: Arc<Store>,
    cookie_name: Cow<'static, str>,
    cookie_controller: C,
}

/// Trait used to control how cookies are stored and retrieved.
pub trait CookieController: Clone {
    fn get<'c>(&self, cookies: &'c Cookies, name: &str) -> Option<Cookie<'c>>;
    fn add(&self, cookies: &Cookies, cookie: Cookie<'static>);
    fn remove(&self, cookies: &Cookies, cookie: Cookie<'static>);
}

impl<Store: SessionStore> SessionManagerLayer<Store> {
    /// Create a new `SessionManagerLayer`.
    ///
    /// Cookies are stored in plaintext by default.
    pub fn new(session_store: Arc<Store>) -> Self {
        Self {
            session_store,
            cookie_name: Cow::Borrowed(DEFAULT_COOKIE_NAME),
            cookie_controller: PlaintextCookie,
        }
    }
}

impl<Store: SessionStore, C: CookieController> SessionManagerLayer<Store, C> {
    /// Set the name of the cookie used to store a session id.
    pub fn cookie_name(mut self, name: impl Into<Cow<'static, str>>) -> Self {
        self.cookie_name = name.into();
        self
    }
}

impl<Store: SessionStore, C: CookieController> Clone for SessionManagerLayer<Store, C> {
    fn clone(&self) -> Self {
        Self {
            session_store: Arc::clone(&self.session_store),
            cookie_name: self.cookie_name.clone(),
            cookie_controller: self.cookie_controller.clone(),
        }
    }
}

impl<S, Store: SessionStore, C: CookieController> Layer<S> for SessionManagerLayer<Store, C> {
    type Service = SessionManager<S, Store, C>;

    fn layer(&self, inner: S) -> Self::Service {
        SessionManager {
            inner,
            layer: self.clone(),
        }
    }
}

/// Store cookies in plaintext (unauthenticated, unencrypted).
#[derive(Clone, Debug)]
pub struct PlaintextCookie;

impl CookieController for PlaintextCookie {
    fn get<'c>(&self, cookies: &'c Cookies, name: &str) -> Option<Cookie<'c>> {
        cookies.get(name)
    }

    fn add(&self, cookies: &Cookies, cookie: Cookie<'static>) {
        cookies.add(cookie)
    }

    fn remove(&self, cookies: &Cookies, cookie: Cookie<'static>) {
        cookies.remove(cookie)
    }
}

/// A middleware that provides [`Session`] as a request extension.
///
/// [`Session`]: crate::session::Session
#[derive(Debug)]
pub struct SessionManager<S, Store: SessionStore, C: CookieController> {
    inner: S,
    layer: SessionManagerLayer<Store, C>,
}

impl<S, Store: SessionStore, C: CookieController> SessionManager<S, Store, C> {
    fn session_cookie<'c>(&self, cookies: &'c Cookies) -> Option<Cookie<'c>> {
        self.layer
            .cookie_controller
            .get(cookies, &self.layer.cookie_name)
    }
}

impl<S, Store: SessionStore, C: CookieController> Clone for SessionManager<S, Store, C>
where
    S: Clone,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            layer: self.layer.clone(),
        }
    }
}

impl<ReqBody, ResBody, S, Store: SessionStore, C: CookieController> Service<Request<ReqBody>>
    for SessionManager<S, Store, C>
where
    S: Service<Request<ReqBody>, Response = Response<ResBody>>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = ResponseFuture<S::Future, C>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(name = "SessionManager", skip(self, req))
    )]
    fn call(&mut self, mut req: Request<ReqBody>) -> Self::Future {
        if let Some(cookies) = req.extensions().get::<Cookies>().cloned() {
            let cookie = self.session_cookie(&cookies).map(Cookie::into_owned);
            let session = Session::from_or_empty(cookie);

            req.extensions_mut().insert(session.clone());

            ResponseFuture {
                state: State::Session {
                    session,
                    cookies,
                    cookie_controller: self.layer.cookie_controller.clone(),
                },
                future: self.inner.call(req),
            }
        } else {
            error!("tower_cookies::CookieManagerLayer must be added before SessionManagerLayer");

            ResponseFuture {
                state: State::Fallback,
                future: self.inner.call(req),
            }
        }
    }
}

fn extract_session_key<B>(req: &Request<B>, config: &CookieConfiguration) -> Option<SessionKey> {
    let jar = CookieJar::from_headers(req.headers());

    let cookie_result = match config.content_security {
        CookieContentSecurity::Signed => jar.signed(&config.key).get(&config.name),
        CookieContentSecurity::Private => jar.private(&config.key).get(&config.name),
    };

    if cookie_result.is_none() && jar.get(&config.name).is_some() {
        warn!(
            "session cookie attached to the incoming request failed to pass cryptographic \
            checks (signature verification/decryption)."
        );
    }

    match SessionKey::decode(cookie_result?.value()) {
        Ok(session_key) => Some(session_key),
        Err(err) => {
            warn!(
                error = %err.display_chain(),
                "invalid session key; ignoring"
            );
            None
        }
    }
}

pin_project! {
    /// Response future for [`SessionManager`].
    pub struct ResponseFuture<F, C: CookieController> {
        state: State<C>,
        #[pin]
        future: F,
    }
}

enum State<C> {
    Session {
        session: Session,
        cookies: Cookies,
        cookie_controller: C,
    },
    Fallback,
}

impl<F, B, E, C: CookieController> Future for ResponseFuture<F, C>
where
    F: Future<Output = Result<Response<B>, E>>,
{
    type Output = Result<Response<B>, E>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let mut res = ready!(this.future.poll(cx)?);

        if let State::Session {
            session,
            cookies,
            cookie_controller,
        } = this.state
        {
            todo!("sync changes in session with cookie jar, which will in turn cause the  `CookieManager` future to set the `Set-Cookie` header");
        }

        Poll::Ready(Ok(res))
    }
}
