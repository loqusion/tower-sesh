// Adapted from https://github.com/serde-rs/json.

use insta::{assert_debug_snapshot, assert_snapshot};
use tower_sesh::{value::Number, Value};

#[test]
fn number() {
    assert_debug_snapshot!(Number::from(1), @"Number(1)");
    assert_debug_snapshot!(Number::from(-1), @"Number(-1)");
    assert_debug_snapshot!(Number::from_f64(1.0).unwrap(), @"Number(1.0)");

    assert_snapshot!(Number::from(1), @"1");
    assert_snapshot!(Number::from(-1), @"-1");
    assert_snapshot!(Number::from_f64(1.0).unwrap(), @"1.0");
}

#[test]
fn value_null() {
    assert_debug_snapshot!(Value::Null, @"Null");
}

#[test]
fn value_bool() {
    assert_debug_snapshot!(Value::Bool(true), @r"
    Bool(
        true,
    )
    ");
    assert_debug_snapshot!(Value::Bool(false), @r"
    Bool(
        false,
    )
    ");
}

#[test]
fn value_number() {
    assert_debug_snapshot!(Value::from(1), @"Number(1)");
    assert_debug_snapshot!(Value::from(-1), @"Number(-1)");
    assert_debug_snapshot!(Value::try_from(1.0).unwrap(), @"Number(1.0)");
    assert_snapshot!(Number::from_f64(1.0).unwrap(), @"1.0");
    assert_snapshot!(Number::from_f64(1.2e40).unwrap(), @"1.2e40");
}

#[test]
fn value_string() {
    assert_debug_snapshot!(Value::from("s"), @r#"
    String(
        "s",
    )
    "#);
}

#[test]
fn value_byte_array() {
    assert_debug_snapshot!(Value::from_bytes(b"hello\xFFworld"), @r#"
    ByteArray(
        "hello\xFFworld",
    )
    "#);
}

#[test]
fn value_array() {
    assert_debug_snapshot!(Value::from([] as [u64; 0]), @r"
    Array(
        [],
    )
    ");
    assert_debug_snapshot!(Value::from([1, 2, 3]), @r"
    Array(
        [
            Number(1),
            Number(2),
            Number(3),
        ],
    )
    ");
}

#[test]
fn value_map() {
    assert_debug_snapshot!(Value::from_iter([] as [(&str, i64); 0]), @r"
    Map(
        {},
    )
    ");
    assert_debug_snapshot!(Value::from_iter([("hello", 32), ("world", 64)]), @r#"
    Map(
        {
            "hello": Number(32),
            "world": Number(64),
        },
    )
    "#);
}

#[test]
fn error() {
    use serde::Deserialize;

    let err = i64::deserialize(Value::try_from(5.0).unwrap()).unwrap_err();
    assert_snapshot!(err, @"invalid type: floating point `5.0`, expected i64");
}
