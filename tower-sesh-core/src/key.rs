//! `SessionKey` and related items.

use std::{
    error::Error as StdError,
    fmt,
    num::{NonZeroU128, TryFromIntError},
};

use base64::Engine;
use rand::distr::{Distribution, StandardUniform};

/// A 128-bit session identifier.
// `NonZeroU128` is used so that `Option<SessionKey>` has the same size as
// `SessionKey`
#[derive(Clone, Hash, PartialEq, Eq)]
pub struct SessionKey(NonZeroU128);

/// Debug implementation does not leak secret.
impl fmt::Debug for SessionKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SessionKey(..)")
    }
}

impl SessionKey {
    const BASE64_ENGINE: base64::engine::GeneralPurpose =
        base64::engine::general_purpose::URL_SAFE_NO_PAD;

    /// Length of a Base64 string returned by the [`encode`] method.
    ///
    /// [`encode`]: SessionKey::encode
    pub const ENCODED_LEN: usize = 22;

    /// Length of output from decoding a Base64-encoded session key string with
    /// the [`decode`] method.
    ///
    /// [`decode`]: SessionKey::decode
    const DECODED_LEN: usize = 16;

    /// Encodes this session key as a URL-safe Base64 string with no padding.
    ///
    /// The returned string uses the URL-safe and filename-safe alphabet (with
    /// `-` and `_`) specified in [RFC 4648].
    ///
    /// [RFC 4648]: https://datatracker.ietf.org/doc/html/rfc4648#section-5
    #[inline]
    #[must_use]
    pub fn encode(&self) -> String {
        SessionKey::BASE64_ENGINE.encode(self.0.get().to_le_bytes())
    }

    /// Decodes a session key string encoded with the URL-safe Base64 alphabet
    /// specified in [RFC 4648]. There must be no padding present in the input.
    ///
    /// [RFC 4648]: https://datatracker.ietf.org/doc/html/rfc4648#section-5
    pub fn decode<B: AsRef<[u8]>>(b: B) -> Result<SessionKey, DecodeSessionKeyError> {
        fn _decode(b: &[u8]) -> Result<SessionKey, DecodeSessionKeyError> {
            use base64::DecodeError;

            let mut buf = [0; const { SessionKey::DECODED_LEN }];
            SessionKey::BASE64_ENGINE
                .decode_slice(b, &mut buf)
                .and_then(|decoded_len| {
                    if decoded_len == SessionKey::DECODED_LEN {
                        Ok(())
                    } else {
                        Err(DecodeError::InvalidLength(decoded_len).into())
                    }
                })?;

            match u128::from_le_bytes(buf).try_into() {
                Ok(v) => Ok(SessionKey(v)),
                Err(_) => Err(DecodeSessionKeyError::Zero),
            }
        }

        _decode(b.as_ref())
    }
}

impl Distribution<SessionKey> for StandardUniform {
    fn sample<R: rand::Rng + ?Sized>(&self, rng: &mut R) -> SessionKey {
        SessionKey(self.sample(rng))
    }
}

impl From<NonZeroU128> for SessionKey {
    #[inline]
    fn from(value: NonZeroU128) -> Self {
        SessionKey(value)
    }
}

impl TryFrom<u128> for SessionKey {
    type Error = TryFromIntError;

    #[inline]
    fn try_from(value: u128) -> Result<Self, Self::Error> {
        NonZeroU128::try_from(value).map(SessionKey::from)
    }
}

impl From<SessionKey> for NonZeroU128 {
    #[inline]
    fn from(value: SessionKey) -> Self {
        value.0
    }
}

/// The error type returned when decoding a session key fails.
#[derive(Debug)]
pub enum DecodeSessionKeyError {
    Base64(base64::DecodeSliceError),
    Zero,
}

impl StdError for DecodeSessionKeyError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            DecodeSessionKeyError::Base64(err) => Some(err),
            DecodeSessionKeyError::Zero => None,
        }
    }
}

impl fmt::Display for DecodeSessionKeyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DecodeSessionKeyError::Base64(_err) => f.write_str("failed to parse base64 string"),
            DecodeSessionKeyError::Zero => f.write_str("session id must be non-zero"),
        }
    }
}

impl From<base64::DecodeSliceError> for DecodeSessionKeyError {
    fn from(value: base64::DecodeSliceError) -> Self {
        DecodeSessionKeyError::Base64(value)
    }
}

#[cfg(test)]
mod test {
    use std::iter;

    use cookie::{Cookie, CookieJar};
    use quickcheck::{quickcheck, Arbitrary};

    use super::*;

    #[test]
    fn parse_error_zero() {
        const INPUT: &str = "AAAAAAAAAAAAAAAAAAAAAA";
        let result = SessionKey::decode(INPUT);
        assert!(
            matches!(result, Err(DecodeSessionKeyError::Zero)),
            "expected decoding to fail"
        );
    }

    impl Arbitrary for SessionKey {
        fn arbitrary(g: &mut quickcheck::Gen) -> Self {
            SessionKey::from(NonZeroU128::arbitrary(g))
        }
    }

    #[derive(Clone, Debug, PartialEq)]
    struct CookieKey(cookie::Key);

    impl From<CookieKey> for cookie::Key {
        fn from(value: CookieKey) -> Self {
            value.0
        }
    }

    impl Arbitrary for CookieKey {
        fn arbitrary(g: &mut quickcheck::Gen) -> Self {
            let inner = {
                let mut buf = [0u8; 64];
                for (i, num) in iter::repeat_with(|| <u8 as Arbitrary>::arbitrary(g))
                    .take(64)
                    .enumerate()
                {
                    buf[i] = num;
                }
                buf
            };
            CookieKey(cookie::Key::from(&inner))
        }
    }

    const COOKIE_NAME: &str = "id";

    fn cookie_from_key(key: SessionKey) -> Cookie<'static> {
        Cookie::build((COOKIE_NAME, key.encode()))
            .http_only(true)
            .same_site(cookie::SameSite::Strict)
            .secure(true)
            .build()
            .into_owned()
    }

    fn cookie_from_key_encrypted(
        session_key: SessionKey,
        master_key: cookie::Key,
    ) -> Cookie<'static> {
        let cookie = cookie_from_key(session_key);
        let mut jar = CookieJar::new();
        jar.private_mut(&master_key).add(cookie);
        jar.get(COOKIE_NAME)
            .expect("cookie should have been added to jar")
            .to_owned()
    }

    fn cookie_from_key_signed(session_key: SessionKey, master_key: cookie::Key) -> Cookie<'static> {
        let cookie = cookie_from_key(session_key);
        let mut jar = CookieJar::new();
        jar.signed_mut(&master_key).add(cookie);
        jar.get(COOKIE_NAME)
            .expect("cookie should have been added to jar")
            .to_owned()
    }

    quickcheck! {
        fn debug_redacts_content(key: SessionKey) -> bool {
            format!("{:?}", key) == "SessionKey(..)"
        }

        fn encode_decode(key: SessionKey) -> bool {
            let encoded = key.encode();
            let decoded = SessionKey::decode(&encoded).unwrap();
            key == decoded
        }

        fn parsable_in_cookie_header_value_plain_stripped(
            key: SessionKey
        ) -> Result<(), Box<dyn std::error::Error>> {
            let cookie = cookie_from_key(key);
            http::HeaderValue::try_from(cookie.stripped().to_string())
                .and(Ok(()))
                .map_err(Into::into)
        }

        fn parsable_in_cookie_header_value_plain_encoded(
            key: SessionKey
        ) -> Result<(), Box<dyn std::error::Error>> {
            let cookie = cookie_from_key(key);
            http::HeaderValue::try_from(cookie.encoded().to_string())
                .and(Ok(()))
                .map_err(Into::into)
        }

        fn parsable_in_cookie_header_value_encrypted_stripped(
            input: (SessionKey, CookieKey)
        ) -> Result<(), Box<dyn std::error::Error>> {
            let (session_key, master_key) = input;
            let cookie = cookie_from_key_encrypted(session_key, master_key.into());
            http::HeaderValue::try_from(cookie.stripped().to_string())
                .and(Ok(()))
                .map_err(Into::into)
        }

        fn parsable_in_cookie_header_value_encrypted_encoded(
            input: (SessionKey, CookieKey)
        ) -> Result<(), Box<dyn std::error::Error>> {
            let (session_key, master_key) = input;
            let cookie = cookie_from_key_encrypted(session_key, master_key.into());
            http::HeaderValue::try_from(cookie.encoded().to_string())
                .and(Ok(()))
                .map_err(Into::into)
        }

        fn parsable_in_cookie_header_value_signed_stripped(
            input: (SessionKey, CookieKey)
        ) -> Result<(), Box<dyn std::error::Error>> {
            let (session_key, master_key) = input;
            let cookie = cookie_from_key_signed(session_key, master_key.into());
            http::HeaderValue::try_from(cookie.stripped().to_string())
                .and(Ok(()))
                .map_err(Into::into)
        }

        fn parsable_in_cookie_header_value_signed_encoded(
            input: (SessionKey, CookieKey)
        ) -> Result<(), Box<dyn std::error::Error>> {
            let (session_key, master_key) = input;
            let cookie = cookie_from_key_signed(session_key, master_key.into());
            http::HeaderValue::try_from(cookie.encoded().to_string())
                .and(Ok(()))
                .map_err(Into::into)
        }
    }
}
