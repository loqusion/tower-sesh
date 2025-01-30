use std::{error::Error as StdError, fmt, num::NonZeroU128};

use base64::Engine;
use rand::{CryptoRng, Rng};

/// A 128-bit session identifier.
// `NonZeroU128` is used so that `Option<SessionKey>` has the same size as
// `SessionKey`
#[derive(Clone, Hash, PartialEq, Eq)]
pub struct SessionKey(NonZeroU128);

impl fmt::Debug for SessionKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SessionKey([REDACTED])")
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

    /// Returns a random [`SessionKey`], generated from [`ThreadRng`].
    ///
    /// Alternatively, you may wish to use [`generate_from_rng`] and pass your
    /// own CSPRNG. See `ThreadRng`'s documentation for notes on security.
    ///
    /// [`ThreadRng`]: rand::rngs::ThreadRng
    /// [`generate_from_rng`]: SessionKey::generate_from_rng
    #[must_use]
    pub fn generate() -> SessionKey {
        SessionKey::generate_from_rng(&mut rand::thread_rng())
    }

    /// Returns a random [`SessionKey`], generated from `rng`.
    ///
    /// Alternatively, you may wish to use [`generate`]. See its documentation
    /// for more.
    ///
    /// [`generate`]: SessionKey::generate
    #[must_use]
    // TODO: Is `: CryptoRng + Rng` problematic if a different version of `rand` is used?
    pub fn generate_from_rng<R: CryptoRng + Rng>(rng: &mut R) -> SessionKey {
        SessionKey(rng.gen())
    }

    /// Encodes this session key as a URL-safe Base64 string with no padding.
    ///
    /// The returned string uses the URL-safe and filename-safe alphabet (with
    /// `-` and `_`) specified in [RFC 4648].
    ///
    /// [RFC 4648]: https://datatracker.ietf.org/doc/html/rfc4648#section-5
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

impl SessionKey {
    // Not public API. Only tests use this.
    #[doc(hidden)]
    #[inline]
    pub fn from_non_zero_u128(value: NonZeroU128) -> SessionKey {
        SessionKey(value)
    }

    // Not public API. Only tests use this.
    #[doc(hidden)]
    #[inline]
    pub fn try_from_u128(value: u128) -> Result<SessionKey, std::num::TryFromIntError> {
        value.try_into().map(SessionKey::from_non_zero_u128)
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
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
    use super::*;
    use quickcheck::{quickcheck, Arbitrary};

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
            SessionKey::from_non_zero_u128(NonZeroU128::arbitrary(g))
        }
    }

    quickcheck! {
        fn encode_decode(id: SessionKey) -> bool {
            let encoded = id.encode();
            let decoded = SessionKey::decode(&encoded).unwrap();
            id == decoded
        }
    }

    #[test]
    fn debug_redacts_content() {
        let s = SessionKey::generate();
        assert_eq!(format!("{:?}", s), "SessionKey([REDACTED])");
    }
}
