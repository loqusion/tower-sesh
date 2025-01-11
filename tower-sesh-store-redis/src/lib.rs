use std::borrow::Cow;

use async_trait::async_trait;
use client::{ConnectionManagerWithRetry, GetConnection};
use redis::{
    aio::ConnectionManagerConfig, AsyncCommands, Client, ExistenceCheck, IntoConnectionInfo,
    RedisResult, SetExpiry, SetOptions,
};
use tower_sesh::{record::Record, session::SessionKey, store::Error, SessionStore};

// Required to use the redis asynchronous interface
#[cfg(not(any(feature = "tokio-comp", feature = "async-std-comp")))]
compile_error!("Either the `tokio-comp` or `async-std-comp` feature must be enabled.");

pub mod client;

const DEFAULT_KEY_PREFIX: &str = "session_";

type Result<T, E = Error> = std::result::Result<T, E>;

pub struct RedisStore<C: GetConnection = ConnectionManagerWithRetry> {
    client: C,
    config: RedisStoreConfig,
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

impl RedisStore {
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
    /// let store = RedisStore::open("redis://127.0.0.1/").await?;
    /// # Ok::<(), redis::RedisError>(())
    /// # }).unwrap();
    /// ```
    pub async fn open<T: IntoConnectionInfo>(params: T) -> RedisResult<Self> {
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
    /// let store = RedisStore::with_client(client).await?;
    /// # Ok::<(), redis::RedisError>(())
    /// # }).unwrap();
    /// ```
    pub async fn with_client(client: Client) -> RedisResult<Self> {
        let client = ConnectionManagerWithRetry::new(client).await?;
        Ok(Self {
            client,
            config: RedisStoreConfig::default(),
        })
    }

    #[doc(hidden)]
    pub async fn with_connection_manager_config(
        client: Client,
        config: ConnectionManagerConfig,
    ) -> RedisResult<Self> {
        let client = ConnectionManagerWithRetry::new_with_config(client, config).await?;
        Ok(Self {
            client,
            config: RedisStoreConfig::default(),
        })
    }
}

impl<C: GetConnection> RedisStore<C> {
    fn redis_key(&self, session_key: &SessionKey) -> String {
        let mut redis_key =
            String::with_capacity(self.config.key_prefix.len() + SessionKey::ENCODED_LEN);
        redis_key.push_str(&self.config.key_prefix);
        redis_key.push_str(&session_key.encode());
        redis_key
    }

    async fn connection(&self) -> Result<<C as GetConnection>::Connection> {
        self.client.connection().await.map_err(|err| todo!())
    }
}

#[async_trait]
impl<C: GetConnection> SessionStore for RedisStore<C> {
    async fn create(&self, record: Record) -> Result<SessionKey> {
        let mut conn = self.connection().await?;

        let expiry = record.set_expiry();
        let serialized = serialize(&record);

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
                .map_err(|err| todo!())?;

            match v {
                redis::Value::Nil => {} // Conflict with NX: key exists
                _ => return Ok(session_key),
            }
        }

        Err(err_max_iterations_reached())
    }

    async fn load(&self, session_key: &SessionKey) -> Result<Option<Record>> {
        let key = self.redis_key(session_key);
        let mut conn = self.connection().await?;

        const WEEK_IN_SECONDS: i64 = 60 * 60 * 24 * 7;
        const DEFAULT_EXPIRY: i64 = 2 * WEEK_IN_SECONDS;

        let (value, expire_time) = redis::pipe()
            .atomic()
            .expire(&key, DEFAULT_EXPIRY) // Ensure the key has a timeout if one isn't set
            .arg("NX")
            .ignore()
            .get(&key)
            .expire_time(&key)
            .query_async::<(Option<String>, i64)>(&mut conn)
            .await
            .map_err(|err| todo!())?;

        match value {
            None => Ok(None),
            Some(value) => Some(deserialize(&value, expire_time)).transpose(),
        }
    }

    async fn update(&self, session_key: &SessionKey, record: Record) -> Result<()> {
        let key = self.redis_key(session_key);
        let mut conn = self.connection().await?;

        let expiry = record.set_expiry();
        let serialized = serialize(&record);

        let _: () = conn
            .set_options(
                &key,
                serialized,
                SetOptions::default().with_expiration(expiry),
            )
            .await
            .map_err(|err| todo!())?;

        Ok(())
    }

    async fn delete(&self, session_key: &SessionKey) -> Result<()> {
        let key = self.redis_key(session_key);
        let mut conn = self.connection().await?;

        let _: () = conn.del(&key).await.map_err(|err| todo!())?;

        Ok(())
    }
}

trait RecordExt {
    fn set_expiry(&self) -> SetExpiry;
}

impl RecordExt for Record {
    fn set_expiry(&self) -> SetExpiry {
        SetExpiry::EXAT(todo!())
    }
}

fn serialize(record: &Record) -> Vec<u8> {
    todo!()
}

fn deserialize(s: &str, ttl: i64) -> Result<Record> {
    debug_assert!(ttl >= 0, "ttl is negative. This is a bug.");
    todo!()
}

struct RedisRecord {
    data: (),
}
impl RedisRecord {
    fn into_record(self, ttl: i64) -> Record {
        todo!()
    }
}

fn err_max_iterations_reached() -> Error {
    todo!()
}
