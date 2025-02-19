//! Custom Redis connection implementations.

use std::{error::Error, fmt};

use async_trait::async_trait;
use futures::FutureExt;
use redis::{
    aio::{ConnectionLike, ConnectionManager, ConnectionManagerConfig},
    Client, Cmd, Pipeline, RedisError, RedisFuture, RedisResult, Value,
};

/// A connection manager that immediately retries a request if it fails due
/// to a dropped connection.
///
/// The default [`ConnectionManager`] behavior is to reconnect if a request
/// fails due to a dropped connection, however that request's error is
/// propagated to the caller instead of re-attempting the request.
#[derive(Clone)]
pub struct ConnectionManagerWithRetry(ConnectionManager);

impl ConnectionManagerWithRetry {
    #[inline]
    pub(crate) async fn new(client: Client) -> RedisResult<Self> {
        let config = ConnectionManagerConfig::default();
        Self::new_with_config(client, config).await
    }

    #[inline]
    pub(crate) async fn new_with_config(
        client: Client,
        config: ConnectionManagerConfig,
    ) -> RedisResult<Self> {
        ConnectionManager::new_with_config(client, config)
            .await
            .map(Self::from)
    }
}

impl From<ConnectionManager> for ConnectionManagerWithRetry {
    #[inline]
    fn from(value: ConnectionManager) -> Self {
        Self(value)
    }
}

impl From<ConnectionManagerWithRetry> for ConnectionManager {
    #[inline]
    fn from(value: ConnectionManagerWithRetry) -> Self {
        value.0
    }
}

// FIXME: `ConnectionManagerWithRetry`'s retry strategy is too naive. We should
// only retry the request after a delay, possibly based on
// `ConnectionManagerConfig`.
impl ConnectionLike for ConnectionManagerWithRetry {
    fn req_packed_command<'a>(&'a mut self, cmd: &'a Cmd) -> RedisFuture<'a, Value> {
        (async move {
            match self.0.send_packed_command(cmd).await {
                Err(err) if err.is_connection_dropped() => self.0.send_packed_command(cmd).await,
                result @ (Err(_) | Ok(_)) => result,
            }
        })
        .boxed()
    }

    fn req_packed_commands<'a>(
        &'a mut self,
        cmd: &'a Pipeline,
        offset: usize,
        count: usize,
    ) -> RedisFuture<'a, Vec<Value>> {
        (async move {
            match self.0.send_packed_commands(cmd, offset, count).await {
                Err(err) if err.is_connection_dropped() => {
                    self.0.send_packed_commands(cmd, offset, count).await
                }
                result @ (Err(_) | Ok(_)) => result,
            }
        })
        .boxed()
    }

    fn get_db(&self) -> i64 {
        self.0.get_db()
    }
}

/// A trait for acquiring a [`ConnectionLike`] object that can be safely sent
/// between threads.
///
/// [`ConnectionLike`]: redis::aio::ConnectionLike
///
/// This trait is sealed and cannot be implemented for types outside of
/// `tower-sesh-store-redis`.
#[doc(hidden)]
#[async_trait]
pub trait GetConnection: Send + Sync + 'static + private::Sealed {
    type Connection: ConnectionLike + Send;

    async fn connection(&self) -> Result<Self::Connection, GetConnectionError>;
}

#[async_trait]
impl GetConnection for ConnectionManagerWithRetry {
    type Connection = ConnectionManagerWithRetry;

    #[inline]
    async fn connection(&self) -> Result<Self::Connection, GetConnectionError> {
        Ok(self.clone())
    }
}
impl private::Sealed for ConnectionManagerWithRetry {}

/// An error returned by [`GetConnection`] methods.
#[doc(hidden)]
pub struct GetConnectionError(RedisError);

impl fmt::Debug for GetConnectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("GetConnectionError").field(&self.0).finish()
    }
}

impl fmt::Display for GetConnectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("failed to acquire redis connection")
    }
}

impl Error for GetConnectionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.0)
    }
}

impl From<RedisError> for GetConnectionError {
    fn from(value: RedisError) -> Self {
        Self(value)
    }
}

mod private {
    pub trait Sealed {}
}
