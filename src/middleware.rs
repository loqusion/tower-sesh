use std::{
    borrow::Cow,
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{ready, Context, Poll},
};

use cookie::{Cookie, CookieJar, SameSite};
use http::{Request, Response};
use pin_project_lite::pin_project;
use tower::{Layer, Service};

use crate::{
    config::{CookieSecurity, PlainCookie, PrivateCookie, SignedCookie},
    session::Session,
    store::SessionStore,
    util::CookieJarExt,
};

/// The default cookie name used by [`SessionLayer`] to store a session key.
const DEFAULT_COOKIE_NAME: &str = "session_key";

/// A layer that provides [`Session`] as a request extension.
///
/// # Example
///
/// TODO: Provide an example
#[derive(Debug)]
pub struct SessionLayer<Store: SessionStore, C: CookieSecurity = PrivateCookie> {
    session_store: Arc<Store>,
    cookie_name: Cow<'static, str>,
    cookie_controller: C,
}

impl<Store: SessionStore> SessionLayer<Store> {
    /// Create a new `SessionLayer`.
    ///
    /// TODO: More documentation
    pub fn new(session_store: Arc<Store>, key: cookie::Key) -> Self {
        Self {
            session_store,
            cookie_name: Cow::Borrowed(DEFAULT_COOKIE_NAME),
            cookie_controller: PrivateCookie::new(key),
        }
    }
}

// TODO: Add customization for session expiry
impl<Store: SessionStore, C: CookieSecurity> SessionLayer<Store, C> {
    /// Authenticate cookies.
    ///
    /// TODO: More documentation
    #[track_caller]
    pub fn signed(self) -> SessionLayer<Store, SignedCookie> {
        let key = self.cookie_controller.into_key();
        SessionLayer {
            session_store: self.session_store,
            cookie_name: self.cookie_name,
            cookie_controller: SignedCookie::new(key),
        }
    }

    /// Encrypt cookies.
    ///
    /// TODO: More documentation
    #[track_caller]
    pub fn private(self) -> SessionLayer<Store, PrivateCookie> {
        let key = self.cookie_controller.into_key();
        SessionLayer {
            session_store: self.session_store,
            cookie_name: self.cookie_name,
            cookie_controller: PrivateCookie::new(key),
        }
    }

    /// Set the [name][mdn] of the cookie used to store a session id.
    ///
    /// Default: `session_key`
    ///
    /// [mdn]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Set-Cookie#cookie-namecookie-value
    pub fn cookie_name(mut self, name: impl Into<Cow<'static, str>>) -> Self {
        self.cookie_name = name.into();
        self
    }

    /// Set the [`Domain`][mdn] attribute in the `Set-Cookie` response header.
    ///
    /// [mdn]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Set-Cookie#domaindomain-value
    pub fn domain(mut self, domain: impl Into<Cow<'static, str>>) -> Self {
        todo!()
    }

    /// Set whether to add the [`HttpOnly`][mdn] attribute in the `Set-Cookie`
    /// response header.
    ///
    /// [mdn]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Set-Cookie#httponly
    pub fn http_only(mut self, enable: bool) -> Self {
        todo!()
    }

    /// Set the [`Path`][mdn] attribute in the `Set-Cookie` response header.
    ///
    /// [mdn]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Set-Cookie#pathpath-value
    pub fn path(mut self, path: impl Into<Cow<'static, str>>) -> Self {
        todo!()
    }

    /// Set the [`SameSite`][mdn] attribute in the `Set-Cookie` response header.
    ///
    /// [mdn]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Set-Cookie#samesitesamesite-value
    pub fn same_site(mut self, same_site: SameSite) -> Self {
        todo!()
    }

    /// Set whether to add the [`Secure`][mdn] attribute in the `Set-Cookie`
    /// response header.
    ///
    /// [mdn]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Set-Cookie#secure
    pub fn secure(mut self, enable: bool) -> Self {
        todo!()
    }
}

impl<Store: SessionStore> SessionLayer<Store, PlainCookie> {
    /// Create a new `SessionLayer` that doesn't sign or encrypt cookies.
    pub fn plain(session_store: Arc<Store>) -> SessionLayer<Store, PlainCookie> {
        SessionLayer {
            session_store,
            cookie_name: Cow::Borrowed(DEFAULT_COOKIE_NAME),
            cookie_controller: PlainCookie,
        }
    }
}

impl<Store: SessionStore, C: CookieSecurity> Clone for SessionLayer<Store, C> {
    fn clone(&self) -> Self {
        Self {
            session_store: Arc::clone(&self.session_store),
            cookie_name: self.cookie_name.clone(),
            cookie_controller: self.cookie_controller.clone(),
        }
    }
}

impl<S, Store: SessionStore, C: CookieSecurity> Layer<S> for SessionLayer<Store, C> {
    type Service = SessionManager<S, Store, C>;

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
pub struct SessionManager<S, Store: SessionStore, C: CookieSecurity> {
    inner: S,
    layer: SessionLayer<Store, C>,
}

impl<S, Store: SessionStore, C: CookieSecurity> SessionManager<S, Store, C> {
    fn session_cookie<'c>(&self, jar: &'c CookieJar) -> Option<Cookie<'c>> {
        self.layer
            .cookie_controller
            .get(jar, &self.layer.cookie_name)
    }
}

impl<ReqBody, ResBody, S, Store: SessionStore, C: CookieSecurity> Service<Request<ReqBody>>
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

    fn call(&mut self, mut req: Request<ReqBody>) -> Self::Future {
        let jar = CookieJar::from_headers(req.headers());
        let cookie = self.session_cookie(&jar);

        todo!()
        // if let Some(cookies) = req.extensions().get::<Cookies>().cloned() {
        //     let cookie = self.session_cookie(&cookies).map(Cookie::into_owned);
        //     let session = Session::from_or_empty(cookie);
        //
        //     req.extensions_mut().insert(session.clone());
        //
        //     ResponseFuture {
        //         state: State::Session {
        //             session,
        //             cookies,
        //             cookie_controller: self.layer.cookie_controller.clone(),
        //         },
        //         future: self.inner.call(req),
        //     }
        // } else {
        //     error!("tower_cookies::CookieManagerLayer must be added before SessionLayer");
        //
        //     ResponseFuture {
        //         state: State::Fallback,
        //         future: self.inner.call(req),
        //     }
        // }
    }
}

// fn extract_session_key<B>(req: &Request<B>, config: &CookieConfiguration) -> Option<SessionKey> {
//     let jar = CookieJar::from_headers(req.headers());
//
//     let cookie_result = match config.content_security {
//         CookieContentSecurity::Signed => jar.signed(&config.key).get(&config.name),
//         CookieContentSecurity::Private => jar.private(&config.key).get(&config.name),
//     };
//
//     if cookie_result.is_none() && jar.get(&config.name).is_some() {
//         warn!(
//             "session cookie attached to the incoming request failed to pass cryptographic \
//             checks (signature verification/decryption)."
//         );
//     }
//
//     match SessionKey::decode(cookie_result?.value()) {
//         Ok(session_key) => Some(session_key),
//         Err(err) => {
//             warn!(
//                 error = %err.display_chain(),
//                 "invalid session key; ignoring"
//             );
//             None
//         }
//     }
// }

pin_project! {
    /// Response future for [`SessionManager`].
    pub struct ResponseFuture<F, C: CookieSecurity> {
        state: State<C>,
        #[pin]
        future: F,
    }
}

enum State<C> {
    Session {
        session: Session,
        cookie_controller: C,
    },
    Fallback,
}

impl<F, B, E, C: CookieSecurity> Future for ResponseFuture<F, C>
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
