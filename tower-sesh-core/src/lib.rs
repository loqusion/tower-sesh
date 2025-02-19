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
/// Default TTL for a session, in seconds.
pub const DEFAULT_SESSION_EXPIRY_SECONDS: u32 = 2 * WEEK_IN_SECONDS;
