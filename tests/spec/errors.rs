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

// yaml-test-suite SU5Z — `#` adjacent to prior content is not a comment.
#[test]
fn comment_indicator_must_be_preceded_by_whitespace() {
    let result: Result<Value, _> = from_str("key: \"value\"# invalid comment\n");
    assert!(
        result.is_err(),
        "expected rejection of inline `#` without preceding whitespace"
    );
}

// yaml-test-suite X4QW — same rule inside the `>`/`|` header line.
#[test]
fn block_scalar_header_rejects_adjacent_hash() {
    let result: Result<Value, _> = from_str("block: >#comment\n  scalar\n");
    assert!(
        result.is_err(),
        "expected rejection of `>#` with no whitespace"
    );
}

// yaml-test-suite SF5V — at most one %YAML directive per document.
#[test]
fn duplicate_yaml_directive_rejected() {
    let result: Result<Value, _> = from_str("%YAML 1.2\n%YAML 1.2\n---\n");
    assert!(
        result.is_err(),
        "expected rejection of duplicate %YAML directive"
    );
}

// yaml-test-suite Y79Y :4..:7 — tab immediately before a block-structural
// indicator is forbidden (cannot stand in for indentation).
#[test]
fn tab_before_block_indicator_rejected() {
    for input in &["-\t-\n", "- \t-\n", "?\t-\n", "? -\n:\t-\n"] {
        let result: Result<Value, _> = from_str(input);
        assert!(
            result.is_err(),
            "expected rejection of tab-as-indentation in {input:?}"
        );
    }
}

// yaml-test-suite A2M4 (spec example 6.2) — tab as inline separation
// before plain content is *valid*; only tabs before another structural
// indicator are rejected.
#[test]
fn tab_as_inline_separation_accepted() {
    let v: Value = from_str("? a\n: -\tb\n  -  -\tc\n     - d\n").unwrap();
    let outer = v.as_mapping().expect("mapping");
    let seq = outer.get("a").expect("key 'a'").as_sequence().expect("seq");
    assert_eq!(seq[0].as_str(), Some("b"));
}

// yaml-test-suite 3HFZ — content after `...` document-end marker on the
// same line is invalid.
#[test]
fn document_end_marker_rejects_trailing_content() {
    let result: Result<Value, _> = from_str("---\nkey: value\n... invalid\n");
    assert!(result.is_err());
}

// yaml-test-suite RXY3 / 5TRB — a `---` or `...` document indicator at
// column 0 inside a multi-line quoted scalar is invalid (it would
// prematurely close the document).
#[test]
fn doc_marker_inside_quoted_scalar_rejected() {
    let r1: Result<Value, _> = from_str("---\n'\n...\n'\n");
    assert!(r1.is_err(), "single-quoted scalar containing `...`");
    let r2: Result<Value, _> = from_str("---\n\"\n---\n\"\n");
    assert!(r2.is_err(), "double-quoted scalar containing `---`");
}

// yaml-test-suite 2G84 — block scalar indent indicator must be a single
// digit 1..9; `0` and multi-digit forms are rejected.
#[test]
fn block_scalar_indent_indicator_validation() {
    let r1: Result<Value, _> = from_str("--- |0\n");
    assert!(r1.is_err(), "indent indicator 0 is invalid");
    let r2: Result<Value, _> = from_str("--- |10\n");
    assert!(r2.is_err(), "two-digit indent indicator is invalid");
}

// yaml-test-suite 4H7K — a stray `]` outside any flow sequence is an
// error.
#[test]
fn stray_flow_close_outside_flow_rejected() {
    let r1: Result<Value, _> = from_str("[ a, b, c ] ]\n");
    assert!(r1.is_err());
    let r2: Result<Value, _> = from_str("{ a: 1 } }\n");
    assert!(r2.is_err());
}

// yaml-test-suite H7TQ — non-numeric trailing content after the version
// of a `%YAML` directive is rejected. (Numeric continuations are
// accepted as a lenient extension; see ZYU8.)
#[test]
fn yaml_directive_rejects_non_numeric_extras() {
    let r: Result<Value, _> = from_str("%YAML 1.2 foo\n---\n");
    assert!(r.is_err());
    // Numeric-looking trailing token still parses.
    let _: Value = from_str("%YAML 1.1 1.2\n---\n").unwrap();
}

// yaml-test-suite 4HVU / EW3V / DMG6 / N4JP / U44R — block-context
// content at a column that does not match any open block scope
// (i.e. "between levels") is rejected. The check fires only at
// positions where a *new* mapping key or sequence entry could
// start, so separator tokens like `:` mid-pair are unaffected.
#[test]
fn between_levels_indentation_rejected() {
    // 4HVU — sequence entries at col 3 then a `-` at col 2.
    let r: Result<Value, _> = from_str("key:\n   - ok\n   - also ok\n  - wrong\n");
    assert!(r.is_err());

    // EW3V — second mapping key at col 1, parent at col 0.
    let r: Result<Value, _> = from_str("k1: v1\n k2: v2\n");
    assert!(r.is_err());

    // DMG6 — nested mapping then over-indented sibling.
    let r: Result<Value, _> = from_str("key:\n  ok: 1\n wrong: 2\n");
    assert!(r.is_err());
}

// Counter-examples: nested blocks (each level deeper) and sibling
// alignments (each at the same level) must continue to parse.
#[test]
fn correctly_indented_blocks_still_parse() {
    let _: Value = from_str("a:\n  b:\n    c: 1\n").unwrap();
    let _: Value = from_str("a: 1\nb: 2\nc: 3\n").unwrap();
    let _: Value = from_str("xs:\n  - 1\n  - 2\n  - 3\n").unwrap();
    let _: Value = from_str("# comment\nkey: value\n").unwrap();
}

// yaml-test-suite 9KBC / CXX2 — `from_str` previously stopped lazily
// at the first complete value, silently swallowing the spec
// violations that follow. The streaming deserializer now drains
// trailing events, surfacing those errors instead of returning a
// partial value.
#[test]
fn from_str_drains_trailing_events_to_surface_errors() {
    // 9KBC — a mapping inlined onto the `---` line is invalid; the
    // continuation key on the next line triggers
    // "mapping values are not allowed in this context" once events
    // past the first scalar are fetched.
    let r: Result<Value, _> = from_str("--- key1: value1\n    key2: value2\n");
    assert!(
        r.is_err(),
        "expected from_str to surface the lazy-only-accept error"
    );

    // CXX2 — anchor + key on the document-start line.
    let r: Result<Value, _> = from_str("--- &anchor a: b\n");
    assert!(r.is_err());
}
