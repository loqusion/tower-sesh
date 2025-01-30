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
    pub const ENCODED_LEN: usize = 22;
    const DECODED_LEN: usize = 16;

    #[must_use]
    pub fn generate() -> SessionKey {
        SessionKey::generate_from_rng(&mut rand::thread_rng())
    }

    #[must_use]
    pub fn generate_from_rng<R: CryptoRng + Rng>(rng: &mut R) -> SessionKey {
        SessionKey(rng.gen())
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

#[derive(Debug)]
pub enum ParseSessionKeyError {
    Base64(base64::DecodeSliceError),
    Zero,
}

impl StdError for ParseSessionKeyError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            ParseSessionKeyError::Base64(err) => Some(err),
            ParseSessionKeyError::Zero => None,
        }
    }
}

impl fmt::Display for ParseSessionKeyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseSessionKeyError::Base64(_err) => f.write_str("failed to parse base64 string"),
            ParseSessionKeyError::Zero => f.write_str("session id must be non-zero"),
        }
    }
}

impl From<base64::DecodeSliceError> for ParseSessionKeyError {
    fn from(value: base64::DecodeSliceError) -> Self {
        ParseSessionKeyError::Base64(value)
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
            matches!(result, Err(ParseSessionKeyError::Zero)),
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
