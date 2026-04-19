// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

// YAML spec: Block scalars (literal | and folded >)

use noyalib::from_str;

#[test]
fn literal_block_scalar() {
    let v: String = from_str("|\n  line1\n  line2\n").unwrap();
    assert_eq!(v, "line1\nline2\n");
}

#[test]
fn literal_block_strip() {
    let v: String = from_str("|-\n  line1\n  line2\n").unwrap();
    assert_eq!(v, "line1\nline2");
}

#[test]
fn literal_block_keep() {
    let v: String = from_str("|+\n  line1\n  line2\n\n").unwrap();
    assert_eq!(v, "line1\nline2\n\n");
}

#[test]
fn folded_block_scalar() {
    let v: String = from_str(">\n  line1\n  line2\n").unwrap();
    assert_eq!(v, "line1 line2\n");
}

#[test]
fn folded_block_strip() {
    let v: String = from_str(">-\n  line1\n  line2\n").unwrap();
    assert_eq!(v, "line1 line2");
}

#[test]
fn folded_block_keep() {
    let v: String = from_str(">+\n  line1\n  line2\n\n").unwrap();
    assert_eq!(v, "line1 line2\n\n");
}

#[test]
fn literal_block_with_indent_indicator() {
    let v: String = from_str("|2\n  line1\n  line2\n").unwrap();
    assert_eq!(v, "line1\nline2\n");
}

#[test]
fn literal_block_preserves_newlines() {
    let v: String = from_str("|\n  line1\n\n  line3\n").unwrap();
    assert_eq!(v, "line1\n\nline3\n");
}

#[test]
fn literal_block_in_mapping() {
    use std::collections::HashMap;
    let m: HashMap<String, String> = from_str("script: |\n  echo hello\n  echo world\n").unwrap();
    assert_eq!(m["script"], "echo hello\necho world\n");
}

#[test]
fn folded_block_in_mapping() {
    use std::collections::HashMap;
    let m: HashMap<String, String> =
        from_str("desc: >\n  This is a\n  long description\n").unwrap();
    assert_eq!(m["desc"], "This is a long description\n");
}

#[test]
fn literal_block_single_line() {
    let v: String = from_str("|\n  single line\n").unwrap();
    assert_eq!(v, "single line\n");
}

#[test]
fn folded_block_single_line() {
    let v: String = from_str(">\n  single line\n").unwrap();
    assert_eq!(v, "single line\n");
}
