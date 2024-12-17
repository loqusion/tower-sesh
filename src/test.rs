use std::sync::atomic::{self, AtomicU64};

use crate::session::SessionKey;

/// A session key that is safe to use in tests without fear of collisions.
///
/// Collisions can cause tests to be flaky, since two tests using the same
/// session key can interact with each other in unexpected ways. For
/// instance, one test can delete the session state of another test and
/// cause a test assertion to fail.
///
/// Actually, a CSPRNG is suitable for this purpose, as collisions for
/// values in the range 1..2^128 are _exceedingly_ rare. Still, the
/// probability of collision is non-zero.
pub fn test_key() -> SessionKey {
    static mut KEY_STATE: AtomicU64 = AtomicU64::new(1);
    let v = unsafe { KEY_STATE.fetch_add(1, atomic::Ordering::SeqCst) as u128 };
    SessionKey::try_from(v).unwrap()
}
