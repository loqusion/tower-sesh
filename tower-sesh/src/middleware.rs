use std::{
    borrow::Cow,
    error::Error,
    fmt,
    marker::PhantomData,
    sync::Arc,
    task::{Context, Poll},
};

use cookie::{Cookie, CookieJar};
use futures_util::{future::BoxFuture, FutureExt};
use http::{header, HeaderMap, HeaderValue, Request, Response};
use tower::{Layer, Service};
use tower_sesh_core::{util::Report, SessionKey, SessionStore};

use crate::{
    config::{CookieSecurity, PlainCookie, PrivateCookie, SignedCookie},
    session::{self, SyncAction},
};

/// A layer that provides [`Session`] as an extractor.
///
/// [`Session`]: crate::Session
///
/// # Examples
///
/// TODO: Provide an example
// NOTE: If an inner service returns an error, the session will not be synced to
// the store.
pub struct SessionLayer<T, Store: SessionStore<T>, C = PrivateCookie> {
    store: Arc<Store>,
    config: Arc<Config>,       // This is put in an `Arc` to make clones cheap.
    cookie_controller: Arc<C>, // Ditto.
    _marker: PhantomData<fn() -> T>,
}

/// A middleware that provides [`Session`] as an extractor.
///
/// [`Session`]: crate::session::Session
pub struct SessionManager<S, T, Store: SessionStore<T>, C> {
    inner: S,
    layer: SessionLayer<T, Store, C>,
}

#[derive(Clone, Debug)]
struct Config {
    cookie_name: Cow<'static, str>,
    domain: Option<Cow<'static, str>>,
    http_only: bool,
    path: Option<Cow<'static, str>>,
    same_site: cookie::SameSite,
    secure: bool,
}

impl Config {
    /// Chosen to avoid session ID name fingerprinting.
    const DEFAULT_COOKIE_NAME: &str = "id";

    // TODO: Add the `Expires` attribute.
    fn cookie(&self, session_key: SessionKey) -> Cookie<'_> {
        let mut cookie = Cookie::build((&*self.cookie_name, session_key.encode()))
            .http_only(self.http_only)
            .same_site(self.same_site)
            .secure(self.secure);

        if let Some(domain) = &self.domain {
            cookie = cookie.domain(&**domain);
        }
        if let Some(path) = &self.path {
            cookie = cookie.path(&**path);
        }

        cookie.build()
    }

    #[inline]
    fn cookie_removal(&self) -> Cookie<'_> {
        let mut cookie = Cookie::new(&*self.cookie_name, "");
        cookie.make_removal();
        cookie
    }
}

impl Default for Config {
    /// Defaults are based on [OWASP recommendations].
    ///
    /// [OWASP recommendations]: https://cheatsheetseries.owasp.org/cheatsheets/Session_Management_Cheat_Sheet.html#cookies
    #[inline]
    fn default() -> Self {
        Config {
            cookie_name: Cow::Borrowed(Config::DEFAULT_COOKIE_NAME),
            domain: None,
            http_only: true,
            path: None,
            same_site: cookie::SameSite::Strict,
            secure: true,
        }
    }
}

impl<T, Store: SessionStore<T>> SessionLayer<T, Store> {
    /// Creates a new `SessionLayer` with default configuration values.
    ///
    /// By default, cookie values are encrypted with the provided 64-byte `key`.
    /// See the [`private`] method documentation for more details.
    ///
    /// To sign cookies with the provided key instead, use [`signed`]. To use
    /// plain cookies that are neither signed nor encrypted (not recommended),
    /// use [`plain`].
    ///
    /// [`private`]: SessionLayer::private
    /// [`signed`]: SessionLayer::signed
    /// [`plain`]: SessionLayer::plain
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::Arc;
    /// use tower_sesh::{middleware::Key, store::MemoryStore, SessionLayer};
    ///
    /// # type SessionData = ();
    /// #
    /// fn key() -> Key {
    ///     // TODO: Where do you get a key?
    /// # Key::from([0; 64])
    /// }
    ///
    /// let key = key();
    /// let store = Arc::new(MemoryStore::<SessionData>::new());
    /// let layer = SessionLayer::new(store, key);
    /// ```
    #[track_caller]
    pub fn new(store: Arc<Store>, key: Key) -> SessionLayer<T, Store> {
        let key = key.into_cookie_key();
        Self {
            store,
            config: Arc::new(Config::default()),
            cookie_controller: Arc::new(PrivateCookie::new(key)),
            _marker: PhantomData,
        }
    }
}

// TODO: Add customization for session expiry
impl<T, Store: SessionStore<T>, C: CookieSecurity> SessionLayer<T, Store, C> {
    /// Authenticates cookies.
    ///
    /// TODO: More documentation
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::Arc;
    /// use tower_sesh::{middleware::Key, store::MemoryStore, SessionLayer};
    ///
    /// # type SessionData = ();
    /// #
    /// fn key() -> Key {
    ///     // TODO: Where do you get a key?
    /// # Key::from([0; 64])
    /// }
    ///
    /// let key = key();
    /// let store = Arc::new(MemoryStore::<SessionData>::new());
    /// let layer = SessionLayer::new(store, key).signed();
    /// ```
    #[track_caller]
    pub fn signed(self) -> SessionLayer<T, Store, SignedCookie> {
        let key = self.cookie_controller.key().to_owned();
        SessionLayer {
            store: self.store,
            config: self.config,
            cookie_controller: Arc::new(SignedCookie::new(key)),
            _marker: PhantomData,
        }
    }

    /// Encrypts cookies.
    ///
    /// TODO: More documentation
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::Arc;
    /// use tower_sesh::{middleware::Key, store::MemoryStore, SessionLayer};
    ///
    /// # type SessionData = ();
    /// #
    /// fn key() -> Key {
    ///     // TODO: Where do you get a key?
    /// # Key::from([0; 64])
    /// }
    ///
    /// let key = key();
    /// let store = Arc::new(MemoryStore::<SessionData>::new());
    /// let layer = SessionLayer::new(store, key).private();
    /// ```
    #[track_caller]
    pub fn private(self) -> SessionLayer<T, Store, PrivateCookie> {
        let key = self.cookie_controller.key().to_owned();
        SessionLayer {
            store: self.store,
            config: self.config,
            cookie_controller: Arc::new(PrivateCookie::new(key)),
            _marker: PhantomData,
        }
    }

    /// Sets the [name] of the cookie used to store a session id.
    ///
    /// [OWASP recommends] that `cookie_name` be terse and undescriptive to
    /// avoid [fingerprinting].
    ///
    /// Default is `"id"`.
    ///
    /// [name]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Set-Cookie#cookie-namecookie-value
    /// [OWASP recommends]:
    ///     https://cheatsheetseries.owasp.org/cheatsheets/Session_Management_Cheat_Sheet.html#session-id-name-fingerprinting
    /// [fingerprinting]: https://wiki.owasp.org/index.php/Category:OWASP_Cookies_Database
    ///
    /// # Panics
    ///
    /// Panics if `name` contains an invalid character.
    ///
    /// # Examples
    ///
    /// ```
    /// use tower_sesh::SessionLayer;
    /// # use std::sync::Arc;
    /// # use tower_sesh::store::MemoryStore;
    ///
    /// # let key = tower_sesh::middleware::Key::from([0; 64]);
    /// # let store = Arc::new(MemoryStore::<()>::new());
    /// let layer = SessionLayer::new(store, key).cookie_name("id");
    /// ```
    #[track_caller]
    pub fn cookie_name(mut self, name: impl Into<Cow<'static, str>>) -> Self {
        let name = name.into();

        if let Err(err) = HeaderValue::try_from(format!("{}=value", name)) {
            panic!("invalid `cookie_name` value: {}", Report::new(err));
        }

        self.config_mut().cookie_name = name;
        self
    }

    /// Sets the [`Domain`] attribute in the `Set-Cookie` response header.
    ///
    /// [OWASP recommends] that `Domain` be omitted so that the cookie is
    /// restricted to the origin server.
    ///
    /// Default is for `Domain` to be omitted.
    ///
    /// [`Domain`]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Set-Cookie#domaindomain-value
    /// [OWASP recommends]:
    ///     https://cheatsheetseries.owasp.org/cheatsheets/Session_Management_Cheat_Sheet.html#domain-and-path-attributes
    ///
    /// # Examples
    ///
    /// ```
    /// use tower_sesh::SessionLayer;
    /// # use std::sync::Arc;
    /// # use tower_sesh::store::MemoryStore;
    ///
    /// # let key = tower_sesh::middleware::Key::from([0; 64]);
    /// # let store = Arc::new(MemoryStore::<()>::new());
    /// let layer = SessionLayer::new(store, key).domain("doc.rust-lang.org");
    /// ```
    pub fn domain(mut self, domain: impl Into<Cow<'static, str>>) -> Self {
        self.config_mut().domain = Some(domain.into());
        self
    }

    /// Sets whether to add the [`HttpOnly`] attribute in the `Set-Cookie`
    /// response header.
    ///
    /// [OWASP recommends] adding `HttpOnly` to prevent session key stealing
    /// through XSS attacks.
    ///
    /// Default is `true`.
    ///
    /// [`HttpOnly`]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Set-Cookie#httponly
    /// [OWASP recommends]:
    ///     https://cheatsheetseries.owasp.org/cheatsheets/Session_Management_Cheat_Sheet.html#httponly-attribute
    pub fn http_only(mut self, enable: bool) -> Self {
        self.config_mut().http_only = enable;
        self
    }

    /// Sets the [`Path`] attribute in the `Set-Cookie` response header.
    ///
    /// [OWASP recommends] that `Path` be as restrictive as possible.
    ///
    /// Default is for `Path` to be omitted.
    ///
    /// [`Path`]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Set-Cookie#pathpath-value
    /// [OWASP recommends]:
    ///     https://cheatsheetseries.owasp.org/cheatsheets/Session_Management_Cheat_Sheet.html#domain-and-path-attributes
    ///
    /// # Examples
    ///
    /// ```
    /// use tower_sesh::SessionLayer;
    /// # use std::sync::Arc;
    /// # use tower_sesh::store::MemoryStore;
    ///
    /// # let key = tower_sesh::middleware::Key::from([0; 64]);
    /// # let store = Arc::new(MemoryStore::<()>::new());
    /// let layer = SessionLayer::new(store, key).path("/std");
    /// ```
    pub fn path(mut self, path: impl Into<Cow<'static, str>>) -> Self {
        self.config_mut().path = Some(path.into());
        self
    }

    /// Sets the [`SameSite`] attribute in the `Set-Cookie` response header.
    ///
    /// For recommendations on setting this attribute, see this [IETF draft].
    ///
    /// Default is [`SameSite::Strict`].
    ///
    /// [`SameSite`]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Set-Cookie#samesitesamesite-value
    /// [IETF draft]: https://datatracker.ietf.org/doc/html/draft-ietf-httpbis-rfc6265bis-20#name-samesite-cookies
    ///
    /// # Examples
    ///
    /// ```
    /// use tower_sesh::{middleware::SameSite, SessionLayer};
    /// # use std::sync::Arc;
    /// # use tower_sesh::store::MemoryStore;
    ///
    /// # let key = tower_sesh::middleware::Key::from([0; 64]);
    /// # let store = Arc::new(MemoryStore::<()>::new());
    /// let layer = SessionLayer::new(store, key).same_site(SameSite::Strict);
    /// ```
    pub fn same_site(mut self, same_site: SameSite) -> Self {
        self.config_mut().same_site = same_site.into_cookie_same_site();
        self
    }

    /// Sets whether to add the [`Secure`] attribute in the `Set-Cookie`
    /// response header.
    ///
    /// [OWASP recommends] adding `Secure` to prevent the disclosure of the
    /// session key through man-in-the-middle attacks.
    ///
    /// Default is `true`.
    ///
    /// [`Secure`]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Set-Cookie#secure
    /// [OWASP recommends]:
    ///     https://cheatsheetseries.owasp.org/cheatsheets/Session_Management_Cheat_Sheet.html#secure-attribute
    pub fn secure(mut self, enable: bool) -> Self {
        self.config_mut().secure = enable;
        self
    }

    fn config_mut(&mut self) -> &mut Config {
        Arc::make_mut(&mut self.config)
    }
}

impl<T, Store: SessionStore<T>> SessionLayer<T, Store, PlainCookie> {
    /// Creates a new `SessionLayer` that doesn't sign or encrypt cookies.
    ///
    /// **WARNING:** Using `plain` is not recommended, as it opens the door to
    /// vulnerabilities such as session fixation and brute-force attacks.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::Arc;
    /// use tower_sesh::{store::MemoryStore, SessionLayer};
    ///
    /// # type SessionData = ();
    /// #
    /// let store = Arc::new(MemoryStore::<SessionData>::new());
    /// let layer = SessionLayer::plain(store);
    /// ```
    pub fn plain(store: Arc<Store>) -> SessionLayer<T, Store, PlainCookie> {
        SessionLayer {
            store,
            config: Arc::new(Config::default()),
            cookie_controller: Arc::new(PlainCookie),
            _marker: PhantomData,
        }
    }
}

impl<T, Store: SessionStore<T>, C: CookieSecurity> Clone for SessionLayer<T, Store, C> {
    fn clone(&self) -> Self {
        Self {
            store: Arc::clone(&self.store),
            config: self.config.clone(),
            cookie_controller: self.cookie_controller.clone(),
            _marker: PhantomData,
        }
    }
}

impl<T, Store: SessionStore<T>, C> fmt::Debug for SessionLayer<T, Store, C>
where
    Store: fmt::Debug,
    C: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SessionLayer")
            .field("store", &self.store)
            .field("config", &self.config)
            .field("cookie_security", &self.cookie_controller)
            .finish_non_exhaustive()
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

impl<S, T, Store: SessionStore<T>, C: CookieSecurity> Clone for SessionManager<S, T, Store, C>
where
    S: Clone,
{
    fn clone(&self) -> Self {
        SessionManager {
            inner: self.inner.clone(),
            layer: self.layer.clone(),
        }
    }
}

impl<S, T, Store: SessionStore<T>, C> fmt::Debug for SessionManager<S, T, Store, C>
where
    S: fmt::Debug,
    Store: fmt::Debug,
    C: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SessionManager")
            .field("inner", &self.inner)
            .field("layer", &self.layer)
            .finish()
    }
}

impl<ReqBody, ResBody, S, T, Store: SessionStore<T>, C: CookieSecurity> Service<Request<ReqBody>>
    for SessionManager<S, T, Store, C>
where
    S: Service<Request<ReqBody>, Response = Response<ResBody>>,
    S::Error: Send,
    S::Future: Send + 'static,
    ResBody: Send,
    T: Send + Sync + 'static,
    C: Send + Sync + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    #[inline]
    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<ReqBody>) -> Self::Future {
        let session_handle = {
            let cookie = session_cookie_from_request_headers(
                req.headers(),
                &self.layer.config.cookie_name,
                self.layer.cookie_controller.as_ref(),
            );
            session::lazy::insert(req.extensions_mut(), cookie, &self.layer.store)
        };

        let fut = self.inner.call(req);

        let store = Arc::clone(&self.layer.store);
        let config = Arc::clone(&self.layer.config);
        let cookie_controller = Arc::clone(&self.layer.cookie_controller);

        async move {
            let mut response = fut.await?;

            if let Some(session) = session_handle.get() {
                let session = session.take();
                let sync_result = session.sync(store.as_ref()).await;

                match sync_result {
                    Ok(SyncAction::Set(session_key)) => {
                        let mut jar = CookieJar::new();
                        let cookie = config.cookie(session_key);
                        cookie_controller.add(&mut jar, cookie.into_owned());

                        let cookie = jar
                            .get(&config.cookie_name)
                            .expect("this cookie should exist");
                        append_set_cookie(response.headers_mut(), cookie);
                    }
                    Ok(SyncAction::Remove) => {
                        let cookie_removal = config.cookie_removal();
                        append_set_cookie(response.headers_mut(), &cookie_removal);
                    }
                    Ok(SyncAction::None) => {}
                    Err(_err) => {
                        error!(err = %Report::new(_err), "error when syncing session to store");
                    }
                }
            }

            Ok(response)
        }
        .boxed()
    }
}

fn session_cookie_from_request_headers(
    headers: &HeaderMap,
    name: &str,
    cookie_controller: &impl CookieSecurity,
) -> Option<Cookie<'static>> {
    for cookie in cookies_from_request(headers) {
        if cookie.name() == name {
            let mut jar = CookieJar::new();
            jar.add_original(cookie.into_owned());

            // `cookie_controller` handles decryption/authentication if the
            // user has it enabled
            if let Some(cookie) = cookie_controller.get(&jar, name) {
                return Some(cookie.into_owned());
            } else {
                // ignore decryption/authentication failure
                break;
            }
        }
    }

    None
}

fn cookies_from_request(headers: &HeaderMap) -> impl Iterator<Item = Cookie<'_>> {
    headers
        .get_all(header::COOKIE)
        .into_iter()
        .filter_map(|header_value| header_value.to_str().ok())
        .flat_map(|cookie_list_str| cookie_list_str.split(';'))
        .filter_map(|cookie_str| Cookie::parse_encoded(cookie_str).ok())
}

#[inline]
fn append_set_cookie(headers: &mut HeaderMap<HeaderValue>, cookie: &Cookie<'_>) {
    match HeaderValue::try_from(cookie.encoded().to_string()) {
        Ok(header_value) => {
            headers.append(header::SET_COOKIE, header_value);
        }
        Err(_err) => {
            error!(err = %Report::new(_err), cookie = %cookie.encoded(), "this is likely a bug");
        }
    }
}

/// A 64-byte cryptographic key used by [`SessionLayer`] to sign or encrypt
/// cookies.
///
/// TODO: Come back after high-level documentation is written
///
/// # Examples
///
/// A key can be constructed from a slice or vector containing 64 bytes:
///
/// ```
/// use tower_sesh::middleware::Key;
///
/// let mut vec: Vec<u8> = vec![0; 64];
/// rand::fill(&mut vec[..]); // Fill with random bytes
/// let key = Key::try_from(vec).unwrap();
/// ```
#[derive(Clone)]
pub struct Key([u8; Key::LEN]);

impl Key {
    /// The size of a key, in bytes.
    pub const LEN: usize = 64;

    #[track_caller]
    fn into_cookie_key(self) -> cookie::Key {
        match cookie::Key::try_from(self.0.as_slice()) {
            Ok(key) => key,
            Err(err) => panic!("failed to convert key to `cookie::Key`: {err}"),
        }
    }
}

impl fmt::Debug for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Key(..)")
    }
}

impl From<[u8; Key::LEN]> for Key {
    fn from(value: [u8; Key::LEN]) -> Self {
        Key(value)
    }
}

impl TryFrom<&[u8]> for Key {
    type Error = KeyError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        <[u8; Key::LEN]>::try_from(value)
            .map(Key::from)
            .map_err(|_| KeyError)
    }
}

impl TryFrom<Vec<u8>> for Key {
    type Error = KeyError;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        Key::try_from(value.as_slice())
    }
}

impl TryFrom<&Vec<u8>> for Key {
    type Error = KeyError;

    fn try_from(value: &Vec<u8>) -> Result<Self, Self::Error> {
        Key::try_from(value.as_slice())
    }
}

#[cfg(test)]
impl quickcheck::Arbitrary for Key {
    fn arbitrary(g: &mut quickcheck::Gen) -> Self {
        let inner = {
            let mut buf = [0u8; Key::LEN];
            for (i, num) in std::iter::repeat_with(|| <u8 as quickcheck::Arbitrary>::arbitrary(g))
                .take(Key::LEN)
                .enumerate()
            {
                buf[i] = num;
            }
            buf
        };
        Key::from(inner)
    }
}

/// The error type returned when a conversion from a byte slice to a [`Key`]
/// fails.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct KeyError;

impl Error for KeyError {}

impl fmt::Display for KeyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("key must be 64 bytes in length")
    }
}

/// The [`SameSite`] cookie attribute, which controls whether or not a cookie is
/// sent with cross-site requests.
///
/// A cookie with a `SameSite` attribute is imposed restrictions on when it is
/// sent to the origin server in a cross-site request:
///
/// - `Strict`: The cookie is never sent in cross-site requests.
/// - `Lax`: The cookie is sent in cross-site top-level navigations.
/// - `None`: The cookie is sent in all cross-site requests if the `Secure`
///   flag is also set; otherwise, the cookie is ignored.
///
/// **Note:** This cookie attribute is an [HTTP draft]! Its meaning and
/// definition are subject to change.
///
/// See also: [Security Considerations].
///
/// [`SameSite`]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Set-Cookie#samesitesamesite-value
/// [HTTP draft]: https://datatracker.ietf.org/doc/html/draft-ietf-httpbis-rfc6265bis-20#name-the-samesite-attribute
/// [Security Considerations]:
///     https://datatracker.ietf.org/doc/html/draft-ietf-httpbis-rfc6265bis-20#name-samesite-cookies
// NOTE: `Copy` should not be implemented in case web standards change in the future.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SameSite {
    /// The cookie is never sent in cross-site requests.
    Strict,

    /// The cookie is sent in cross-site top-level navigations.
    Lax,

    /// The cookie is sent in all cross-site requests if the `Secure` flag is
    /// also set; otherwise, the cookie is ignored.
    None,
}

impl SameSite {
    fn into_cookie_same_site(self) -> cookie::SameSite {
        match self {
            SameSite::Strict => cookie::SameSite::Strict,
            SameSite::Lax => cookie::SameSite::Lax,
            SameSite::None => cookie::SameSite::None,
        }
    }
}

#[cfg(test)]
mod test {
    use quickcheck::quickcheck;

    use super::*;

    quickcheck! {
        fn key_debug_redacts_content(key: Key) -> bool {
            format!("{:?}", key) == "Key(..)"
        }

        fn converting_key_does_not_panic(key: Key) -> bool {
            key.into_cookie_key();
            true
        }
    }
}
