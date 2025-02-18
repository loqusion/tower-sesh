//! Cookie configuration.

use std::fmt;

use cookie::{Cookie, CookieJar, Key};

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
/// [`SameSite`]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Set-Cookie#samesitesamesite-value
/// [HTTP draft]: https://tools.ietf.org/html/draft-west-cookie-incrementalism-00
// NOTE: `Copy` should not be implemented in case web standards change in the future.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SameSite {
    Strict,
    Lax,
    None,
}

impl SameSite {
    #[allow(dead_code)]
    pub(crate) fn from_cookie_same_site(value: cookie::SameSite) -> SameSite {
        match value {
            cookie::SameSite::Strict => SameSite::Strict,
            cookie::SameSite::Lax => SameSite::Lax,
            cookie::SameSite::None => SameSite::None,
        }
    }

    #[allow(dead_code)]
    pub(crate) fn into_cookie_same_site(self) -> cookie::SameSite {
        match self {
            SameSite::Strict => cookie::SameSite::Strict,
            SameSite::Lax => cookie::SameSite::Lax,
            SameSite::None => cookie::SameSite::None,
        }
    }
}

impl fmt::Display for SameSite {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SameSite::Strict => f.write_str("Strict"),
            SameSite::Lax => f.write_str("Lax"),
            SameSite::None => f.write_str("None"),
        }
    }
}

/// Trait used to control how cookies are stored and retrieved.
#[doc(hidden)]
pub trait CookieSecurity: Clone + private::Sealed {
    fn get<'c>(&self, jar: &'c CookieJar, name: &str) -> Option<Cookie<'c>>;
    fn add(&self, jar: &mut CookieJar, cookie: Cookie<'static>);
    fn remove(&self, jar: &mut CookieJar, cookie: Cookie<'static>);
    fn into_key(self) -> Key;
}

#[doc(hidden)]
#[derive(Clone, Debug)]
pub struct SignedCookie {
    key: Key,
}

#[doc(hidden)]
#[derive(Clone, Debug)]
pub struct PrivateCookie {
    key: Key,
}

#[doc(hidden)]
#[non_exhaustive]
#[derive(Clone, Debug)]
pub struct PlainCookie;

impl SignedCookie {
    pub(crate) fn new(key: Key) -> Self {
        Self { key }
    }
}

impl PrivateCookie {
    pub(crate) fn new(key: Key) -> Self {
        Self { key }
    }
}

impl CookieSecurity for SignedCookie {
    #[inline]
    fn get<'c>(&self, jar: &'c CookieJar, name: &str) -> Option<Cookie<'c>> {
        jar.signed(&self.key).get(name)
    }

    #[inline]
    fn add(&self, jar: &mut CookieJar, cookie: Cookie<'static>) {
        jar.signed_mut(&self.key).add(cookie)
    }

    #[inline]
    fn remove(&self, jar: &mut CookieJar, cookie: Cookie<'static>) {
        jar.signed_mut(&self.key).remove(cookie)
    }

    #[inline]
    fn into_key(self) -> Key {
        self.key
    }
}
impl private::Sealed for SignedCookie {}

impl CookieSecurity for PrivateCookie {
    #[inline]
    fn get<'c>(&self, jar: &'c CookieJar, name: &str) -> Option<Cookie<'c>> {
        jar.private(&self.key).get(name)
    }

    #[inline]
    fn add(&self, jar: &mut CookieJar, cookie: Cookie<'static>) {
        jar.private_mut(&self.key).add(cookie)
    }

    #[inline]
    fn remove(&self, jar: &mut CookieJar, cookie: Cookie<'static>) {
        jar.private_mut(&self.key).remove(cookie)
    }

    #[inline]
    fn into_key(self) -> Key {
        self.key
    }
}
impl private::Sealed for PrivateCookie {}

impl CookieSecurity for PlainCookie {
    #[inline]
    fn get<'c>(&self, jar: &'c CookieJar, name: &str) -> Option<Cookie<'c>> {
        jar.get(name).cloned()
    }

    #[inline]
    fn add(&self, jar: &mut CookieJar, cookie: Cookie<'static>) {
        jar.add(cookie)
    }

    #[inline]
    fn remove(&self, jar: &mut CookieJar, cookie: Cookie<'static>) {
        jar.remove(cookie)
    }

    #[inline]
    #[track_caller]
    fn into_key(self) -> Key {
        unimplemented!("use `SessionLayer::new()` to sign or encrypt cookies")
    }
}
impl private::Sealed for PlainCookie {}

mod private {
    pub trait Sealed {}
}
