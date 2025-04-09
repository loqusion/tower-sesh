use std::time::Duration;

use futures_util::{stream, StreamExt, TryStreamExt};
use rand::{Rng, SeedableRng};
use tower_sesh_core::{store::SessionStoreRng, SessionKey, SessionStore, Ttl};

use crate::support::{ttl, ttl_expired, ttl_strict, ttl_strict_of, SessionData, TestRng, TtlExt};

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
    mut store: impl SessionStore<SessionData> + SessionStoreRng<TestRng>,
) {
    let mut rng = TestRng::seed_from_u64(2848227658);
    let session_key = rng.random::<SessionKey>();
    store.rng(rng);

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
    mut store: impl SessionStore<SessionData> + SessionStoreRng<TestRng>,
) {
    let mut rng = TestRng::seed_from_u64(999412874);
    let session_key = rng.random::<SessionKey>();
    store.rng(rng);

    let record = store.load(&session_key).await.unwrap();
    assert!(record.is_none());
}

pub async fn test_loading_an_expired_session_returns_none_create(
    mut store: impl SessionStore<SessionData> + SessionStoreRng<TestRng>,
) {
    let rng = TestRng::seed_from_u64(31348441);
    store.rng(rng);

    let five_microseconds_from_now = Ttl::now_local().unwrap() + Duration::from_millis(50);
    let session_key = store
        .create(&SessionData::sample(), five_microseconds_from_now)
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_millis(90)).await;

    let record = store.load(&session_key).await.unwrap();
    assert!(record.is_none());
}

pub async fn test_loading_an_expired_session_returns_none_update_nonexisting(
    mut store: impl SessionStore<SessionData> + SessionStoreRng<TestRng>,
) {
    let mut rng = TestRng::seed_from_u64(880523847);
    let session_key = rng.random::<SessionKey>();
    store.rng(rng);

    let five_microseconds_from_now = Ttl::now_local().unwrap() + Duration::from_millis(50);
    store
        .update(
            &session_key,
            &SessionData::sample(),
            five_microseconds_from_now,
        )
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_millis(90)).await;

    let record = store.load(&session_key).await.unwrap();
    assert!(record.is_none());
}

pub async fn test_loading_an_expired_session_returns_none_update_existing(
    mut store: impl SessionStore<SessionData> + SessionStoreRng<TestRng>,
) {
    let rng = TestRng::seed_from_u64(92143371);
    store.rng(rng);
    let session_key = store.create(&SessionData::sample(), ttl()).await.unwrap();

    let five_microseconds_from_now = Ttl::now_local().unwrap() + Duration::from_millis(50);
    store
        .update(
            &session_key,
            &SessionData::sample(),
            five_microseconds_from_now,
        )
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_millis(90)).await;

    let record = store.load(&session_key).await.unwrap();
    assert!(record.is_none());
}

pub async fn test_loading_an_expired_session_returns_none_update_ttl(
    mut store: impl SessionStore<SessionData> + SessionStoreRng<TestRng>,
) {
    let rng = TestRng::seed_from_u64(2587831351);
    store.rng(rng);
    let session_key = store.create(&SessionData::sample(), ttl()).await.unwrap();

    let five_microseconds_from_now = Ttl::now_local().unwrap() + Duration::from_millis(50);
    store
        .update_ttl(&session_key, five_microseconds_from_now)
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_millis(90)).await;

    let record = store.load(&session_key).await.unwrap();
    assert!(record.is_none());
}

pub async fn test_loading_session_after_create_with_ttl_in_past(
    mut store: impl SessionStore<SessionData> + SessionStoreRng<TestRng>,
) {
    let rng = TestRng::seed_from_u64(1710010949);
    store.rng(rng);

    let session_key = store
        .create(&SessionData::sample(), ttl_expired())
        .await
        .unwrap();

    let record = store.load(&session_key).await.unwrap();
    assert!(record.is_none());
}

pub async fn test_loading_session_after_update_nonexisting_with_ttl_in_past(
    mut store: impl SessionStore<SessionData> + SessionStoreRng<TestRng>,
) {
    let mut rng = TestRng::seed_from_u64(1710010949);
    let session_key = rng.random::<SessionKey>();
    store.rng(rng);

    store
        .update(&session_key, &SessionData::sample(), ttl_expired())
        .await
        .unwrap();

    let record = store.load(&session_key).await.unwrap();
    assert!(record.is_none());
}

pub async fn test_loading_session_after_update_existing_with_ttl_in_past(
    mut store: impl SessionStore<SessionData> + SessionStoreRng<TestRng>,
) {
    let rng = TestRng::seed_from_u64(1710010949);
    store.rng(rng);
    let session_key = store.create(&SessionData::sample(), ttl()).await.unwrap();

    store
        .update(&session_key, &SessionData::sample(), ttl_expired())
        .await
        .unwrap();

    let record = store.load(&session_key).await.unwrap();
    assert!(record.is_none());
}

pub async fn test_loading_session_after_update_ttl_with_ttl_in_past(
    mut store: impl SessionStore<SessionData> + SessionStoreRng<TestRng>,
) {
    let rng = TestRng::seed_from_u64(1710010949);
    store.rng(rng);
    let session_key = store.create(&SessionData::sample(), ttl()).await.unwrap();

    store.update_ttl(&session_key, ttl_expired()).await.unwrap();

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
    mut store: impl SessionStore<SessionData> + SessionStoreRng<TestRng>,
) {
    let mut rng = TestRng::seed_from_u64(200708635);
    let session_key = rng.random::<SessionKey>();
    store.rng(rng);

    store
        .update(&session_key, &SessionData::sample(), ttl())
        .await
        .unwrap();
    store.delete(&session_key).await.unwrap();

    let record = store.load(&session_key).await.unwrap();
    assert!(record.is_none());
}

pub async fn test_delete_does_not_error_for_missing_entry(
    mut store: impl SessionStore<SessionData> + SessionStoreRng<TestRng>,
) {
    let mut rng = TestRng::seed_from_u64(136113526);
    let session_key = rng.random::<SessionKey>();
    store.rng(rng);

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
    mut store: impl SessionStore<SessionData> + SessionStoreRng<TestRng>,
) {
    let mut rng = TestRng::seed_from_u64(1551031452);
    let session_key = rng.random::<SessionKey>();
    store.rng(rng);

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

pub async fn test_update_ttl_extends_session_that_would_otherwise_expire(
    mut store: impl SessionStore<SessionData> + SessionStoreRng<TestRng>,
) {
    let rng = TestRng::seed_from_u64(1171023902);
    store.rng(rng);

    let before = Ttl::now_local().unwrap();
    let strict_ttl = ttl_strict_of(before);
    let data = SessionData::sample_with(1171023902);
    let session_key = store.create(&data, strict_ttl).await.unwrap();

    let updated_ttl = ttl();
    store.update_ttl(&session_key, updated_ttl).await.unwrap();

    let sleep_until_duration = strict_ttl - Ttl::now_local().unwrap();
    if sleep_until_duration.is_positive() {
        let sleep_until_duration = sleep_until_duration.unsigned_abs();
        tokio::time::sleep(sleep_until_duration + Duration::from_millis(10)).await;
    }

    let record = store.load(&session_key).await.unwrap().unwrap();
    assert_eq!(record.data, data);
    assert_eq!(record.ttl.normalize(), updated_ttl.normalize());
}

pub async fn test_update_ttl_does_not_revive_expired_session(
    mut store: impl SessionStore<SessionData> + SessionStoreRng<TestRng>,
) {
    let rng = TestRng::seed_from_u64(2495922455);
    store.rng(rng);

    let five_microseconds_from_now = Ttl::now_local().unwrap() + Duration::from_millis(50);
    let session_key = store
        .create(&SessionData::sample(), five_microseconds_from_now)
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_millis(90)).await;

    store.update_ttl(&session_key, ttl()).await.unwrap();
    let record = store.load(&session_key).await.unwrap();
    assert!(record.is_none());
}
