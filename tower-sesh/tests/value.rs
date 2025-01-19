// Adapted from https://github.com/serde-rs/json.

//! This module asserts lossless serialization/deserialization for the subset
//! of values the `Value` enum supports.
//!
//! In other words, `T` -> `Value` -> data -> `Value` -> `T` must be lossless
//! if `T` is supported.

use std::{
    borrow::Borrow,
    fmt::{Debug, Display},
};

use serde::{de::DeserializeOwned, Deserialize, Deserializer, Serialize, Serializer};
use tower_sesh::Value;

macro_rules! assert_value {
    ($value:expr, @$snapshot:literal) => {{
        let serialized = serde_json::to_string(&$value).unwrap();
        insta::assert_snapshot!(serialized, @$snapshot);
    }};
}

/// A function which serializes a `T` into `Ok`.
trait SerializeFn<T: ?Sized, E> {
    type Output;

    fn serialize(&mut self, value: &T) -> Result<Self::Output, E>;
}

impl<F, T, E> SerializeFn<T, E> for F
where
    F: FnMut(&T) -> Result<Vec<u8>, E>,
    T: ?Sized + Serialize,
{
    type Output = Vec<u8>;

    fn serialize(&mut self, value: &T) -> Result<Self::Output, E> {
        self(value)
    }
}

/// A function which deserializes `Input` into `T`.
trait DeserializeFn<'de, T, E> {
    type Input: ?Sized;

    fn deserialize(&mut self, input: &'de Self::Input) -> Result<T, E>;
}

impl<'de, F, T, E> DeserializeFn<'de, T, E> for F
where
    F: FnMut(&'de [u8]) -> Result<T, E>,
    T: Deserialize<'de>,
{
    type Input = [u8];

    fn deserialize(&mut self, input: &'de Self::Input) -> Result<T, E> {
        self(input)
    }
}

fn test_lossless<T, S, D, Output, ESerialize, EDeserialize>(
    values: &[T],
    mut serialize: S,
    mut deserialize: D,
) where
    S: SerializeFn<T, ESerialize, Output = Output>,
    D: for<'de> DeserializeFn<'de, T, EDeserialize, Input = [u8]>,
    Output: Borrow<[u8]>,
    ESerialize: Debug,
    EDeserialize: Debug,
{
    for value in values {
        let data = serialize.serialize(value).unwrap();
        let borrowed = data.borrow();
        let _deserialized_value = deserialize.deserialize(borrowed).unwrap();
    }
}

fn test_thing<T, D, E>(value: &T, d: D)
where
    for<'a> T: Serialize + Debug + 'a,
    D: for<'a> FnOnce(&'a [u8]) -> Result<T, E>,
    E: Debug,
{
    let data = serde_json::to_vec(&value).unwrap();
    let v = d(&data).unwrap();
    println!("{v:?}");
    assert!(false, "success");
}

#[test]
fn test_write_null() {
    // test_thing(&"hi", |bytes| serde_json::from_slice(bytes));
    // test_lossless(&[()], serde_json::to_vec, serde_json::from_slice);
}
