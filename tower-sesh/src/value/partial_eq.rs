// Adapted from https://github.com/serde-rs/json.

use super::Value;

fn eq_i64(value: &Value, other: i64) -> bool {
    value.as_i64() == Some(other)
}

fn eq_u64(value: &Value, other: u64) -> bool {
    value.as_u64() == Some(other)
}

fn eq_f32(value: &Value, other: f32) -> bool {
    match value {
        Value::Number(n) => n.as_f32() == Some(other),
        _ => false,
    }
}

fn eq_f64(value: &Value, other: f64) -> bool {
    value.as_f64() == Some(other)
}

fn eq_bool(value: &Value, other: bool) -> bool {
    value.as_bool() == Some(other)
}

fn eq_str(value: &Value, other: &str) -> bool {
    value.as_str() == Some(other)
}

macro_rules! partial_eq_numeric {
    ($($eq:ident [$($ty:ty)*])*) => {
        $($(
            impl PartialEq<$ty> for Value {
                fn eq(&self, other: &$ty) -> bool {
                    $eq(self, *other as _)
                }
            }

            impl<'a> PartialEq<$ty> for &'a Value {
                fn eq(&self, other: &$ty) -> bool {
                    $eq(self, *other as _)
                }
            }

            impl<'a> PartialEq<$ty> for &'a mut Value {
                fn eq(&self, other: &$ty) -> bool {
                    $eq(self, *other as _)
                }
            }
        )*)*
    };
}

partial_eq_numeric! {
    eq_i64[i8 i16 i32 i64 isize]
    eq_u64[u8 u16 u32 u64 usize]
    eq_f32[f32]
    eq_f64[f64]
    eq_bool[bool]
}

macro_rules! partial_eq_str {
    ($($ty:ty)*) => {
        $(
            impl PartialEq<$ty> for Value {
                fn eq(&self, other: &$ty) -> bool {
                    eq_str(self, other)
                }
            }
        )*
    };
}

partial_eq_str! {
    str &str String
}
