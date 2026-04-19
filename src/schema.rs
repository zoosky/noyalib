//! YAML 1.2 schema validation helpers.
//!
//! This module provides validation functions for the three standard YAML 1.2
//! schemas:
//!
//! - **Failsafe Schema**: The most basic schema, supporting only strings,
//!   sequences, and mappings.
//! - **JSON Schema**: Extends failsafe with nulls, booleans, integers, and
//!   floats (JSON-compatible).
//! - **Core Schema**: The default schema, with more flexible tag resolution.
//!
//! # Example
//!
//! ```rust
//! use noyalib::{
//!     from_str, validate_core_schema, validate_json_schema, Value,
//! };
//!
//! let yaml = "count: 42\nenabled: true";
//! let value: Value = from_str(yaml).unwrap();
//!
//! // Validate against JSON schema
//! assert!(validate_json_schema(&value).is_ok());
//!
//! // Validate against core schema
//! assert!(validate_core_schema(&value).is_ok());
//! ```

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use crate::error::{Error, Result};
use crate::value::{Number, Value};

/// Validate a value against the YAML 1.2 Failsafe Schema.
///
/// The failsafe schema is the most basic schema and supports only:
/// - Strings (tag:yaml.org,2002:str)
/// - Sequences (tag:yaml.org,2002:seq)
/// - Mappings (tag:yaml.org,2002:map)
///
/// # Errors
///
/// Returns an error if the value contains types not supported by the failsafe
/// schema (nulls, booleans, numbers, or tagged values).
///
/// # Example
///
/// ```rust
/// use noyalib::{validate_failsafe_schema, Mapping, Value};
///
/// // Valid failsafe values
/// let valid = Value::String("hello".to_string());
/// assert!(validate_failsafe_schema(&valid).is_ok());
///
/// // Invalid: numbers not allowed in failsafe
/// let invalid = Value::from(42);
/// assert!(validate_failsafe_schema(&invalid).is_err());
/// ```
pub fn validate_failsafe_schema(value: &Value) -> Result<()> {
    validate_failsafe_recursive(value, &mut String::from("root"))
}

fn validate_failsafe_recursive(value: &Value, path: &mut String) -> Result<()> {
    use std::fmt::Write;
    match value {
        Value::String(_) => Ok(()),
        Value::Sequence(seq) => {
            let base_len = path.len();
            for (i, item) in seq.iter().enumerate() {
                write!(path, "[{i}]").unwrap();
                validate_failsafe_recursive(item, path)?;
                path.truncate(base_len);
            }
            Ok(())
        }
        Value::Mapping(map) => {
            let base_len = path.len();
            for (key, val) in map.iter() {
                write!(path, ".{key}").unwrap();
                validate_failsafe_recursive(val, path)?;
                path.truncate(base_len);
            }
            Ok(())
        }
        Value::Null => Err(Error::Invalid(format!(
            "failsafe schema: null not allowed at {path}"
        ))),
        Value::Bool(_) => Err(Error::Invalid(format!(
            "failsafe schema: boolean not allowed at {path}"
        ))),
        Value::Number(_) => Err(Error::Invalid(format!(
            "failsafe schema: number not allowed at {path}"
        ))),
        Value::Tagged(_) => Err(Error::Invalid(format!(
            "failsafe schema: tagged values not allowed at {path}"
        ))),
    }
}

/// Validate a value against the YAML 1.2 JSON Schema.
///
/// The JSON schema extends the failsafe schema with types that are
/// compatible with JSON:
/// - Null (tag:yaml.org,2002:null)
/// - Boolean (tag:yaml.org,2002:bool)
/// - Integer (tag:yaml.org,2002:int)
/// - Float (tag:yaml.org,2002:float)
/// - Strings, sequences, and mappings from failsafe
///
/// # Errors
///
/// Returns an error if the value contains types not supported by JSON schema
/// (primarily tagged values with non-standard tags).
///
/// # Example
///
/// ```rust
/// use noyalib::{validate_json_schema, Value};
///
/// // Valid JSON schema values
/// let valid = Value::from(42);
/// assert!(validate_json_schema(&valid).is_ok());
///
/// let null = Value::Null;
/// assert!(validate_json_schema(&null).is_ok());
/// ```
pub fn validate_json_schema(value: &Value) -> Result<()> {
    validate_json_recursive(value, &mut String::from("root"))
}

fn validate_json_recursive(value: &Value, path: &mut String) -> Result<()> {
    use std::fmt::Write;
    match value {
        Value::Null | Value::Bool(_) | Value::String(_) => Ok(()),
        Value::Number(n) => {
            // JSON doesn't support NaN or Infinity
            if let Number::Float(f) = n {
                if f.is_nan() || f.is_infinite() {
                    return Err(Error::Invalid(format!(
                        "JSON schema: NaN/Infinity not allowed at {path}"
                    )));
                }
            }
            Ok(())
        }
        Value::Sequence(seq) => {
            let base_len = path.len();
            for (i, item) in seq.iter().enumerate() {
                write!(path, "[{i}]").unwrap();
                validate_json_recursive(item, path)?;
                path.truncate(base_len);
            }
            Ok(())
        }
        Value::Mapping(map) => {
            let base_len = path.len();
            for (key, val) in map.iter() {
                write!(path, ".{key}").unwrap();
                validate_json_recursive(val, path)?;
                path.truncate(base_len);
            }
            Ok(())
        }
        Value::Tagged(_) => Err(Error::Invalid(format!(
            "JSON schema: tagged values not allowed at {path}"
        ))),
    }
}

/// Validate a value against the YAML 1.2 Core Schema.
///
/// The core schema is the default YAML schema and is the most flexible.
/// It supports all JSON schema types plus:
/// - NaN and Infinity for floats
/// - Tagged values with standard YAML tags
///
/// # Errors
///
/// Returns an error only for malformed values (this is very permissive).
///
/// # Example
///
/// ```rust
/// use noyalib::{validate_core_schema, Value};
///
/// // Core schema accepts everything
/// let value = Value::from(f64::NAN);
/// assert!(validate_core_schema(&value).is_ok());
///
/// let null = Value::Null;
/// assert!(validate_core_schema(&null).is_ok());
/// ```
pub fn validate_core_schema(value: &Value) -> Result<()> {
    validate_core_recursive(value, &mut String::from("root"))
}

fn validate_core_recursive(value: &Value, path: &mut String) -> Result<()> {
    use std::fmt::Write;
    match value {
        Value::Null | Value::Bool(_) | Value::String(_) | Value::Number(_) => Ok(()),
        Value::Sequence(seq) => {
            let base_len = path.len();
            for (i, item) in seq.iter().enumerate() {
                write!(path, "[{i}]").unwrap();
                validate_core_recursive(item, path)?;
                path.truncate(base_len);
            }
            Ok(())
        }
        Value::Mapping(map) => {
            let base_len = path.len();
            for (key, val) in map.iter() {
                write!(path, ".{key}").unwrap();
                validate_core_recursive(val, path)?;
                path.truncate(base_len);
            }
            Ok(())
        }
        Value::Tagged(tagged) => {
            let base_len = path.len();
            write!(path, "!{}", tagged.tag()).unwrap();
            let result = validate_core_recursive(tagged.value(), path);
            path.truncate(base_len);
            result
        }
    }
}

/// Check if a value is valid JSON (can be serialized to JSON).
///
/// This is equivalent to `validate_json_schema` but returns a boolean
/// instead of a Result.
#[must_use]
pub fn is_json_compatible(value: &Value) -> bool {
    validate_json_schema(value).is_ok()
}

/// Check if a value uses only failsafe schema types.
#[must_use]
pub fn is_failsafe_compatible(value: &Value) -> bool {
    validate_failsafe_schema(value).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::Mapping;

    #[test]
    fn test_failsafe_valid() {
        assert!(validate_failsafe_schema(&Value::String("test".to_string())).is_ok());
        assert!(validate_failsafe_schema(&Value::Sequence(vec![])).is_ok());
        assert!(validate_failsafe_schema(&Value::Mapping(Mapping::new())).is_ok());
    }

    #[test]
    fn test_failsafe_invalid() {
        assert!(validate_failsafe_schema(&Value::Null).is_err());
        assert!(validate_failsafe_schema(&Value::Bool(true)).is_err());
        assert!(validate_failsafe_schema(&Value::from(42)).is_err());
    }

    #[test]
    fn test_json_valid() {
        assert!(validate_json_schema(&Value::Null).is_ok());
        assert!(validate_json_schema(&Value::Bool(true)).is_ok());
        assert!(validate_json_schema(&Value::from(42)).is_ok());
        assert!(validate_json_schema(&Value::from(3.125)).is_ok());
        assert!(validate_json_schema(&Value::String("test".to_string())).is_ok());
    }

    #[test]
    fn test_json_invalid_nan() {
        let nan = Value::from(f64::NAN);
        assert!(validate_json_schema(&nan).is_err());
    }

    #[test]
    fn test_json_invalid_infinity() {
        let inf = Value::from(f64::INFINITY);
        assert!(validate_json_schema(&inf).is_err());
    }

    #[test]
    fn test_core_accepts_all() {
        assert!(validate_core_schema(&Value::Null).is_ok());
        assert!(validate_core_schema(&Value::Bool(true)).is_ok());
        assert!(validate_core_schema(&Value::from(42)).is_ok());
        assert!(validate_core_schema(&Value::from(f64::NAN)).is_ok());
        assert!(validate_core_schema(&Value::from(f64::INFINITY)).is_ok());
    }

    #[test]
    fn test_is_json_compatible() {
        assert!(is_json_compatible(&Value::from(42)));
        assert!(!is_json_compatible(&Value::from(f64::NAN)));
    }

    #[test]
    fn test_is_failsafe_compatible() {
        assert!(is_failsafe_compatible(&Value::String("test".to_string())));
        assert!(!is_failsafe_compatible(&Value::from(42)));
    }
}
