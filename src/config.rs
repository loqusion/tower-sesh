use std::borrow::Cow;

use cookie::{Key, SameSite};

pub(crate) enum CookieContentSecurity {
    Signed,
    Private,
}

pub(crate) struct CookieConfiguration {
    pub(crate) secure: bool,
    pub(crate) http_only: bool,
    pub(crate) name: Cow<'static, str>,
    pub(crate) same_site: SameSite,
    pub(crate) path: Cow<'static, str>,
    pub(crate) domain: Option<Cow<'static, str>>,
    pub(crate) max_age: Option<()>, // todo
    pub(crate) content_security: CookieContentSecurity,
    pub(crate) key: Key,
}
