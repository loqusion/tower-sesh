use futures::FutureExt;
use redis::{
    aio::{ConnectionLike, ConnectionManager, ConnectionManagerConfig},
    Client, Cmd, Pipeline, RedisFuture, RedisResult, Value,
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
    pub(crate) async fn new(client: Client) -> RedisResult<Self> {
        ConnectionManager::new(client).await.map(Self::from)
    }

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
    fn from(value: ConnectionManager) -> Self {
        Self(value)
    }
}

impl From<ConnectionManagerWithRetry> for ConnectionManager {
    fn from(value: ConnectionManagerWithRetry) -> Self {
        value.0
    }
}

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
