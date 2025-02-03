#[doc(inline)]
pub use crate::key::SessionKey;
#[doc(inline)]
pub use crate::store::{Record, SessionStore};

pub mod key;
pub mod store;

#[cfg(not(feature = "__private"))]
mod __private {
    pub trait Sealed {}
}

#[cfg(feature = "__private")]
#[cfg_attr(docsrs, doc(hidden))]
pub mod __private {
    pub trait Sealed {}
}
