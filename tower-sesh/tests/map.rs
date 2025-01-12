// Adapted from https://github.com/serde-rs/json.

use tower_sesh::{value::Map, Value};

#[ignore = "unimplemented"]
#[test]
fn test_sorted_order() {
    const EXPECTED: &[&str] = &["a", "b", "c"];

    let v: Value = serde_json::from_str(r#"{"b":null,"a":null,"c":null}"#).unwrap();
    let keys: Vec<_> = v.as_map().unwrap().keys().collect();
    assert_eq!(keys, EXPECTED);
}

#[ignore = "unimplemented"]
#[test]
fn test_append() {
    const EXPECTED: &[&str] = &["a", "b", "c"];

    let mut v: Value = serde_json::from_str(r#"{"b":null,"a":null,"c":null}"#).unwrap();
    let val = v.as_map_mut().unwrap();
    let mut m = Map::new();
    m.append(val);
    let keys: Vec<_> = m.keys().collect();

    assert_eq!(keys, EXPECTED);
    assert!(!val.is_empty());
}

#[ignore = "unimplemented"]
#[test]
fn test_retain() {
    let mut v: Value = serde_json::from_str(r#"{"b":null,"a":null,"c":null}"#).unwrap();
    let val = v.as_map_mut().unwrap();
    val.retain(|k, _| k.as_str() != "b");

    let keys: Vec<_> = val.keys().collect();
    assert_eq!(keys, &["a", "c"]);
}
