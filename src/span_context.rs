//! Thread-local span context for wiring source locations into `Spanned<T>`.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use std::cell::RefCell;
use std::sync::Arc;

use rustc_hash::FxHashMap;

use crate::value::Value;

/// Parallel tree of source spans, built alongside `Value` during loading.
#[derive(Debug, Clone)]
pub(crate) enum SpanTree {
    /// A leaf node (scalar, alias, null).
    Leaf(usize, usize),
    /// A sequence with its own span and per-element span trees.
    Sequence {
        start: usize,
        end: usize,
        items: Vec<SpanTree>,
    },
    /// A mapping with its own span and per-entry (key-span, value-tree) pairs.
    Mapping {
        start: usize,
        end: usize,
        entries: Vec<((usize, usize), SpanTree)>,
    },
}

/// Holds the span map and source string for the current deserialization.
#[derive(Debug)]
pub struct SpanContext {
    /// Maps `&Value` pointer address → `(start_byte, end_byte)`.
    pub spans: FxHashMap<usize, (usize, usize)>,
    /// The original source string (for `Location::from_index`).
    pub source: Arc<str>,
}

thread_local! {
    static SPAN_CONTEXT: RefCell<Option<SpanContext>> = const { RefCell::new(None) };
}

/// RAII guard that owns the span context and clears the thread-local on
/// drop. Holding the guard keeps the context alive so callers can borrow
/// it via [`SpanContextGuard::as_ref`] without cloning the span map.
pub(crate) struct SpanContextGuard {
    ctx: SpanContext,
}

impl SpanContextGuard {
    pub(crate) fn as_ref(&self) -> &SpanContext {
        &self.ctx
    }
}

impl Drop for SpanContextGuard {
    fn drop(&mut self) {
        SPAN_CONTEXT.with(|cell| {
            *cell.borrow_mut() = None;
        });
    }
}

/// Install a thread-local span context. Returns an RAII guard that owns
/// the context and clears the thread-local on drop. The thread-local
/// stores only the source `Arc<str>` (the hot lookup path consults the
/// guard's `SpanContext` directly, avoiding a map clone per parse).
pub(crate) fn set_span_context(ctx: SpanContext) -> SpanContextGuard {
    let thread_local_ctx = SpanContext {
        spans: FxHashMap::default(),
        source: Arc::clone(&ctx.source),
    };
    SPAN_CONTEXT.with(|cell| {
        *cell.borrow_mut() = Some(thread_local_ctx);
    });
    SpanContextGuard { ctx }
}

/// Walk a `Value` tree and a `SpanTree` in lockstep, collecting pointer → span
/// mappings.
pub(crate) fn build_span_map(value: &Value, tree: &SpanTree) -> FxHashMap<usize, (usize, usize)> {
    let mut map = FxHashMap::default();
    walk(value, tree, &mut map);
    map
}

fn walk(value: &Value, tree: &SpanTree, map: &mut FxHashMap<usize, (usize, usize)>) {
    // Walk the tree in DFS order to build the pointer → span map.
    let p: *const Value = value;
    let ptr = p as usize;
    match tree {
        SpanTree::Leaf(start, end) => {
            let _ = map.insert(ptr, (*start, *end));
        }
        SpanTree::Sequence { start, end, items } => {
            let _ = map.insert(ptr, (*start, *end));
            if let Value::Sequence(seq) = value {
                for (v, t) in seq.iter().zip(items.iter()) {
                    walk(v, t, map);
                }
            }
        }
        SpanTree::Mapping {
            start,
            end,
            entries,
        } => {
            let _ = map.insert(ptr, (*start, *end));
            if let Value::Mapping(mapping) = value {
                // Mapping entries are in insertion order (IndexMap), matching SpanTree order.
                for ((_, v), (_, vt)) in mapping.iter().zip(entries.iter()) {
                    walk(v, vt, map);
                }
            }
        }
    }
}
