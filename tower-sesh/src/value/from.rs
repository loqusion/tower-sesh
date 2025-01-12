// Adapted from https://github.com/serde-rs/json.

use std::borrow::Cow;

use super::{error::ErrorImpl, Error, Map, Number, Value};

impl From<()> for Value {
    /// Convert [`()`] to [`Value::Null`].
    ///
    /// [`()`]: primitive@unit
    ///
    /// # Examples
    ///
    /// ```
    /// # use tower_sesh::Value;
    /// #
    /// let u = ();
    /// let x: Value = u.into();
    /// ```
    #[allow(clippy::let_unit_value)]
    fn from(value: ()) -> Self {
        let _ = value;
        Value::Null
    }
}

impl From<bool> for Value {
    /// Convert [boolean] to [`Value::Bool`].
    ///
    /// [boolean]: bool
    ///
    /// # Examples
    ///
    /// ```
    /// # use tower_sesh::Value;
    /// #
    /// let b = false;
    /// let x: Value = b.into();
    /// ```
    fn from(value: bool) -> Self {
        Value::Bool(value)
    }
}

macro_rules! from_integer {
    ($($ty:ident)*) => {
        $(
            impl From<$ty> for Value {
                fn from(value: $ty) -> Self {
                    Value::Number(value.into())
                }
            }
        )*
    };
}

from_integer! {
    u8 u16 u32 u64 usize
    i8 i16 i32 i64 isize
}

impl TryFrom<f32> for Value {
    type Error = Error;

    /// Convert a [32-bit floating point number] to [`Value::Number`], or return
    /// an error if infinite or NaN.
    ///
    /// [32-bit floating point number]: f32
    ///
    /// # Examples
    ///
    /// ```
    /// # use tower_sesh::Value;
    /// #
    /// let f: f32 = 13.37;
    /// let x: Value = f.try_into().unwrap();
    /// ```
    fn try_from(value: f32) -> Result<Self, Self::Error> {
        Number::from_f32(value)
            .map(Value::Number)
            .ok_or_else(|| Error::from(ErrorImpl::FloatMustBeFinite))
    }
}

impl TryFrom<f64> for Value {
    type Error = Error;

    /// Convert a [64-bit floating point number] to [`Value::Number`], or return
    /// an error if infinite or NaN.
    ///
    /// [64-bit floating point number]: f64
    ///
    /// # Examples
    ///
    /// ```
    /// # use tower_sesh::Value;
    /// #
    /// let f: f64 = 13.37;
    /// let x: Value = f.try_into().unwrap();
    /// ```
    fn try_from(value: f64) -> Result<Self, Self::Error> {
        Number::from_f64(value)
            .map(Value::Number)
            .ok_or_else(|| Error::from(ErrorImpl::FloatMustBeFinite))
    }
}

impl From<Number> for Value {
    /// Convert [`Number`] to [`Value::Number`].
    ///
    /// # Examples
    ///
    /// ```
    /// use tower_sesh::{value::Number, Value};
    ///
    /// let n = Number::from(7);
    /// let x: Value = n.into();
    /// ```
    fn from(value: Number) -> Self {
        Value::Number(value)
    }
}

impl From<String> for Value {
    /// Convert [`String`] to [`Value::String`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use tower_sesh::Value;
    /// #
    /// let s: String = "lorem".to_owned();
    /// let x: Value = s.into();
    /// ```
    fn from(value: String) -> Self {
        Value::String(value)
    }
}

impl From<&str> for Value {
    /// Convert [string slice] to [`Value::String`].
    ///
    /// [string slice]: str
    ///
    /// # Examples
    ///
    /// ```
    /// # use tower_sesh::Value;
    /// #
    /// let s: &str = "lorem";
    /// let x: Value = s.into();
    /// ```
    fn from(value: &str) -> Self {
        Value::String(value.to_owned())
    }
}

impl<'a> From<Cow<'a, str>> for Value {
    /// Convert [clone-on-write] string to [`Value::String`].
    ///
    /// [clone-on-write]: std::borrow::Cow
    ///
    /// # Examples
    ///
    /// ```
    /// use std::borrow::Cow;
    /// use tower_sesh::Value;
    ///
    /// let s: Cow<str> = Cow::Borrowed("lorem");
    /// let x: Value = s.into();
    ///
    /// let s: Cow<str> = Cow::Owned("lorem".to_owned());
    /// let x: Value = s.into();
    /// ```
    fn from(value: Cow<'a, str>) -> Self {
        Value::String(value.into_owned())
    }
}

impl<T> From<Vec<T>> for Value
where
    T: Into<Value>,
{
    /// Convert a [`Vec`] to [`Value::Array`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use tower_sesh::Value;
    /// #
    /// let v = vec!["lorem", "ipsum", "dolor"];
    /// let x: Value = v.into();
    /// ```
    fn from(value: Vec<T>) -> Self {
        Value::Array(value.into_iter().map(Into::into).collect())
    }
}

impl<T, const N: usize> From<[T; N]> for Value
where
    T: Into<Value>,
{
    /// Convert an [array] to [`Value::Array`].
    ///
    /// [array]: primitive@array
    ///
    /// # Examples
    ///
    /// ```
    /// # use tower_sesh::Value;
    /// #
    /// let v: [&str; 3] = ["lorem", "ipsum", "dolor"];
    /// let x: Value = v.into();
    /// ```
    fn from(value: [T; N]) -> Self {
        Value::Array(value.into_iter().map(Into::into).collect())
    }
}

impl<T> From<&[T]> for Value
where
    T: Into<Value> + Clone,
{
    /// Convert a [slice] to [`Value::Array`].
    ///
    /// [slice]: primitive@slice
    ///
    /// # Examples
    ///
    /// ```
    /// # use tower_sesh::Value;
    /// #
    /// let v: &[&str] = &["lorem", "ipsum", "dolor"];
    /// let x: Value = v.into();
    /// ```
    fn from(value: &[T]) -> Self {
        Value::Array(value.iter().cloned().map(Into::into).collect())
    }
}

impl<T> FromIterator<T> for Value
where
    T: Into<Value>,
{
    /// Create a [`Value::Array`] by collecting an iterator of array elements.
    ///
    /// # Examples
    ///
    /// ```
    /// # use tower_sesh::Value;
    /// #
    /// let v = std::iter::repeat(42).take(5);
    /// let x: Value = v.collect();
    /// ```
    ///
    /// ```
    /// # use tower_sesh::Value;
    /// #
    /// let v: Vec<_> = vec!["lorem", "ipsum", "dolor"];
    /// let x: Value = v.into_iter().collect();
    /// ```
    ///
    /// ```
    /// # use tower_sesh::Value;
    /// #
    /// let x: Value = Value::from_iter(vec!["lorem", "ipsum", "dolor"]);
    /// ```
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Value::Array(iter.into_iter().map(Into::into).collect())
    }
}

impl<K, V> FromIterator<(K, V)> for Value
where
    K: Into<String>,
    V: Into<Value>,
{
    /// Create a [`Value::Map`] by collecting an iterator of key-value pairs.
    ///
    /// # Examples
    ///
    /// ```
    /// # use tower_sesh::Value;
    /// #
    /// let v: Vec<_> = vec![("lorem", 40), ("ipsum", 2)];
    /// let x: Value = v.into_iter().collect();
    /// ```
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        Value::Map(
            iter.into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
        )
    }
}

impl From<Map<String, Value>> for Value {
    /// Convert [`Map`] to [`Value::Map`].
    ///
    /// # Examples
    ///
    /// ```
    /// use tower_sesh::{value::Map, Value};
    ///
    /// let mut m = Map::new();
    /// m.insert("Lorem".to_owned(), "ipsum".into());
    /// let x: Value = m.into();
    /// ```
    fn from(value: Map<String, Value>) -> Self {
        Value::Map(value)
    }
}

impl<T> From<Option<T>> for Value
where
    T: Into<Value>,
{
    /// Convert using `T`'s `Into<Value>` implementation if `Some`, or
    /// [`Value::Null`] if `None`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use tower_sesh::Value;
    /// #
    /// let opt: Option<i32> = Some(3);
    /// let x: Value = opt.into();
    /// assert_eq!(x, Value::from(3));
    ///
    /// let opt: Option<i32> = None;
    /// let x: Value = opt.into();
    /// assert_eq!(x, Value::Null);
    /// ```
    fn from(opt: Option<T>) -> Self {
        match opt {
            None => Value::Null,
            Some(value) => value.into(),
        }
    }
}
