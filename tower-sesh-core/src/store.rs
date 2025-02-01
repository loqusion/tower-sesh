use std::{error::Error as StdError, fmt};

use async_trait::async_trait;
use time::OffsetDateTime;

use crate::SessionKey;

type Result<T, E = Error> = std::result::Result<T, E>;

// TODO: MUST mention that the data format used by a session store must be
// self-describing, i.e. it implements `Deserializer::deserialize_any`. (This
// is because `Value`'s `Deserialize::deserialize` delegates to
// `Deserializer::deserialize_any`.)
//
// TODO: `Record` should be removed because you can't construct a `Record` without
// transferring ownership or cloning.
//
// TODO: Method signatures need a rework.
#[async_trait]
pub trait SessionStore<T>: 'static + Send + Sync {
    async fn create(&self, record: &Record<T>) -> Result<SessionKey>;

    async fn load(&self, session_key: &SessionKey) -> Result<Option<Record<T>>>;

    async fn update(&self, session_key: &SessionKey, record: &Record<T>) -> Result<()>;

    async fn delete(&self, session_key: &SessionKey) -> Result<()>;
}

#[derive(Clone, Debug)]
pub struct Record<T> {
    data: T,
    expiry: OffsetDateTime,
}

impl<T> Record<T> {
    pub fn unix_timestamp(&self) -> i64 {
        self.expiry.unix_timestamp()
    }
}

#[derive(Debug)]
pub enum Error {}

impl StdError for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {}
    }
}
