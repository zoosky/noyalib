// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Phase 2A mutation tests.
//!
//! Each test parses a YAML document, applies a path-targeted edit
//! via [`noyalib::cst::Document::set`] or `replace_span`, and
//! checks that the result is byte-identical to the expected output
//! — surrounding indentation, comments, and other entries are
//! preserved verbatim.

#![allow(missing_docs)]

use noyalib::cst::parse_document;

#[test]
fn set_replaces_only_the_target_value() {
    let mut doc = parse_document("name: foo\nversion: 0.0.1\n").unwrap();
    doc.set("version", "0.0.2").unwrap();
    assert_eq!(doc.to_string(), "name: foo\nversion: 0.0.2\n");
}

#[test]
fn set_preserves_inline_comment() {
    let mut doc = parse_document("name: foo  # the project\nversion: 0.0.1\n").unwrap();
    doc.set("version", "0.0.2").unwrap();
    assert_eq!(
        doc.to_string(),
        "name: foo  # the project\nversion: 0.0.2\n"
    );
}

#[test]
fn set_preserves_blank_lines_between_entries() {
    let src = "name: foo\n\n\nversion: 0.0.1\n";
    let mut doc = parse_document(src).unwrap();
    doc.set("version", "0.0.2").unwrap();
    assert_eq!(doc.to_string(), "name: foo\n\n\nversion: 0.0.2\n");
}

#[test]
fn set_targets_nested_mapping_value() {
    let src = "package:\n  name: foo\n  version: 0.0.1\n";
    let mut doc = parse_document(src).unwrap();
    doc.set("package.version", "0.0.2").unwrap();
    assert_eq!(doc.to_string(), "package:\n  name: foo\n  version: 0.0.2\n");
}

#[test]
fn set_targets_sequence_index() {
    let src = "items:\n  - one\n  - two\n  - three\n";
    let mut doc = parse_document(src).unwrap();
    doc.set("items[1]", "TWO").unwrap();
    assert_eq!(doc.to_string(), "items:\n  - one\n  - TWO\n  - three\n");
}

#[test]
fn set_can_introduce_quotes() {
    let mut doc = parse_document("version: 1.2\n").unwrap();
    doc.set("version", "\"1.2\"").unwrap();
    assert_eq!(doc.to_string(), "version: \"1.2\"\n");
}

#[test]
fn set_returns_path_not_found_error() {
    let mut doc = parse_document("name: foo\n").unwrap();
    let err = doc.set("missing.key", "x").unwrap_err();
    assert!(err.to_string().contains("path not found"));
}

#[test]
fn set_rejects_invalid_replacement() {
    let mut doc = parse_document("name: foo\n").unwrap();
    // A bare `[` does not balance — splicing it leaves the stream
    // syntactically broken. The document is rolled back unchanged.
    let before = doc.to_string();
    let err = doc.set("name", "[").unwrap_err();
    assert!(err.to_string().contains("YAML parse error"));
    assert_eq!(doc.to_string(), before);
}

#[test]
fn span_at_returns_none_for_missing_path() {
    let doc = parse_document("name: foo\n").unwrap();
    assert!(doc.span_at("missing").is_none());
}

#[test]
fn get_returns_source_slice() {
    let doc = parse_document("items:\n  - one\n  - two\n").unwrap();
    assert_eq!(doc.get("items[0]"), Some("one"));
    assert_eq!(doc.get("items[1]"), Some("two"));
}

#[test]
fn replace_span_round_trip_after_no_op() {
    let src = "key: value\nother: 1\n";
    let mut doc = parse_document(src).unwrap();
    // Replace the same bytes with themselves — should be a no-op.
    let (s, e) = doc.span_at("key").unwrap();
    let original = doc.source()[s..e].to_owned();
    doc.replace_span(s, e, &original).unwrap();
    assert_eq!(doc.to_string(), src);
}

#[test]
fn replace_span_rejects_non_char_boundary() {
    // Multi-byte UTF-8 in the source, then attempt to splice mid-character.
    let mut doc = parse_document("emoji: 🦀\n").unwrap();
    // The crab emoji is 4 bytes starting at index 7. Position 8 is mid-glyph.
    let err = doc.replace_span(8, 9, "x").unwrap_err();
    assert!(err.to_string().contains("character boundary"));
}
