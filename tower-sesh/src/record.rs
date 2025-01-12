use std::{borrow::Borrow, collections::HashMap, hash::Hash};

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use time::OffsetDateTime;

use crate::{
    value::{from_value, from_value_borrowed, to_value, Error},
    Value,
};

#[derive(Clone, Debug)]
pub struct Record {
    data: HashMap<String, Value>,
    expiry: OffsetDateTime,
}

// Data manipulation
impl Record {
    pub fn insert<T>(&mut self, k: String, v: T) -> Result<Option<Value>, Error>
    where
        T: Serialize,
    {
        to_value(v).map(|v| self.data.insert(k, v))
    }

    pub fn get_value<Q>(&self, k: &Q) -> Option<&Value>
    where
        String: Borrow<Q>,
        Q: ?Sized + Ord + Eq + Hash,
    {
        self.data.get(k)
    }

    pub fn get_value_mut<Q>(&mut self, k: &Q) -> Option<&mut Value>
    where
        String: Borrow<Q>,
        Q: ?Sized + Ord + Eq + Hash,
    {
        self.data.get_mut(k)
    }

    fn get_borrowed<'de, Q, T>(&'de self, k: &Q) -> Result<Option<T>, Error>
    where
        String: Borrow<Q>,
        Q: ?Sized + Ord + Eq + Hash,
        T: Deserialize<'de>,
    {
        self.get_value(k).map(from_value_borrowed).transpose()
    }

    fn get_owned<Q, T>(&self, k: &Q) -> Result<Option<T>, Error>
    where
        String: Borrow<Q>,
        Q: ?Sized + Ord + Eq + Hash,
        T: DeserializeOwned,
    {
        self.get_value(k)
            .map(|value| from_value(value.clone()))
            .transpose()
    }

    pub fn remove_value<Q>(&mut self, key: &Q) -> Option<Value>
    where
        String: Borrow<Q>,
        Q: ?Sized + Ord + Eq + Hash,
    {
        self.data.remove(key)
    }

    pub fn clear(&mut self) {
        self.data.clear();
    }
}

impl Record {
    pub fn unix_timestamp(&self) -> i64 {
        self.expiry.unix_timestamp()
    }
}
