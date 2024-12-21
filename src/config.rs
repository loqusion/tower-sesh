use cookie::{Cookie, CookieJar, Key};

pub enum CookieContentSecurity {
    Signed,
    Private,
}

/// Trait used to control how cookies are stored and retrieved.
#[doc(hidden)]
pub trait CookieController: Clone {
    fn get<'c>(&self, jar: &'c CookieJar, name: &str) -> Option<Cookie<'c>>;
    fn add(&self, jar: &mut CookieJar, cookie: Cookie<'static>);
    fn remove(&self, jar: &mut CookieJar, cookie: Cookie<'static>);
    fn into_key(self) -> Key;
}

#[doc(hidden)]
#[derive(Clone, Debug)]
pub struct SignedCookieController {
    key: Key,
}

#[doc(hidden)]
#[derive(Clone, Debug)]
pub struct PrivateCookieController {
    key: Key,
}

impl SignedCookieController {
    pub(crate) fn new(key: Key) -> Self {
        Self { key }
    }
}

impl PrivateCookieController {
    pub(crate) fn new(key: Key) -> Self {
        Self { key }
    }
}

impl CookieController for SignedCookieController {
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

impl CookieController for PrivateCookieController {
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
