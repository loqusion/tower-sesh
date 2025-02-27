// The code in this module is derived from the `serde_json` crate by @dtolnay.
// TODO: Discuss notable changes from `serde_json::Value`
//
// Dual licensed MIT and Apache 2.0.

//! The `Value` enum, a loosely typed way of representing any session value.
//!
//! For more information, see the [documentation for `Value`].
//!
//! [documentation for `Value`]: Value

use std::{
    fmt::{self, Write},
    mem,
};

use serde::{de::DeserializeOwned, Deserialize, Serialize};

mod de;
mod error;
mod from;
mod index;
mod number;
mod partial_eq;
mod ser;

pub mod map;

#[doc(inline)]
pub use self::error::Error;
#[doc(inline)]
pub use self::index::Index;
#[doc(inline)]
pub use self::map::Map;
#[doc(inline)]
pub use self::number::Number;

/// A loosely typed value that can be stored in a session.
///
/// Though this data structure looks quite similar to (and is, in fact, largely
/// based on) [`serde_json::Value`], there are a few key differences:
///
/// - Special floating-point values ([∞][infinity], [−∞][neg-infinity], and
///   [NaN]) are not implicitly coerced to `Null` in conversion methods.
/// - Byte arrays are added, enabling more efficient
///   serialization/deserialization for some data formats.
///
/// [`serde_json::Value`]: https://docs.rs/serde_json/latest/serde_json/enum.Value.html
/// [infinity]: f64::INFINITY
/// [neg-infinity]: f64::NEG_INFINITY
/// [NaN]: f64::NAN
#[derive(Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum Value {
    #[default]
    Null,
    Bool(bool),
    Number(Number),
    String(String),
    ByteArray(Vec<u8>),
    Array(Vec<Value>),
    Map(Map<String, Value>),
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Null => f.write_str("Null"),
            Value::Bool(boolean) => f.debug_tuple("Bool").field(boolean).finish(),
            Value::Number(number) => fmt::Debug::fmt(number, f),
            Value::String(string) => f.debug_tuple("String").field(string).finish(),
            Value::ByteArray(bytes) => f
                .debug_tuple("ByteArray")
                .field(&DebugByteArray(bytes))
                .finish(),
            Value::Array(vec) => f.debug_tuple("Array").field(vec).finish(),
            Value::Map(map) => f.debug_tuple("Map").field(map).finish(),
        }
    }
}

struct DebugByteArray<'a>(&'a [u8]);

// Copied from https://doc.rust-lang.org/1.84.1/src/core/str/lossy.rs.html#113-145.
impl fmt::Debug for DebugByteArray<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_char('"')?;

        for chunk in self.0.utf8_chunks() {
            // Valid part.
            // Here we partially parse UTF-8 again which is suboptimal.
            {
                let valid = chunk.valid();
                let mut from = 0;
                for (i, c) in valid.char_indices() {
                    let esc = c.escape_debug();
                    // If char needs escaping, flush backlog so far and write, else skip
                    if esc.len() != 1 {
                        f.write_str(&valid[from..i])?;
                        for c in esc {
                            f.write_char(c)?;
                        }
                        from = i + c.len_utf8();
                    }
                }
                f.write_str(&valid[from..])?;
            }

            // Broken parts of string as hex escape.
            for &b in chunk.invalid() {
                write!(f, "\\x{:02X}", b)?;
            }
        }

        f.write_char('"')
    }
}

impl Value {
    /// Index into an array or map. A string index can be used to access a value
    /// in a map, and a `usize` index can be used to access an element of an
    /// array.
    ///
    /// Returns `None` if the type of `self` does not match the type of the
    /// index, for example if the index is a string and `self` is an array or a
    /// number. Also returns `None` if the given key does not exist in the map
    /// or the given index is not within the bounds of the array.
    ///
    /// ```
    /// # use tower_sesh::Value;
    /// #
    /// let map = Value::from_iter([("A", 65), ("B", 66), ("C", 64)]);
    /// assert_eq!(*map.get("A").unwrap(), 65);
    ///
    /// let array = Value::from(["A", "B", "C"]);
    /// assert_eq!(*array.get(2).unwrap(), "C");
    ///
    /// assert_eq!(array.get("A"), None);
    /// ```
    ///
    /// Square brackets can also be used to index into a value in a more concise
    /// way. This returns `Value::Null` in cases where `get` would have returned
    /// `None`.
    ///
    /// ```
    /// # use tower_sesh::Value;
    /// #
    /// let map = Value::from_iter([
    ///     ("A", &["a", "á", "à"] as &[_]),
    ///     ("B", &["b", "b́"]),
    ///     ("C", &["c", "ć", "ć̣", "ḉ"]),
    /// ]);
    /// assert_eq!(map["B"][0], "b");
    ///
    /// assert_eq!(map["D"], Value::Null);
    /// assert_eq!(map[0]["x"]["y"]["z"], Value::Null);
    /// ```
    pub fn get<I: Index>(&self, index: I) -> Option<&Value> {
        index.index_into(self)
    }

    /// Mutably index into an array or map. A string index can be used to access
    /// a value in a map, and a `usize` index can be used to access an element
    /// of an array.
    ///
    /// Returns `None` if the type of `self` does not match the type of the
    /// index, for example if the index is a string and `self` is an array or a
    /// number. Also returns `None` if the given key does not exist in the map
    /// or the given index is not within the bounds of the array.
    ///
    /// ```
    /// # use tower_sesh::Value;
    /// #
    /// let mut map = Value::from_iter([("A", 65), ("B", 66), ("C", 67)]);
    /// *map.get_mut("A").unwrap() = Value::from(69);
    ///
    /// let mut array = Value::from(["A", "B", "C"]);
    /// *array.get_mut(2).unwrap() = Value::from("D");
    /// ```
    pub fn get_mut<I: Index>(&mut self, index: I) -> Option<&mut Value> {
        index.index_into_mut(self)
    }

    /// Returns `true` if the `Value` is a `Map`. Returns `false` otherwise.
    ///
    /// For any `Value` on which `is_map` returns `true`, [`as_map`] and
    /// [`as_map_mut`] are guaranteed to return the [`Map`] representation.
    ///
    /// [`as_map`]: Value::as_map
    /// [`as_map_mut`]: Value::as_map_mut
    ///
    /// ```
    /// # use tower_sesh::Value;
    /// #
    /// let map = Value::from_iter([
    ///     ("a", Value::from_iter([("nested", true)])),
    ///     ("b", Value::from(["an", "array"])),
    /// ]);
    ///
    /// assert!(map.is_map());
    /// assert!(map["a"].is_map());
    ///
    /// assert!(!map["b"].is_map())
    /// ```
    pub fn is_map(&self) -> bool {
        self.as_map().is_some()
    }

    /// If the `Value` is a `Map`, returns the associated [`Map`]. Returns
    /// `None` otherwise.
    ///
    /// ```
    /// # use tower_sesh::Value;
    /// #
    /// let v = Value::from_iter([
    ///     ("a", Value::from_iter([("nested", true)])),
    ///     ("b", Value::from(["an", "array"])),
    /// ]);
    ///
    /// assert_eq!(v["a"].as_map().unwrap().len(), 1);
    ///
    /// assert_eq!(v["b"].as_map(), None);
    /// ```
    pub fn as_map(&self) -> Option<&Map<String, Value>> {
        match self {
            Value::Map(map) => Some(map),
            _ => None,
        }
    }

    /// If the `Value` is a `Map`, returns the associated mutable [`Map`].
    /// Returns `None` otherwise.
    ///
    /// ```
    /// # use tower_sesh::{value::Map, Value};
    /// #
    /// let mut v = Value::from_iter([
    ///     ("a", Value::from_iter([("nested", true)])),
    /// ]);
    ///
    /// v["a"].as_map_mut().unwrap().clear();
    /// assert_eq!(v, Value::from_iter([("a", Value::Map(Map::new()))]));
    /// ```
    pub fn as_map_mut(&mut self) -> Option<&mut Map<String, Value>> {
        match self {
            Value::Map(map) => Some(map),
            _ => None,
        }
    }

    /// Returns `true` if the `Value` is an `Array`. Returns `false` otherwise.
    ///
    /// For any `Value` on which `is_array` returns true, [`as_array`] and
    /// [`as_array_mut`] are guaranteed to return the vector representing the
    /// array.
    ///
    /// **NOTE**: If the `Value` is a `ByteArray`, this method will return
    /// `false`.
    ///
    /// [`as_array`]: Value::as_array
    /// [`as_array_mut`]: Value::as_array_mut
    ///
    /// ```
    /// # use tower_sesh::Value;
    /// #
    /// let map = Value::from_iter([
    ///     ("a", Value::from(["an", "array"])),
    ///     ("b", Value::from_bytes(b"a byte array")),
    ///     ("c", Value::from_iter([("a", "map")])),
    /// ]);
    ///
    /// assert!(map["a"].is_array());
    ///
    /// assert!(!map["b"].is_array());
    /// assert!(!map["c"].is_array());
    /// ```
    pub fn is_array(&self) -> bool {
        self.as_array().is_some()
    }

    /// If the `Value` is an `Array`, returns the associated vector. Returns
    /// `None` otherwise.
    ///
    /// **NOTE**: If the `Value` is a `ByteArray`, this method will return
    /// `None`.
    ///
    /// ```
    /// # use tower_sesh::Value;
    /// #
    /// let v = Value::from_iter([
    ///     ("a", Value::from(["an", "array"])),
    ///     ("b", Value::from_bytes(b"a byte array")),
    ///     ("c", Value::from_iter([("a", "map")])),
    /// ]);
    ///
    /// assert_eq!(v["a"].as_array().unwrap().len(), 2);
    ///
    /// assert_eq!(v["b"].as_array(), None);
    /// assert_eq!(v["c"].as_array(), None);
    /// ```
    pub fn as_array(&self) -> Option<&Vec<Value>> {
        match self {
            Value::Array(vec) => Some(vec),
            _ => None,
        }
    }

    /// If the `Value` is an `Array`, returns the associated mutable vector.
    /// Returns `None` otherwise.
    ///
    /// **NOTE**: If the `Value` is a `ByteArray`, this method will return
    /// `None`.
    ///
    /// ```
    /// # use tower_sesh::{value::Map, Value};
    /// #
    /// let mut v = Value::from_iter([
    ///     ("a", ["an", "array"]),
    /// ]);
    ///
    /// v["a"].as_array_mut().unwrap().clear();
    /// assert_eq!(v, Value::from_iter([("a", &[] as &[&str])]));
    /// ```
    pub fn as_array_mut(&mut self) -> Option<&mut Vec<Value>> {
        match self {
            Value::Array(vec) => Some(vec),
            _ => None,
        }
    }

    /// Returns `true` if the `Value` is a `String`. Returns `false` otherwise.
    ///
    /// For any `Value` on which `is_string` returns `true`, [`as_str`] is
    /// guaranteed to return the string slice.
    ///
    /// [`as_str`]: Value::as_str
    ///
    /// ```
    /// # use tower_sesh::Value;
    /// #
    /// let v = Value::from_iter([
    ///     ("a", Value::from("some string")),
    ///     ("b", Value::from_bytes(b"some bytes")),
    ///     ("c", Value::from(false)),
    /// ]);
    ///
    /// assert!(v["a"].is_string());
    ///
    /// assert!(!v["b"].is_string());
    /// assert!(!v["c"].is_string());
    /// ```
    pub fn is_string(&self) -> bool {
        self.as_str().is_some()
    }

    /// If the `Value` is a `String`, returns the associated `str`. Returns
    /// `None` otherwise.
    ///
    /// ```
    /// # use tower_sesh::Value;
    /// #
    /// let v = Value::from_iter([
    ///     ("a", Value::from("some string")),
    ///     ("b", Value::from_bytes(b"some bytes")),
    ///     ("c", Value::from(false)),
    /// ]);
    ///
    /// assert_eq!(v["a"].as_str(), Some("some string"));
    ///
    /// assert_eq!(v["b"].as_str(), None);
    /// assert_eq!(v["c"].as_str(), None);
    /// ```
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    /// Returns `true` if the `Value` is a `ByteArray`. Returns `false`
    /// otherwise.
    ///
    /// For any `Value` on which `is_bytes` returns `true`, [`as_bytes`] is
    /// guaranteed to return the byte slice.
    ///
    /// [`as_bytes`]: Value::as_bytes
    ///
    /// ```
    /// # use tower_sesh::Value;
    /// #
    /// let v = Value::from_iter([
    ///     ("a", Value::from_bytes(b"some bytes")),
    ///     ("b", Value::from(false)),
    ///     ("c", Value::from("some string")),
    /// ]);
    ///
    /// assert!(v["a"].is_bytes());
    ///
    /// assert!(!v["b"].is_bytes());
    /// assert!(!v["c"].is_bytes());
    /// ```
    pub fn is_bytes(&self) -> bool {
        self.as_bytes().is_some()
    }

    /// If the `Value` is a `ByteArray`, returns the associated `&[u8]`.
    /// Returns `None` otherwise.
    ///
    /// ```
    /// # use tower_sesh::Value;
    /// #
    /// let v = Value::from_iter([
    ///     ("a", Value::from_bytes(b"some bytes")),
    ///     ("b", Value::from(false)),
    ///     ("c", Value::from("some string")),
    /// ]);
    ///
    /// assert_eq!(v["a"].as_bytes(), Some(b"some bytes".as_slice()));
    ///
    /// assert_eq!(v["b"].as_bytes(), None);
    /// assert_eq!(v["c"].as_bytes(), None);
    /// ```
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Value::ByteArray(bytes) => Some(bytes),
            _ => None,
        }
    }

    /// Returns `true` if the `Value` is a `Number`. Returns `false` otherwise.
    ///
    /// For any `Value` on which `is_number` returns `true`, [`as_number`] is
    /// guaranteed to return the [`Number`].
    ///
    /// [`as_number`]: Value::as_number
    ///
    /// ```
    /// # use tower_sesh::Value;
    /// #
    /// let v = Value::from_iter([
    ///     ("a", Value::from(1)),
    ///     ("b", Value::from("2")),
    /// ]);
    ///
    /// assert!(v["a"].is_number());
    ///
    /// assert!(!v["b"].is_number());
    /// ```
    pub fn is_number(&self) -> bool {
        self.as_number().is_some()
    }

    /// If the `Value` is a `Number`, returns the associated [`Number`]. Returns
    /// `None` otherwise.
    ///
    /// ```
    /// # use tower_sesh::{value::Number, Value};
    /// #
    /// let v = Value::from_iter([
    ///     ("a", Value::from(1)),
    ///     ("b", Value::try_from(2.2f64).unwrap_or_default()),
    ///     ("c", Value::from(-3)),
    ///     ("d", Value::from("4")),
    /// ]);
    ///
    /// assert_eq!(v["a"].as_number(), Some(&Number::from(1u64)));
    /// assert_eq!(v["b"].as_number(), Some(&Number::from_f64(2.2).unwrap()));
    /// assert_eq!(v["c"].as_number(), Some(&Number::from(-3i64)));
    ///
    /// assert_eq!(v["d"].as_number(), None);
    /// ```
    pub fn as_number(&self) -> Option<&Number> {
        match self {
            Value::Number(number) => Some(number),
            _ => None,
        }
    }

    /// Returns `true` if the `Value` is an integer between [`i64::MIN`] and
    /// [`i64::MAX`].
    ///
    /// For any `Value` on which `is_i64` returns `true`, [`as_i64`] is
    /// guaranteed to return the integer value.
    ///
    /// [`as_i64`]: Value::as_i64
    ///
    /// ```
    /// # use tower_sesh::Value;
    /// #
    /// let big = i64::max_value() as u64 + 10;
    /// let v = Value::from_iter([
    ///     ("a", Value::from(64)),
    ///     ("b", Value::from(big)),
    ///     ("c", Value::try_from(256.0).unwrap_or_default()),
    /// ]);
    ///
    /// assert!(v["a"].is_i64());
    ///
    /// // Greater than `i64::MAX`.
    /// assert!(!v["b"].is_i64());
    ///
    /// // Numbers with a decimal point are not considered integers.
    /// assert!(!v["c"].is_i64());
    /// ```
    pub fn is_i64(&self) -> bool {
        match self {
            Value::Number(number) => number.is_i64(),
            _ => false,
        }
    }

    /// Returns `true` if the `Value` is an integer between `0` and
    /// [`u64::MAX`].
    ///
    /// For any `Value` on which `is_u64` returns `true`, [`as_u64`] is
    /// guaranteed to return the integer value.
    ///
    /// [`as_u64`]: Value::as_u64
    ///
    /// ```
    /// # use tower_sesh::Value;
    /// #
    /// let v = Value::from_iter([
    ///     ("a", Value::from(64)),
    ///     ("b", Value::from(-64)),
    ///     ("c", Value::try_from(256.0).unwrap_or_default()),
    /// ]);
    ///
    /// assert!(v["a"].is_u64());
    ///
    /// // Negative integer.
    /// assert!(!v["b"].is_u64());
    ///
    /// // Numbers with a decimal point are not considered integers.
    /// assert!(!v["c"].is_u64());
    /// ```
    pub fn is_u64(&self) -> bool {
        match self {
            Value::Number(number) => number.is_u64(),
            _ => false,
        }
    }

    /// Returns `true` if the `Value` is a number that can be represented by
    /// `f64`.
    ///
    /// For any `Value` on which `is_f64` returns `true`, [`as_f64`] is
    /// guaranteed to return the floating point value.
    ///
    /// This function returns `true` if and only if both [`is_i64`] and
    /// [`is_u64`] return `false`.
    ///
    /// [`as_f64`]: Value::as_f64
    /// [`is_i64`]: Value::is_i64
    /// [`is_u64`]: Value::is_u64
    ///
    /// ```
    /// # use tower_sesh::Value;
    /// #
    /// let v = Value::from_iter([
    ///     ("a", Value::try_from(256.0).unwrap_or_default()),
    ///     ("b", Value::from(64)),
    ///     ("c", Value::from(-64)),
    /// ]);
    ///
    /// assert!(v["a"].is_f64());
    ///
    /// // Integers.
    /// assert!(!v["b"].is_f64());
    /// assert!(!v["c"].is_f64());
    /// ```
    pub fn is_f64(&self) -> bool {
        match self {
            Value::Number(number) => number.is_f64(),
            _ => false,
        }
    }

    /// If the `Value` is an integer, represent it as `i64` if possible. Returns
    /// `None` otherwise.
    ///
    /// ```
    /// # use tower_sesh::Value;
    /// #
    /// let big = i64::max_value() as u64 + 10;
    /// let v = Value::from_iter([
    ///     ("a", Value::from(64)),
    ///     ("b", Value::from(big)),
    ///     ("c", Value::try_from(256.0).unwrap_or_default()),
    /// ]);
    ///
    /// assert_eq!(v["a"].as_i64(), Some(64));
    /// assert_eq!(v["b"].as_i64(), None);
    /// assert_eq!(v["c"].as_i64(), None);
    /// ```
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Value::Number(number) => number.as_i64(),
            _ => None,
        }
    }

    /// If the `Value` is an integer, represent it as `u64` if possible. Returns
    /// `None` otherwise.
    ///
    /// ```
    /// # use tower_sesh::Value;
    /// #
    /// let v = Value::from_iter([
    ///     ("a", Value::from(64)),
    ///     ("b", Value::from(-64)),
    ///     ("c", Value::try_from(256.0).unwrap_or_default()),
    /// ]);
    ///
    /// assert_eq!(v["a"].as_u64(), Some(64));
    /// assert_eq!(v["b"].as_u64(), None);
    /// assert_eq!(v["c"].as_u64(), None);
    /// ```
    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Value::Number(number) => number.as_u64(),
            _ => None,
        }
    }

    /// If the `Value` is a number, represent it as `f64` if possible. Returns
    /// `None` otherwise.
    ///
    /// ```
    /// # use tower_sesh::Value;
    /// #
    /// let v = Value::from_iter([
    ///     ("a", Value::try_from(256.0).unwrap_or_default()),
    ///     ("b", Value::from(64)),
    ///     ("c", Value::from(-64)),
    /// ]);
    ///
    /// assert_eq!(v["a"].as_f64(), Some(256.0));
    /// assert_eq!(v["b"].as_f64(), Some(64.0));
    /// assert_eq!(v["c"].as_f64(), Some(-64.0));
    /// ```
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Number(number) => number.as_f64(),
            _ => None,
        }
    }

    /// Returns `true` if the `Value` is a Boolean. Returns `false` otherwise.
    ///
    /// For any `Value` on which `is_boolean` returns `true`, [`as_bool`] is
    /// guaranteed to return the boolean value.
    ///
    /// [`as_bool`]: Value::as_bool
    ///
    /// ```
    /// # use tower_sesh::Value;
    /// #
    /// let v = Value::from_iter([
    ///     ("a", Value::from(false)),
    ///     ("b", Value::from("false")),
    /// ]);
    ///
    /// assert!(v["a"].is_boolean());
    ///
    /// // The string `"false"` is a string, not a boolean.
    /// assert!(!v["b"].is_boolean());
    /// ```
    pub fn is_boolean(&self) -> bool {
        self.as_bool().is_some()
    }

    /// If the `Value` is a Boolean, returns the associated `bool`. Returns
    /// `None` otherwise.
    ///
    /// ```
    /// # use tower_sesh::Value;
    /// #
    /// let v = Value::from_iter([
    ///     ("a", Value::from(false)),
    ///     ("b", Value::from("false")),
    /// ]);
    ///
    /// assert_eq!(v["a"].as_bool(), Some(false));
    ///
    /// // The string `"false"` is a string, not a boolean.
    /// assert_eq!(v["b"].as_bool(), None);
    /// ```
    pub fn as_bool(&self) -> Option<bool> {
        match *self {
            Value::Bool(b) => Some(b),
            _ => None,
        }
    }

    /// Returns `true` if the `Value` is a `Null`. Returns `false` otherwise.
    ///
    /// For any `Value` on which `is_null` returns `true`, [`as_null`] is
    /// guaranteed to return `Some(())`.
    ///
    /// [`as_null`]: Value::as_null
    ///
    /// ```
    /// # use tower_sesh::Value;
    /// #
    /// let v = Value::from_iter([
    ///     ("a", Value::Null),
    ///     ("b", Value::try_from(f64::NAN).unwrap_or_default()),
    ///     ("c", Value::from(false)),
    /// ]);
    ///
    /// assert!(v["a"].is_null());
    /// assert!(v["b"].is_null());
    ///
    /// // The boolean `false` is not null.
    /// assert!(!v["c"].is_null());
    /// ```
    pub fn is_null(&self) -> bool {
        self.as_null().is_some()
    }

    /// If the `Value` is a `Null`, returns `()`. Returns `None` otherwise.
    ///
    /// ```
    /// # use tower_sesh::Value;
    /// #
    /// let v = Value::from_iter([
    ///     ("a", Value::Null),
    ///     ("b", Value::try_from(f64::NAN).unwrap_or_default()),
    ///     ("c", Value::from(false)),
    /// ]);
    ///
    /// assert_eq!(v["a"].as_null(), Some(()));
    /// assert_eq!(v["b"].as_null(), Some(()));
    ///
    /// // The boolean `false` is not null.
    /// assert_eq!(v["c"].as_null(), None);
    /// ```
    pub fn as_null(&self) -> Option<()> {
        match *self {
            Value::Null => Some(()),
            _ => None,
        }
    }

    /// Takes the value out of the `Value`, leaving a `Null` in its place.
    ///
    /// ```
    /// # use tower_sesh::Value;
    /// #
    /// let mut v = Value::from_iter([("x", "y")]);
    /// assert_eq!(v["x"].take(), "y");
    /// assert_eq!(v.get("x"), Some(&Value::Null));
    /// ```
    pub fn take(&mut self) -> Value {
        mem::replace(self, Value::Null)
    }
}

impl Value {
    /// Create a `Value::ByteArray`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use tower_sesh::Value;
    /// #
    /// let v = Value::from_bytes(b"some bytes");
    /// let v = Value::from_bytes([82, 117, 115, 116]);
    /// let v = Value::from_bytes(vec![115, 101, 115, 104]);
    /// ```
    pub fn from_bytes<B>(bytes: B) -> Value
    where
        B: Into<Vec<u8>>,
    {
        Value::ByteArray(bytes.into())
    }
}

#[doc(hidden)]
pub fn to_value<T>(value: T) -> Result<Value, Error>
where
    T: Serialize,
{
    value.serialize(ser::Serializer)
}

#[doc(hidden)]
pub fn from_value<T>(value: Value) -> Result<T, Error>
where
    T: DeserializeOwned,
{
    T::deserialize(value)
}

#[doc(hidden)]
pub fn from_value_borrowed<'de, T>(value: &'de Value) -> Result<T, Error>
where
    T: Deserialize<'de>,
{
    T::deserialize(value)
}
