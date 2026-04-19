// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

// YAML spec: Error cases — invalid YAML that should be rejected

use std::collections::HashMap;

use noyalib::{from_str, Value};

#[test]
fn invalid_indentation() {
    let result: Result<Value, _> = from_str("a: 1\n b: 2\n");
    // This may parse differently than expected — at minimum should not panic
    let _ = result;
}

#[test]
fn unclosed_flow_sequence() {
    let result: Result<Vec<i64>, _> = from_str("[1, 2, 3");
    assert!(result.is_err());
}

#[test]
fn unclosed_flow_mapping() {
    let result: Result<HashMap<String, i64>, _> = from_str("{a: 1, b: 2");
    assert!(result.is_err());
}

#[test]
fn tab_as_indentation() {
    // YAML spec forbids tabs for indentation
    let result: Result<Value, _> = from_str("a:\n\tb: 1\n");
    assert!(result.is_err());
}

#[test]
fn type_mismatch_string_as_int() {
    let result: Result<i64, _> = from_str("hello");
    assert!(result.is_err());
}

#[test]
fn type_mismatch_mapping_as_seq() {
    let result: Result<Vec<String>, _> = from_str("a: 1\nb: 2\n");
    assert!(result.is_err());
}

#[test]
fn type_mismatch_seq_as_mapping() {
    let result: Result<HashMap<String, String>, _> = from_str("- a\n- b\n");
    assert!(result.is_err());
}

#[test]
fn empty_yaml_is_error() {
    let result: Result<i64, _> = from_str("");
    assert!(result.is_err());
}

#[test]
fn stray_scalar_after_mapping() {
    let result: Result<HashMap<String, String>, _> = from_str("foo: bar\ninvalid\n");
    // Should fail or produce unexpected results
    let _ = result;
}

#[test]
fn max_depth_exceeded() {
    use noyalib::{from_str_with_config, ParserConfig};

    // Create YAML that nests 10 levels deep, but set limit to 5
    let yaml = "a:\n  b:\n    c:\n      d:\n        e:\n          f: 1\n";
    let config = ParserConfig::new().max_depth(5);
    let result: Result<Value, _> = from_str_with_config(yaml, &config);
    assert!(result.is_err(), "should reject excessive nesting");
}

#[test]
fn max_document_length_exceeded() {
    use noyalib::{from_str_with_config, ParserConfig};

    let config = ParserConfig::new().max_document_length(10);
    let yaml = "this is more than 10 bytes";
    let result: Result<Value, _> = from_str_with_config(yaml, &config);
    assert!(result.is_err(), "should reject oversized document");
}

#[test]
fn missing_required_struct_field() {
    use serde::Deserialize;

    #[allow(dead_code)]
    #[derive(Debug, Deserialize)]
    struct Required {
        name: String,
        age: i64,
    }

    let result: Result<Required, _> = from_str("name: John\n");
    assert!(result.is_err());
}

#[test]
fn wrong_type_in_sequence() {
    let result: Result<Vec<i64>, _> = from_str("- 1\n- hello\n- 3\n");
    assert!(result.is_err());
}

#[test]
fn invalid_escape_in_double_quote() {
    let result: Result<String, _> = from_str("\"\\z\"");
    // Invalid escape — should error
    assert!(result.is_err());
}

#[test]
fn no_panic_on_any_input() {
    // Fuzz-like test: various malformed inputs should not panic
    let inputs = [
        "",
        "---",
        "...",
        "[",
        "{",
        "- - -",
        "!!",
        "&",
        "*",
        "---\n---",
        "key: [unclosed",
        "key: {unclosed",
        ":\n:",
        "- :\n  - :",
    ];

    for input in inputs {
        let _ = from_str::<Value>(input);
    }
}
