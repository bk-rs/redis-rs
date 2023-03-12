pub use bb8;
pub use redis;

use core::ops::{Deref, DerefMut};

use async_trait::async_trait;
use redis::{
    aio::{Connection, ConnectionLike},
    cmd, Client, Cmd, ErrorKind, IntoConnectionInfo, Pipeline, RedisError, RedisFuture, Value,
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
        let pong: String = cmd("PING").query_async(conn).await?;
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

impl core::fmt::Debug for RedisConnection {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
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

    pub fn inner(&self) -> &Connection {
        &self.connection
    }

    pub fn into_inner(self) -> Connection {
        self.connection
    }

    pub fn set_close_required_with_error(&mut self, err: &RedisError) {
        let val = match err.kind() {
            ErrorKind::ResponseError => {
                // https://github.com/redis-rs/redis-rs/blob/0.21.5/src/parser.rs#L122
                false
            }
            ErrorKind::AuthenticationFailed => true,
            ErrorKind::TypeError => {
                // https://github.com/redis-rs/redis-rs/blob/0.21.5/src/types.rs#L21
                true
            }
            ErrorKind::ExecAbortError => {
                // https://github.com/redis-rs/redis-rs/blob/0.21.5/src/parser.rs#L123
                false
            }
            ErrorKind::BusyLoadingError => {
                // https://github.com/redis-rs/redis-rs/blob/0.21.5/src/parser.rs#L124
                false
            }
            ErrorKind::NoScriptError => {
                // https://github.com/redis-rs/redis-rs/blob/0.21.5/src/parser.rs#L125
                false
            }
            ErrorKind::InvalidClientConfig => true,
            ErrorKind::Moved => {
                // https://github.com/redis-rs/redis-rs/blob/0.21.5/src/parser.rs#L126
                false
            }
            ErrorKind::Ask => {
                // https://github.com/redis-rs/redis-rs/blob/0.21.5/src/parser.rs#L127
                false
            }
            ErrorKind::TryAgain => {
                // https://github.com/redis-rs/redis-rs/blob/0.21.5/src/parser.rs#L128
                false
            }
            ErrorKind::ClusterDown => {
                // https://github.com/redis-rs/redis-rs/blob/0.21.5/src/parser.rs#L129
                false
            }
            ErrorKind::CrossSlot => {
                // https://github.com/redis-rs/redis-rs/blob/0.21.5/src/parser.rs#L130
                false
            }
            ErrorKind::MasterDown => {
                // https://github.com/redis-rs/redis-rs/blob/0.21.5/src/parser.rs#L131
                false
            }
            ErrorKind::IoError => {
                // TODO, is_connection_dropped is_connection_refusal
                //
                true
            }
            ErrorKind::ClientError => true,
            ErrorKind::ExtensionError => {
                // https://github.com/redis-rs/redis-rs/blob/0.21.5/src/types.rs#L315-L319
                // https://github.com/redis-rs/redis-rs/blob/0.21.5/src/types.rs#L367
                //
                match err.code() {
                    Some("NOAUTH") => true,
                    Some("WRONGPASS") => true,
                    _ => true,
                }
            }
            ErrorKind::ReadOnly => {
                // https://github.com/redis-rs/redis-rs/blob/0.21.5/src/parser.rs#L132
                false
            }
            _ => true,
        };
        self.set_close_required(val)
    }

    pub fn set_close_required(&mut self, val: bool) {
        self.close_required = val
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
        Box::pin(async move {
            match self.connection.req_packed_command(cmd).await {
                Ok(value) => Ok(value),
                Err(err) => {
                    self.set_close_required_with_error(&err);
                    Err(err)
                }
            }
        })
    }

    fn req_packed_commands<'a>(
        &'a mut self,
        cmd: &'a Pipeline,
        offset: usize,
        count: usize,
    ) -> RedisFuture<'a, Vec<Value>> {
        Box::pin(async move {
            match self
                .connection
                .req_packed_commands(cmd, offset, count)
                .await
            {
                Ok(value) => Ok(value),
                Err(err) => {
                    self.set_close_required_with_error(&err);
                    Err(err)
                }
            }
        })
    }

    fn get_db(&self) -> i64 {
        self.connection.get_db()
    }
}
