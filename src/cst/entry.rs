// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Path-shaped mutable handle to a CST node — the `Entry` "pro"
//! interface that complements the functional `Document::set` /
//! `remove` / `push_back` / `insert_after` methods.
//!
//! The functional methods stay first-class: when the caller knows
//! the operation up front, `doc.set("server.port", "9090")?` is the
//! shortest path. `Entry` exists for the *idiomatic* / *composable*
//! cases:
//!
//! - chaining nested mutations without re-typing the prefix
//!   (`doc.entry("server").set_value(...)?`),
//! - reading-then-writing a field by the same handle
//!   (`if entry.exists() { entry.set("...")?; }`),
//! - inserting into a child mapping by name without manual
//!   path-string concatenation
//!   (`doc.entry("metadata.labels").insert("env", "prod")?`),
//! - keeping a single mutable borrow across a sequence of edits.
//!
//! Every operation routes through the same lossless CST splice path
//! the functional methods use — comments, blank lines, sibling
//! entries, and indentation are preserved byte-for-byte across
//! every Entry-driven edit.

use crate::cst::Document;
use crate::error::{Error, Result};
use crate::prelude::*;
use crate::value::Value;

/// A path-shaped mutable handle to a node in a [`Document`].
///
/// Created by [`Document::entry`]. Every method routes through the
/// same CST splice the functional API uses, so edits are byte-faithful
/// outside the touched span.
///
/// # Examples
///
/// Single-level edit, equivalent to `Document::set`:
///
/// ```
/// use noyalib::cst::parse_document;
///
/// let mut doc = parse_document("port: 8080\n").unwrap();
/// doc.entry("port").set("9090").unwrap();
/// assert_eq!(doc.to_string(), "port: 9090\n");
/// ```
///
/// Nested edit by chained drill-down — no string concatenation:
///
/// ```
/// use noyalib::cst::parse_document;
///
/// let mut doc = parse_document(
///     "server:\n  host: localhost\n  port: 8080\n",
/// ).unwrap();
/// doc.entry("server").entry("port").set("9090").unwrap();
/// assert!(doc.to_string().contains("port: 9090"));
/// ```
///
/// Insert into a mapping by key without manual path strings:
///
/// ```
/// use noyalib::cst::parse_document;
///
/// let mut doc = parse_document(
///     "metadata:\n  labels:\n    app: noyalib\n",
/// ).unwrap();
/// doc.entry("metadata.labels").insert("env", "prod").unwrap();
/// let out = doc.to_string();
/// assert!(out.contains("app: noyalib"));
/// assert!(out.contains("env: prod"));
/// ```
#[derive(Debug)]
pub struct Entry<'a> {
    doc: &'a mut Document,
    path: String,
}

impl<'a> Entry<'a> {
    pub(crate) fn new(doc: &'a mut Document, path: String) -> Self {
        Self { doc, path }
    }

    /// The path this entry represents, as a dotted/indexed string.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// `true` if a node currently exists at this path.
    #[must_use]
    pub fn exists(&self) -> bool {
        self.doc.span_at(&self.path).is_some()
    }

    /// The current source slice at this path, or `None` if the path
    /// does not resolve. Forwards to [`Document::get`].
    #[must_use]
    pub fn get(&self) -> Option<&str> {
        self.doc.get(&self.path)
    }

    /// The byte range `(start, end)` at this path, or `None` if the
    /// path does not resolve. Forwards to [`Document::span_at`].
    #[must_use]
    pub fn span_at(&self) -> Option<(usize, usize)> {
        self.doc.span_at(&self.path)
    }

    /// Comments that decorate this node. Forwards to
    /// [`Document::comments_at`].
    #[must_use]
    pub fn comments(&self) -> crate::cst::CommentBundle {
        self.doc.comments_at(&self.path)
    }

    /// Replace the value at this path with `fragment`. Forwards to
    /// [`Document::set`].
    ///
    /// Consumes the entry. Returns the [`Document`]'s reference back
    /// via the borrow expiring; chain a fresh `doc.entry(...)` for
    /// the next edit.
    ///
    /// # Errors
    ///
    /// Same as [`Document::set`] — invalid path, parse-after-splice
    /// failure, etc.
    pub fn set(self, fragment: &str) -> Result<()> {
        self.doc.set(&self.path, fragment)
    }

    /// Replace the value at this path with the YAML emission of
    /// `value`. Forwards to [`Document::set_value`].
    ///
    /// # Errors
    ///
    /// Same as [`Document::set_value`].
    pub fn set_value(self, value: &Value) -> Result<()> {
        self.doc.set_value(&self.path, value)
    }

    /// Remove the entry at this path. Forwards to
    /// [`Document::remove`].
    ///
    /// # Errors
    ///
    /// Same as [`Document::remove`].
    pub fn remove(self) -> Result<()> {
        self.doc.remove(&self.path)
    }

    /// Insert a `key: value` pair into the mapping at this path.
    /// If the key already exists, the value is replaced losslessly;
    /// if it is new, a sibling line is spliced after the last
    /// existing entry with matching indent. Forwards to
    /// [`Document::insert_entry`].
    ///
    /// # Errors
    ///
    /// - The mapping at `self.path` does not exist.
    /// - The mapping is empty (no anchor for indent — use
    ///   [`Self::set`] with a fragment to give the mapping its first
    ///   entry).
    /// - The same parse-after-splice errors as the underlying
    ///   `replace_span`.
    pub fn insert(self, key: &str, fragment: &str) -> Result<()> {
        self.doc.insert_entry(&self.path, key, fragment)
    }

    /// Insert a `key: value` pair where the value is a typed
    /// [`Value`]. The value is YAML-emitted via the standard
    /// serializer and routed through [`Self::insert`].
    ///
    /// # Errors
    ///
    /// As [`Self::insert`].
    pub fn insert_value(self, key: &str, value: &Value) -> Result<()> {
        let fragment = crate::to_string(value)?;
        // `to_string` adds a trailing `\n` for top-level emission;
        // strip it so the spliced fragment fits on a single mapping
        // line.
        let trimmed = fragment.trim_end_matches('\n');
        self.doc.insert_entry(&self.path, key, trimmed)
    }

    /// Append `fragment` as a new item to the sequence at this path.
    /// Forwards to [`Document::push_back`].
    ///
    /// # Errors
    ///
    /// Same as [`Document::push_back`] — the path must point to a
    /// non-empty block sequence.
    pub fn push_back(self, fragment: &str) -> Result<()> {
        self.doc.push_back(&self.path, fragment)
    }

    /// Insert `fragment` as a new sequence item immediately after
    /// the item at this path. Forwards to [`Document::insert_after`].
    ///
    /// # Errors
    ///
    /// Same as [`Document::insert_after`] — the path must end in an
    /// index and resolve to a sequence item in a block sequence.
    pub fn insert_after(self, fragment: &str) -> Result<()> {
        self.doc.insert_after(&self.path, fragment)
    }

    /// Drill down to a child path. The returned [`Entry`] represents
    /// `self.path + "." + child`, holding the same mutable borrow.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::cst::parse_document;
    ///
    /// let mut doc = parse_document(
    ///     "a:\n  b:\n    c: 1\n",
    /// ).unwrap();
    /// doc.entry("a").entry("b").entry("c").set("2").unwrap();
    /// assert!(doc.to_string().contains("c: 2"));
    /// ```
    #[must_use]
    pub fn entry(self, child: &str) -> Entry<'a> {
        let combined = compose_path(&self.path, child);
        Entry::new(self.doc, combined)
    }
}

impl Document {
    /// Return a path-shaped mutable handle to the node at `path`.
    ///
    /// The handle is the "pro" mutation interface — chainable,
    /// composable, ergonomic for nested edits — that complements the
    /// functional [`Document::set`] / [`Document::remove`] /
    /// [`Document::push_back`] / [`Document::insert_after`] methods
    /// (all of which remain available for direct one-shot operations).
    ///
    /// `entry` itself is infallible — the path is recorded but not
    /// resolved at this point. Operations on the returned entry
    /// surface path-resolution and splice-failure errors via their
    /// own `Result`.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::cst::parse_document;
    ///
    /// let mut doc = parse_document(
    ///     "metadata:\n  labels:\n    app: noyalib\n",
    /// ).unwrap();
    /// doc.entry("metadata.labels").insert("env", "prod").unwrap();
    ///
    /// let out = doc.to_string();
    /// assert!(out.contains("app: noyalib"));
    /// assert!(out.contains("env: prod"));
    /// ```
    pub fn entry<'a>(&'a mut self, path: &str) -> Entry<'a> {
        Entry::new(self, path.to_owned())
    }
}

/// Concatenate a parent path and a child segment with a dot
/// separator, except when the parent is empty (in which case the
/// child is the whole path) or the child already begins with `[`
/// (an index — no separator needed).
fn compose_path(parent: &str, child: &str) -> String {
    if parent.is_empty() {
        return child.to_owned();
    }
    if child.starts_with('[') {
        return format!("{parent}{child}");
    }
    format!("{parent}.{child}")
}

// ── Trait impls ──────────────────────────────────────────────────────

impl Error {
    /// Internal — surface a generic "no entry" message used by the
    /// upcoming `or_insert` machinery (v0.0.2). Defined here so the
    /// public Error API does not grow a new variant for the in-flight
    /// work yet.
    #[doc(hidden)]
    #[must_use]
    pub fn entry_not_found(path: &str) -> Self {
        Error::Parse(format!("entry not found at path: {path}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cst::parse_document;

    #[test]
    fn entry_set_replaces_value_losslessly() {
        let mut doc = parse_document(
            "# top comment\nport: 8080  # inline comment\n",
        )
        .unwrap();
        doc.entry("port").set("9090").unwrap();
        let out = doc.to_string();
        assert!(out.contains("port: 9090"));
        assert!(out.contains("# top comment"));
        assert!(out.contains("# inline comment"));
    }

    #[test]
    fn entry_chains_through_dotted_path() {
        let mut doc = parse_document(
            "server:\n  host: localhost\n  port: 8080\n",
        )
        .unwrap();
        doc.entry("server").entry("port").set("9090").unwrap();
        assert!(doc.to_string().contains("port: 9090"));
    }

    #[test]
    fn entry_insert_into_mapping() {
        let mut doc = parse_document(
            "metadata:\n  labels:\n    app: noyalib\n",
        )
        .unwrap();
        doc.entry("metadata.labels")
            .insert("env", "prod")
            .unwrap();
        let out = doc.to_string();
        assert!(out.contains("app: noyalib"));
        assert!(out.contains("env: prod"));
    }

    #[test]
    fn entry_insert_value_typed() {
        let mut doc = parse_document(
            "metadata:\n  labels:\n    app: noyalib\n",
        )
        .unwrap();
        let v = Value::Number(crate::Number::Integer(3));
        doc.entry("metadata.labels")
            .insert_value("replicas", &v)
            .unwrap();
        let out = doc.to_string();
        assert!(out.contains("replicas: 3"));
    }

    #[test]
    fn entry_remove() {
        let mut doc = parse_document(
            "a: 1\nb: 2\nc: 3\n",
        )
        .unwrap();
        doc.entry("b").remove().unwrap();
        let out = doc.to_string();
        assert!(out.contains("a: 1"));
        assert!(!out.contains("b:"));
        assert!(out.contains("c: 3"));
    }

    #[test]
    fn entry_push_back_to_sequence() {
        let mut doc = parse_document(
            "items:\n  - one\n  - two\n",
        )
        .unwrap();
        doc.entry("items").push_back("three").unwrap();
        let out = doc.to_string();
        assert!(out.contains("- one"));
        assert!(out.contains("- two"));
        assert!(out.contains("- three"));
    }

    #[test]
    fn entry_insert_after_in_sequence() {
        let mut doc = parse_document(
            "items:\n  - one\n  - three\n",
        )
        .unwrap();
        doc.entry("items[0]").insert_after("two").unwrap();
        let out = doc.to_string();
        let one_pos = out.find("one").unwrap();
        let two_pos = out.find("two").unwrap();
        let three_pos = out.find("three").unwrap();
        assert!(one_pos < two_pos);
        assert!(two_pos < three_pos);
    }

    #[test]
    fn entry_get_reads_source_slice() {
        let doc = parse_document("port: 8080\n").unwrap();
        // `entry` needs `&mut self`, so reads via Entry require a
        // throwaway doc — but the `get` method is a pure read.
        let mut doc = doc;
        let e = doc.entry("port");
        assert_eq!(e.get(), Some("8080"));
    }

    #[test]
    fn entry_exists_distinguishes_present_and_absent() {
        let mut doc = parse_document("a: 1\n").unwrap();
        assert!(doc.entry("a").exists());
        assert!(!doc.entry("b").exists());
    }

    #[test]
    fn entry_path_returns_recorded_string() {
        let mut doc = parse_document("a:\n  b: 1\n").unwrap();
        let e = doc.entry("a").entry("b");
        assert_eq!(e.path(), "a.b");
    }

    #[test]
    fn entry_with_index_uses_no_dot_separator() {
        let mut doc = parse_document(
            "items:\n  - one\n  - two\n",
        )
        .unwrap();
        // `entry("items").entry("[0]")` should produce `items[0]`,
        // not `items.[0]`.
        let e = doc.entry("items").entry("[0]");
        assert_eq!(e.path(), "items[0]");
    }

    #[test]
    fn entry_comments_forwards_to_document() {
        let mut doc = parse_document(
            "# decorator\nport: 8080  # inline\n",
        )
        .unwrap();
        let bundle = doc.entry("port").comments();
        assert_eq!(bundle.before.len(), 1);
        assert_eq!(bundle.before[0].text, " decorator");
        assert_eq!(bundle.inline.as_ref().unwrap().text, " inline");
    }

    #[test]
    fn entry_set_on_nonexistent_path_errors() {
        let mut doc = parse_document("a: 1\n").unwrap();
        let err = doc.entry("nonexistent").set("2").unwrap_err();
        assert!(err.to_string().contains("path not found"));
    }

    #[test]
    fn entry_repeated_edits_compose() {
        // Multiple Entry-driven edits in sequence — each releases
        // its borrow at the end of its statement, the next can
        // reborrow doc cleanly.
        let mut doc = parse_document(
            "name: noyalib\nversion: 0.0.1\n",
        )
        .unwrap();
        doc.entry("name").set("renamed").unwrap();
        doc.entry("version").set("0.0.2").unwrap();
        let out = doc.to_string();
        assert!(out.contains("name: renamed"));
        assert!(out.contains("version: 0.0.2"));
    }
}
