use std::borrow::Cow;

use super::{error::ErrorImpl, Error, Number, Value};

impl From<()> for Value {
    fn from(_value: ()) -> Self {
        Value::Null
    }
}

impl From<bool> for Value {
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

    /// Convert a 32-bit floating point number to `Value::Number`, or return an
    /// error if infinite or NaN.
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

    /// Convert a 64-bit floating point number to `Value::Number`, or return an
    /// error if infinite or NaN.
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

impl From<String> for Value {
    fn from(value: String) -> Self {
        Value::String(value)
    }
}

impl From<&str> for Value {
    fn from(value: &str) -> Self {
        Value::String(value.to_owned())
    }
}

impl<'a> From<Cow<'a, str>> for Value {
    fn from(value: Cow<'a, str>) -> Self {
        Value::String(value.into_owned())
    }
}

impl<T> From<Vec<T>> for Value
where
    T: Into<Value>,
{
    fn from(value: Vec<T>) -> Self {
        Value::Array(value.into_iter().map(Into::into).collect())
    }
}

impl<T, const N: usize> From<[T; N]> for Value
where
    T: Into<Value>,
{
    fn from(value: [T; N]) -> Self {
        Value::Array(value.into_iter().map(Into::into).collect())
    }
}

impl<T> From<&[T]> for Value
where
    T: Into<Value> + Clone,
{
    fn from(value: &[T]) -> Self {
        Value::Array(value.iter().cloned().map(Into::into).collect())
    }
}

impl<T> FromIterator<T> for Value
where
    T: Into<Value>,
{
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

impl<T> From<Option<T>> for Value
where
    T: Into<Value>,
{
    fn from(opt: Option<T>) -> Self {
        match opt {
            None => Value::Null,
            Some(value) => value.into(),
        }
    }
}
