use std::{any::Any, collections::HashMap};

use time::OffsetDateTime;

type AnyMap = HashMap<String, Box<dyn AnyClone + Send + Sync>>;

#[derive(Clone)]
pub struct Record {
    data: AnyMap,
    expiry: OffsetDateTime,
}

impl Record {
    pub fn unix_timestamp(&self) -> i64 {
        self.expiry.unix_timestamp()
    }
}

trait AnyClone: Any {}

impl Clone for Box<dyn AnyClone + Send + Sync> {
    fn clone(&self) -> Self {
        todo!()
    }
}
