// Adapted from https://github.com/serde-rs/json.

use std::{fmt, ops};

use super::{Map, Value};

/// A type that can be used to index into a [`Value`].
///
/// The [`get`] and [`get_mut`] methods of `Value` accept any type that
/// implements `Index`, as does the [square-bracket indexing operator]. This
/// trait is implemented for strings which are used as the index into a map,
/// and for `usize` which is used as the index into an array.
///
/// [`get`]: Value::get
/// [`get_mut`]: Value::get_mut
/// [square-bracket indexing operator]: Value#impl-Index<Idx>-for-Value
///
/// This trait is sealed and cannot be implemented for types outside of
/// `tower_sesh`.
///
/// # Examples
///
/// ```
/// # use tower_sesh::Value;
/// #
/// let value = Value::from_iter([("inner", [1, 2, 3])]);
///
/// // Data is a map so it can be indexed with a string.
/// let inner = &value["inner"];
///
/// // Inner is an array so it can be indexed with an integer.
/// let first = &inner[0];
///
/// assert_eq!(first, 1);
/// ```
pub trait Index: private::Sealed {
    /// Return `None` if the key is not already in the array or map.
    #[doc(hidden)]
    fn index_into<'v>(&self, v: &'v Value) -> Option<&'v Value>;

    /// Return `None` if the key is not already in the array or map.
    #[doc(hidden)]
    fn index_into_mut<'v>(&self, v: &'v mut Value) -> Option<&'v mut Value>;

    /// Panic if array index is out of bounds. If key is not already in the
    /// map, insert it with a value of `Null`. Panic if `Value` is a type that
    /// cannot be indexed into, except if `Value` is `Null` then it can be
    /// treated as an empty map.
    #[doc(hidden)]
    fn index_or_insert<'v>(&self, v: &'v mut Value) -> &'v mut Value;
}

impl Index for usize {
    fn index_into<'v>(&self, v: &'v Value) -> Option<&'v Value> {
        match v {
            Value::Array(vec) => vec.get(*self),
            _ => None,
        }
    }

    fn index_into_mut<'v>(&self, v: &'v mut Value) -> Option<&'v mut Value> {
        match v {
            Value::Array(vec) => vec.get_mut(*self),
            _ => None,
        }
    }

    fn index_or_insert<'v>(&self, v: &'v mut Value) -> &'v mut Value {
        match v {
            Value::Array(vec) => {
                let len = vec.len();
                vec.get_mut(*self).unwrap_or_else(|| {
                    panic!(
                        "array index out of bounds: the len is {} but the index is {}",
                        len, self
                    )
                })
            }
            _ => panic!("invalid index into non-array variant {}", Type(v)),
        }
    }
}
impl private::Sealed for usize {}

impl Index for str {
    fn index_into<'v>(&self, v: &'v Value) -> Option<&'v Value> {
        match v {
            Value::Map(map) => map.get(self),
            _ => None,
        }
    }

    fn index_into_mut<'v>(&self, v: &'v mut Value) -> Option<&'v mut Value> {
        match v {
            Value::Map(map) => map.get_mut(self),
            _ => None,
        }
    }

    fn index_or_insert<'v>(&self, v: &'v mut Value) -> &'v mut Value {
        if let Value::Null = v {
            *v = Value::Map(Map::new());
        }
        match v {
            Value::Map(map) => map.entry(self.to_owned()).or_insert(Value::Null),
            _ => panic!("invalid index into non-map variant {}", Type(v)),
        }
    }
}
impl private::Sealed for str {}

impl Index for String {
    fn index_into<'v>(&self, v: &'v Value) -> Option<&'v Value> {
        self[..].index_into(v)
    }

    fn index_into_mut<'v>(&self, v: &'v mut Value) -> Option<&'v mut Value> {
        self[..].index_into_mut(v)
    }

    fn index_or_insert<'v>(&self, v: &'v mut Value) -> &'v mut Value {
        self[..].index_or_insert(v)
    }
}
impl private::Sealed for String {}

impl<T> Index for &T
where
    T: ?Sized + Index,
{
    fn index_into<'v>(&self, v: &'v Value) -> Option<&'v Value> {
        (**self).index_into(v)
    }

    fn index_into_mut<'v>(&self, v: &'v mut Value) -> Option<&'v mut Value> {
        (**self).index_into_mut(v)
    }

    fn index_or_insert<'v>(&self, v: &'v mut Value) -> &'v mut Value {
        (**self).index_or_insert(v)
    }
}
impl<T> private::Sealed for &T where T: ?Sized + Index {}

mod private {
    pub trait Sealed {}
}

/// Used in panic messages.
struct Type<'a>(&'a Value);

impl fmt::Display for Type<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self.0 {
            Value::Null => f.write_str("Null"),
            Value::Bool(_) => f.write_str("Bool"),
            Value::Number(_) => f.write_str("Number"),
            Value::String(_) => f.write_str("String"),
            Value::ByteArray(_) => f.write_str("ByteArray"),
            Value::Array(_) => f.write_str("Array"),
            Value::Map(_) => f.write_str("Map"),
        }
    }
}

impl<Idx> ops::Index<Idx> for Value
where
    Idx: Index,
{
    type Output = Value;

    /// Index into a `Value` using the syntax `value[0]` or `value["k"]`.
    ///
    /// Returns `Value::Null` if the type of `self` does not match the type of
    /// the index, for example if the index is a string and `self` is an array
    /// or a number. Also returns `Value::Null` if the given key does not exist
    /// in the map or the given index is not within the bounds of the array.
    ///
    /// # Examples
    ///
    /// ```
    /// # use tower_sesh::Value;
    /// #
    /// let value = Value::from_iter([
    ///     ("x", Value::from_iter([
    ///         ("y", ["z", "zz"]),
    ///     ])),
    /// ]);
    ///
    /// assert_eq!(value["x"]["y"], Value::from(["z", "zz"]));
    /// assert_eq!(value["x"]["y"][0], Value::from("z"));
    ///
    /// assert_eq!(value["a"], Value::Null); // returns null for undefined values
    /// assert_eq!(value["a"]["b"], Value::Null); // does not panic
    /// ```
    fn index(&self, index: Idx) -> &Self::Output {
        static NULL: Value = Value::Null;
        index.index_into(self).unwrap_or(&NULL)
    }
}

impl<Idx> ops::IndexMut<Idx> for Value
where
    Idx: Index,
{
    /// Write into a `Value` using the syntax `value[0] = ...` or
    /// `value["k"] = ...`.
    ///
    /// If the index is a number, the value must be an array of length bigger
    /// than the index. Indexing into a value that is not an array or an array
    /// that is too small will panic.
    ///
    /// If the index is a string, the value must be a map (or null which is
    /// treated like an empty map). If the key is not already present in the
    /// map, it will be inserted with a value of null. Indexing into a value
    /// that is neither a map nor null will panic.
    ///
    /// # Examples
    ///
    /// ```
    /// # use tower_sesh::Value;
    /// #
    /// let mut value = Value::from_iter([("x", 0)]);
    ///
    /// // replace an existing key
    /// value["x"] = Value::from(1);
    ///
    /// // insert a new key
    /// value["y"] = Value::from([false, false, false]);
    ///
    /// // replace an array value
    /// value["y"][0] = Value::from(true);
    ///
    /// // inserted a deeply nested key
    /// value["a"]["b"]["c"]["d"] = Value::from(true);
    ///
    /// println!("{:?}", value);
    /// ```
    fn index_mut(&mut self, index: Idx) -> &mut Self::Output {
        index.index_or_insert(self)
    }
}
