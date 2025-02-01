use std::{
    borrow::Cow,
    future::Future,
    marker::PhantomData,
    pin::Pin,
    sync::Arc,
    task::{ready, Context, Poll},
};

use cookie::{Cookie, CookieJar, SameSite};
use http::{Request, Response};
use pin_project_lite::pin_project;
use tower::{Layer, Service};
use tower_sesh_core::SessionStore;

use crate::{
    config::{CookieSecurity, PlainCookie, PrivateCookie, SignedCookie},
    session::{self, Session},
    util::CookieJarExt,
};

/// The default cookie name used by [`SessionLayer`].
const DEFAULT_COOKIE_NAME: &str = "id";

/// A layer that provides [`Session`] as a request extension.
///
/// # Example
///
/// TODO: Provide an example
///
/// # Test
///
/// TODO: Replace with example
///
/// ```no_run
/// use std::sync::Arc;
/// use tower_sesh::{store::MemoryStore, SessionLayer};
///
/// #[derive(Clone)]
/// struct SessionData {
///     foo: String,
///     bar: u64,
/// }
///
/// let key = cookie::Key::generate();
/// let store = Arc::new(MemoryStore::<SessionData>::new());
/// let session_layer = SessionLayer::new(store, key);
/// ```
#[derive(Debug)]
pub struct SessionLayer<T, Store: SessionStore<T>, C: CookieSecurity = PrivateCookie> {
    store: Arc<Store>,
    cookie_name: Cow<'static, str>,
    cookie_controller: C,
    _marker: PhantomData<fn() -> T>,
}

impl<T, Store: SessionStore<T>> SessionLayer<T, Store> {
    /// Create a new `SessionLayer`.
    ///
    /// TODO: More documentation
    // TODO: Try to remove `cookie` from this crate's public API
    pub fn new(store: Arc<Store>, key: cookie::Key) -> SessionLayer<T, Store> {
        Self {
            store,
            cookie_name: Cow::Borrowed(DEFAULT_COOKIE_NAME),
            cookie_controller: PrivateCookie::new(key),
            _marker: PhantomData,
        }
    }
}

// TODO: Add customization for session expiry
impl<T, Store: SessionStore<T>, C: CookieSecurity> SessionLayer<T, Store, C> {
    /// Authenticate cookies.
    ///
    /// TODO: More documentation
    #[track_caller]
    pub fn signed(self) -> SessionLayer<T, Store, SignedCookie> {
        let key = self.cookie_controller.into_key();
        SessionLayer {
            store: self.store,
            cookie_name: self.cookie_name,
            cookie_controller: SignedCookie::new(key),
            _marker: PhantomData,
        }
    }

    /// Encrypt cookies.
    ///
    /// TODO: More documentation
    #[track_caller]
    pub fn private(self) -> SessionLayer<T, Store, PrivateCookie> {
        let key = self.cookie_controller.into_key();
        SessionLayer {
            store: self.store,
            cookie_name: self.cookie_name,
            cookie_controller: PrivateCookie::new(key),
            _marker: PhantomData,
        }
    }

    /// Set the [name] of the cookie used to store a session id.
    ///
    /// Default: `"id"`
    ///
    /// See also: [Session ID Name Fingerprinting] on the OWASP Session
    /// Management Cheat Sheet.
    ///
    /// [name]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Set-Cookie#cookie-namecookie-value
    /// [Session ID Name Fingerprinting]: https://cheatsheetseries.owasp.org/cheatsheets/Session_Management_Cheat_Sheet.html#session-id-name-fingerprinting
    pub fn cookie_name(mut self, name: impl Into<Cow<'static, str>>) -> Self {
        self.cookie_name = name.into();
        self
    }

    /// Set the [`Domain`] attribute in the `Set-Cookie` response header.
    ///
    /// [`Domain`]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Set-Cookie#domaindomain-value
    pub fn domain(mut self, domain: impl Into<Cow<'static, str>>) -> Self {
        todo!()
    }

    /// Set whether to add the [`HttpOnly`] attribute in the `Set-Cookie`
    /// response header.
    ///
    /// [`HttpOnly`]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Set-Cookie#httponly
    pub fn http_only(mut self, enable: bool) -> Self {
        todo!()
    }

    /// Set the [`Path`] attribute in the `Set-Cookie` response header.
    ///
    /// [`Path`]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Set-Cookie#pathpath-value
    pub fn path(mut self, path: impl Into<Cow<'static, str>>) -> Self {
        todo!()
    }

    /// Set the [`SameSite`] attribute in the `Set-Cookie` response header.
    ///
    /// [`SameSite`]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Set-Cookie#samesitesamesite-value
    pub fn same_site(mut self, same_site: SameSite) -> Self {
        todo!()
    }

    /// Set whether to add the [`Secure`] attribute in the `Set-Cookie`
    /// response header.
    ///
    /// [`Secure`]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Set-Cookie#secure
    pub fn secure(mut self, enable: bool) -> Self {
        todo!()
    }
}

impl<T, Store: SessionStore<T>> SessionLayer<T, Store, PlainCookie> {
    /// Create a new `SessionLayer` that doesn't sign or encrypt cookies.
    pub fn plain(store: Arc<Store>) -> SessionLayer<T, Store, PlainCookie> {
        SessionLayer {
            store,
            cookie_name: Cow::Borrowed(DEFAULT_COOKIE_NAME),
            cookie_controller: PlainCookie,
            _marker: PhantomData,
        }
    }
}

impl<T, Store: SessionStore<T>, C: CookieSecurity> Clone for SessionLayer<T, Store, C> {
    fn clone(&self) -> Self {
        Self {
            store: Arc::clone(&self.store),
            cookie_name: self.cookie_name.clone(),
            cookie_controller: self.cookie_controller.clone(),
            _marker: PhantomData,
        }
    }
}

impl<S, T, Store: SessionStore<T>, C: CookieSecurity> Layer<S> for SessionLayer<T, Store, C> {
    type Service = SessionManager<S, T, Store, C>;

    fn layer(&self, inner: S) -> Self::Service {
        SessionManager {
            inner,
            layer: self.clone(),
        }
    }
}

/// A middleware that provides [`Session`] as a request extension.
///
/// [`Session`]: crate::session::Session
#[derive(Clone, Debug)]
pub struct SessionManager<S, T, Store: SessionStore<T>, C: CookieSecurity> {
    inner: S,
    layer: SessionLayer<T, Store, C>,
}

impl<S, T, Store: SessionStore<T>, C: CookieSecurity> SessionManager<S, T, Store, C> {
    fn session_cookie<'c>(&self, jar: &'c CookieJar) -> Option<Cookie<'c>> {
        self.layer
            .cookie_controller
            .get(jar, &self.layer.cookie_name)
    }
}

impl<ReqBody, ResBody, S, T, Store: SessionStore<T>, C: CookieSecurity> Service<Request<ReqBody>>
    for SessionManager<S, T, Store, C>
where
    S: Service<Request<ReqBody>, Response = Response<ResBody>>,
    T: 'static + Send + Sync,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = ResponseFuture<S::Future, T, C>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<ReqBody>) -> Self::Future {
        let jar = CookieJar::from_headers(req.headers());
        let cookie = self.session_cookie(&jar).map(Cookie::into_owned);
        session::lazy::insert(cookie, &self.layer.store, req.extensions_mut());

        // pass the request to the inner service...

        // FIXME: Don't panic here, propagate the error instead.
        let session: Option<Session<T>> =
            session::lazy::take(req.extensions_mut()).expect("this panic should be removed");

        todo!()
    }
}

pin_project! {
    /// Response future for [`SessionManager`].
    pub struct ResponseFuture<F, T, C: CookieSecurity> {
        state: State<T, C>,
        #[pin]
        future: F,
    }
}

enum State<T, C> {
    Session {
        session: Session<T>,
        cookie_controller: C,
    },
    Fallback,
}

impl<F, B, E, T, C: CookieSecurity> Future for ResponseFuture<F, T, C>
where
    F: Future<Output = Result<Response<B>, E>>,
{
    type Output = Result<Response<B>, E>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let mut res = ready!(this.future.poll(cx)?);

        if let State::Session {
            session,
            cookie_controller,
        } = this.state
        {
            todo!("sync changes in session state to store and set the `Set-Cookie` header");
        }

        Poll::Ready(Ok(res))
    }
}
