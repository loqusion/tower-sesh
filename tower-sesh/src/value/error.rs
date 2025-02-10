// Adapted from https://github.com/serde-rs/json.

use std::{error::Error as StdError, fmt};

pub struct Error {
    // TODO: Compare benchmarks when `err` is boxed.
    err: ErrorImpl,
}

pub(super) enum ErrorImpl {
    /// Catchall for a general error.
    Message(Box<str>),

    /// Map key is a non-finite float value.
    FloatKeyMustBeFinite,

    /// Float is a non-finite value.
    FloatMustBeFinite,

    /// Map key is not a string.
    KeyMustBeAString,

    /// Number is bigger than the maximum value of its type.
    NumberOutOfRange,
}

impl From<ErrorImpl> for Error {
    #[inline]
    fn from(err: ErrorImpl) -> Self {
        Error { err }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        None
    }
}

impl fmt::Display for ErrorImpl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use ErrorImpl::*;
        match self {
            Message(msg) => f.write_str(msg),
            FloatKeyMustBeFinite => f.write_str("float key must be finite (got NaN or +/-inf)"),
            FloatMustBeFinite => f.write_str("float must be finite (got NaN or +/-inf)"),
            KeyMustBeAString => f.write_str("key must be a string"),
            NumberOutOfRange => f.write_str("number out of range"),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.err, f)
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Error({:?})", self.err.to_string())
    }
}

impl serde::de::Error for Error {
    #[cold]
    fn custom<T>(msg: T) -> Self
    where
        T: fmt::Display,
    {
        make_error(msg.to_string())
    }
}

impl serde::ser::Error for Error {
    #[cold]
    fn custom<T>(msg: T) -> Self
    where
        T: std::fmt::Display,
    {
        make_error(msg.to_string())
    }
}

fn make_error(msg: String) -> Error {
    Error {
        err: ErrorImpl::Message(msg.into_boxed_str()),
    }
}
