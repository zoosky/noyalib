// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Entry::or_insert / or_insert_with / or_insert_value /
//! and_modify — std-collections-style ergonomics over the
//! lossless CST splice path.

#![allow(missing_docs)]

use noyalib::cst::parse_document;
use noyalib::{Number, Value};

const TWO_SPACE_DOC: &str = "\
metadata:
  labels:
    app: noyalib
";

#[test]
fn or_insert_runs_when_path_is_vacant() {
    let mut doc = parse_document(TWO_SPACE_DOC).unwrap();
    let inserted = doc.entry("metadata.labels.env").or_insert("prod").unwrap();
    assert!(inserted, "vacant path must report inserted=true");
    assert!(doc.to_string().contains("env: prod"));
}

#[test]
fn or_insert_no_op_when_path_is_occupied() {
    let mut doc = parse_document(TWO_SPACE_DOC).unwrap();
    let inserted = doc
        .entry("metadata.labels.app")
        .or_insert("staging")
        .unwrap();
    assert!(!inserted, "occupied path must report inserted=false");
    // Original value untouched.
    assert!(doc.to_string().contains("app: noyalib"));
    assert!(!doc.to_string().contains("app: staging"));
}

#[test]
fn or_insert_with_lazy_default() {
    let mut doc = parse_document(TWO_SPACE_DOC).unwrap();
    let mut called = 0;
    let _ = doc
        .entry("metadata.labels.env")
        .or_insert_with(|| {
            called += 1;
            "prod".to_owned()
        })
        .unwrap();
    assert_eq!(called, 1);

    // On the occupied branch, the closure must NOT run.
    let mut called2 = 0;
    let _ = doc
        .entry("metadata.labels.app")
        .or_insert_with(|| {
            called2 += 1;
            "should-not-run".to_owned()
        })
        .unwrap();
    assert_eq!(called2, 0);
}

#[test]
fn or_insert_value_typed_default() {
    let mut doc = parse_document(TWO_SPACE_DOC).unwrap();
    let inserted = doc
        .entry("metadata.labels.replicas")
        .or_insert_value(&Value::Number(Number::Integer(3)))
        .unwrap();
    assert!(inserted);
    assert!(doc.to_string().contains("replicas: 3"));
}

#[test]
fn or_insert_value_no_op_when_occupied() {
    let mut doc = parse_document(TWO_SPACE_DOC).unwrap();
    let inserted = doc
        .entry("metadata.labels.app")
        .or_insert_value(&Value::String("ignored".into()))
        .unwrap();
    assert!(!inserted);
    assert!(doc.to_string().contains("app: noyalib"));
}

#[test]
fn or_insert_top_level_key_errors_actionably() {
    // Top-level paths can't be added by or_insert (no parent
    // mapping); the error must point at the workaround.
    let mut doc = parse_document(TWO_SPACE_DOC).unwrap();
    let err = doc.entry("brand_new_top_key").or_insert("x").unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("top-level"),
        "error must mention top-level: {msg}"
    );
}

#[test]
fn or_insert_at_sequence_index_errors_actionably() {
    let mut doc = parse_document("items:\n  - one\n  - two\n").unwrap();
    let err = doc.entry("items[5]").or_insert("x").unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("push_back") || msg.contains("insert_after"),
        "error must redirect to push_back / insert_after: {msg}"
    );
}

#[test]
fn and_modify_runs_when_occupied() {
    let mut doc = parse_document("service:\n  port: 8080\n").unwrap();
    let _ = doc
        .entry("service.port")
        .and_modify(|d| {
            let _ = d.set("service.port", "9090");
        })
        .or_insert("8080") // no-op: path now still occupied
        .unwrap();
    assert!(doc.to_string().contains("port: 9090"));
}

#[test]
fn and_modify_skipped_when_vacant() {
    let mut doc = parse_document("service:\n  port: 8080\n").unwrap();
    let mut ran = 0;
    let _ = doc
        .entry("service.replicas")
        .and_modify(|_| ran += 1)
        .or_insert("3")
        .unwrap();
    assert_eq!(ran, 0);
    assert!(doc.to_string().contains("replicas: 3"));
}

#[test]
fn chained_modify_then_or_insert_idempotent() {
    // Standard "increment-or-default" pattern: the closure
    // always sees the current value if present, otherwise the
    // default supplies a fresh entry.
    let mut doc = parse_document("counters:\n  hits: 1\n").unwrap();
    let _ = doc
        .entry("counters.hits")
        .and_modify(|d| {
            let _ = d.set("counters.hits", "2");
        })
        .or_insert("1")
        .unwrap();
    assert!(doc.to_string().contains("hits: 2"));

    // Same pipeline on a missing key — and_modify is skipped,
    // or_insert provides the default.
    let _ = doc
        .entry("counters.misses")
        .and_modify(|d| {
            let _ = d.set("counters.misses", "999");
        })
        .or_insert("0")
        .unwrap();
    assert!(doc.to_string().contains("misses: 0"));
}

#[test]
fn or_insert_preserves_byte_faithful_outside_target() {
    // The whole point of routing through replace_span:
    // comments, blank lines, and sibling formatting outside
    // the touched span survive verbatim.
    let src = "\
# project metadata
metadata:
  labels:
    app: noyalib  # the project name

    team: platform
";
    let mut doc = parse_document(src).unwrap();
    let _ = doc.entry("metadata.labels.env").or_insert("prod").unwrap();
    let out = doc.to_string();
    assert!(out.contains("# project metadata"));
    assert!(out.contains("# the project name"));
    assert!(out.contains("team: platform"));
    assert!(out.contains("env: prod"));
}
