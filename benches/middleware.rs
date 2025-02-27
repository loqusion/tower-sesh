use std::{sync::Arc, time::Duration};

use axum::{body::Body, routing, Router};
use divan::black_box;
use http::{header, Request};
use tower::ServiceExt;
use tower_sesh::{store::MemoryStore, Session, SessionLayer};
use tower_sesh_core::{time::now, SessionKey, SessionStore};

use build_single_rt as build_rt;

const THREADS: &[usize] = &[0, 1, 2, 4, 8, 16];

trait SessionStoreInit<T>: SessionStore<T> {
    fn init() -> Self;
}

impl<T> SessionStoreInit<T> for MemoryStore<T>
where
    T: Clone + Send + Sync + 'static,
{
    fn init() -> Self {
        MemoryStore::new()
    }
}

#[derive(Clone)]
struct SimpleSessionData {
    #[allow(dead_code)]
    name: String,
}

impl SimpleSessionData {
    fn sample() -> Self {
        Self {
            name: "Hello, World!".to_owned(),
        }
    }
}

fn main() {
    divan::main();
}

#[divan::bench(threads = THREADS)]
fn control(bencher: divan::Bencher) {
    let rt = build_rt();

    bencher.bench(|| {
        rt.block_on(async {});
    });
}

#[divan::bench_group(threads = THREADS)]
mod baseline {
    use super::*;

    #[divan::bench(
        name = "tower-sesh",
        types = [MemoryStore<SimpleSessionData>]
    )]
    fn tower_sesh<S: SessionStoreInit<SimpleSessionData>>(bencher: divan::Bencher) {
        let rt = build_rt();
        let store = Arc::new(S::init());
        let layer = SessionLayer::plain(store);

        async fn handler() {}

        let app = Router::new().route("/", routing::get(handler)).layer(layer);
        let request = || Request::builder().uri("/").body(Body::empty()).unwrap();

        bencher
            .with_inputs(|| (app.clone(), request()))
            .bench_values(|(app, request)| {
                rt.block_on(async move {
                    app.oneshot(request).await.unwrap();
                });
            });
    }
}

#[divan::bench_group(threads = THREADS)]
mod extractor_no_load {
    use super::*;

    #[divan::bench(
        name = "tower-sesh",
        types = [MemoryStore<SimpleSessionData>]
    )]
    fn tower_sesh<S: SessionStoreInit<SimpleSessionData>>(bencher: divan::Bencher) {
        let rt = build_rt();
        let store = Arc::new(S::init());
        let layer = SessionLayer::plain(store);

        async fn handler(_session: Session<SimpleSessionData>) {}

        let app = Router::new().route("/", routing::get(handler)).layer(layer);
        let request = || Request::builder().uri("/").body(Body::empty()).unwrap();

        bencher
            .with_inputs(|| (app.clone(), request()))
            .bench_values(|(app, request)| {
                rt.block_on(async move {
                    app.oneshot(request).await.unwrap();
                });
            });
    }
}

#[divan::bench_group(threads = THREADS)]
mod load_and_use {
    use super::*;

    #[divan::bench(
        name = "tower-sesh",
        types = [MemoryStore<SimpleSessionData>]
    )]
    fn tower_sesh<S: SessionStoreInit<SimpleSessionData>>(bencher: divan::Bencher) {
        let rt = build_rt();
        let store = Arc::new(S::init());
        let layer = SessionLayer::plain(Arc::clone(&store)).cookie_name("id");

        let key = SessionKey::try_from(1).unwrap();
        rt.block_on(store.update(
            &key,
            &SimpleSessionData::sample(),
            now() + Duration::from_secs(10),
        ))
        .unwrap();

        async fn handler(session: Session<SimpleSessionData>) {
            let mut data = session.get();
            black_box(&mut *data);
        }

        let app = Router::new().route("/", routing::get(handler)).layer(layer);
        let request = || {
            Request::builder()
                .uri("/")
                .header(header::COOKIE, format!("id={}", key.encode()))
                .body(Body::empty())
                .unwrap()
        };

        bencher
            .with_inputs(|| (app.clone(), request()))
            .bench_values(|(app, request)| {
                rt.block_on(async move {
                    app.oneshot(request).await.unwrap();
                });
            });
    }
}

#[divan::bench_group(threads = THREADS)]
mod load_and_update {
    use super::*;

    #[divan::bench(
        name = "tower-sesh",
        types = [MemoryStore<SimpleSessionData>]
    )]
    fn tower_sesh<S: SessionStoreInit<SimpleSessionData>>(bencher: divan::Bencher) {
        let rt = build_rt();
        let store = Arc::new(S::init());
        let layer = SessionLayer::plain(Arc::clone(&store)).cookie_name("id");

        let key = SessionKey::try_from(1).unwrap();
        rt.block_on(store.update(
            &key,
            &SimpleSessionData::sample(),
            now() + Duration::from_secs(10),
        ))
        .unwrap();

        async fn handler(session: Session<SimpleSessionData>) {
            session.insert(SimpleSessionData::sample());
        }

        let app = Router::new().route("/", routing::get(handler)).layer(layer);
        let request = || {
            Request::builder()
                .uri("/")
                .header(header::COOKIE, format!("id={}", key.encode()))
                .body(Body::empty())
                .unwrap()
        };

        bencher
            .with_inputs(|| (app.clone(), request()))
            .bench_values(|(app, request)| {
                rt.block_on(async move {
                    app.oneshot(request).await.unwrap();
                });
            });
    }
}

#[divan::bench_group(threads = THREADS)]
mod create {
    use super::*;

    #[divan::bench(
        name = "tower-sesh",
        types = [MemoryStore<SimpleSessionData>]
    )]
    fn tower_sesh<S: SessionStoreInit<SimpleSessionData>>(bencher: divan::Bencher) {
        let rt = build_rt();
        let store = Arc::new(S::init());
        let layer = SessionLayer::plain(Arc::clone(&store)).cookie_name("id");

        async fn handler(session: Session<SimpleSessionData>) {
            session.insert(SimpleSessionData::sample());
        }

        let app = Router::new().route("/", routing::get(handler)).layer(layer);
        let request = || Request::builder().uri("/").body(Body::empty()).unwrap();

        bencher
            .with_inputs(|| (app.clone(), request()))
            .bench_values(|(app, request)| {
                rt.block_on(async move {
                    app.oneshot(request).await.unwrap();
                });
            });
    }
}

#[allow(dead_code)]
fn build_single_rt() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed building the Runtime")
}

#[allow(dead_code)]
fn build_multi_rt() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed building the Runtime")
}
