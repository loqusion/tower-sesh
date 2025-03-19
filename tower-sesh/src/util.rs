use cookie::{Cookie, CookieJar};
use http::{header, HeaderMap};

pub(crate) trait CookieJarExt {
    fn from_headers_single(headers: &HeaderMap, name: &str) -> Self;
}

impl CookieJarExt for CookieJar {
    fn from_headers_single(headers: &HeaderMap, name: &str) -> Self {
        let mut jar = CookieJar::new();

        for cookie in cookies_from_request(headers) {
            if cookie.name() == name {
                jar.add_original(cookie);
                return jar;
            }
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
