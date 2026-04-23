//! Thread-local span context for wiring source locations into `Spanned<T>`.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use crate::error::Location;
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
    pub spans: HashMap<usize, (usize, usize)>,
    /// The original source string (for `Location::from_index`).
    pub source: Arc<str>,
}

thread_local! {
    static SPAN_CONTEXT: RefCell<Option<SpanContext>> = const { RefCell::new(None) };
}

/// RAII guard that clears the thread-local on drop.
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

/// Set the thread-local span context. Returns an RAII guard that clears it on
/// drop.
pub(crate) fn set_span_context(ctx: SpanContext) -> SpanContextGuard {
    let cloned = SpanContext {
        spans: ctx.spans.clone(),
        source: Arc::clone(&ctx.source),
    };
    SPAN_CONTEXT.with(|cell| {
        *cell.borrow_mut() = Some(ctx);
    });
    SpanContextGuard { ctx: cloned }
}

/// Look up source locations for a `Value` pointer address.
#[allow(dead_code)]
pub(crate) fn lookup_span(value_ptr: usize) -> Option<(Location, Location)> {
    SPAN_CONTEXT.with(|cell| {
        let borrow = cell.borrow();
        let ctx = borrow.as_ref()?;
        let &(start, end) = ctx.spans.get(&value_ptr)?;
        let start_loc = Location::from_index(&ctx.source, start);
        let end_loc = Location::from_index(&ctx.source, end);
        Some((start_loc, end_loc))
    })
}

/// Walk a `Value` tree and a `SpanTree` in lockstep, collecting pointer → span
/// mappings.
pub(crate) fn build_span_map(value: &Value, tree: &SpanTree) -> HashMap<usize, (usize, usize)> {
    let mut map = HashMap::new();
    walk(value, tree, &mut map);
    map
}

fn walk(value: &Value, tree: &SpanTree, map: &mut HashMap<usize, (usize, usize)>) {
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
