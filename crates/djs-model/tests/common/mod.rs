//! Helpers shared by the integration tests.

use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;

/// Parses `json` into `T` and checks that serializing it again keeps every field of the input
/// (ignoring the top-level fields listed in `unmodelled`).
pub(crate) fn round_trip<T: Serialize + DeserializeOwned>(json: &str, unmodelled: &[&str]) -> T {
    let value: T = serde_json::from_str(json).unwrap();
    let mut expected: Value = serde_json::from_str(json).unwrap();
    if let Some(object) = expected.as_object_mut() {
        for field in unmodelled {
            object.remove(*field);
        }
    }
    let actual = serde_json::to_value(&value).unwrap();
    assert_subset(&expected, &actual, "$");
    value
}

/// Asserts that everything in `expected` is present and equal in `actual`. Absent and
/// null/false/empty collapse into the same value (D-049), so such fields may be missing.
pub(crate) fn assert_subset(expected: &Value, actual: &Value, path: &str) {
    match (expected, actual) {
        (Value::Object(expected), Value::Object(actual)) => {
            for (key, value) in expected {
                let Some(actual) = actual.get(key) else {
                    let collapsible = value.is_null()
                        || *value == Value::Bool(false)
                        || value.as_array().is_some_and(Vec::is_empty);
                    assert!(collapsible, "{path}.{key} was lost");
                    continue;
                };
                assert_subset(value, actual, &format!("{path}.{key}"));
            }
        }
        (Value::Array(expected), Value::Array(actual)) => {
            assert_eq!(expected.len(), actual.len(), "{path} length");
            for (index, (expected, actual)) in expected.iter().zip(actual).enumerate() {
                assert_subset(expected, actual, &format!("{path}[{index}]"));
            }
        }
        _ => assert_eq!(expected, actual, "{path}"),
    }
}
