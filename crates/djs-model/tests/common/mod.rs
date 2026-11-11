//! Helpers shared by the integration tests.

use djs_model::util::Timestamp;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;

/// Parses `json` into `T` and checks that serializing it again keeps every field of the input,
/// ignoring the fields listed in `unmodelled`.
///
/// Each entry of `unmodelled` is a path: `field`, `object.field`, or `array[].field` (every
/// element of the array).
pub(crate) fn round_trip<T: Serialize + DeserializeOwned>(json: &str, unmodelled: &[&str]) -> T {
    let value: T = serde_json::from_str(json).unwrap();
    let mut expected: Value = serde_json::from_str(json).unwrap();
    for path in unmodelled {
        let segments: Vec<&str> = path.split('.').collect();
        remove_path(&mut expected, &segments);
    }
    let actual = serde_json::to_value(&value).unwrap();
    assert_subset(&expected, &actual, "$");
    value
}

/// Removes the field at `segments` (see [`round_trip`]) from `value`.
fn remove_path(value: &mut Value, segments: &[&str]) {
    let Some((first, rest)) = segments.split_first() else {
        return;
    };
    let (key, every_element) = match first.strip_suffix("[]") {
        Some(key) => (key, true),
        None => (*first, false),
    };
    let Some(object) = value.as_object_mut() else {
        return;
    };
    if rest.is_empty() && !every_element {
        object.remove(key);
        return;
    }
    let Some(child) = object.get_mut(key) else {
        return;
    };
    if every_element {
        for element in child.as_array_mut().into_iter().flatten() {
            remove_path(element, rest);
        }
    } else {
        remove_path(child, rest);
    }
}

/// Asserts that everything in `expected` is present and equal in `actual`. Absent and
/// null/false/empty collapse into the same value (D-049), so such fields may be missing, and
/// timestamps only need to denote the same instant.
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
        (Value::String(expected), Value::String(actual)) if expected != actual => {
            // Timestamps are always written with microseconds; compare them as instants.
            let parse = |text: &str| text.parse::<Timestamp>().ok();
            assert!(
                parse(expected).is_some() && parse(expected) == parse(actual),
                "{path}: {expected:?} != {actual:?}"
            );
        }
        _ => assert_eq!(expected, actual, "{path}"),
    }
}
