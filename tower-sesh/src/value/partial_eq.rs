use super::Value;

fn eq_str(value: &Value, other: &str) -> bool {
    value.as_str() == Some(other)
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

macro_rules! partial_eq_numeric {
    ($($eq:ident [$($ty:ty)*])*) => {
        $($(
            impl PartialEq<$ty> for Value {
                fn eq(&self, other: &$ty) -> bool {
                    $eq(self, other as _)
                }
            }

            impl PartialEq<&$ty> for Value {
                fn eq(&self, other: &&$ty) -> bool {
                    $eq(self, other as _)
                }
            }
        )*)*
    };
}
