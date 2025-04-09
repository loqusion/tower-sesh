pub mod middleware;
pub use middleware::*;
pub mod store;
pub use store::*;

use tower_sesh_core::{store::SessionStoreRng, SessionStore};

use crate::support::{SessionData, TestRng};

pub async fn test_smoke(_store: impl SessionStore<SessionData> + SessionStoreRng<TestRng>) {}
