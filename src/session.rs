use std::{any::Any, collections::HashMap, fmt, num::NonZeroU128, sync::Arc};

use async_trait::async_trait;
use base64::Engine;
use parking_lot::Mutex;
use rand::{CryptoRng, Rng};
use tower_cookies::Cookie;

use crate::store::SessionStore;

pub struct Session {
    inner: Arc<SessionInner>,
    store: Arc<dyn SessionStore>,
}

impl Session {
    /// Returns a [`Session`] by attempting to parse `cookie` as an [`Id`],
    /// falling back to an empty session if `cookie` is `None` or parsing failed.
    #[must_use]
    pub(crate) fn from_or_empty(
        cookie: Option<Cookie<'static>>,
        store: Arc<dyn SessionStore>,
    ) -> Self {
        let session_id = cookie.as_ref().and_then(|c| Id::decode(c.value()).ok());
        let inner = SessionInner {
            session_id: Mutex::new(session_id),
        };

        Session {
            inner: Arc::new(inner),
            store,
        }
    }
}

impl Clone for Session {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
            store: Arc::clone(&self.store),
        }
    }
}

#[cfg(feature = "axum")]
#[async_trait]
impl<S> axum::extract::FromRequestParts<S> for Session {
    type Rejection = SessionRejection;

    async fn from_request_parts(
        parts: &mut http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<Session>()
            .cloned()
            .ok_or(SessionRejection)
    }
}

define_rejection! {
    #[status = INTERNAL_SERVER_ERROR]
    #[body = "Missing request extension"]
    /// Rejection for [`Session`] if an expected request extension
    /// was not found.
    pub struct SessionRejection;
}

#[derive(Debug)]
struct SessionInner {
    session_id: Mutex<Option<Id>>,
}

/// A 128-bit session identifier.
// `NonZeroU128` is used so that `Option<Id>` has the same size as `Id`
#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq)]
pub struct Id(pub(crate) NonZeroU128);

impl Id {
    const BASE64_ENGINE: base64::engine::general_purpose::GeneralPurpose =
        base64::engine::general_purpose::URL_SAFE_NO_PAD;
    const DECODED_LEN: usize = 16;

    #[must_use]
    pub fn generate() -> Self {
        Id::generate_from_rng(&mut rand::thread_rng())
    }

    #[must_use]
    pub fn generate_from_rng<R: CryptoRng + Rng>(rng: &mut R) -> Self {
        Self(rng.gen())
    }

    #[must_use]
    pub fn encode(&self) -> String {
        Id::BASE64_ENGINE.encode(self.0.get().to_le_bytes())
    }

    pub fn decode(s: &str) -> Result<Id, ParseIdError> {
        use base64::DecodeError;

        let mut buf = [0; const { Id::DECODED_LEN }];
        Id::BASE64_ENGINE
            .decode_slice(s.as_bytes(), &mut buf)
            .and_then(|decoded_len| {
                if decoded_len == Id::DECODED_LEN {
                    Ok(())
                } else {
                    Err(DecodeError::InvalidLength(decoded_len).into())
                }
            })?;

        match u128::from_le_bytes(buf).try_into() {
            Ok(v) => Ok(Id(v)),
            Err(_) => Err(ParseIdError::Zero),
        }
    }
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.encode())
    }
}

impl From<NonZeroU128> for Id {
    #[inline]
    fn from(value: NonZeroU128) -> Self {
        Self(value)
    }
}

impl TryFrom<u128> for Id {
    type Error = std::num::TryFromIntError;

    #[inline]
    fn try_from(value: u128) -> Result<Self, Self::Error> {
        value.try_into().map(Id)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ParseIdError {
    #[error("")]
    Base64(#[from] base64::DecodeSliceError),
    #[error("session id must be non-zero")]
    Zero,
}

type AnyMap = HashMap<String, Box<dyn AnyClone + Send + Sync>>;

#[derive(Clone)]
pub struct Record {
    id: Id,
    data: AnyMap,
}

trait AnyClone: Any {}

impl Clone for Box<dyn AnyClone + Send + Sync> {
    fn clone(&self) -> Self {
        todo!()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use quickcheck::{quickcheck, Arbitrary};

    #[test]
    fn id_parse_error_zero() {
        const INPUT: &str = "AAAAAAAAAAAAAAAAAAAAAA";
        let result = Id::decode(INPUT);
        assert!(
            matches!(result, Err(ParseIdError::Zero)),
            "expected decoding to fail with `ParseIdError::Zero`"
        );
    }

    impl Arbitrary for Id {
        fn arbitrary(g: &mut quickcheck::Gen) -> Self {
            Id::from(NonZeroU128::arbitrary(g))
        }
    }

    quickcheck! {
        fn id_encode_decode(id: Id) -> bool {
            let encoded = id.encode();
            let decoded = Id::decode(&encoded).unwrap();
            id == decoded
        }
    }
}
