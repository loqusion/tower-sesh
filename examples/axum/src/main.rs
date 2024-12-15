//! Run with
//!
//! ```not_rust
//! cargo run -p example-axum
//! ```

use std::sync::Arc;

use axum::{routing::get, Router};
use tower::ServiceBuilder;
use tower_cookies::CookieManagerLayer;
use tower_sesh::{store::MemoryStore, Session, SessionManagerLayer};

#[tokio::main]
async fn main() {
    let store = MemoryStore::new();
    let middleware = ServiceBuilder::new()
        .layer(CookieManagerLayer::new())
        .layer(SessionManagerLayer::new(Arc::new(store)));

    // build our application with a route
    let app = Router::new().route("/", get(handler)).layer(middleware);

    // run it
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    println!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

async fn handler(session: Session) {
    todo!()
}
