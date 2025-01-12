// Adapted from https://github.com/serde-rs/json.

use std::{fmt, hash::Hash};

use serde::{
    de::{self, Unexpected, Visitor},
    forward_to_deserialize_any, Deserialize, Serialize,
};

use super::error::Error;

/// Represents a number, whether integer or floating point.
///
/// May only represent values which are representable by [`i64`], [`u64`], or
/// [finite] [`f64`].
///
/// [finite]: f64::is_finite
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Number {
    n: NumberImpl,
}

#[derive(Copy, Clone)]
enum NumberImpl {
    PosInt(u64),
    /// Always less than zero.
    NegInt(i64),
    /// Always finite.
    Float(f64),
}

impl PartialEq for NumberImpl {
    fn eq(&self, other: &Self) -> bool {
        use NumberImpl::*;
        match (self, other) {
            (PosInt(a), PosInt(b)) => a.eq(b),
            (NegInt(a), NegInt(b)) => a.eq(b),
            (Float(a), Float(b)) => a.eq(b),
            _ => false,
        }
    }
}

// NaN cannot be represented, so this is valid
impl Eq for NumberImpl {}

impl Hash for NumberImpl {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        use NumberImpl::*;
        match *self {
            PosInt(i) => i.hash(state),
            NegInt(i) => i.hash(state),
            Float(f) => {
                if f == 0.0f64 {
                    0.0f64.to_bits().hash(state)
                } else {
                    f.to_bits().hash(state)
                }
            }
        }
    }
}

impl Number {
    /// Returns `true` if the `Number` is an integer between [`i64::MIN`] and
    /// [`i64::MAX`].
    ///
    /// For any `Number` on which `is_i64` returns `true`, [`as_i64`] is
    /// guaranteed to return the integer value.
    ///
    /// [`as_i64`]: Number::as_i64
    pub fn is_i64(&self) -> bool {
        use NumberImpl::*;
        match self.n {
            PosInt(v) => v <= i64::MAX as u64,
            NegInt(_) => true,
            Float(_) => false,
        }
    }

    /// Returns `true` if the `Number` is an integer between `0` and
    /// [`u64::MAX`].
    ///
    /// For any `Number` on which `is_u64` returns `true`, [`as_u64`] is
    /// guaranteed to return the integer value.
    ///
    /// [`as_u64`]: Number::as_u64
    pub fn is_u64(&self) -> bool {
        use NumberImpl::*;
        match self.n {
            PosInt(_) => true,
            NegInt(_) | Float(_) => false,
        }
    }

    /// Returns `true` if the `Number` can be represented by [`f64`].
    ///
    /// For any `Number` on which `is_f64` returns `true`, [`as_f64`] is
    /// guaranteed to return the floating point value.
    ///
    /// This function returns `true` if and only if both [`is_i64`] and
    /// [`is_u64`] return `false`.
    ///
    /// [`as_f64`]: Number::as_f64
    /// [`is_i64`]: Number::is_i64
    /// [`is_u64`]: Number::is_u64
    pub fn is_f64(&self) -> bool {
        use NumberImpl::*;
        match self.n {
            Float(_) => true,
            PosInt(_) | NegInt(_) => false,
        }
    }

    /// If the `Number` is an integer, represent it as [`i64`] if possible.
    /// Returns `None` otherwise.
    pub fn as_i64(&self) -> Option<i64> {
        use NumberImpl::*;
        match self.n {
            PosInt(n) => {
                if n <= i64::MAX as u64 {
                    Some(n as i64)
                } else {
                    None
                }
            }
            NegInt(n) => Some(n),
            Float(_) => None,
        }
    }

    /// If the `Number` is an integer, represent it as [`u64`] if possible.
    /// Returns `None` otherwise.
    pub fn as_u64(&self) -> Option<u64> {
        use NumberImpl::*;
        match self.n {
            PosInt(n) => Some(n),
            NegInt(_) | Float(_) => None,
        }
    }

    /// Represents the number as [`f64`] if possible. Returns `None` otherwise.
    pub fn as_f64(&self) -> Option<f64> {
        use NumberImpl::*;
        match self.n {
            PosInt(n) => Some(n as f64),
            NegInt(n) => Some(n as f64),
            Float(n) => Some(n),
        }
    }

    /// Converts a [finite] [`f64`] to a `Number`. Infinite or NaN values are
    /// not valid `Number`s.
    ///
    /// [finite]: f64::is_finite
    ///
    /// ```
    /// # use tower_sesh::value::Number;
    /// #
    /// assert!(Number::from_f64(256.0).is_some());
    ///
    /// assert!(Number::from_f64(f64::NAN).is_none());
    /// ```
    pub fn from_f64(f: f64) -> Option<Number> {
        if f.is_finite() {
            let n = NumberImpl::Float(f);
            Some(Number { n })
        } else {
            None
        }
    }

    /// Converts an [`i128`] to a `Number`. Returns `None` for numbers smaller
    /// than [`i64::MIN`] or larger than [`u64::MAX`].
    ///
    /// ```
    /// # use tower_sesh::value::Number;
    /// #
    /// assert!(Number::from_i128(256).is_some());
    /// ```
    pub fn from_i128(i: i128) -> Option<Number> {
        let n = if let Ok(u) = u64::try_from(i) {
            NumberImpl::PosInt(u)
        } else if let Ok(i) = i64::try_from(i) {
            NumberImpl::NegInt(i)
        } else {
            return None;
        };
        Some(Number { n })
    }

    /// Converts a [`u128`] to a `Number`. Returns `None` for numbers larger
    /// than [`u64::MAX`].
    ///
    /// ```
    /// # use tower_sesh::value::Number;
    /// #
    /// assert!(Number::from_u128(256).is_some());
    /// ```
    pub fn from_u128(u: u128) -> Option<Number> {
        let n = if let Ok(u) = u64::try_from(u) {
            NumberImpl::PosInt(u)
        } else {
            return None;
        };
        Some(Number { n })
    }

    pub(super) fn as_f32(&self) -> Option<f32> {
        use NumberImpl::*;
        match self.n {
            PosInt(n) => Some(n as f32),
            NegInt(n) => Some(n as f32),
            Float(n) => Some(n as f32),
        }
    }

    pub(super) fn from_f32(f: f32) -> Option<Number> {
        if f.is_finite() {
            let n = NumberImpl::Float(f as f64);
            Some(Number { n })
        } else {
            None
        }
    }
}

impl fmt::Display for Number {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        use NumberImpl::*;
        match self.n {
            PosInt(u) => formatter.write_str(itoa::Buffer::new().format(u)),
            NegInt(i) => formatter.write_str(itoa::Buffer::new().format(i)),
            Float(f) => formatter.write_str(ryu::Buffer::new().format_finite(f)),
        }
    }
}

impl fmt::Debug for Number {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Number({})", self)
    }
}

impl Serialize for Number {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use NumberImpl::*;
        match self.n {
            PosInt(u) => serializer.serialize_u64(u),
            NegInt(i) => serializer.serialize_i64(i),
            Float(f) => serializer.serialize_f64(f),
        }
    }
}

impl<'de> Deserialize<'de> for Number {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct NumberVisitor;

        impl Visitor<'_> for NumberVisitor {
            type Value = Number;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a number")
            }

            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E> {
                Ok(v.into())
            }

            fn visit_i128<E>(self, v: i128) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Number::from_i128(v).ok_or_else(|| de::Error::custom("number out of range"))
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E> {
                Ok(v.into())
            }

            fn visit_u128<E>(self, v: u128) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Number::from_u128(v).ok_or_else(|| de::Error::custom("number out of range"))
            }

            fn visit_f32<E>(self, v: f32) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Number::from_f32(v).ok_or_else(|| de::Error::custom("not a valid number"))
            }

            fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Number::from_f64(v).ok_or_else(|| de::Error::custom("not a valid number"))
            }
        }

        deserializer.deserialize_any(NumberVisitor)
    }
}

macro_rules! deserialize_any {
    () => {
        fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Error>
        where
            V: Visitor<'de>,
        {
            match self.n {
                NumberImpl::PosInt(u) => visitor.visit_u64(u),
                NumberImpl::NegInt(i) => visitor.visit_i64(i),
                NumberImpl::Float(f) => visitor.visit_f64(f),
            }
        }
    };
}

impl<'de> serde::Deserializer<'de> for Number {
    type Error = Error;

    deserialize_any!();

    forward_to_deserialize_any! {
        i8 i16 i32 i64 i128
        u8 u16 u32 u64 u128 f32 f64
        bool char str string bytes byte_buf option unit unit_struct
        newtype_struct seq tuple tuple_struct map struct enum identifier
        ignored_any
    }
}

impl<'de> serde::Deserializer<'de> for &Number {
    type Error = Error;

    deserialize_any!();

    forward_to_deserialize_any! {
        i8 i16 i32 i64 i128
        u8 u16 u32 u64 u128 f32 f64
        bool char str string bytes byte_buf option unit unit_struct
        newtype_struct seq tuple tuple_struct map struct enum identifier
        ignored_any
    }
}

macro_rules! from_unsigned {
    ($($ty:ty)*) => {
        $(
            impl From<$ty> for Number {
                fn from(u: $ty) -> Self {
                    let n = NumberImpl::PosInt(u as u64);
                    Number { n }
                }
            }
        )*
    };
}

macro_rules! from_signed {
    ($($ty:ty)*) => {
        $(
            impl From<$ty> for Number {
                fn from(i: $ty) -> Self {
                    let n = if i < 0 {
                        NumberImpl::NegInt(i as i64)
                    } else {
                        NumberImpl::PosInt(i as u64)
                    };
                    Number { n }
                }
            }
        )*
    };
}

from_unsigned! {
    u8 u16 u32 u64 usize
}
from_signed! {
    i8 i16 i32 i64 isize
}

impl Number {
    #[cold]
    pub(crate) fn unexpected(&self) -> Unexpected {
        use NumberImpl::*;
        match self.n {
            PosInt(u) => Unexpected::Unsigned(u),
            NegInt(i) => Unexpected::Signed(i),
            Float(f) => Unexpected::Float(f),
        }
    }
}
