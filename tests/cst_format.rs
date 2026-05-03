// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Tests for the CST formatter.

use noyalib::cst::format;

#[test]
fn test_basic_formatting() {
    let input = "a: 1\nb: 2\n";
    let formatted = format(input).unwrap();
    assert_eq!(formatted, "a: 1\nb: 2\n");
}

#[test]
fn test_nested_formatting() {
    let input = "key:\n value: 1\n";
    let formatted = format(input).unwrap();
    assert_eq!(formatted, "key:\n  value: 1\n");
}

#[test]
fn test_messy_spacing() {
    let input = "a  :   1\nb: 2\n";
    let formatted = format(input).unwrap();
    assert_eq!(formatted, "a: 1\nb: 2\n");
}

#[test]
fn test_preserve_comments() {
    let input = "a: 1 # comment\n# standalone\nb: 2\n";
    let formatted = format(input).unwrap();
    assert_eq!(formatted, "a: 1 # comment\n# standalone\nb: 2\n");
}

#[test]
fn test_nested_block_sequence() {
    let input = "items:\n  - sub:\n      - 1\n";
    let formatted = format(input).unwrap();
    assert_eq!(formatted, "items:\n  - sub:\n      - 1\n");
}
