//! Lightweight JSON Schema validator for the `--output-schema` flag.
//!
//! Validates a `serde_json::Value` against a JSON Schema document (draft 7
//! subset).  Supported keywords:
//!
//! - `type`         — string, number, integer, boolean, array, object, null
//! - `properties`   — nested schemas for object properties
//! - `required`     — list of required property names
//! - `items`        — schema for every element of an array
//! - `enum`         — closed set of allowed values
//! - `minimum` / `maximum`   — numeric range
//! - `minLength` / `maxLength` — string length
//! - `minItems` / `maxItems`  — array length
//!
//! Any schema keyword not listed above is silently ignored.

use serde_json::Value;

/// A list of human-readable validation errors.
pub type ValidationErrors = Vec<String>;

/// Validate `value` against `schema` and return collected errors.
///
/// Returns `Ok(())` when the value is valid, or `Err(errors)` with at least
/// one error message when it is not.
pub fn validate(value: &Value, schema: &Value) -> Result<(), ValidationErrors> {
    let mut errors = ValidationErrors::new();
    validate_inner(value, schema, "$", &mut errors);
    if errors.is_empty() { Ok(()) } else { Err(errors) }
}

// ── Internal recursive validator ──────────────────────────────────────────────

fn validate_inner(value: &Value, schema: &Value, path: &str, errors: &mut ValidationErrors) {
    let obj = match schema.as_object() {
        Some(o) => o,
        None => return, // empty/non-object schema — everything valid
    };

    // ── type ─────────────────────────────────────────────────────────────────
    if let Some(type_val) = obj.get("type") {
        let allowed: Vec<&str> = match type_val {
            Value::String(s) => vec![s.as_str()],
            Value::Array(arr) => arr.iter().filter_map(|v| v.as_str()).collect(),
            _ => vec![],
        };
        if !allowed.is_empty() && !allowed.iter().any(|t| type_matches(value, t)) {
            errors.push(format!(
                "{}: expected type {:?}, got {}",
                path,
                allowed,
                json_type_name(value)
            ));
            return; // further checks would be noisy
        }
    }

    // ── enum ─────────────────────────────────────────────────────────────────
    if let Some(Value::Array(variants)) = obj.get("enum") {
        if !variants.contains(value) {
            errors.push(format!(
                "{}: value not in enum {:?}",
                path,
                variants.iter().map(|v| v.to_string()).collect::<Vec<_>>()
            ));
        }
    }

    // ── string constraints ────────────────────────────────────────────────────
    if let Some(s) = value.as_str() {
        if let Some(min) = obj.get("minLength").and_then(|v| v.as_u64()) {
            if (s.chars().count() as u64) < min {
                errors.push(format!("{}: string length {} < minLength {}", path, s.len(), min));
            }
        }
        if let Some(max) = obj.get("maxLength").and_then(|v| v.as_u64()) {
            if (s.chars().count() as u64) > max {
                errors.push(format!("{}: string length {} > maxLength {}", path, s.len(), max));
            }
        }
    }

    // ── numeric constraints ───────────────────────────────────────────────────
    if let Some(n) = value.as_f64() {
        if let Some(min) = obj.get("minimum").and_then(|v| v.as_f64()) {
            if n < min {
                errors.push(format!("{}: value {} < minimum {}", path, n, min));
            }
        }
        if let Some(max) = obj.get("maximum").and_then(|v| v.as_f64()) {
            if n > max {
                errors.push(format!("{}: value {} > maximum {}", path, n, max));
            }
        }
    }

    // ── object constraints ────────────────────────────────────────────────────
    if let Some(map) = value.as_object() {
        // required
        if let Some(Value::Array(req)) = obj.get("required") {
            for field in req {
                if let Some(key) = field.as_str() {
                    if !map.contains_key(key) {
                        errors.push(format!("{}: missing required property '{}'", path, key));
                    }
                }
            }
        }
        // properties
        if let Some(Value::Object(props)) = obj.get("properties") {
            for (key, sub_schema) in props {
                if let Some(child) = map.get(key) {
                    let child_path = format!("{}.{}", path, key);
                    validate_inner(child, sub_schema, &child_path, errors);
                }
            }
        }
    }

    // ── array constraints ─────────────────────────────────────────────────────
    if let Some(arr) = value.as_array() {
        if let Some(min) = obj.get("minItems").and_then(|v| v.as_u64()) {
            if (arr.len() as u64) < min {
                errors.push(format!("{}: array length {} < minItems {}", path, arr.len(), min));
            }
        }
        if let Some(max) = obj.get("maxItems").and_then(|v| v.as_u64()) {
            if (arr.len() as u64) > max {
                errors.push(format!("{}: array length {} > maxItems {}", path, arr.len(), max));
            }
        }
        if let Some(items_schema) = obj.get("items") {
            for (i, item) in arr.iter().enumerate() {
                let item_path = format!("{}[{}]", path, i);
                validate_inner(item, items_schema, &item_path, errors);
            }
        }
    }
}

fn type_matches(value: &Value, type_name: &str) -> bool {
    match type_name {
        "string"  => value.is_string(),
        "number"  => value.is_number(),
        "integer" => value.is_i64() || value.is_u64(),
        "boolean" => value.is_boolean(),
        "array"   => value.is_array(),
        "object"  => value.is_object(),
        "null"    => value.is_null(),
        _         => true, // unknown type — pass
    }
}

fn json_type_name(v: &Value) -> &'static str {
    match v {
        Value::Null      => "null",
        Value::Bool(_)   => "boolean",
        Value::Number(n) => if n.is_f64() { "number" } else { "integer" },
        Value::String(_) => "string",
        Value::Array(_)  => "array",
        Value::Object(_) => "object",
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn type_string_valid() {
        let schema = json!({"type": "string"});
        assert!(validate(&json!("hello"), &schema).is_ok());
    }

    #[test]
    fn type_string_wrong_type() {
        let schema = json!({"type": "string"});
        let errs = validate(&json!(42), &schema).unwrap_err();
        assert!(!errs.is_empty());
        assert!(errs[0].contains("expected type"));
    }

    #[test]
    fn required_field_missing() {
        let schema = json!({
            "type": "object",
            "required": ["name", "age"],
            "properties": {
                "name": {"type": "string"},
                "age":  {"type": "integer"}
            }
        });
        let errs = validate(&json!({"name": "Alice"}), &schema).unwrap_err();
        assert!(errs.iter().any(|e| e.contains("age")));
    }

    #[test]
    fn required_fields_all_present() {
        let schema = json!({
            "type": "object",
            "required": ["name"],
            "properties": {"name": {"type": "string"}}
        });
        assert!(validate(&json!({"name": "Alice", "extra": 1}), &schema).is_ok());
    }

    #[test]
    fn nested_property_type_mismatch() {
        let schema = json!({
            "type": "object",
            "properties": {
                "count": {"type": "integer"}
            }
        });
        let errs = validate(&json!({"count": "not-a-number"}), &schema).unwrap_err();
        assert!(errs.iter().any(|e| e.contains("count")));
    }

    #[test]
    fn min_length_pass_and_fail() {
        let schema = json!({"type": "string", "minLength": 3});
        assert!(validate(&json!("abc"), &schema).is_ok());
        let errs = validate(&json!("ab"), &schema).unwrap_err();
        assert!(errs[0].contains("minLength"));
    }

    #[test]
    fn min_max_numeric() {
        let schema = json!({"type": "number", "minimum": 0.0, "maximum": 100.0});
        assert!(validate(&json!(50), &schema).is_ok());
        let errs = validate(&json!(-1), &schema).unwrap_err();
        assert!(errs[0].contains("minimum"));
    }

    #[test]
    fn enum_valid_and_invalid() {
        let schema = json!({"enum": ["a", "b", "c"]});
        assert!(validate(&json!("a"), &schema).is_ok());
        let errs = validate(&json!("d"), &schema).unwrap_err();
        assert!(errs[0].contains("enum"));
    }

    #[test]
    fn array_items_validated() {
        let schema = json!({
            "type": "array",
            "items": {"type": "string"}
        });
        assert!(validate(&json!(["x", "y"]), &schema).is_ok());
        let errs = validate(&json!(["x", 1]), &schema).unwrap_err();
        assert!(errs.iter().any(|e| e.contains("[1]")));
    }

    #[test]
    fn array_min_max_items() {
        let schema = json!({"type": "array", "minItems": 2, "maxItems": 4});
        assert!(validate(&json!([1, 2, 3]), &schema).is_ok());
        let errs = validate(&json!([1]), &schema).unwrap_err();
        assert!(errs[0].contains("minItems"));
    }
}
