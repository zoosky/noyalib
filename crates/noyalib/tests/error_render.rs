// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Regression tests for issue #2 — `Error::render()` +
//! `RenderOptions` + `CroppedRegion`.

#![allow(missing_docs)]

use noyalib::{from_str, CroppedRegion, RenderOptions, Value};

#[test]
fn render_default_includes_error_keyword() {
    let source = "a:\n  b: 1\n   c: 2\n"; // misaligned indent
    let err = from_str::<Value>(source).unwrap_err();
    let rendered = err.render(source);
    assert!(rendered.contains("error"), "got: {rendered}");
}

#[test]
fn render_with_options_zero_radius_is_single_line() {
    let source = "a: [unclosed";
    let err = from_str::<Value>(source).unwrap_err();
    let opts = RenderOptions {
        crop_radius: 0,
        color: false,
    };
    let rendered = err.render_with_options(source, &opts);
    // Single-line render should not include the multi-line gutter `|`.
    assert!(!rendered.contains(" | "), "got: {rendered}");
}

#[test]
fn render_with_options_color_emits_ansi_escapes() {
    let source = "a:\n  b: 1\n   c: 2\n";
    let err = from_str::<Value>(source).unwrap_err();
    let opts = RenderOptions {
        crop_radius: 2,
        color: true,
    };
    let rendered = err.render_with_options(source, &opts);
    assert!(
        rendered.contains("\x1b["),
        "expected ANSI escape, got: {rendered:?}"
    );
    assert!(rendered.contains("\x1b[0m"), "expected reset code");
}

#[test]
fn render_options_default() {
    let opts = RenderOptions::default();
    assert_eq!(opts.crop_radius, 2);
    assert!(!opts.color);
}

#[test]
fn cropped_region_basic() {
    let src = "line 1\nline 2\nline 3\nline 4\nline 5\n";
    let r = CroppedRegion::extract(src, 3, 1);
    assert_eq!(r.lines, vec!["line 2", "line 3", "line 4"]);
    assert_eq!(r.focus_index, 1);
    assert_eq!(r.focus_line, 3);
    assert_eq!(r.low_line, 2);
}

#[test]
fn cropped_region_clamps_at_top() {
    let src = "a\nb\nc\nd\n";
    let r = CroppedRegion::extract(src, 1, 2);
    assert_eq!(r.lines, vec!["a", "b", "c"]);
    assert_eq!(r.focus_index, 0);
    assert_eq!(r.focus_line, 1);
}

#[test]
fn cropped_region_clamps_at_bottom() {
    let src = "a\nb\nc\n";
    let r = CroppedRegion::extract(src, 3, 5);
    assert_eq!(r.lines, vec!["a", "b", "c"]);
    assert_eq!(r.focus_index, 2);
    assert_eq!(r.focus_line, 3);
}

#[test]
fn cropped_region_empty_source() {
    let r = CroppedRegion::extract("", 1, 2);
    assert!(r.lines.is_empty());
    assert_eq!(r.focus_line, 0);
}

#[test]
fn cropped_region_utf8_boundary_safe() {
    // Multi-byte UTF-8 across line boundaries — extract must
    // never split a code point.
    let src = "α\nβ\nγ\nδ\n";
    let r = CroppedRegion::extract(src, 2, 1);
    assert_eq!(r.lines, vec!["α", "β", "γ"]);
}

#[test]
fn render_no_location_falls_back_to_display() {
    // An error without source span (e.g. a synthetic one) renders
    // its Display rather than panicking.
    let e = noyalib::Error::Custom("synthetic".into());
    let rendered = e.render("source bytes");
    assert!(rendered.contains("synthetic"));
}
