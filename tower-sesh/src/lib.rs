#[doc(inline)]
pub use middleware::SessionLayer;
#[doc(inline)]
pub use session::Session;
#[doc(inline)]
pub use value::Value;

// TODO: Remove `cookie` crate from public API if possible
pub use ::cookie;

#[macro_use]
mod macros;

pub mod middleware;
pub mod session;
pub mod store;
pub mod value;

#[doc(hidden)]
pub mod config;
#[cfg(feature = "_test")]
#[doc(hidden)]
pub mod test;

mod util;
