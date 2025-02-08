#![cfg_attr(docsrs, feature(doc_auto_cfg))]

//! A Tower middleware for strongly typed, efficient sessions.
//!
//! **🚧 UNDER CONSTRUCTION 🚧**
//!
//! This crate is being actively developed. Its public API is open to change at
//! any time.
//!
//! To track development of this crate, visit its [GitHub repository].
//!
//! [GitHub repository]: https://github.com/loqusion/tower-sesh

#[doc(inline)]
pub use middleware::SessionLayer;
#[doc(inline)]
pub use session::Session;
#[doc(inline)]
pub use value::Value;

#[macro_use]
mod macros;

pub mod config;
pub mod middleware;
pub mod session;
pub mod store;
pub mod value;

mod util;
