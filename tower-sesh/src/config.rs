//! Cookie configuration.

use cookie::{Cookie, CookieJar, Key};

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
