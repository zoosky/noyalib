// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Regression tests for issue #5 — `RcRecursive` / `ArcRecursive`
//! / `RcRecursion` / `ArcRecursion`.

#![allow(missing_docs)]

use noyalib::{ArcRecursion, ArcRecursive, RcRecursion, RcRecursive};

#[test]
fn rc_recursive_late_init() {
    let r: RcRecursive<String> = RcRecursive::empty();
    assert!(r.borrow().is_none());
    let prev = r.set("hello".to_string());
    assert!(prev.is_none());
    assert_eq!(r.borrow().as_deref(), Some("hello"));
}

#[test]
fn rc_recursive_set_returns_previous() {
    let r = RcRecursive::new(1_i32);
    let prev = r.set(2);
    assert_eq!(prev, Some(1));
    assert_eq!(r.borrow().as_ref(), Some(&2));
}

#[test]
fn rc_recursive_take_returns_value_and_empties() {
    let r = RcRecursive::new(42_i32);
    let val = r.take();
    assert_eq!(val, Some(42));
    assert!(r.borrow().is_none());
}

#[test]
fn rc_recursive_strong_count_tracks_clones() {
    let r = RcRecursive::new(1_i32);
    assert_eq!(r.strong_count(), 1);
    let r2 = r.clone();
    assert_eq!(r.strong_count(), 2);
    assert_eq!(r2.strong_count(), 2);
    drop(r2);
    assert_eq!(r.strong_count(), 1);
}

#[test]
fn rc_recursive_downgrade_does_not_count() {
    let r = RcRecursive::new(1_i32);
    let _w: RcRecursion<i32> = r.downgrade();
    assert_eq!(
        r.strong_count(),
        1,
        "weak ref should not increment strong count"
    );
}

#[test]
fn rc_recursion_upgrade_after_drop_returns_none() {
    let weak: RcRecursion<i32> = {
        let strong = RcRecursive::new(1_i32);
        strong.downgrade()
    };
    assert!(weak.upgrade().is_none());
}

#[test]
fn rc_recursion_upgrade_while_strong_alive() {
    let strong = RcRecursive::new(99_i32);
    let weak = strong.downgrade();
    let upgraded = weak.upgrade().expect("upgrade while strong alive");
    assert_eq!(upgraded.borrow().as_ref(), Some(&99));
}

#[test]
fn arc_recursive_basic_lock() {
    let r = ArcRecursive::new(7_i32);
    let guard = r.lock();
    assert_eq!(*guard, Some(7));
}

#[test]
fn arc_recursive_thread_safe() {
    use std::thread;
    let r = ArcRecursive::new(0_i32);
    let mut handles = Vec::new();
    for i in 0..8 {
        let r = r.clone();
        handles.push(thread::spawn(move || {
            let _ = r.set(i);
        }));
    }
    for h in handles {
        h.join().unwrap();
    }
    // The last writer wins; we don't care which.
    let final_val = *r.lock();
    assert!(final_val.is_some());
    assert!((0..8).contains(&final_val.unwrap()));
}

#[test]
fn arc_recursion_upgrade_round_trip() {
    let strong = ArcRecursive::new("payload".to_string());
    let weak: ArcRecursion<String> = strong.downgrade();
    let upgraded = weak.upgrade().expect("strong is alive");
    assert_eq!(upgraded.lock().as_deref(), Some("payload"));
}

#[test]
fn arc_recursion_upgrade_after_drop_returns_none() {
    let weak: ArcRecursion<i32> = {
        let strong = ArcRecursive::new(1_i32);
        strong.downgrade()
    };
    assert!(weak.upgrade().is_none());
}

#[test]
fn cyclic_reference_storage_release() {
    // Build a "node" that holds a weak reference to itself —
    // the canonical cyclic-graph shape. Storage releases
    // when the only strong reference is dropped.
    struct Node {
        _self_ref: RcRecursion<Self>,
    }
    let node = RcRecursive::<Node>::empty();
    let weak = node.downgrade();
    let _ = node.set(Node { _self_ref: weak });
    // The strong count is 1 — the weak inside doesn't count.
    assert_eq!(node.strong_count(), 1);
    drop(node);
    // No leak: the cycle is broken because the inner ref is weak.
}

#[test]
fn rc_recursive_serde_roundtrip_via_serde_json() {
    let r = RcRecursive::new(42_i32);
    let json = serde_json::to_string(&r).unwrap();
    assert_eq!(json, "42");
    let parsed: RcRecursive<i32> = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.borrow().as_ref(), Some(&42));
}

#[test]
fn arc_recursive_serde_roundtrip_via_serde_json() {
    let r = ArcRecursive::new("hi".to_string());
    let json = serde_json::to_string(&r).unwrap();
    assert_eq!(json, "\"hi\"");
    let parsed: ArcRecursive<String> = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.lock().as_deref(), Some("hi"));
}

#[test]
fn rc_recursive_empty_serializes_as_null() {
    let r: RcRecursive<i32> = RcRecursive::empty();
    let json = serde_json::to_string(&r).unwrap();
    assert_eq!(json, "null");
}
