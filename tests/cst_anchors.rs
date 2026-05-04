// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Phase 2.3 — anchor / alias management ("Smart Aliases").
//!
//! Covers the full discovery surface (`anchors`, `aliases`,
//! `aliases_of`), the propagation contract (edits to anchored
//! values are visible at every alias site after re-load), and the
//! materialise primitives that "break" an alias by inlining the
//! anchored value's source text into the alias position.
//!
//! Block-valued anchors are explicitly out-of-scope for v0.0.1
//! materialisation — those return an actionable error and the
//! source is left untouched. The expectation is that callers fall
//! back to `Document::anchors()` + `Document::replace_span()` for
//! manual block splicing.

#![allow(missing_docs)]

use noyalib::cst::parse_document;

// ── Discovery ───────────────────────────────────────────────────────

#[test]
fn anchors_and_aliases_in_one_doc() {
    let src = "\
defaults: &cfg
  port: 8080
service:
  <<: *cfg
  host: localhost
backup: *cfg
";
    let doc = parse_document(src).unwrap();
    let anchors = doc.anchors();
    let aliases = doc.aliases();
    assert_eq!(anchors.len(), 1);
    assert_eq!(anchors[0].name, "cfg");
    assert_eq!(aliases.len(), 2);
    assert!(aliases.iter().all(|a| a.name == "cfg"));
}

#[test]
fn anchor_mark_span_is_only_the_lexeme() {
    let src = "x: &name 1\n";
    let doc = parse_document(src).unwrap();
    let a = &doc.anchors()[0];
    assert_eq!(&src[a.mark_span.0..a.mark_span.1], "&name");
}

#[test]
fn alias_mark_span_is_only_the_lexeme() {
    let src = "x: &name 1\ny: *name\n";
    let doc = parse_document(src).unwrap();
    let a = &doc.aliases()[0];
    assert_eq!(&src[a.mark_span.0..a.mark_span.1], "*name");
}

#[test]
fn aliases_of_unknown_anchor_is_empty_not_error() {
    // Discovery is read-only — looking up a non-existent anchor
    // returns an empty list, not an error. That matches the typical
    // "is this anchor referenced anywhere?" check pattern.
    let src = "x: 1\ny: 2\n";
    let doc = parse_document(src).unwrap();
    assert!(doc.aliases_of("ghost").is_empty());
}

#[test]
fn many_anchors_preserve_source_order() {
    let src = "a: &one 1\nb: &two 2\nc: &three 3\n";
    let doc = parse_document(src).unwrap();
    let anchors = doc.anchors();
    let names: Vec<&str> = anchors.iter().map(|a| a.name.as_str()).collect();
    assert_eq!(names, vec!["one", "two", "three"]);
    let positions: Vec<usize> = anchors.iter().map(|a| a.mark_span.0).collect();
    assert!(positions.windows(2).all(|w| w[0] < w[1]));
}

// ── Propagation contract — edits propagate via re-load ──────────────

#[test]
fn editing_anchored_value_propagates_to_alias_sites() {
    // The contract documented in the module rustdoc: when the user
    // mutates the anchored value via `set`, every alias site sees
    // the new value when the typed Value tree is read.
    let src = "\
defaults: &cfg
  port: 8080
  host: localhost
primary:
  <<: *cfg
secondary:
  <<: *cfg
";
    let mut doc = parse_document(src).unwrap();
    doc.set("defaults.port", "9090").unwrap();

    let v = doc.as_value();
    assert_eq!(v["defaults"]["port"].as_i64(), Some(9090));
    assert_eq!(v["primary"]["port"].as_i64(), Some(9090));
    assert_eq!(v["secondary"]["port"].as_i64(), Some(9090));

    // The source still shows one anchor and two aliases — only the
    // anchored value's bytes changed, not the wiring.
    assert_eq!(doc.anchors().len(), 1);
    assert_eq!(doc.aliases().len(), 2);
}

#[test]
fn editing_anchored_value_preserves_byte_faithful_outside_target() {
    let src = "\
# project defaults
defaults: &cfg
  port: 8080  # bound port
service:
  <<: *cfg
  host: localhost
";
    let mut doc = parse_document(src).unwrap();
    doc.set("defaults.port", "9090").unwrap();

    let out = doc.to_string();
    assert!(out.contains("# project defaults"));
    assert!(out.contains("# bound port"));
    assert!(out.contains("&cfg"));
    assert!(out.contains("*cfg"));
    assert!(out.contains("port: 9090"));
}

// ── Materialise — scalar paths ──────────────────────────────────────

#[test]
fn materialise_inlines_scalar_anchor_text() {
    let src = "a: &n 7\nb: *n\n";
    let mut doc = parse_document(src).unwrap();
    let alias_pos = doc.aliases()[0].mark_span.0;
    doc.materialise_alias_at(alias_pos).unwrap();

    let out = doc.to_string();
    assert_eq!(out, "a: &n 7\nb: 7\n");
    assert!(doc.aliases().is_empty());
    // The anchor itself is untouched — it still labels its value.
    assert_eq!(doc.anchors().len(), 1);
}

#[test]
fn materialise_with_double_quoted_scalar_keeps_quotes() {
    // Style preservation: the materialised text is the *source
    // bytes* of the anchored value, including quoting.
    let src = "a: &greet \"hello world\"\nb: *greet\n";
    let mut doc = parse_document(src).unwrap();
    let pos = doc.aliases()[0].mark_span.0;
    doc.materialise_alias_at(pos).unwrap();
    assert_eq!(
        doc.to_string(),
        "a: &greet \"hello world\"\nb: \"hello world\"\n"
    );
}

#[test]
fn materialise_with_single_quoted_scalar_keeps_quotes() {
    let src = "a: &g 'O''Brien'\nb: *g\n";
    let mut doc = parse_document(src).unwrap();
    let pos = doc.aliases()[0].mark_span.0;
    doc.materialise_alias_at(pos).unwrap();
    assert_eq!(doc.to_string(), "a: &g 'O''Brien'\nb: 'O''Brien'\n");
}

#[test]
fn materialise_aliases_of_handles_three_in_one_call() {
    let src = "a: &x 7\nb: *x\nc: *x\nd: *x\n";
    let mut doc = parse_document(src).unwrap();
    let n = doc.materialise_aliases_of("x").unwrap();
    assert_eq!(n, 3);
    let out = doc.to_string();
    assert!(!out.contains('*'));
    assert_eq!(out, "a: &x 7\nb: 7\nc: 7\nd: 7\n");
}

#[test]
fn materialise_one_does_not_break_others() {
    let src = "a: &x 1\nb: *x\nc: *x\n";
    let mut doc = parse_document(src).unwrap();
    let first_alias = doc.aliases()[0].mark_span.0;
    doc.materialise_alias_at(first_alias).unwrap();

    let out = doc.to_string();
    // First alias inlined; second alias still pointing at &x.
    assert_eq!(out, "a: &x 1\nb: 1\nc: *x\n");
    assert_eq!(doc.aliases_of("x").len(), 1);
}

// ── Materialise — error paths ───────────────────────────────────────

#[test]
fn materialise_block_anchor_errors_and_leaves_source_unchanged() {
    let src = "\
defaults: &cfg
  port: 8080
  host: localhost
service: *cfg
";
    let mut doc = parse_document(src).unwrap();
    let pos = doc.aliases()[0].mark_span.0;
    let err = doc.materialise_alias_at(pos).unwrap_err();
    let msg = err.to_string();
    // Error must point at the limitation and the workaround.
    assert!(msg.contains("multi-line"));
    assert!(msg.contains("scalar-valued"));
    assert!(msg.contains("replace_span"));
    // Source must be untouched on error.
    assert_eq!(doc.to_string(), src);
    assert_eq!(doc.anchors().len(), 1);
    assert_eq!(doc.aliases().len(), 1);
}

#[test]
fn materialise_unknown_position_errors_clearly() {
    let mut doc = parse_document("a: &x 1\nb: *x\n").unwrap();
    let err = doc.materialise_alias_at(0).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("no alias mark begins at byte 0"));
}

// ── Edits over anchor-decorated regions stay byte-faithful ──────────

#[test]
fn anchor_marks_unchanged_after_unrelated_edit() {
    let src = "\
flags: &flags
  debug: false
other: 1
";
    let mut doc = parse_document(src).unwrap();
    let before = doc.anchors();
    doc.set("other", "2").unwrap();
    let after = doc.anchors();
    assert_eq!(before, after);
    assert!(doc.to_string().contains("&flags"));
    assert!(doc.to_string().contains("other: 2"));
}
