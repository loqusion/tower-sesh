use axum::{routing::method_routing, Router};
use axum_test::TestServer;

use crate::{config::CookieContentSecurity, SessionStore};

pub async fn basic_workflow<Data>(
    store: impl SessionStore<Data>,
    policy: CookieContentSecurity,
) -> anyhow::Result<()> {
    let app = Router::new().route("/test", method_routing::get(|| async { "hi" }));
    let server = TestServer::builder()
        .http_transport()
        .expect_success_by_default()
        .save_cookies()
        .build(app)?;

    Ok(())
}
