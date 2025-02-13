use rand::{CryptoRng, RngCore};

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
