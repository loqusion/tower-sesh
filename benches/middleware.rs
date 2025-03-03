#![allow(clippy::disallowed_types)]

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use axum::{body::Body, routing, Router};
use divan::black_box;
use http::{header, Request};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use time::{Date, Month, OffsetDateTime, Time};
use tower::ServiceExt;
use tower_sesh::{Session, SessionLayer};
use tower_sesh_core::{time::now, SessionKey};

use build_multi_rt as build_rt;
use common::{tower_sesh_impl, tower_sessions_impl};

const THREADS: &[usize] = &[0, 1, 2, 4, 8, 16];

const NUM_KEYS: usize = 3000;

const NUM_KEYS_ERROR_MESSAGE: &str = "\
    `NUM_KEYS` is not large enough to cover all iterations\n\
    lower the iteration count with `sample_count` or `sample_size`, or increase `NUM_KEYS`\
";

mod common {
    use std::hash::Hash;

    use dashmap::DashMap;

    /// A common store implementation used for both `tower-sesh` and `tower-sessions`
    /// so we can get a fair comparison
    pub struct MemoryStore<K, V> {
        map: DashMap<K, V>,
    }

    impl<K, V> std::fmt::Debug for MemoryStore<K, V> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("MemoryStore { .. }")
        }
    }

    /// This is never actually used: both `tower-sesh` and `tower-sessions` keep
    /// their stores behind an `Arc` and delegate to `Arc::clone`. This is only
    /// necessary because `tower-sessions`'s `SessionManagerLayer` uses
    /// `#[derive(Clone)]`.
    impl<K, V> Clone for MemoryStore<K, V> {
        fn clone(&self) -> Self {
            unimplemented!()
        }
    }

    impl<K, V> MemoryStore<K, V>
    where
        K: Eq + Hash,
    {
        pub fn new() -> Self {
            MemoryStore {
                map: DashMap::new(),
            }
        }
    }

    pub mod tower_sesh_impl {
        use async_trait::async_trait;
        use tower_sesh_core::{
            store::{Error, Record, SessionStoreImpl},
            SessionKey, SessionStore, Ttl,
        };

        use crate::time_now;

        use super::MemoryStore as MemoryStoreBase;

        pub type MemoryStore<T> = MemoryStoreBase<SessionKey, Record<T>>;

        type Result<T, E = Error> = std::result::Result<T, E>;

        pub trait SessionStoreInit<T>: SessionStore<T> {
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

        impl<T> SessionStore<T> for MemoryStore<T> where T: Clone + Send + Sync + 'static {}

        #[async_trait]
        impl<T> SessionStoreImpl<T> for MemoryStore<T>
        where
            T: Clone + Send + Sync + 'static,
        {
            async fn create(&self, data: &T, ttl: Ttl) -> Result<SessionKey> {
                let record = Record::new(data.clone(), ttl);

                let session_key = rand::random::<SessionKey>();
                match self.map.entry(session_key.clone()) {
                    dashmap::Entry::Vacant(entry) => {
                        entry.insert(record);
                        Ok(session_key)
                    }
                    dashmap::Entry::Occupied(_) => {
                        unreachable!(
                            "collisions are not included in benchmarks; \
                            try running benchmarks again"
                        )
                    }
                }
            }

            async fn load(&self, session_key: &SessionKey) -> Result<Option<Record<T>>> {
                Ok(self
                    .map
                    .get(session_key)
                    .as_deref()
                    .cloned()
                    .filter(|record| record.ttl >= time_now()))
            }

            async fn update(&self, session_key: &SessionKey, data: &T, ttl: Ttl) -> Result<()> {
                let record = Record::new(data.clone(), ttl);
                self.map.insert(session_key.clone(), record);
                Ok(())
            }

            async fn update_ttl(&self, session_key: &SessionKey, ttl: Ttl) -> Result<()> {
                if let Some(mut record) = self.map.get_mut(session_key) {
                    record.ttl = ttl;
                }
                Ok(())
            }

            async fn delete(&self, session_key: &SessionKey) -> Result<()> {
                self.map.remove(session_key);
                Ok(())
            }
        }
    }

    pub mod tower_sessions_impl {
        use async_trait::async_trait;
        use tower_sessions::{
            session::{Id, Record},
            session_store::Result,
            SessionStore,
        };

        use crate::time_now;

        use super::MemoryStore as MemoryStoreBase;

        pub type MemoryStore = MemoryStoreBase<Id, Record>;

        pub trait SessionStoreInit: SessionStore + Clone {
            fn init() -> Self;
        }

        impl SessionStoreInit for MemoryStore {
            fn init() -> Self {
                MemoryStore::new()
            }
        }

        #[async_trait]
        impl SessionStore for MemoryStore {
            async fn create(&self, session_record: &mut Record) -> Result<()> {
                match self.map.entry(session_record.id) {
                    dashmap::Entry::Vacant(entry) => {
                        entry.insert(session_record.clone());
                        Ok(())
                    }
                    dashmap::Entry::Occupied(_) => {
                        unreachable!(
                            "collisions are not included in benchmarks; \
                            try running benchmarks again"
                        )
                    }
                }
            }

            async fn save(&self, session_record: &Record) -> Result<()> {
                self.map.insert(session_record.id, session_record.clone());
                Ok(())
            }

            async fn load(&self, session_id: &Id) -> Result<Option<Record>> {
                Ok(self
                    .map
                    .get(session_id)
                    .as_deref()
                    .cloned()
                    .filter(|record| record.expiry_date >= time_now()))
            }

            async fn delete(&self, session_id: &Id) -> Result<()> {
                self.map.remove(session_id);
                Ok(())
            }
        }
    }
}

type DbId = u64;

#[derive(Clone, Debug, Deserialize, Serialize)]
struct SessionData {
    user_id: DbId,
    authenticated: bool,
    roles: Vec<String>,
    preferences: Preferences,
    cart: Vec<CartItem>,
    csrf_token: String,
    flash_messages: Vec<String>,
    rate_limit: RateLimit,
    workflow_state: WorkflowState,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Preferences {
    theme: Theme,
    language: Language,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
enum Theme {
    Light,
    Dark,
}

/// The two languages
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
enum Language {
    #[serde(alias = "en-US")]
    EnUs,
    #[serde(alias = "en-GB")]
    EnGb,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct CartItem {
    item_id: DbId,
    name: String,
    quantity: u64,
    price: Decimal,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RateLimit {
    failed_login_attempts: u64,
    #[serde(with = "time::serde::rfc3339")]
    last_attempt: OffsetDateTime,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct WorkflowState {
    step: u64,
    total_steps: u64,
    data: WorkflowData,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct WorkflowData {
    address: String,
}

impl SessionData {
    /// Key for the `tower-sessions` hash map.
    ///
    /// See https://docs.rs/tower-sessions/latest/tower_sessions/#strongly-typed-sessions
    const KEY: &str = "data";

    fn sample() -> Self {
        SessionData {
            user_id: 12345,
            authenticated: true,
            roles: vec!["admin".to_owned(), "editor".to_owned()],
            preferences: Preferences {
                theme: Theme::Dark,
                language: Language::EnUs,
            },
            cart: vec![
                CartItem {
                    item_id: 101,
                    name: "Laptop".to_owned(),
                    quantity: 1,
                    price: Decimal::new(99999, 2),
                },
                CartItem {
                    item_id: 202,
                    name: "Mouse".to_owned(),
                    quantity: 2,
                    price: Decimal::new(2550, 2),
                },
            ],
            csrf_token: "abc123xyz".to_owned(),
            flash_messages: vec![
                "Welcome back!".to_owned(),
                "Your order has been placed successfully.".to_owned(),
            ],
            rate_limit: RateLimit {
                failed_login_attempts: 1,
                last_attempt: OffsetDateTime::new_utc(
                    Date::from_calendar_date(2025, Month::February, 28).unwrap(),
                    Time::from_hms(0, 34, 56).unwrap(),
                ),
            },
            workflow_state: WorkflowState {
                step: 2,
                total_steps: 5,
                data: WorkflowData {
                    address: "123 Main St, NY".to_owned(),
                },
            },
        }
    }

    fn modify(&mut self) {
        self.workflow_state.step = self.workflow_state.step.wrapping_add(1);
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
        types = [tower_sesh_impl::MemoryStore<SessionData>]
    )]
    fn tower_sesh<S: tower_sesh_impl::SessionStoreInit<SessionData>>(bencher: divan::Bencher) {
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

    #[divan::bench(
        name = "tower-sessions",
        types = [tower_sessions_impl::MemoryStore]
    )]
    fn tower_sessions<S: tower_sessions_impl::SessionStoreInit>(bencher: divan::Bencher) {
        let rt = build_rt();
        let store = S::init();
        let layer = tower_sessions::SessionManagerLayer::new(store);

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
        types = [tower_sesh_impl::MemoryStore<SessionData>]
    )]
    fn tower_sesh<S: tower_sesh_impl::SessionStoreInit<SessionData>>(bencher: divan::Bencher) {
        let rt = build_rt();
        let store = Arc::new(S::init());
        let layer = SessionLayer::plain(store);

        async fn handler(_session: Session<SessionData>) {}

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

    #[divan::bench(
        name = "tower-sessions",
        types = [tower_sessions_impl::MemoryStore]
    )]
    fn tower_sessions<S: tower_sessions_impl::SessionStoreInit>(bencher: divan::Bencher) {
        let rt = build_rt();
        let store = S::init();
        let layer = tower_sessions::SessionManagerLayer::new(store);

        async fn handler(_session: tower_sessions::Session) {}

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
        types = [tower_sesh_impl::MemoryStore<SessionData>]
    )]
    fn tower_sesh<S: tower_sesh_impl::SessionStoreInit<SessionData>>(bencher: divan::Bencher) {
        let rt = build_rt();
        let store = Arc::new(S::init());
        let layer = SessionLayer::plain(Arc::clone(&store)).cookie_name("id");

        let keys = keys();
        for key in &keys {
            rt.block_on(store.update(key, &SessionData::sample(), now() + Duration::from_secs(10)))
                .unwrap();
        }

        async fn handler(session: Session<SessionData>) {
            let data = session.get();
            black_box(&*data);
        }

        let app = Router::new().route("/", routing::get(handler)).layer(layer);
        let keys_iter = MutexIter::new(keys.into_iter());
        let request = || {
            Request::builder()
                .uri("/")
                .header(
                    header::COOKIE,
                    format!(
                        "id={}",
                        keys_iter.next().expect(NUM_KEYS_ERROR_MESSAGE).encode()
                    ),
                )
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

    #[allow(clippy::to_string_in_format_args)]
    #[divan::bench(
        name = "tower-sessions",
        types = [tower_sessions_impl::MemoryStore]
    )]
    fn tower_sessions<S: tower_sessions_impl::SessionStoreInit>(bencher: divan::Bencher) {
        let rt = build_rt();
        let store = S::init();

        let ids = ids();
        for id in &ids {
            rt.block_on(store.save(&tower_sessions::session::Record {
                id: *id,
                data: HashMap::from([(
                    SessionData::KEY.to_owned(),
                    serde_json::to_value(SessionData::sample()).unwrap(),
                )]),
                expiry_date: time_now() + Duration::from_secs(100),
            }))
            .unwrap();
        }

        let layer = tower_sessions::SessionManagerLayer::new(store).with_name("id");

        async fn handler(session: tower_sessions::Session) {
            let data = session.get::<SessionData>(SessionData::KEY).await.unwrap();
            black_box(&data);
        }

        let app = Router::new().route("/", routing::get(handler)).layer(layer);
        let ids_iter = MutexIter::new(ids.into_iter());
        let request = || {
            Request::builder()
                .uri("/")
                .header(
                    header::COOKIE,
                    format!(
                        "id={}",
                        ids_iter.next().expect(NUM_KEYS_ERROR_MESSAGE).to_string()
                    ),
                )
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
        types = [tower_sesh_impl::MemoryStore<SessionData>]
    )]
    fn tower_sesh<S: tower_sesh_impl::SessionStoreInit<SessionData>>(bencher: divan::Bencher) {
        let rt = build_rt();
        let store = Arc::new(S::init());
        let layer = SessionLayer::plain(Arc::clone(&store)).cookie_name("id");

        let keys = keys();
        for key in &keys {
            rt.block_on(store.update(key, &SessionData::sample(), now() + Duration::from_secs(10)))
                .unwrap();
        }

        async fn handler(session: Session<SessionData>) {
            let mut data = session.get_or_insert_with(SessionData::sample);
            // For parity with `tower-sessions`
            data.modify();
        }

        let app = Router::new().route("/", routing::get(handler)).layer(layer);
        let keys_iter = MutexIter::new(keys.into_iter());
        let request = || {
            Request::builder()
                .uri("/")
                .header(
                    header::COOKIE,
                    format!(
                        "id={}",
                        keys_iter.next().expect(NUM_KEYS_ERROR_MESSAGE).encode()
                    ),
                )
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

    #[allow(clippy::to_string_in_format_args)]
    #[divan::bench(
        name = "tower-sessions",
        types = [tower_sessions_impl::MemoryStore]
    )]
    fn tower_sessions<S: tower_sessions_impl::SessionStoreInit>(bencher: divan::Bencher) {
        let rt = build_rt();
        let store = S::init();

        let ids = ids();
        for id in &ids {
            rt.block_on(store.save(&tower_sessions::session::Record {
                id: *id,
                data: HashMap::from([(
                    SessionData::KEY.to_owned(),
                    serde_json::to_value(SessionData::sample()).unwrap(),
                )]),
                expiry_date: time_now() + Duration::from_secs(10),
            }))
            .unwrap();
        }

        let layer = tower_sessions::SessionManagerLayer::new(store).with_name("id");

        async fn handler(session: tower_sessions::Session) {
            let mut data = session
                .get::<SessionData>(SessionData::KEY)
                .await
                .unwrap()
                .unwrap();
            // We must actually modify the data, or else it will not register as modified
            data.modify();
            session.insert(SessionData::KEY, data).await.unwrap();
        }

        let app = Router::new().route("/", routing::get(handler)).layer(layer);
        let ids_iter = MutexIter::new(ids.into_iter());
        let request = || {
            Request::builder()
                .uri("/")
                .header(
                    header::COOKIE,
                    format!(
                        "id={}",
                        ids_iter.next().expect(NUM_KEYS_ERROR_MESSAGE).to_string()
                    ),
                )
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
        types = [tower_sesh_impl::MemoryStore<SessionData>]
    )]
    fn tower_sesh<S: tower_sesh_impl::SessionStoreInit<SessionData>>(bencher: divan::Bencher) {
        let rt = build_rt();
        let store = Arc::new(S::init());
        let layer = SessionLayer::plain(Arc::clone(&store)).cookie_name("id");

        async fn handler(session: Session<SessionData>) {
            session.insert(SessionData::sample());
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

    #[divan::bench(
        name = "tower-sessions",
        types = [tower_sessions_impl::MemoryStore]
    )]
    fn tower_sessions<S: tower_sessions_impl::SessionStoreInit>(bencher: divan::Bencher) {
        let rt = build_rt();
        let store = S::init();
        let layer = tower_sessions::SessionManagerLayer::new(store);

        async fn handler(session: tower_sessions::Session) {
            session
                .insert(SessionData::KEY, SessionData::sample())
                .await
                .unwrap();
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

#[inline]
fn time_now() -> OffsetDateTime {
    OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc())
}

fn keys() -> Vec<SessionKey> {
    (1..=NUM_KEYS.try_into().unwrap())
        .map(|n| SessionKey::try_from(n).unwrap())
        .collect()
}

fn ids() -> Vec<tower_sessions::session::Id> {
    (1..=NUM_KEYS.try_into().unwrap())
        .map(tower_sessions::session::Id)
        .collect()
}

struct MutexIter<I> {
    iter: Mutex<I>,
}

impl<I, T> MutexIter<I>
where
    I: Iterator<Item = T>,
{
    fn new(iter: I) -> MutexIter<I> {
        let iter = Mutex::new(iter);
        MutexIter { iter }
    }

    #[track_caller]
    fn next(&self) -> Option<T> {
        self.iter.lock().unwrap().next()
    }
}
