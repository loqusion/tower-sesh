use std::{error::Error, fmt, iter};

use cookie::{Cookie, CookieJar};
use http::{header, HeaderMap};

pub(crate) trait ErrorExt {
    fn display_chain(&self) -> DisplayChain<'_>;
}

impl<E> ErrorExt for E
where
    E: Error + 'static,
{
    /// Returns an object that implements [`Display`] for printing the
    /// whole error chain.
    ///
    /// [`Display`]: std::fmt::Display
    fn display_chain(&self) -> DisplayChain<'_> {
        DisplayChain { inner: self }
    }
}

pub(crate) struct DisplayChain<'a> {
    inner: &'a (dyn Error + 'static),
}

impl fmt::Display for DisplayChain<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.inner)?;

        for error in iter::successors(Some(self.inner), |err| (*err).source()).skip(1) {
            write!(f, ": {}", error)?;
        }

        Ok(())
    }
}

pub(crate) trait CookieJarExt {
    fn from_headers(headers: &HeaderMap) -> Self;
}

impl CookieJarExt for CookieJar {
    fn from_headers(headers: &HeaderMap) -> Self {
        let mut jar = CookieJar::new();
        for cookie in cookies_from_request(headers) {
            jar.add_original(cookie);
        }
        jar
    }
}

fn cookies_from_request(headers: &HeaderMap) -> impl Iterator<Item = Cookie<'static>> + '_ {
    headers
        .get_all(header::COOKIE)
        .into_iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(';'))
        .filter_map(|cookie| Cookie::parse_encoded(cookie.to_owned()).ok())
}
