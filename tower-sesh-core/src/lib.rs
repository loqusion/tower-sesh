//! Core of `tower-sesh`.
//!
//! # Warning
//!
//! The API of this crate is not meant for general use and does _not_ follow
//! Semantic Versioning. If you are building a custom session store, you should
//! pin an exact version of `tower-sesh-core` to avoid breakages:
//!
//! ```not_rust
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

// Not public API. Meant to discourage implementing `SessionStore` to avoid
// breaking changes in dependent crates.
#[doc(hidden)]
pub mod __private {
    pub trait Sealed {}
}
