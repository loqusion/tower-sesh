//! Core of `tower-sesh`.
//!
//! # Warning
//!
//! The API of this crate is not meant for general use and does _not_ follow
//! Semantic Versioning. If you are building a custom session store, you should
//! pin an exact version of `tower-sesh-core` to avoid breakages:
//!
//! ```toml
//! tower-sesh-core = { version = "=X.Y.Z" }
//! ```
//!
//! And then keep releases in sync with `tower-sesh-core`.

#[doc(inline)]
pub use crate::key::SessionKey;
#[doc(inline)]
pub use crate::store::{Record, SessionStore};

pub mod key;
pub mod store;

const WEEK_IN_SECONDS: u32 = 60 * 60 * 24 * 7;
/// Default expiry offset for a session, in seconds.
pub const DEFAULT_SESSION_EXPIRY_SECONDS: u32 = 2 * WEEK_IN_SECONDS;

/// Returns the current date and time with the local system's UTC offset.
///
/// If the system's UTC offset could not be found, then [`now_utc`] is used
/// instead.
///
/// [`now_utc`]: store::Ttl::now_utc
pub fn now() -> store::Ttl {
    store::Ttl::now_local().unwrap_or_else(|_| store::Ttl::now_utc())
}
