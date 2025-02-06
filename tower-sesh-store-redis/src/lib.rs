#[cfg(not(any(feature = "tokio-comp", feature = "async-std-comp")))]
compile_error!("Either the `tokio-comp` or `async-std-comp` feature must be enabled.");

use std::{borrow::Cow, marker::PhantomData};

use async_trait::async_trait;
use client::{ConnectionManagerWithRetry, GetConnection};
use redis::{
    aio::ConnectionManagerConfig, AsyncCommands, Client, ExistenceCheck, IntoConnectionInfo,
    RedisResult, SetExpiry, SetOptions,
};
use serde::{de::DeserializeOwned, Serialize};
use tower_sesh_core::{
    store::Error,
    store::{SessionStoreImpl, Ttl},
    Record, SessionKey, SessionStore,
};

pub mod client;

const DEFAULT_KEY_PREFIX: &str = "session:";

type Result<T, E = Error> = std::result::Result<T, E>;

pub struct RedisStore<T, C: GetConnection = ConnectionManagerWithRetry> {
    client: C,
    config: RedisStoreConfig,
    _marker: PhantomData<fn() -> T>,
}

struct RedisStoreConfig {
    key_prefix: Cow<'static, str>,
}

impl Default for RedisStoreConfig {
    fn default() -> Self {
        Self {
            key_prefix: Cow::Borrowed(DEFAULT_KEY_PREFIX),
        }
    }
}

impl<T> RedisStore<T> {
    /// Connect to a redis server and return a store.
    ///
    /// When opening a client a URL in the following format should be used:
    ///
    /// ```not_rust
    /// redis://<host>:<port>/...
    /// ```
    ///
    /// # Example
    ///
    /// ```no_run
    /// use tower_sesh_store_redis::RedisStore;
    ///
    /// # tokio_test::block_on(async {
    /// # type SessionData = ();
    /// let store = RedisStore::<SessionData>::open("redis://127.0.0.1/").await?;
    /// # Ok::<(), redis::RedisError>(())
    /// # }).unwrap();
    /// ```
    pub async fn open<I: IntoConnectionInfo>(params: I) -> RedisResult<RedisStore<T>> {
        let client = Client::open(params)?;
        Self::with_client(client).await
    }

    /// Create a new redis store with the provided client.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use tower_sesh_store_redis::RedisStore;
    ///
    /// # tokio_test::block_on(async {
    /// let client = redis::Client::open("redis://127.0.0.1/")?;
    /// # type SessionData = ();
    /// let store = RedisStore::<SessionData>::with_client(client).await?;
    /// # Ok::<(), redis::RedisError>(())
    /// # }).unwrap();
    /// ```
    pub async fn with_client(client: Client) -> RedisResult<RedisStore<T>> {
        let client = ConnectionManagerWithRetry::new(client).await?;
        Ok(Self {
            client,
            config: RedisStoreConfig::default(),
            _marker: PhantomData,
        })
    }

    #[doc(hidden)]
    pub async fn with_connection_manager_config(
        client: Client,
        config: ConnectionManagerConfig,
    ) -> RedisResult<RedisStore<T>> {
        let client = ConnectionManagerWithRetry::new_with_config(client, config).await?;
        Ok(Self {
            client,
            config: RedisStoreConfig::default(),
            _marker: PhantomData,
        })
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
}

macro_rules! ensure_redis_ttl {
    ($ttl:ident) => {
        if $ttl < 0 {
            return Err(Error::message(format!(
                "unexpected timestamp value: {}",
                $ttl
            )));
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

        // Collision resolution
        // (This is statistically improbable for a sufficiently large session key)
        const MAX_RETRIES: usize = 4;
        for _ in 0..MAX_RETRIES {
            let session_key = SessionKey::generate();
            let key = self.redis_key(&session_key);

            let v: redis::Value = conn
                .set_options(
                    &key,
                    &serialized,
                    SetOptions::default()
                        .conditional_set(ExistenceCheck::NX)
                        .with_expiration(expiry),
                )
                .await
                .map_err(Error::store)?;

            match v {
                redis::Value::Nil => {} // Conflict with NX: key exists
                _ => return Ok(session_key),
            }
        }

        Err(err_max_iterations_reached())
    }

    async fn load(&self, session_key: &SessionKey) -> Result<Option<Record<T>>> {
        let key = self.redis_key(session_key);
        let mut conn = self.connection().await?;

        const WEEK_IN_SECONDS: i64 = 60 * 60 * 24 * 7;
        const DEFAULT_EXPIRY: i64 = 2 * WEEK_IN_SECONDS;

        let (value, timestamp) = redis::pipe()
            .atomic()
            .expire(&key, DEFAULT_EXPIRY) // Ensure the key has a timeout if one isn't set
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
                ensure_redis_ttl!(timestamp);
                Some(deserialize(&value).and_then(|data| to_record(data, timestamp))).transpose()
            }
        }
    }

    async fn update(&self, session_key: &SessionKey, data: &T, ttl: Ttl) -> Result<()> {
        let key = self.redis_key(session_key);
        let mut conn = self.connection().await?;

        let expiry = set_expiry_from_ttl(ttl)?;
        let serialized = serialize(data)?;

        let _: () = conn
            .set_options(
                &key,
                serialized,
                SetOptions::default().with_expiration(expiry),
            )
            .await
            .map_err(Error::store)?;

        Ok(())
    }

    async fn update_ttl(&self, session_key: &SessionKey, ttl: Ttl) -> Result<()> {
        let key = self.redis_key(session_key);
        let mut conn = self.connection().await?;

        let _: () = conn
            .expire_at(key, ttl.unix_timestamp())
            .await
            .map_err(Error::store)?;

        todo!()
    }

    async fn delete(&self, session_key: &SessionKey) -> Result<()> {
        let key = self.redis_key(session_key);
        let mut conn = self.connection().await?;

        let _: () = conn.del(&key).await.map_err(Error::store)?;

        Ok(())
    }
}

fn set_expiry_from_ttl(ttl: Ttl) -> Result<SetExpiry> {
    let timestamp = u64::try_from(ttl.unix_timestamp()).map_err(
        #[cold]
        |_| Error::message(format!("unexpected negative timestamp: {}", ttl)),
    )?;

    Ok(SetExpiry::EXAT(timestamp))
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

fn err_max_iterations_reached() -> Error {
    Error::message("max iterations reached when handling session key collisions")
}
