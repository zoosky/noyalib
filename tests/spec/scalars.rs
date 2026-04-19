// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

// YAML spec: Scalar types and quoting

use noyalib::{from_str, Value};

#[test]
fn plain_scalar_string() {
    let v: String = from_str("hello world").unwrap();
    assert_eq!(v, "hello world");
}

#[test]
fn double_quoted_scalar() {
    let v: String = from_str("\"hello world\"").unwrap();
    assert_eq!(v, "hello world");
}

#[test]
fn single_quoted_scalar() {
    let v: String = from_str("'hello world'").unwrap();
    assert_eq!(v, "hello world");
}

#[test]
fn double_quoted_escape_sequences() {
    let v: String = from_str(r#""a\nb\tc""#).unwrap();
    assert_eq!(v, "a\nb\tc");
}

#[test]
fn double_quoted_unicode_escape() {
    let v: String = from_str(r#""caf\u00e9""#).unwrap();
    assert_eq!(v, "caf\u{00e9}");
}

#[test]
fn single_quoted_no_escapes() {
    let v: String = from_str(r"'a\nb'").unwrap();
    assert_eq!(v, r"a\nb");
}

#[test]
fn single_quoted_doubled_single_quote() {
    let v: String = from_str("'it''s'").unwrap();
    assert_eq!(v, "it's");
}

#[test]
fn empty_string_double_quoted() {
    let v: String = from_str("\"\"").unwrap();
    assert_eq!(v, "");
}

#[test]
fn empty_string_single_quoted() {
    let v: String = from_str("''").unwrap();
    assert_eq!(v, "");
}

#[test]
fn multiline_plain_scalar() {
    let v: String = from_str("a\nb\nc").unwrap();
    assert_eq!(v, "a b c");
}

#[test]
fn multiline_double_quoted() {
    let v: String = from_str("\"a\nb\nc\"").unwrap();
    assert_eq!(v, "a b c");
}

#[test]
fn string_that_looks_like_integer() {
    let v: Value = from_str("'42'").unwrap();
    assert!(v.is_string());
    assert_eq!(v.as_str(), Some("42"));
}

#[test]
fn string_that_looks_like_bool() {
    let v: Value = from_str("'true'").unwrap();
    assert!(v.is_string());
    assert_eq!(v.as_str(), Some("true"));
}

#[test]
fn string_that_looks_like_null() {
    let v: Value = from_str("'null'").unwrap();
    assert!(v.is_string());
    assert_eq!(v.as_str(), Some("null"));
}

#[test]
fn string_that_looks_like_float() {
    let v: Value = from_str("'3.14'").unwrap();
    assert!(v.is_string());
    assert_eq!(v.as_str(), Some("3.14"));
}

#[test]
fn string_with_special_yaml_chars() {
    let v: String = from_str("\"key: value\"").unwrap();
    assert_eq!(v, "key: value");
}

#[test]
fn string_with_hash() {
    let v: String = from_str("\"no # comment\"").unwrap();
    assert_eq!(v, "no # comment");
}
