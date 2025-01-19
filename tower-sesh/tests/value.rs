// Adapted from https://github.com/serde-rs/json.

//! This module asserts lossless serialization/deserialization for the subset
//! of values the `Value` enum supports.
//!
//! In other words, `T` -> `Value` -> data -> `Value` -> `T` must be lossless
//! if `T` is supported.

use std::{borrow::Borrow, fmt::Debug};

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
    type Ok;

    fn serialize(&mut self, value: &T) -> Result<Self::Ok, E>;
}

impl<F, T, E> SerializeFn<T, E> for F
where
    F: FnMut(&T) -> Result<Vec<u8>, E>,
    T: ?Sized + Serialize,
{
    type Ok = Vec<u8>;

    fn serialize(&mut self, value: &T) -> Result<Self::Ok, E> {
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

fn test_lossless<'out, 'input, 'de, T, S, D, Out, In, ESerialize, EDeserialize>(
    values: &[T],
    mut serialize: S,
    mut deserialize: D,
) where
    T: PartialEq + Debug + Serialize + Deserialize<'de>,
    S: SerializeFn<T, ESerialize, Ok = Out>,
    D: DeserializeFn<'de, T, EDeserialize, Input = In>,
    Out: Borrow<In> + 'out,
    In: ?Sized + 'input,
    'out: 'de + 'input,
    'input: 'de,
    ESerialize: Debug,
    EDeserialize: Debug,
{
    for value in values {
        let data = serialize.serialize(value).unwrap();
        let borrowed: &In = data.borrow();
        let _deserialized_value = deserialize.deserialize(borrowed).unwrap();
    }
}

#[test]
fn test_write_null() {
    test_lossless(&[()], serde_json::to_vec, serde_json::from_slice);
}
