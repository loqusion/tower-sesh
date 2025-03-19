//! Utilities.

use std::{error::Error, fmt, iter};

/// An error reporter that prints an error and its sources.
///
/// This is a minimal port of the [`Report`][nightly-report] struct in nightly
/// Rust.
///
/// [nightly-report]: std::error::Report
pub struct Report<E = Box<dyn Error>> {
    /// The error being reported.
    error: E,
}

impl<E> Report<E>
where
    Report<E>: From<E>,
{
    /// Creates a new `Report` from an input error.
    pub fn new(error: E) -> Report<E> {
        Self::from(error)
    }
}

impl<E> From<E> for Report<E>
where
    E: Error,
{
    fn from(error: E) -> Self {
        Report { error }
    }
}

impl<E> fmt::Display for Report<E>
where
    E: Error,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.error)?;

        let sources = iter::successors(self.error.source(), |err| (*err).source());

        for cause in sources {
            write!(f, ": {cause}")?;
        }

        Ok(())
    }
}

impl<E> fmt::Debug for Report<E>
where
    Report<E>: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
