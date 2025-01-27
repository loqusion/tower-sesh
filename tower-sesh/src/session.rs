use std::{fmt, num::NonZeroU128, sync::Arc};

use async_trait::async_trait;
use base64::Engine;
use http::Extensions;
use parking_lot::Mutex;
use rand::{CryptoRng, Rng};

use crate::record::Record;

pub struct Session(Arc<Mutex<SessionInner>>);

struct SessionInner {
    session_id: Option<SessionKey>,
    record: Option<Record>,
    status: SessionStatus,
}

enum SessionStatus {
    Unchanged,
    Renewed,
    Changed,
    Purged,
}

impl Session {
    pub(crate) fn extract(extensions: &mut Extensions) -> Option<Self> {
        extensions
            .get::<Arc<Mutex<SessionInner>>>()
            .cloned()
            .map(Session)
    }

    pub fn get<V>(&self, key: &str) -> Result<Option<&V>, GetError> {
        let mut data = self.0.lock();
        todo!()
    }

    pub fn insert<V>(&self, key: &str, value: V) -> Result<Option<V>, InsertError> {
        let mut data = self.0.lock();
        todo!()
    }

    pub fn remove<V>(&self, key: &str) -> Result<Option<V>, RemoveError> {
        let mut data = self.0.lock();
        todo!()
    }

    pub fn clear(&self) {
        let mut data = self.0.lock();
        todo!()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum GetError {}
#[derive(Debug, thiserror::Error)]
pub enum InsertError {}
#[derive(Debug, thiserror::Error)]
pub enum RemoveError {}

impl Clone for Session {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
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
        Session::extract(&mut parts.extensions).ok_or(SessionRejection)
    }
}

define_rejection! {
    #[status = INTERNAL_SERVER_ERROR]
    #[body = "Missing request extension"]
    /// Rejection for [`Session`] if an expected request extension
    /// was not found.
    pub struct SessionRejection;
}

/// A 128-bit session identifier.
// `NonZeroU128` is used so that `Option<SessionKey>` has the same size as
// `SessionKey`
#[derive(Clone, Copy, Hash, Eq, PartialEq)]
pub struct SessionKey(NonZeroU128);

impl SessionKey {
    const BASE64_ENGINE: base64::engine::GeneralPurpose =
        base64::engine::general_purpose::URL_SAFE_NO_PAD;
    pub const ENCODED_LEN: usize = 22;
    const DECODED_LEN: usize = 16;

    #[must_use]
    pub fn generate() -> Self {
        SessionKey::generate_from_rng(&mut rand::thread_rng())
    }

    #[must_use]
    pub fn generate_from_rng<R: CryptoRng + Rng>(rng: &mut R) -> Self {
        Self(rng.gen())
    }

    #[must_use]
    pub fn encode(&self) -> String {
        SessionKey::BASE64_ENGINE.encode(self.0.get().to_le_bytes())
    }

    pub fn decode(s: &str) -> Result<SessionKey, ParseSessionKeyError> {
        use base64::DecodeError;

        let mut buf = [0; const { SessionKey::DECODED_LEN }];
        SessionKey::BASE64_ENGINE
            .decode_slice(s.as_bytes(), &mut buf)
            .and_then(|decoded_len| {
                if decoded_len == SessionKey::DECODED_LEN {
                    Ok(())
                } else {
                    Err(DecodeError::InvalidLength(decoded_len).into())
                }
            })?;

        match u128::from_le_bytes(buf).try_into() {
            Ok(v) => Ok(SessionKey(v)),
            Err(_) => Err(ParseSessionKeyError::Zero),
        }
    }
}

impl fmt::Debug for SessionKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple(stringify!(SessionKey))
            .field(&format_args!("[REDACTED]"))
            .finish()
    }
}

impl From<NonZeroU128> for SessionKey {
    #[inline]
    fn from(value: NonZeroU128) -> Self {
        Self(value)
    }
}

impl TryFrom<u128> for SessionKey {
    type Error = std::num::TryFromIntError;

    #[inline]
    fn try_from(value: u128) -> Result<Self, Self::Error> {
        value.try_into().map(SessionKey)
    }
}

/// An error which can be returned when decoding a [`SessionKey`] from a
/// base64 string.
#[derive(Debug, thiserror::Error)]
pub enum ParseSessionKeyError {
    #[error("failed to parse base64 string")]
    Base64(#[from] base64::DecodeSliceError),
    #[error("session id must be non-zero")]
    Zero,
}

#[cfg(test)]
mod test {
    use super::*;
    use quickcheck::{quickcheck, Arbitrary};

    #[test]
    fn traits() {
        fn assert_send<T: Send>() {}
        assert_send::<Session>();
    }

    #[test]
    fn session_key_parse_error_zero() {
        const INPUT: &str = "AAAAAAAAAAAAAAAAAAAAAA";
        let result = SessionKey::decode(INPUT);
        assert!(
            matches!(result, Err(ParseSessionKeyError::Zero)),
            "expected decoding to fail with `ParseIdError::Zero`"
        );
    }

    impl Arbitrary for SessionKey {
        fn arbitrary(g: &mut quickcheck::Gen) -> Self {
            SessionKey::from(NonZeroU128::arbitrary(g))
        }
    }

    quickcheck! {
        fn session_key_encode_decode(id: SessionKey) -> bool {
            let encoded = id.encode();
            let decoded = SessionKey::decode(&encoded).unwrap();
            id == decoded
        }
    }

    #[test]
    fn session_key_debug_redacts_contents() {
        let s = SessionKey::generate();
        assert_eq!(
            format!("{:?}", s),
            concat!(stringify!(SessionKey), "([REDACTED])")
        );
    }
}
