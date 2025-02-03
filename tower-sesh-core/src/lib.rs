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
