// Adapted from https://github.com/serde-rs/json.
//
// Instead of checking that a value serializes to a given JSON string, this
// checks that a value serializes to a given `tower_sesh::Value`, and
// additionally checks that the `tower_sesh::Value` serializes and deserializes
// to various data formats without any errors or data loss.

use std::{collections::BTreeMap, fmt::Debug};

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tower_sesh::{
    value::{from_value, to_value},
    Value,
};

macro_rules! treemap {
    () => {
        ::std::collections::BTreeMap::new()
    };
    ($($k:expr => $v:expr),+ $(,)?) => {
        let mut m = ::std::collections::BTreeMap::new();
        $(
            m.insert($k, v);
        )+
        m
    };
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
enum Animal {
    Dog,
    Frog(String, Vec<isize>),
    Cat { age: usize, name: String },
    AntHive(Vec<String>),
}

#[track_caller]
fn check<T, TOwned, S, D, ES, ED>(data: &T, expected: &Value, serialize: S, deserialize: D)
where
    T: PartialEq + PartialEq<TOwned> + ToOwned<Owned = TOwned> + Serialize + Debug + ?Sized,
    TOwned: DeserializeOwned + Debug,
    S: for<'a> FnOnce(&'a Value) -> Result<Vec<u8>, ES>,
    D: for<'a> FnOnce(&'a [u8]) -> Result<Value, ED>,
    ES: Debug,
    ED: Debug,
{
    let value = to_value(data).unwrap();
    assert_eq!(value, *expected);

    let serialized = serialize(&value).unwrap();
    let value_deserialized = deserialize(&serialized).unwrap();
    let data_deserialized = from_value::<TOwned>(value_deserialized.clone()).unwrap();

    assert_eq!(value, value_deserialized);
    assert_eq!(*data, data_deserialized);
}

/// Workaround for the compiler being unable to infer the lifetime
/// See https://users.rust-lang.org/t/implementation-of-fnonce-is-not-general-enough/78006/4
macro_rules! f {
    ($f:expr) => {{
        |__v: &_| ($f)(__v)
    }};
}

fn check_all<T, TOwned>(values: &[(&T, Value)])
where
    T: PartialEq + PartialEq<TOwned> + ToOwned<Owned = TOwned> + Serialize + Debug + ?Sized,
    TOwned: PartialEq + DeserializeOwned + Debug,
{
    for (data, expected) in values {
        check(
            *data,
            expected,
            serde_json::to_vec,
            f!(serde_json::from_slice),
        );
        check(
            *data,
            expected,
            rmp_serde::to_vec,
            f!(rmp_serde::from_slice),
        );
    }
}

#[test]
fn test_write_null() {
    check_all(&[(&(), Value::Null)]);
}

#[test]
fn test_write_u64() {
    check_all(&[(&3u64, Value::from(3)), (&u64::MAX, Value::from(u64::MAX))]);
}

#[test]
fn test_write_i64() {
    check_all(&[
        (&3i64, Value::from(3)),
        (&-2i64, Value::from(-2)),
        (&-1234i64, Value::from(-1234)),
        (&i64::MIN, Value::from(i64::MIN)),
    ]);
}

#[test]
fn test_write_f64() {
    check_all(&[
        (&3.0, Value::try_from(3.0).unwrap()),
        (&3.1, Value::try_from(3.1).unwrap()),
        (&-1.5, Value::try_from(-1.5).unwrap()),
        (&0.5, Value::try_from(0.5).unwrap()),
        (&f64::MIN, Value::try_from(f64::MIN).unwrap()),
        (&f64::MAX, Value::try_from(f64::MAX).unwrap()),
        (&f64::EPSILON, Value::try_from(f64::EPSILON).unwrap()),
    ]);
}

macro_rules! test_nonfinite {
    ($($name:ident : $e:expr)+) => {
        $(
            #[test]
            #[should_panic = "float must be finite"]
            fn $name() {
                check_all(&[($e, Value::Null)]);
            }
        )+
    };
}

test_nonfinite! {
    test_write_f64_pos_nan: &f64::NAN.copysign(1.0)
    test_write_f64_neg_nan: &f64::NAN.copysign(-1.0)
    test_write_f64_pos_inf: &f64::INFINITY
    test_write_f64_neg_inf: &f64::NEG_INFINITY
    test_write_f32_pos_nan: &f32::NAN.copysign(1.0)
    test_write_f32_neg_nan: &f32::NAN.copysign(-1.0)
    test_write_f32_pos_inf: &f32::INFINITY
    test_write_f32_neg_inf: &f32::NEG_INFINITY
}

#[test]
fn test_write_str() {
    check_all(&[("", Value::from("")), ("foo", Value::from("foo"))]);
}

#[test]
fn test_write_char() {
    check_all(&[
        (&'n', Value::from("n")),
        (&'"', Value::from("\"")),
        (&'\\', Value::from("\\")),
        (&'/', Value::from("/")),
        (&'\x08', Value::from("\x08")),
        (&'\x0C', Value::from("\x0C")),
        (&'\n', Value::from("\n")),
        (&'\r', Value::from("\r")),
        (&'\t', Value::from("\t")),
        (&'\x0B', Value::from("\x0B")),
        (&'\u{3A3}', Value::from("\u{3A3}")),
    ]);
}

#[test]
fn test_write_list() {
    check_all(&[
        (&vec![], Value::from([] as [bool; 0])),
        (&vec![true], Value::from([true])),
        (&vec![true, false], Value::from([true, false])),
    ]);

    check_all(&[
        (
            &vec![vec![], vec![], vec![]] as &Vec<Vec<i32>>,
            Value::from(vec![vec![], vec![], vec![]] as Vec<Vec<i32>>),
        ),
        (
            &vec![vec![1, 2, 3], vec![], vec![]],
            Value::from(vec![vec![1, 2, 3], vec![], vec![]]),
        ),
        (
            &vec![vec![], vec![1, 2, 3], vec![]],
            Value::from(vec![vec![], vec![1, 2, 3], vec![]]),
        ),
        (
            &vec![vec![], vec![], vec![1, 2, 3]],
            Value::from(vec![vec![], vec![], vec![1, 2, 3]]),
        ),
    ]);

    let long_test_list = Value::from([
        Value::from(false),
        Value::Null,
        Value::from([Value::from("foo\nbar"), Value::try_from(3.5).unwrap()]),
    ]);

    check_all(&[(&long_test_list, long_test_list.clone())])
}

// TODO: Fill in the rest
#[test]
#[ignore = "unimplemented"]
fn test_write_object() {
    check_all(&[(
        &treemap!() as &BTreeMap<String, ()>,
        Value::from_iter([] as [(&str, ()); 0]),
    )]);
}

#[test]
fn test_write_tuple() {
    check_all(&[(&(5,), Value::from([5]))]);

    check_all(&[(
        &(5, (6, "abc".to_owned())),
        Value::from([
            Value::from(5),
            Value::from([Value::from(6), Value::from("abc")]),
        ]),
    )]);
}

// TODO: Fill in the rest
#[test]
#[ignore = "unimplemented"]
fn test_write_enum() {
    check_all(&[(&Animal::Dog, to_value(Animal::Dog).unwrap())]);
}

#[test]
fn test_write_option() {
    check_all(&[
        (&None, Value::Null),
        (&Some("jodhpurs".to_owned()), Value::from("jodhpurs")),
    ]);

    check_all(&[(
        &Some(vec!["foo".to_owned(), "bar".to_owned()]),
        Value::from(["foo", "bar"]),
    )])
}
