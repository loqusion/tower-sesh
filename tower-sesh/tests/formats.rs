// Adapted from https://github.com/serde-rs/json.
// This makes additional checks for data formats other than JSON.

use std::{collections::BTreeMap, fmt::Debug};

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{
    value::{from_value, to_value},
    Value,
};

macro_rules! treemap {
    () => {
        ::std::collections::BTreeMap::new()
    };
    ($($k:expr => $v:expr),+ $(,)?) => {{
        let mut m = ::std::collections::BTreeMap::new();
        $(
            m.insert($k, $v);
        )+
        m
    }};
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
            serde_json::to_vec_pretty,
            f!(serde_json::from_slice),
        );
        check(
            *data,
            expected,
            rmp_serde::to_vec_named,
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
        (&3.0, Value::from(3.0)),
        (&3.1, Value::from(3.1)),
        (&-1.5, Value::from(-1.5)),
        (&0.5, Value::from(0.5)),
        (&f64::MIN, Value::from(f64::MIN)),
        (&f64::MAX, Value::from(f64::MAX)),
        (&f64::EPSILON, Value::from(f64::EPSILON)),
        // Edge case from:
        // https://github.com/serde-rs/json/issues/536#issuecomment-583714900
        (&2.638344616030823e-256, Value::from(2.638344616030823e-256)),
    ]);
}

#[test]
fn test_write_str() {
    check_all(&[("", Value::from("")), ("foo", Value::from("foo"))]);
}

#[test]
fn test_write_bool() {
    check_all(&[(&true, Value::from(true)), (&false, Value::from(false))]);
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
        Value::from([Value::from("foo\nbar"), Value::from(3.5)]),
    ]);

    check_all(&[(&long_test_list, long_test_list.clone())])
}

#[test]
fn test_write_object() {
    check_all(&[
        (&treemap!(), Value::from_iter([] as [(&str, ()); 0])),
        (
            &treemap!("a".to_owned() => true),
            Value::from_iter([("a", true)]),
        ),
        (
            &treemap!(
                "a".to_owned() => true,
                "b".to_owned() => false,
            ),
            Value::from_iter([("a", true), ("b", false)]),
        ),
    ]);

    check_all(&[
        (
            &treemap!(
                "a".to_owned() => treemap!(),
                "b".to_owned() => treemap!(),
                "c".to_owned() => treemap!(),
            ),
            Value::from_iter([
                ("a", Value::from_iter([] as [(&str, ()); 0])),
                ("b", Value::from_iter([] as [(&str, ()); 0])),
                ("c", Value::from_iter([] as [(&str, ()); 0])),
            ]),
        ),
        (
            &treemap!(
                "a".to_owned() => treemap!(
                    "a".to_owned() => treemap!("a".to_owned() => vec![1, 2, 3]),
                    "b".to_owned() => treemap!(),
                    "c".to_owned() => treemap!(),
                ),
                "b".to_owned() => treemap!(),
                "c".to_owned() => treemap!(),
            ),
            Value::from_iter([
                (
                    "a",
                    Value::from_iter([
                        ("a", Value::from_iter([("a", vec![1, 2, 3])])),
                        ("b", Value::from_iter([] as [(&str, ()); 0])),
                        ("c", Value::from_iter([] as [(&str, ()); 0])),
                    ]),
                ),
                ("b", Value::from_iter([] as [(&str, ()); 0])),
                ("c", Value::from_iter([] as [(&str, ()); 0])),
            ]),
        ),
        (
            &treemap!(
                "a".to_owned() => treemap!(),
                "b".to_owned() => treemap!(
                    "a".to_owned() => treemap!("a".to_owned() => vec![1, 2, 3]),
                    "b".to_owned() => treemap!(),
                    "c".to_owned() => treemap!(),
                ),
                "c".to_owned() => treemap!(),
            ),
            Value::from_iter([
                ("a", Value::from_iter([] as [(&str, ()); 0])),
                (
                    "b",
                    Value::from_iter([
                        ("a", Value::from_iter([("a", vec![1, 2, 3])])),
                        ("b", Value::from_iter([] as [(&str, ()); 0])),
                        ("c", Value::from_iter([] as [(&str, ()); 0])),
                    ]),
                ),
                ("c", Value::from_iter([] as [(&str, ()); 0])),
            ]),
        ),
        (
            &treemap!(
                "a".to_owned() => treemap!(),
                "b".to_owned() => treemap!(),
                "c".to_owned() => treemap!(
                    "a".to_owned() => treemap!("a".to_owned() => vec![1, 2, 3]),
                    "b".to_owned() => treemap!(),
                    "c".to_owned() => treemap!(),
                ),
            ),
            Value::from_iter([
                ("a", Value::from_iter([] as [(&str, ()); 0])),
                ("b", Value::from_iter([] as [(&str, ()); 0])),
                (
                    "c",
                    Value::from_iter([
                        ("a", Value::from_iter([("a", vec![1, 2, 3])])),
                        ("b", Value::from_iter([] as [(&str, ()); 0])),
                        ("c", Value::from_iter([] as [(&str, ()); 0])),
                    ]),
                ),
            ]),
        ),
    ]);

    check_all(&[(&treemap!('c' => ()), Value::from_iter([("c", ())]))]);

    let complex_obj = Value::from_iter([(
        "b",
        vec![
            Value::from_iter([("c", "\x0c\x1f\r")]),
            Value::from_iter([("c", "")]),
        ],
    )]);

    check_all(&[(&complex_obj.clone(), complex_obj)])
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

#[test]
fn test_write_enum() {
    check_all(&[
        (&Animal::Dog, to_value(Animal::Dog).unwrap()),
        (
            &Animal::Frog("Henry".to_owned(), vec![]),
            Value::from_iter([(
                "Frog",
                vec![Value::from("Henry"), Value::from([] as [isize; 0])],
            )]),
        ),
        (
            &Animal::Frog("Henry".to_owned(), vec![349]),
            Value::from_iter([("Frog", vec![Value::from("Henry"), Value::from([349])])]),
        ),
        (
            &Animal::Frog("Henry".to_owned(), vec![349, 102]),
            Value::from_iter([("Frog", vec![Value::from("Henry"), Value::from([349, 102])])]),
        ),
        (
            &Animal::Cat {
                age: 5,
                name: "Kate".to_owned(),
            },
            Value::from_iter([(
                "Cat",
                Value::from_iter([("age", Value::from(5)), ("name", Value::from("Kate"))]),
            )]),
        ),
        (
            &Animal::AntHive(vec!["Bob".to_owned(), "Stuart".to_owned()]),
            Value::from_iter([("AntHive", vec!["Bob", "Stuart"])]),
        ),
    ]);
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

#[test]
fn test_write_newtype_struct() {
    #[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
    struct Newtype(BTreeMap<String, i32>);

    let inner = Newtype(treemap!("inner".to_owned() => 123));
    let outer = treemap!("outer".to_owned() => to_value(&inner).unwrap());

    check_all(&[(&inner, Value::from_iter([("inner", 123)]))]);
    check_all(&[(
        &outer,
        Value::from_iter([("outer", Value::from_iter([("inner", 123)]))]),
    )])
}

macro_rules! check_field_reordering {
    (
        serialize: $serialize:expr,
        deserialize: $deserialize:expr,
        v1: $v1:ident : $ty1:ty,
        v2: $v2:ident : $ty2:ty,
        expected: $expected:expr
        $(,)?
    ) => {{
        let ser = $serialize($v1).unwrap();
        let de: $ty2 = $deserialize(&ser).unwrap();
        assert_eq!($v1, &de);

        let ser = $serialize($v2).unwrap();
        let de: $ty1 = $deserialize(&ser).unwrap();
        assert_eq!(&de, $v2);

        let value = to_value($v1).unwrap();
        let value2 = to_value($v2).unwrap();
        assert_eq!(&value, $expected);
        assert_eq!(&value2, $expected);

        let ser = $serialize(&value).unwrap();
        let de: Value = $deserialize(&ser).unwrap();
        assert_eq!(value, de);
        assert_eq!(&from_value::<T1>(value.clone()).unwrap(), $v1);
    }};
}

fn check_field_reordering<T1, T2>(v1: &T1, v2: &T2, expected: &Value)
where
    T1: PartialEq + PartialEq<T2> + DeserializeOwned + Serialize + Debug,
    T2: DeserializeOwned + Serialize + Debug,
{
    check_field_reordering!(
        serialize: serde_json::to_vec,
        deserialize: serde_json::from_slice,
        v1: v1: T1,
        v2: v2: T2,
        expected: expected,
    );
    check_field_reordering!(
        serialize: serde_json::to_vec_pretty,
        deserialize: serde_json::from_slice,
        v1: v1: T1,
        v2: v2: T2,
        expected: expected,
    );
    check_field_reordering!(
        serialize: rmp_serde::to_vec_named,
        deserialize: rmp_serde::from_slice,
        v1: v1: T1,
        v2: v2: T2,
        expected: expected,
    );
}

#[test]
fn test_field_reordering() {
    #[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
    #[serde(rename = "Person")]
    struct Person1 {
        age: usize,
        name: String,
    }

    #[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
    #[serde(rename = "Person")]
    struct Person2 {
        name: String,
        age: usize,
    }

    impl PartialEq<Person2> for Person1 {
        fn eq(&self, other: &Person2) -> bool {
            self.age == other.age && self.name == other.name
        }
    }

    impl PartialEq<Person1> for Person2 {
        fn eq(&self, other: &Person1) -> bool {
            self.age == other.age && self.name == other.name
        }
    }

    check_field_reordering(
        &Person1 {
            age: -1isize as usize,
            name: "Persephone".to_owned(),
        },
        &Person2 {
            name: "Persephone".to_owned(),
            age: -1isize as usize,
        },
        &Value::from_iter([
            ("age", Value::from(-1isize as usize)),
            ("name", Value::from("Persephone")),
        ]),
    );
}
