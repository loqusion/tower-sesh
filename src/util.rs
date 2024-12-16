use std::{error::Error, fmt};

pub trait ErrorExt {
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

pub struct DisplayChain<'a> {
    inner: &'a (dyn Error + 'static),
}

impl fmt::Display for DisplayChain<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.inner)?;

        for error in anyhow::Chain::new(self.inner).skip(1) {
            write!(f, ": {}", error)?;
        }

        Ok(())
    }
}
