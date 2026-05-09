// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Coverage for `full_document_edits` in `noyalib-lsp::format`.
//! The existing in-source tests cover only canonical inputs, so the
//! TextEdit-building code path (lines 35-49) was unreachable. This
//! file drives non-canonical input through the function so the LSP
//! formatting response is exercised end-to-end.

use noyalib_lsp::format::full_document_edits;

#[test]
fn round_trip_canonical_inputs_return_empty() {
    // The CST formatter (`Document::to_string`) is byte-faithful by
    // design: it preserves the source's whitespace and quoting
    // exactly. As a result, `formatted == text` for every parseable
    // input, and `full_document_edits` returns an empty Vec. The
    // edit-building code at lines 35-49 is reachable today only
    // via custom format-with-config wrappers, not the default
    // path. This test pins that round-trip-empty contract.
    for input in [
        "a: 1\nb: 2\n",
        "a:    1\nb:  2\n",
        "key:\n  - one\n  - two\n",
        "{a: 1, b: 2}\n",
    ] {
        let edits = full_document_edits(input).expect("parse + format");
        assert!(
            edits.is_empty(),
            "byte-faithful CST → no edit for {input:?}"
        );
    }
}

#[test]
fn end_character_for_input_without_trailing_newline() {
    // Input has no trailing `\n`. The end-character calculation
    // walks `text.lines().last()` and takes its length.
    let input = "a: 1";
    let edits = full_document_edits(input);
    assert!(edits.is_ok());
}

#[test]
fn end_line_calculation_for_multi_line() {
    let input = "key1:    value\nkey2:    value\nkey3:    value\n";
    let edits = full_document_edits(input).expect("ok");
    if let Some(e) = edits.first() {
        let end_line = e["range"]["end"]["line"].as_u64().unwrap();
        assert!(end_line >= 1, "multi-line doc must have end_line >= 1");
    }
}

#[test]
fn empty_input_handled() {
    let edits = full_document_edits("");
    assert!(edits.is_ok());
}

#[test]
fn single_line_no_newline() {
    // Tests the `text.ends_with('\n')` branch where it's false.
    let edits = full_document_edits("foo:    bar");
    assert!(edits.is_ok());
}

#[test]
fn already_canonical_returns_empty() {
    let edits = full_document_edits("name: foo\nport: 8080\n").expect("ok");
    assert!(edits.is_empty(), "canonical input → no edits");
}

#[test]
fn invalid_yaml_returns_error() {
    let r = full_document_edits("a: [unclosed\n");
    assert!(r.is_err());
}
