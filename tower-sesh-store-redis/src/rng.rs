//! Types related to the `rand` crate.

use rand::{CryptoRng, RngCore, TryCryptoRng, TryRngCore};

/// A marker type indicating that `RngStore` should use `ThreadRng`.
#[non_exhaustive]
pub struct PhantomThreadRng;

/// All methods are unimplemented.
impl RngCore for PhantomThreadRng {
    fn next_u32(&mut self) -> u32 {
        unreachable!()
    }

    fn next_u64(&mut self) -> u64 {
        unreachable!()
    }

    fn fill_bytes(&mut self, dst: &mut [u8]) {
        let _ = dst;
        unreachable!()
    }
}

impl CryptoRng for PhantomThreadRng {}

/// Wrapper around [`TryRngCore`] implementation which implements [`RngCore`]
/// by panicking on potential errors.
///
/// Unlike [`UnwrapErr`], this mutably borrows an RNG.
///
/// [`UnwrapErr`]: https://docs.rs/rand_core/0.9.0/rand_core/struct.UnwrapErr.html
#[derive(Debug)]
pub(crate) struct UnwrapErrBorrowed<'a, R: TryRngCore>(pub &'a mut R);

impl<R: TryRngCore> RngCore for UnwrapErrBorrowed<'_, R> {
    fn next_u32(&mut self) -> u32 {
        self.0.try_next_u32().unwrap()
    }

    fn next_u64(&mut self) -> u64 {
        self.0.try_next_u64().unwrap()
    }

    fn fill_bytes(&mut self, dst: &mut [u8]) {
        self.0.try_fill_bytes(dst).unwrap()
    }
}

impl<R: TryCryptoRng> CryptoRng for UnwrapErrBorrowed<'_, R> {}
