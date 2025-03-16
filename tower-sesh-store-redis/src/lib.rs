#![cfg_attr(docsrs, feature(doc_auto_cfg))]

//! The Redis store for [`tower-sesh`].
//!
//! [`tower-sesh`]: https://docs.rs/tower-sesh/latest/tower_sesh/

#![doc(test(
    no_crate_inject,
    attr(
        deny(warnings, rust_2018_idioms, single_use_lifetimes),
        allow(dead_code, unused_assignments, unused_variables)
    )
))]

#[cfg(not(any(feature = "tokio-comp", feature = "async-std-comp")))]
compile_error!("Either the `tokio-comp` or `async-std-comp` feature must be enabled.");

use std::{borrow::Cow, fmt, marker::PhantomData};

use async_trait::async_trait;
use connection::{ConnectionManagerWithRetry, GetConnection};
use rand::{rngs::ThreadRng, Rng};
use redis::{
    aio::ConnectionManagerConfig, AsyncCommands, Client, ExistenceCheck, IntoConnectionInfo,
    RedisResult, SetExpiry, SetOptions,
};
use serde::{de::DeserializeOwned, Serialize};
use tower_sesh_core::{
    store::{Error, SessionStoreImpl},
    time::SESSION_EXPIRY_SECONDS_DEFAULT,
    Record, SessionKey, SessionStore, Ttl,
};

pub use redis;

pub mod connection;

type Result<T, E = Error> = std::result::Result<T, E>;

pub struct RedisStore<T, C: GetConnection = ConnectionManagerWithRetry> {
    client: C,
    config: Config,

    #[cfg(feature = "test-util")]
    rng: Option<Box<parking_lot::Mutex<dyn rand::CryptoRng + Send + 'static>>>,

    _marker: PhantomData<fn() -> T>,
}

#[derive(Clone, Debug)]
struct Config {
    key_prefix: Cow<'static, str>,
}

const DEFAULT_KEY_PREFIX: &str = "session:";

impl Default for Config {
    #[inline]
    fn default() -> Self {
        Self {
            key_prefix: Cow::Borrowed(DEFAULT_KEY_PREFIX),
        }
    }
}

impl<T> RedisStore<T> {
    /// Connects to a redis server and returns a store with default
    /// configuration values.
    ///
    /// `info` should be a string containing a URL in the following format:
    ///
    /// ```not_rust
    /// {redis|rediss}://[<username>][:<password>@]<hostname>[:port][/<db>]
    /// ```
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use tower_sesh_store_redis::RedisStore;
    ///
    /// # type SessionData = ();
    /// #
    /// # tokio_test::block_on(async {
    /// let store = RedisStore::<SessionData>::open("redis://127.0.0.1/").await?;
    /// # Ok::<(), redis::RedisError>(())
    /// # }).unwrap();
    /// ```
    pub async fn open<I: IntoConnectionInfo>(info: I) -> RedisResult<RedisStore<T>> {
        let client = Client::open(info)?;
        ConnectionManagerWithRetry::new(client)
            .await
            .map(RedisStore::with_client)
    }

    /// Connects to a redis server and returns a store with the given
    /// configuration.
    ///
    /// This allows you to configure the [connection manager]'s reconnect
    /// mechanism and request timing.
    ///
    /// [connection manager]: redis::aio::ConnectionManager
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use redis::aio::ConnectionManagerConfig;
    /// use tower_sesh_store_redis::RedisStore;
    ///
    /// # type SessionData = ();
    /// #
    /// # tokio_test::block_on(async {
    /// let config = ConnectionManagerConfig::default()
    ///     .set_number_of_retries(4);
    /// let store = RedisStore::<SessionData>::with_config("redis://127.0.0.1/", config).await?;
    /// # Ok::<(), redis::RedisError>(())
    /// # }).unwrap();
    /// ```
    pub async fn with_config<I: IntoConnectionInfo>(
        info: I,
        config: ConnectionManagerConfig,
    ) -> RedisResult<RedisStore<T>> {
        let client = Client::open(info)?;
        ConnectionManagerWithRetry::with_config(client, config)
            .await
            .map(RedisStore::with_client)
    }
}

impl<T, C: GetConnection> RedisStore<T, C> {
    #[cfg(feature = "test-util")]
    #[inline]
    fn with_client(client: C) -> RedisStore<T, C> {
        Self {
            client,
            config: Config::default(),
            rng: None,
            _marker: PhantomData,
        }
    }

    #[cfg(not(feature = "test-util"))]
    #[inline]
    fn with_client(client: C) -> RedisStore<T, C> {
        Self {
            client,
            config: Config::default(),
            _marker: PhantomData,
        }
    }
}

impl<T, C: GetConnection> RedisStore<T, C> {
    /// Set the Redis key prefix used to store sessions.
    ///
    /// When a session is stored, the Redis [key] is constructed by appending
    /// the Base64-encoded session key to the prefix, e.g.
    /// `session:ym5hy39HMVwYUJpPW6x_sQ`.
    ///
    /// Default is `"session:"`.
    ///
    /// [key]: https://redis.io/docs/latest/develop/use/keyspace/
    pub fn key_prefix(mut self, prefix: impl Into<Cow<'static, str>>) -> RedisStore<T, C> {
        self.config.key_prefix = prefix.into();
        self
    }
}

impl<T, C: GetConnection> fmt::Debug for RedisStore<T, C>
where
    C: fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RedisStore")
            .field("client", &self.client)
            .field("config", &self.config)
            .finish()
    }
}

impl<T, C: GetConnection> RedisStore<T, C> {
    fn redis_key(&self, session_key: &SessionKey) -> String {
        let mut redis_key =
            String::with_capacity(self.config.key_prefix.len() + SessionKey::ENCODED_LEN);
        redis_key.push_str(&self.config.key_prefix);
        redis_key.push_str(&session_key.encode());
        redis_key
    }

    async fn connection(&self) -> Result<<C as GetConnection>::Connection> {
        self.client.connection().await.map_err(Error::store)
    }

    #[cfg(feature = "test-util")]
    fn random_key(&self) -> SessionKey {
        if let Some(rng) = &self.rng {
            rng.lock().random()
        } else {
            ThreadRng::default().random()
        }
    }

    #[cfg(not(feature = "test-util"))]
    #[inline]
    fn random_key(&self) -> SessionKey {
        ThreadRng::default().random()
    }
}

macro_rules! ensure_redis_timestamp {
    ($timestamp:ident) => {
        if $timestamp < 0 {
            return Err(err_redis_timestamp($timestamp));
        }
    };
}

impl<T, C: GetConnection> SessionStore<T> for RedisStore<T, C> where
    T: 'static + Send + Sync + Serialize + DeserializeOwned
{
}

#[async_trait]
impl<T, C: GetConnection> SessionStoreImpl<T> for RedisStore<T, C>
where
    T: 'static + Send + Sync + Serialize + DeserializeOwned,
{
    async fn create(&self, data: &T, ttl: Ttl) -> Result<SessionKey> {
        let mut conn = self.connection().await?;

        let expiry = set_expiry_from_ttl(ttl)?;
        let serialized = serialize(data)?;

        let options = SetOptions::default()
            .conditional_set(ExistenceCheck::NX) // Only set the key if it does not exist
            .with_expiration(expiry);

        // Collision resolution
        // (This is statistically improbable for a sufficiently large session key)
        const MAX_RETRIES: usize = 8;
        for _ in 0..MAX_RETRIES {
            let session_key = self.random_key();
            let key = self.redis_key(&session_key);

            let v: redis::Value = conn
                .set_options(&key, &serialized, options)
                .await
                .map_err(Error::store)?;

            match v {
                redis::Value::Nil => {} // Conflict with NX: key exists
                _ => return Ok(session_key),
            }
        }

        Err(Error::max_iterations_reached())
    }

    async fn load(&self, session_key: &SessionKey) -> Result<Option<Record<T>>> {
        let key = self.redis_key(session_key);
        let mut conn = self.connection().await?;

        let (value, timestamp) = redis::pipe()
            .atomic()
            .expire(&key, i64::from(SESSION_EXPIRY_SECONDS_DEFAULT)) // Ensure the key has a timeout if one isn't set
            .arg("NX")
            .ignore()
            .get(&key)
            .expire_time(&key)
            .query_async::<(Option<Vec<u8>>, i64)>(&mut conn)
            .await
            .map_err(Error::store)?;

        match value {
            None => Ok(None),
            Some(value) => {
                ensure_redis_timestamp!(timestamp);
                deserialize(&value)
                    .and_then(|data| to_record(data, timestamp))
                    .map(Some)
            }
        }
    }

    async fn update(&self, session_key: &SessionKey, data: &T, ttl: Ttl) -> Result<()> {
        let key = self.redis_key(session_key);
        let mut conn = self.connection().await?;

        let expiry = set_expiry_from_ttl(ttl)?;
        let serialized = serialize(data)?;

        let options = SetOptions::default().with_expiration(expiry);

        let _: () = conn
            .set_options(&key, serialized, options)
            .await
            .map_err(Error::store)?;

        Ok(())
    }

    async fn update_ttl(&self, session_key: &SessionKey, ttl: Ttl) -> Result<()> {
        let key = self.redis_key(session_key);
        let mut conn = self.connection().await?;

        let timestamp = timestamp_from_ttl(ttl)?;

        let _: () = conn.expire_at(key, timestamp).await.map_err(Error::store)?;

        Ok(())
    }

    async fn delete(&self, session_key: &SessionKey) -> Result<()> {
        let key = self.redis_key(session_key);
        let mut conn = self.connection().await?;

        let _: () = conn.del(&key).await.map_err(Error::store)?;

        Ok(())
    }
}

#[doc(hidden)]
#[cfg(feature = "test-util")]
impl<T, C: GetConnection, Rng> tower_sesh_core::store::SessionStoreRng<Rng> for RedisStore<T, C>
where
    Rng: rand::CryptoRng + Send + 'static,
{
    fn rng(&mut self, rng: Rng) {
        self.rng = Some(Box::new(parking_lot::Mutex::new(rng)));
    }
}

fn set_expiry_from_ttl(ttl: Ttl) -> Result<SetExpiry> {
    match u64::try_from(ttl.unix_timestamp()) {
        Ok(timestamp) => Ok(SetExpiry::EXAT(timestamp)),
        Err(_) => Err(err_negative_unix_timestamp(ttl)),
    }
}

fn timestamp_from_ttl(ttl: Ttl) -> Result<i64> {
    match ttl.unix_timestamp() {
        timestamp if timestamp >= 0 => Ok(timestamp),
        _ => Err(err_negative_unix_timestamp(ttl)),
    }
}

fn serialize<T>(value: &T) -> Result<Vec<u8>>
where
    T: Serialize,
{
    rmp_serde::to_vec_named(value).map_err(Error::serde)
}

fn deserialize<T>(s: &[u8]) -> Result<T>
where
    T: DeserializeOwned,
{
    rmp_serde::from_slice(s).map_err(Error::serde)
}

fn to_record<T>(data: T, timestamp: i64) -> Result<Record<T>> {
    match Ttl::from_unix_timestamp(timestamp) {
        Ok(ttl) => Ok(Record::new(data, ttl)),
        Err(err) => Err(Error::message(format!("invalid timestamp: {}", err))),
    }
}

#[cold]
fn err_redis_timestamp(timestamp: i64) -> Error {
    Error::message(format!(
        "Redis returned an unexpected timestamp value: {}",
        timestamp
    ))
}

#[cold]
fn err_negative_unix_timestamp(ttl: Ttl) -> Error {
    Error::message(format!(
        "calling `.unix_timestamp()` resulted in unexpected negative timestamp: {}",
        ttl
    ))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_constraints() {
        fn require_traits<T: SessionStore<()> + Send + Sync + 'static>() {}

        require_traits::<RedisStore<(), ConnectionManagerWithRetry>>();
    }
}
