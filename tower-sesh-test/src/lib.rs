use std::time::Duration;

use futures_util::{stream, StreamExt, TryStreamExt};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha20Rng as TestRng;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use time::{Date, Month, OffsetDateTime, Time, UtcDateTime};
use tower_sesh_core::{store::SessionStoreRng, SessionKey, SessionStore, Ttl};

#[doc(hidden)]
pub mod __private {
    pub use paste;
    pub use tokio;
}

#[macro_export]
macro_rules! test_suite {
    ($store:expr) => {
        $crate::test_suite! {
            @impl $store =>
            smoke
            create_does_collision_resolution
            #[cfg_attr(miri, ignore = "strict time requirements are incompatible with miri")]
            loading_session_after_create
            #[cfg_attr(miri, ignore = "strict time requirements are incompatible with miri")]
            loading_session_after_update_nonexisting
            #[cfg_attr(miri, ignore = "strict time requirements are incompatible with miri")]
            loading_session_after_update_existing
            #[cfg_attr(miri, ignore = "strict time requirements are incompatible with miri")]
            loading_session_after_update_ttl
            loading_a_missing_session_returns_none
            loading_an_expired_session_returns_none_create
            loading_an_expired_session_returns_none_update_nonexisting
            loading_an_expired_session_returns_none_update_existing
            loading_an_expired_session_returns_none_update_ttl
            delete_after_create
            delete_after_update
            delete_does_not_error_for_missing_entry
            ttl_with_999_999_999_nanoseconds_create
            ttl_with_999_999_999_nanoseconds_update_nonexisting
            ttl_with_999_999_999_nanoseconds_update_existing
            ttl_with_999_999_999_nanoseconds_update_ttl
        }
    };

    (
        @impl $store:expr =>
        $(
            $(#[$m:meta])*
            $test:ident
        )+
    ) => {
        $(
            $(#[$m])*
            #[$crate::__private::tokio::test]
            async fn $test() {
                $crate::__private::paste::paste! {
                    $crate::[<test_ $test>]($store).await;
                }
            }
        )+
    };
}

pub async fn test_smoke(_store: impl SessionStore<SessionData> + SessionStoreRng<TestRng>) {}

pub async fn test_create_does_collision_resolution(
    mut store: impl SessionStore<SessionData> + SessionStoreRng<TestRng>,
) {
    async fn assert_unique_keys_and_data(
        keys: &[SessionKey],
        store: &impl SessionStore<SessionData>,
    ) {
        let data = stream::iter(keys.iter().map(|key| store.load(key)))
            .buffered(keys.len())
            .map_ok(|record| record.unwrap().data)
            .try_collect::<Vec<_>>()
            .await
            .unwrap();

        for (i, (key1, data1)) in keys.iter().zip(data.iter()).enumerate() {
            for (key2, data2) in keys.iter().zip(data.iter()).skip(i + 1) {
                assert_ne!(key1, key2);
                assert_ne!(data1, data2);
            }
        }
    }

    let test_cases = [1, 2, 3]
        .into_iter()
        .map(SessionData::sample_with)
        .collect::<Vec<_>>();
    let mut keys: Vec<SessionKey> = Vec::new();

    let rng = TestRng::seed_from_u64(4787236816789423423);
    assert_eq!(rng.clone().random::<SessionKey>(), rng.clone().random()); // sanity check

    for data in test_cases.iter() {
        store.rng(rng.clone());
        let created_key = store.create(data, ttl()).await.unwrap();
        keys.push(created_key);

        assert_unique_keys_and_data(&keys, &store).await;
    }
}

pub async fn test_loading_session_after_create(
    mut store: impl SessionStore<SessionData> + SessionStoreRng<TestRng>,
) {
    let rng = TestRng::seed_from_u64(3005911574);
    store.rng(rng);

    let data = SessionData::sample();
    let ttl = ttl_strict();
    let session_key = store.create(&data, ttl).await.unwrap();

    let record = store.load(&session_key).await.unwrap().unwrap();
    assert_eq!(record.data, data);
    assert_eq!(record.ttl.normalize(), ttl.normalize());
}

pub async fn test_loading_session_after_update_nonexisting(
    store: impl SessionStore<SessionData> + SessionStoreRng<TestRng>,
) {
    let mut rng = TestRng::seed_from_u64(2848227658);
    let session_key = rng.random::<SessionKey>();

    let record = store.load(&session_key).await.unwrap();
    assert!(record.is_none());

    let data = SessionData::sample();
    let ttl = ttl_strict();
    store.update(&session_key, &data, ttl).await.unwrap();

    let record = store.load(&session_key).await.unwrap().unwrap();
    assert_eq!(record.data, data);
    assert_eq!(record.ttl.normalize(), ttl.normalize());
}

pub async fn test_loading_session_after_update_existing(
    mut store: impl SessionStore<SessionData> + SessionStoreRng<TestRng>,
) {
    let rng = TestRng::seed_from_u64(2280217217);
    store.rng(rng);
    let session_key = store.create(&SessionData::sample(), ttl()).await.unwrap();

    let data = SessionData::sample();
    let ttl = ttl_strict();
    store.update(&session_key, &data, ttl).await.unwrap();

    let record = store.load(&session_key).await.unwrap().unwrap();
    assert_eq!(record.data, data);
    assert_eq!(record.ttl.normalize(), ttl.normalize());
}

pub async fn test_loading_session_after_update_ttl(
    mut store: impl SessionStore<SessionData> + SessionStoreRng<TestRng>,
) {
    let rng = TestRng::seed_from_u64(122915542);
    store.rng(rng);
    let data = SessionData::sample();
    let session_key = store.create(&data, ttl()).await.unwrap();

    let ttl = ttl_strict();
    store.update_ttl(&session_key, ttl).await.unwrap();

    let record = store.load(&session_key).await.unwrap().unwrap();
    assert_eq!(record.data, data);
    assert_eq!(record.ttl.normalize(), ttl.normalize());
}

pub async fn test_loading_a_missing_session_returns_none(
    store: impl SessionStore<SessionData> + SessionStoreRng<TestRng>,
) {
    let mut rng = TestRng::seed_from_u64(999412874);
    let session_key = rng.random::<SessionKey>();

    let record = store.load(&session_key).await.unwrap();
    assert!(record.is_none());
}

pub async fn test_loading_an_expired_session_returns_none_create(
    mut store: impl SessionStore<SessionData> + SessionStoreRng<TestRng>,
) {
    let rng = TestRng::seed_from_u64(31348441);
    store.rng(rng);

    let five_microseconds_from_now = Ttl::now_local().unwrap() + Duration::from_micros(5);
    let session_key = store
        .create(&SessionData::sample(), five_microseconds_from_now)
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_micros(10)).await;

    let record = store.load(&session_key).await.unwrap();
    assert!(record.is_none());
}

pub async fn test_loading_an_expired_session_returns_none_update_nonexisting(
    store: impl SessionStore<SessionData> + SessionStoreRng<TestRng>,
) {
    let mut rng = TestRng::seed_from_u64(880523847);
    let session_key = rng.random::<SessionKey>();

    let five_microseconds_from_now = Ttl::now_local().unwrap() + Duration::from_micros(5);
    store
        .update(
            &session_key,
            &SessionData::sample(),
            five_microseconds_from_now,
        )
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_micros(10)).await;

    let record = store.load(&session_key).await.unwrap();
    assert!(record.is_none());
}

pub async fn test_loading_an_expired_session_returns_none_update_existing(
    mut store: impl SessionStore<SessionData> + SessionStoreRng<TestRng>,
) {
    let rng = TestRng::seed_from_u64(92143371);
    store.rng(rng);
    let session_key = store.create(&SessionData::sample(), ttl()).await.unwrap();

    let five_microseconds_from_now = Ttl::now_local().unwrap() + Duration::from_micros(5);
    store
        .update(
            &session_key,
            &SessionData::sample(),
            five_microseconds_from_now,
        )
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_micros(10)).await;

    let record = store.load(&session_key).await.unwrap();
    assert!(record.is_none());
}

pub async fn test_loading_an_expired_session_returns_none_update_ttl(
    mut store: impl SessionStore<SessionData> + SessionStoreRng<TestRng>,
) {
    let rng = TestRng::seed_from_u64(2587831351);
    store.rng(rng);
    let session_key = store.create(&SessionData::sample(), ttl()).await.unwrap();

    let five_microseconds_from_now = Ttl::now_local().unwrap() + Duration::from_micros(5);
    store
        .update_ttl(&session_key, five_microseconds_from_now)
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_micros(10)).await;

    let record = store.load(&session_key).await.unwrap();
    assert!(record.is_none());
}

pub async fn test_delete_after_create(
    mut store: impl SessionStore<SessionData> + SessionStoreRng<TestRng>,
) {
    let rng = TestRng::seed_from_u64(306111374);
    store.rng(rng);

    let session_key = store.create(&SessionData::sample(), ttl()).await.unwrap();
    store.delete(&session_key).await.unwrap();

    let record = store.load(&session_key).await.unwrap();
    assert!(record.is_none());
}

pub async fn test_delete_after_update(
    store: impl SessionStore<SessionData> + SessionStoreRng<TestRng>,
) {
    let mut rng = TestRng::seed_from_u64(200708635);
    let session_key = rng.random::<SessionKey>();

    store
        .update(&session_key, &SessionData::sample(), ttl())
        .await
        .unwrap();
    store.delete(&session_key).await.unwrap();

    let record = store.load(&session_key).await.unwrap();
    assert!(record.is_none());
}

pub async fn test_delete_does_not_error_for_missing_entry(
    store: impl SessionStore<SessionData> + SessionStoreRng<TestRng>,
) {
    let mut rng = TestRng::seed_from_u64(136113526);
    let session_key = rng.random::<SessionKey>();

    let record = store.load(&session_key).await.unwrap();
    assert!(record.is_none());

    store.delete(&session_key).await.unwrap();
}

fn ttl_edge_case() -> Ttl {
    (Ttl::now_local().unwrap() + Duration::from_secs(10 * 60))
        .replace_nanosecond(1_000_000_000 - 1)
        .unwrap()
}

pub async fn test_ttl_with_999_999_999_nanoseconds_create(
    mut store: impl SessionStore<SessionData> + SessionStoreRng<TestRng>,
) {
    let rng = TestRng::seed_from_u64(747720501);
    store.rng(rng);

    let ttl = ttl_edge_case();
    let session_key = store.create(&SessionData::sample(), ttl).await.unwrap();
    let record = store.load(&session_key).await.unwrap().unwrap();
    assert_eq!(record.ttl.normalize(), ttl.normalize());
}

pub async fn test_ttl_with_999_999_999_nanoseconds_update_nonexisting(
    store: impl SessionStore<SessionData> + SessionStoreRng<TestRng>,
) {
    let mut rng = TestRng::seed_from_u64(1551031452);
    let session_key = rng.random::<SessionKey>();

    let ttl = ttl_edge_case();
    store
        .update(&session_key, &SessionData::sample(), ttl)
        .await
        .unwrap();
    let record = store.load(&session_key).await.unwrap().unwrap();
    assert_eq!(record.ttl.normalize(), ttl.normalize());
}

pub async fn test_ttl_with_999_999_999_nanoseconds_update_existing(
    mut store: impl SessionStore<SessionData> + SessionStoreRng<TestRng>,
) {
    let rng = TestRng::seed_from_u64(2177610229);
    store.rng(rng);
    let session_key = store.create(&SessionData::sample(), ttl()).await.unwrap();

    let ttl = ttl_edge_case();
    store
        .update(&session_key, &SessionData::sample(), ttl)
        .await
        .unwrap();
    let record = store.load(&session_key).await.unwrap().unwrap();
    assert_eq!(record.ttl.normalize(), ttl.normalize());
}

pub async fn test_ttl_with_999_999_999_nanoseconds_update_ttl(
    mut store: impl SessionStore<SessionData> + SessionStoreRng<TestRng>,
) {
    let rng = TestRng::seed_from_u64(337520113);
    store.rng(rng);
    let session_key = store.create(&SessionData::sample(), ttl()).await.unwrap();

    let ttl = ttl_edge_case();
    store.update_ttl(&session_key, ttl).await.unwrap();
    let record = store.load(&session_key).await.unwrap().unwrap();
    assert_eq!(record.ttl.normalize(), ttl.normalize());
}

#[doc(hidden)]
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct SessionData {
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
struct DbId(u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
struct Preferences {
    theme: Theme,
    language: Language,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
enum Theme {
    Light,
    Dark,
}

/// The two languages
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
enum Language {
    #[serde(alias = "en-US")]
    EnUs,
    #[serde(alias = "en-GB")]
    EnGb,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
struct CartItem {
    item_id: DbId,
    name: String,
    quantity: u64,
    price: Decimal,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
struct RateLimit {
    failed_login_attempts: u64,
    #[serde(with = "time::serde::rfc3339")]
    last_attempt: OffsetDateTime,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
struct WorkflowState {
    step: u64,
    total_steps: u64,
    data: WorkflowData,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
struct WorkflowData {
    address: String,
}

impl SessionData {
    fn sample() -> Self {
        SessionData::sample_with(12345)
    }

    fn sample_with(user_id: u64) -> Self {
        SessionData {
            user_id: DbId(user_id),
            authenticated: true,
            roles: vec!["admin".to_owned(), "editor".to_owned()],
            preferences: Preferences {
                theme: Theme::Dark,
                language: Language::EnUs,
            },
            cart: vec![
                CartItem {
                    item_id: DbId(101),
                    name: "Laptop".to_owned(),
                    quantity: 1,
                    price: Decimal::new(99999, 2),
                },
                CartItem {
                    item_id: DbId(202),
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
}

fn ttl() -> Ttl {
    let now = Ttl::now_local().unwrap();
    ttl_of(now)
}

fn ttl_of(f: Ttl) -> Ttl {
    f + Duration::from_secs(10 * 60)
}

fn ttl_strict() -> Ttl {
    let now = Ttl::now_local().unwrap();
    ttl_strict_of(now)
}

fn ttl_strict_of(f: Ttl) -> Ttl {
    f + Duration::from_millis(1500)
}

trait TtlExt {
    type Normalized;

    fn normalize(self) -> Self::Normalized;
}

impl TtlExt for Ttl {
    type Normalized = UtcDateTime;

    fn normalize(self) -> Self::Normalized {
        self.replace_nanosecond(0).unwrap().to_utc()
    }
}
