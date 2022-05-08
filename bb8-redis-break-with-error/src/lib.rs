pub use bb8;
pub use redis;

use core::{
    fmt,
    ops::{Deref, DerefMut},
};

use async_trait::async_trait;
use redis::{
    aio::{Connection, ConnectionLike},
    Client, Cmd, ErrorKind, IntoConnectionInfo, Pipeline, RedisError, RedisFuture, Value,
};

/// A `bb8::ManageConnection` for `redis::Client::get_async_connection`.
#[derive(Clone, Debug)]
pub struct RedisConnectionManager {
    client: Client,
}

impl RedisConnectionManager {
    /// Create a new `RedisConnectionManager`.
    /// See `redis::Client::open` for a description of the parameter types.
    pub fn new<T: IntoConnectionInfo>(info: T) -> Result<RedisConnectionManager, RedisError> {
        Ok(RedisConnectionManager {
            client: Client::open(info.into_connection_info()?)?,
        })
    }
}

#[async_trait]
impl bb8::ManageConnection for RedisConnectionManager {
    type Connection = RedisConnection;
    type Error = RedisError;

    async fn connect(&self) -> Result<Self::Connection, Self::Error> {
        self.client
            .get_async_connection()
            .await
            .map(RedisConnection::new)
    }

    async fn is_valid(&self, conn: &mut Self::Connection) -> Result<(), Self::Error> {
        let pong: String = redis::cmd("PING").query_async(conn).await?;
        match pong.as_str() {
            "PONG" => Ok(()),
            _ => Err((ErrorKind::ResponseError, "ping request").into()),
        }
    }

    fn has_broken(&self, conn: &mut Self::Connection) -> bool {
        conn.is_close_required()
    }
}

//
pub struct RedisConnection {
    connection: Connection,
    close_required: bool,
}

impl fmt::Debug for RedisConnection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RedisConnection")
            .field("connection", &"")
            .field("close_required", &self.close_required)
            .finish()
    }
}

impl RedisConnection {
    pub fn new(connection: Connection) -> Self {
        Self {
            connection,
            close_required: false,
        }
    }

    pub fn set_close_required_with_error(&mut self, err: &RedisError) {
        let close_required = match err.kind() {
            ErrorKind::ResponseError => true,
            ErrorKind::AuthenticationFailed => true,
            ErrorKind::TypeError => false,
            ErrorKind::ExecAbortError => false,
            ErrorKind::BusyLoadingError => false,
            ErrorKind::NoScriptError => false,
            ErrorKind::InvalidClientConfig => true,
            ErrorKind::Moved => false,
            ErrorKind::Ask => false,
            ErrorKind::TryAgain => false,
            ErrorKind::ClusterDown => false,
            ErrorKind::CrossSlot => true,
            ErrorKind::MasterDown => false,
            ErrorKind::IoError => true,
            ErrorKind::ClientError => true,
            ErrorKind::ExtensionError => false,
            ErrorKind::ReadOnly => false,
            _ => true,
        };
        self.close_required = close_required
    }

    pub fn set_close_required(&mut self) {
        self.close_required = true
    }

    pub fn is_close_required(&self) -> bool {
        self.close_required
    }
}

impl Deref for RedisConnection {
    type Target = Connection;

    fn deref(&self) -> &Connection {
        &self.connection
    }
}

impl DerefMut for RedisConnection {
    fn deref_mut(&mut self) -> &mut Connection {
        &mut self.connection
    }
}

//
impl ConnectionLike for RedisConnection {
    fn req_packed_command<'a>(&'a mut self, cmd: &'a Cmd) -> RedisFuture<'a, Value> {
        self.deref_mut().req_packed_command(cmd)
    }

    fn req_packed_commands<'a>(
        &'a mut self,
        cmd: &'a crate::Pipeline,
        offset: usize,
        count: usize,
    ) -> RedisFuture<'a, Vec<Value>> {
        self.deref_mut().req_packed_commands(cmd, offset, count)
    }

    fn get_db(&self) -> i64 {
        self.deref().get_db()
    }
}

//
pub trait ConnectionCloseRequiredLike {
    fn set_close_required_with_error(&mut self, err: &RedisError);
    fn set_close_required(&mut self);
    fn is_close_required(&self) -> bool;
}

impl ConnectionCloseRequiredLike for RedisConnection {
    fn set_close_required_with_error(&mut self, err: &RedisError) {
        RedisConnection::set_close_required_with_error(self, err)
    }

    fn set_close_required(&mut self) {
        RedisConnection::set_close_required(self)
    }

    fn is_close_required(&self) -> bool {
        RedisConnection::is_close_required(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::error;

    #[tokio::test]
    async fn test_close_required_like() -> Result<(), Box<dyn error::Error>> {
        let manager = RedisConnectionManager::new("redis://127.0.0.1:6379")?;
        let pool = bb8::Pool::builder()
            .test_on_check_out(false)
            .build(manager)
            .await?;

        let mut conn =
            match tokio::time::timeout(tokio::time::Duration::from_millis(100), pool.get()).await {
                Ok(Ok(x)) => x,
                Ok(Err(err)) => panic!("{}", err),
                Err(_) => return Ok(()),
            };

        async fn ping<C>(conn: &mut C) -> Result<(), Box<dyn error::Error>>
        where
            C: ConnectionLike + ConnectionCloseRequiredLike,
        {
            let _ = redis::cmd("PING").query_async(conn).await.map_err(|err| {
                conn.set_close_required_with_error(&err);
                err
            })?;
            Ok(())
        }

        ping(conn.deref_mut()).await?;

        Ok(())
    }
}
