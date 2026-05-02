// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Public `Document` handle and parse / mutation entry points.

use core::fmt::Write as _;

use crate::cst::builder::{document_boundaries, parse_full};
use crate::cst::green::{GreenChild, GreenNode};
use crate::cst::syntax::SyntaxKind;
use crate::error::{Error, Result};
use crate::path::{parse_query_path, QuerySegment};
use crate::prelude::*;
use crate::span_context::SpanTree;
use crate::value::{Number, Value};

/// A YAML document with byte-faithful source preservation, typed
/// data access, and path-targeted edits.
///
/// `Document` carries three coordinated views of the same input:
/// an immutable green tree that reproduces the source byte-for-byte,
/// a typed [`Value`] for data access, and an internal span tree
/// that maps any [`Value`]-shaped path back to a byte range. Edits
/// flow through [`Document::replace_span`] (the primitive) and
/// [`Document::set`] (the path-shaped wrapper); untouched bytes —
/// indentation, comments, blank lines, sibling entries — are
/// preserved verbatim.
///
/// # Examples
///
/// Read-only round-trip:
///
/// ```
/// use noyalib::cst::parse_document;
///
/// let src = "name: noyalib  # the project\nversion: 0.0.1\n";
/// let doc = parse_document(src).unwrap();
/// assert_eq!(doc.to_string(), src);
/// ```
///
/// Path-targeted edit:
///
/// ```
/// use noyalib::cst::parse_document;
///
/// let mut doc = parse_document("name: foo\nversion: 0.0.1\n").unwrap();
/// doc.set("version", "0.0.2").unwrap();
/// assert_eq!(doc.to_string(), "name: foo\nversion: 0.0.2\n");
/// ```
#[derive(Debug, Clone)]
pub struct Document {
    source: Arc<str>,
    green: GreenNode,
    value: Value,
    span_tree: SpanTree,
}

impl Document {
    /// Borrow the root [`GreenNode`].
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::cst::{parse_document, SyntaxKind};
    ///
    /// let doc = parse_document("foo: 1\n").unwrap();
    /// assert_eq!(doc.syntax().kind(), SyntaxKind::Document);
    /// ```
    #[must_use]
    pub fn syntax(&self) -> &GreenNode {
        &self.green
    }

    /// Borrow the typed [`Value`] view of the document.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::cst::parse_document;
    ///
    /// let doc = parse_document("name: noyalib\n").unwrap();
    /// assert_eq!(doc.as_value()["name"].as_str(), Some("noyalib"));
    /// ```
    #[must_use]
    pub fn as_value(&self) -> &Value {
        &self.value
    }

    /// The original source bytes for this document. After an edit
    /// reflects the *current* source.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::cst::parse_document;
    ///
    /// let src = "key: 1\n";
    /// let doc = parse_document(src).unwrap();
    /// assert_eq!(doc.source(), src);
    /// ```
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Resolve a `path` to the byte range of the value at that path,
    /// if any.
    ///
    /// Path syntax matches the rest of the crate (`foo.bar`,
    /// `items[0]`, `items[0].name`). Wildcard / recursive-descent
    /// segments are not supported here — they have no single span.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::cst::parse_document;
    ///
    /// let doc = parse_document("name: noyalib\nversion: 0.0.1\n").unwrap();
    /// let (s, e) = doc.span_at("version").unwrap();
    /// assert_eq!(&doc.source()[s..e], "0.0.1");
    /// ```
    #[must_use]
    pub fn span_at(&self, path: &str) -> Option<(usize, usize)> {
        let segments = parse_query_path(path);
        let (s, e) = resolve_span(&self.value, &self.span_tree, &segments)?;
        // The multi-line plain scalar reader advances past trailing
        // whitespace / newlines before deciding to terminate, so leaf
        // spans extend into trailing trivia. Trim those bytes back so
        // the returned span covers content only.
        Some(trim_trailing_blank(&self.source, s, e))
    }

    /// Return the source slice of the value at `path`.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::cst::parse_document;
    ///
    /// let doc = parse_document("items:\n  - one\n  - two\n").unwrap();
    /// assert_eq!(doc.get("items[1]"), Some("two"));
    /// ```
    #[must_use]
    pub fn get(&self, path: &str) -> Option<&str> {
        let (s, e) = self.span_at(path)?;
        Some(&self.source[s..e])
    }

    /// Replace the bytes in `start..end` with `replacement` and
    /// re-parse. The caller is responsible for `replacement` being a
    /// syntactically valid fragment in that position; if the spliced
    /// source fails to parse, the original document is left
    /// unchanged and the parse error is returned.
    ///
    /// # Errors
    ///
    /// - `Error::Parse` if the resulting source is not valid YAML.
    /// - `Error::Parse` if `start..end` is out of bounds or not a
    ///   character boundary.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::cst::parse_document;
    ///
    /// let mut doc = parse_document("a: 1\n").unwrap();
    /// let (s, e) = doc.span_at("a").unwrap();
    /// doc.replace_span(s, e, "42").unwrap();
    /// assert_eq!(doc.to_string(), "a: 42\n");
    /// ```
    pub fn replace_span(&mut self, start: usize, end: usize, replacement: &str) -> Result<()> {
        if start > end || end > self.source.len() {
            return Err(Error::Parse(format!(
                "replace_span range {start}..{end} out of bounds (source length {})",
                self.source.len()
            )));
        }
        if !self.source.is_char_boundary(start) || !self.source.is_char_boundary(end) {
            return Err(Error::Parse(format!(
                "replace_span range {start}..{end} is not a character boundary"
            )));
        }
        let mut new_source =
            String::with_capacity(self.source.len() - (end - start) + replacement.len());
        new_source.push_str(&self.source[..start]);
        new_source.push_str(replacement);
        new_source.push_str(&self.source[end..]);
        let parsed = parse_full(&new_source)?;
        self.source = parsed.source;
        self.green = parsed.green;
        self.value = parsed.value;
        self.span_tree = parsed.span_tree;
        Ok(())
    }

    /// Replace the value at `path` with `fragment`.
    ///
    /// `fragment` is spliced verbatim into the source — the caller
    /// supplies the YAML representation. This deliberately matches
    /// no scalar style automatically; choose double-quoted, plain,
    /// or block style to suit. Auto-formatting (the `Emit` trait
    /// from the design doc) is a follow-up.
    ///
    /// # Errors
    ///
    /// - `Error::Parse(...)` with "path not found" if `path` does
    ///   not resolve in the current document.
    /// - The same errors as [`Document::replace_span`] otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::cst::parse_document;
    ///
    /// let mut doc = parse_document("name: foo\nversion: 0.0.1\n").unwrap();
    /// doc.set("version", "0.0.2").unwrap();
    /// assert_eq!(doc.to_string(), "name: foo\nversion: 0.0.2\n");
    /// ```
    pub fn set(&mut self, path: &str, fragment: &str) -> Result<()> {
        let (s, e) = self
            .span_at(path)
            .ok_or_else(|| Error::Parse(format!("path not found: {path}")))?;
        self.replace_span(s, e, fragment)
    }

    /// Replace the value at `path` with a typed [`Value`], formatting
    /// the YAML fragment to match the existing scalar style at the
    /// target site.
    ///
    /// Style matching:
    /// - `PlainScalar` — emit plain when safe, double-quoted otherwise.
    /// - `SingleQuotedScalar` — wrap in `'…'` (only string values).
    /// - `DoubleQuotedScalar` — wrap in `"…"` with standard escapes
    ///   (only string values).
    /// - `LiteralScalar` / `FoldedScalar` — currently rejected; block
    ///   scalar formatting is a follow-up.
    ///
    /// Non-string values (numbers, booleans, null) are emitted plain
    /// regardless of the existing style — quoting them would change
    /// the parsed type round-trip.
    ///
    /// # Errors
    ///
    /// - Path not found.
    /// - Target is a collection or block scalar.
    /// - Caller passed a `Sequence` / `Mapping` (use `set` with a
    ///   pre-formatted fragment for those — `set_value` is scalar-only
    ///   for now).
    /// - The same errors as [`Document::replace_span`] otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::cst::parse_document;
    /// use noyalib::Value;
    ///
    /// let mut doc = parse_document("name: noyalib\nversion: 0.0.1\n").unwrap();
    /// doc.set_value("version", &Value::String("0.0.2".into())).unwrap();
    /// assert_eq!(doc.to_string(), "name: noyalib\nversion: 0.0.2\n");
    /// ```
    pub fn set_value(&mut self, path: &str, value: &Value) -> Result<()> {
        let (s, e) = self
            .span_at(path)
            .ok_or_else(|| Error::Parse(format!("path not found: {path}")))?;
        let kind = leaf_kind_at(&self.green, s).ok_or_else(|| {
            Error::Parse("could not locate green-tree leaf at target span".into())
        })?;
        // Neighbour-aware styling: when the site is currently emitted
        // plain (so there is no quoting *intent* to preserve) and a
        // sibling style dominates the surrounding `BlockMapping`,
        // match the neighbours.
        let neighbour = sibling_dominant_scalar_kind(&self.green, s)
            .filter(|_| kind == SyntaxKind::PlainScalar);
        let entry_col = entry_indent_column(&self.source, s);
        let ctx = SiteContext {
            kind,
            neighbour,
            entry_col,
        };
        let fragment = format_value_for_site(value, &ctx)?;
        self.replace_span(s, e, &fragment)
    }

    /// Remove the value at `path` along with its surrounding entry
    /// (key + colon for mappings, `-` indicator for sequences).
    /// Trailing whitespace and the line break are removed too so the
    /// surrounding entries close up with no orphan blank line.
    ///
    /// Restrictions in this phase:
    /// - Block context only — flow-collection entry removal (`[a, b, c]`
    ///   → `[a, c]`) is a follow-up.
    /// - The value must end on the line where its key / `-` indicator
    ///   appears (single-line scalars). Multi-line values and nested
    ///   collections are deferred to the same follow-up that handles
    ///   block-scalar replacement in `set_value`.
    /// - Removing the only entry of a block mapping or sequence is
    ///   rejected — the result would parse differently (an empty
    ///   block becomes `null`), and the caller needs to express that
    ///   intent explicitly.
    ///
    /// # Errors
    ///
    /// - Path not found.
    /// - Restrictions above.
    /// - The same parse-after-edit errors as
    ///   [`Document::replace_span`]; on failure the document is left
    ///   unchanged.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::cst::parse_document;
    ///
    /// let mut doc = parse_document("a: 1\nb: 2\nc: 3\n").unwrap();
    /// doc.remove("b").unwrap();
    /// assert_eq!(doc.to_string(), "a: 1\nc: 3\n");
    /// ```
    pub fn remove(&mut self, path: &str) -> Result<()> {
        let segments = parse_query_path(path);
        let (line_start, line_end) =
            entry_line_span(&self.value, &self.span_tree, &self.source, &segments)?;
        self.replace_span(line_start, line_end, "")
    }

    /// Append a new item to the block sequence at `path`.
    ///
    /// `fragment` is the YAML representation of the *value* — the
    /// `- ` indicator and the surrounding indentation are synthesized
    /// from the existing items so the new line matches the file's
    /// shape. Block sequences only in this phase; flow sequences
    /// (`[…]`) and empty sequences are rejected.
    ///
    /// # Errors
    ///
    /// - `path` does not resolve to a sequence.
    /// - The sequence is a flow collection (`[…]`).
    /// - The sequence has no existing items to anchor indentation on.
    /// - The same parse-after-edit errors as
    ///   [`Document::replace_span`].
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::cst::parse_document;
    ///
    /// let mut doc = parse_document("items:\n  - one\n  - two\n").unwrap();
    /// doc.push_back("items", "three").unwrap();
    /// assert_eq!(doc.to_string(), "items:\n  - one\n  - two\n  - three\n");
    /// ```
    pub fn push_back(&mut self, path: &str, fragment: &str) -> Result<()> {
        let target = path_value(&self.value, path)
            .ok_or_else(|| Error::Parse(format!("path not found: {path}")))?;
        let seq = match target {
            Value::Sequence(s) => s,
            _ => {
                return Err(Error::Parse(
                    "push_back: target path is not a sequence".into(),
                ));
            }
        };
        if seq.is_empty() {
            return Err(Error::Parse(
                "push_back: empty sequence has no anchor for indentation — use `set` with a fragment instead"
                    .into(),
            ));
        }
        // Find the byte range of the LAST existing item to anchor
        // dash indentation and the splice position.
        let item_path = format!("{path}[{}]", seq.len() - 1);
        let (last_start, last_end) = self
            .span_at(&item_path)
            .ok_or_else(|| Error::Parse("push_back: could not resolve last item span".into()))?;
        let dash_col = column_of_preceding_dash(&self.source, last_start).ok_or_else(|| {
            Error::Parse(
                "push_back: only block sequences are supported (no `-` anchor before last item)"
                    .into(),
            )
        })?;
        let line_end = end_of_line(&self.source, last_end);
        let indent: String = " ".repeat(dash_col);
        let new_line = format!("{indent}- {fragment}\n");
        self.replace_span(line_end, line_end, &new_line)
    }

    /// Insert a new sequence item immediately after the item at
    /// `item_path` (e.g. `"items[1]"`).
    ///
    /// `fragment` is the YAML representation of the value; the
    /// `- ` indicator and indentation are derived from the item at
    /// `item_path`.
    ///
    /// # Errors
    ///
    /// - `item_path` does not end in an index.
    /// - The path does not resolve to a sequence item in a block
    ///   sequence.
    /// - The same parse-after-edit errors as
    ///   [`Document::replace_span`].
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::cst::parse_document;
    ///
    /// let mut doc = parse_document("items:\n  - one\n  - three\n").unwrap();
    /// doc.insert_after("items[0]", "two").unwrap();
    /// assert_eq!(
    ///     doc.to_string(),
    ///     "items:\n  - one\n  - two\n  - three\n",
    /// );
    /// ```
    pub fn insert_after(&mut self, item_path: &str, fragment: &str) -> Result<()> {
        let segments = parse_query_path(item_path);
        if !matches!(segments.last(), Some(QuerySegment::Index(_))) {
            return Err(Error::Parse(
                "insert_after: path must end with a sequence index, e.g. `items[2]`".into(),
            ));
        }
        let (item_start, item_end) = self
            .span_at(item_path)
            .ok_or_else(|| Error::Parse(format!("path not found: {item_path}")))?;
        let dash_col = column_of_preceding_dash(&self.source, item_start).ok_or_else(|| {
            Error::Parse(
                "insert_after: only block sequences are supported (no `-` anchor before item)"
                    .into(),
            )
        })?;
        let line_end = end_of_line(&self.source, item_end);
        let indent: String = " ".repeat(dash_col);
        let new_line = format!("{indent}- {fragment}\n");
        self.replace_span(line_end, line_end, &new_line)
    }
}

impl fmt::Display for Document {
    /// Re-emit the document. For any input that parses successfully,
    /// the result equals the original bytes verbatim. `Display`
    /// drives `Document::to_string()` via the standard `ToString`
    /// blanket impl.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut out = String::with_capacity(self.green.text_len());
        self.green.write_text(&mut out);
        f.write_str(&out)
    }
}

/// Parse a YAML stream into an editable [`Document`].
///
/// # Errors
///
/// Returns the same parse errors as [`crate::from_str`] — the green
/// tree is built off the same scanner, so every strictness fix in
/// the regular parser applies here too.
///
/// # Examples
///
/// ```
/// use noyalib::cst::parse_document;
///
/// assert_eq!(parse_document("a: 1\n").unwrap().to_string(), "a: 1\n");
/// ```
pub fn parse_document(input: &str) -> Result<Document> {
    let parsed = parse_full(input)?;
    Ok(Document {
        source: parsed.source,
        green: parsed.green,
        value: parsed.value,
        span_tree: parsed.span_tree,
    })
}

/// Parse a YAML stream and return one [`Document`] per logical
/// document.
///
/// Boundaries follow YAML 1.2.2 §9.1: an explicit `...` end marker
/// closes the current document, and a fresh `---` opens the next.
/// Trivia (comments, blank lines) between an explicit `...` and the
/// next document is treated as the next document's prologue;
/// trailing trivia at end-of-stream is attached to the last
/// document so concatenating each document's source reproduces the
/// original input byte-for-byte.
///
/// # Errors
///
/// Same as [`parse_document`].
///
/// # Examples
///
/// Single document:
///
/// ```
/// use noyalib::cst::parse_stream;
///
/// let src = "---\nfoo: 1\n";
/// let docs = parse_stream(src).unwrap();
/// assert_eq!(docs.len(), 1);
/// assert_eq!(docs[0].to_string(), src);
/// ```
///
/// Two documents — split on `---`:
///
/// ```
/// use noyalib::cst::{parse_stream, Document};
///
/// let src = "---\nfoo: 1\n---\nbar: 2\n";
/// let docs = parse_stream(src).unwrap();
/// assert_eq!(docs.len(), 2);
/// assert_eq!(docs[0].as_value()["foo"].as_i64(), Some(1));
/// assert_eq!(docs[1].as_value()["bar"].as_i64(), Some(2));
/// let joined: String = docs.iter().map(Document::source).collect();
/// assert_eq!(joined, src);
/// ```
pub fn parse_stream(input: &str) -> Result<Vec<Document>> {
    let bounds = document_boundaries(input)?;
    if bounds.len() <= 1 {
        return Ok(vec![parse_document(input)?]);
    }
    let mut out = Vec::with_capacity(bounds.len());
    for (s, e) in bounds {
        if s == e {
            continue;
        }
        out.push(parse_document(&input[s..e])?);
    }
    Ok(out)
}

// ── Path resolution ─────────────────────────────────────────────────

fn trim_trailing_blank(source: &str, start: usize, mut end: usize) -> (usize, usize) {
    let bytes = source.as_bytes();
    while end > start {
        match bytes[end - 1] {
            b' ' | b'\t' | b'\n' | b'\r' => end -= 1,
            _ => break,
        }
    }
    (start, end)
}

fn resolve_span(
    value: &Value,
    span_tree: &SpanTree,
    segments: &[QuerySegment],
) -> Option<(usize, usize)> {
    if segments.is_empty() {
        return Some(match span_tree {
            SpanTree::Leaf(s, e) => (*s, *e),
            SpanTree::Sequence { start, end, .. } | SpanTree::Mapping { start, end, .. } => {
                (*start, *end)
            }
        });
    }
    let (head, tail) = segments.split_first()?;
    match (head, value, span_tree) {
        (QuerySegment::Key(k), Value::Mapping(m), SpanTree::Mapping { entries, .. }) => {
            // `m` (an IndexMap) preserves insertion order, matching
            // the parallel order in `entries` (see `span_context::walk`).
            for ((mk, mv), (_, child_tree)) in m.iter().zip(entries.iter()) {
                if mk == k {
                    return resolve_span(mv, child_tree, tail);
                }
            }
            None
        }
        (QuerySegment::Index(i), Value::Sequence(seq), SpanTree::Sequence { items, .. }) => {
            let v = seq.get(*i)?;
            let t = items.get(*i)?;
            resolve_span(v, t, tail)
        }
        // Wildcard / recursive descent are unsupported because they
        // do not resolve to a *single* span; the caller would need a
        // multi-span API.
        _ => None,
    }
}

// ── Entry-line resolution (used by `remove`) ────────────────────────

/// Find the byte range of the *entire* mapping entry or sequence entry
/// addressed by `segments` — including its key / `-` indicator,
/// leading indentation, and trailing line break — so a caller can
/// splice the empty string in to delete it.
fn entry_line_span(
    value: &Value,
    span_tree: &SpanTree,
    source: &str,
    segments: &[QuerySegment],
) -> Result<(usize, usize)> {
    if segments.is_empty() {
        return Err(Error::Parse(
            "remove requires a non-empty path (cannot remove the document root)".into(),
        ));
    }

    let (head, tail) = segments
        .split_first()
        .ok_or_else(|| Error::Parse("path not found".into()))?;

    // Recurse into nested mappings / sequences until the segment list
    // identifies the *parent* of the entry to remove.
    if !tail.is_empty() {
        let (child_value, child_tree) = match (head, value, span_tree) {
            (QuerySegment::Key(k), Value::Mapping(m), SpanTree::Mapping { entries, .. }) => {
                let pos = m
                    .iter()
                    .position(|(mk, _)| mk == k)
                    .ok_or_else(|| Error::Parse(format!("path not found: missing key {k:?}")))?;
                (
                    m.iter().nth(pos).map(|(_, v)| v).expect("pos in range"),
                    &entries[pos].1,
                )
            }
            (QuerySegment::Index(i), Value::Sequence(seq), SpanTree::Sequence { items, .. }) => (
                seq.get(*i).ok_or_else(|| {
                    Error::Parse(format!("path not found: index {i} out of bounds"))
                })?,
                items.get(*i).ok_or_else(|| {
                    Error::Parse(format!("path not found: index {i} out of bounds"))
                })?,
            ),
            _ => return Err(Error::Parse("path not found".into())),
        };
        return entry_line_span(child_value, child_tree, source, tail);
    }

    // Final segment — locate this entry's key / dash and value.
    match (head, value, span_tree) {
        (QuerySegment::Key(k), Value::Mapping(m), SpanTree::Mapping { entries, .. }) => {
            if m.len() <= 1 {
                return Err(Error::Parse(
                    "remove cannot delete the only entry of a mapping".into(),
                ));
            }
            let pos = m
                .iter()
                .position(|(mk, _)| mk == k)
                .ok_or_else(|| Error::Parse(format!("path not found: missing key {k:?}")))?;
            let ((key_start, _key_end), child_tree) = &entries[pos];
            let raw_value_end = match child_tree {
                SpanTree::Leaf(_, e) => *e,
                SpanTree::Sequence { end, .. } | SpanTree::Mapping { end, .. } => *end,
            };
            let (_, value_end) = trim_trailing_blank(source, *key_start, raw_value_end);
            require_single_line(source, *key_start, value_end)?;
            Ok(line_extent(source, *key_start, value_end))
        }
        (QuerySegment::Index(i), Value::Sequence(seq), SpanTree::Sequence { items, .. }) => {
            if seq.len() <= 1 {
                return Err(Error::Parse(
                    "remove cannot delete the only entry of a sequence".into(),
                ));
            }
            let item_tree = items
                .get(*i)
                .ok_or_else(|| Error::Parse(format!("path not found: index {i} out of bounds")))?;
            let (value_start, raw_value_end) = match item_tree {
                SpanTree::Leaf(s, e) => (*s, *e),
                SpanTree::Sequence { start, end, .. } | SpanTree::Mapping { start, end, .. } => {
                    (*start, *end)
                }
            };
            let (_, value_end) = trim_trailing_blank(source, value_start, raw_value_end);
            // The `-` indicator sits before the value on the same line,
            // separated by inline whitespace. Walk backward to find it.
            let dash_pos = locate_preceding_dash(source, value_start).ok_or_else(|| {
                Error::Parse(
                    "remove: could not locate '-' indicator preceding sequence item".into(),
                )
            })?;
            require_single_line(source, dash_pos, value_end)?;
            Ok(line_extent(source, dash_pos, value_end))
        }
        _ => Err(Error::Parse("path not found".into())),
    }
}

/// Walk backward from `value_start` past inline whitespace and find
/// the `-` indicator that opened this sequence entry. Returns its
/// byte offset, or `None` if no dash is found on the same line.
/// Resolve `path` against `value` and return the addressed value.
/// Mirrors the resolution logic of `span_at` but works directly on
/// the typed [`Value`] tree.
fn path_value<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    let segments = parse_query_path(path);
    let mut cur = value;
    for seg in &segments {
        match (seg, cur) {
            (QuerySegment::Key(k), Value::Mapping(m)) => {
                let (_k, v) = m.iter().find(|(mk, _)| *mk == k)?;
                cur = v;
            }
            (QuerySegment::Index(i), Value::Sequence(seq)) => {
                cur = seq.get(*i)?;
            }
            _ => return None,
        }
    }
    Some(cur)
}

/// Column of the `-` indicator on the same line as `value_start`,
/// found by walking backward over inline whitespace. `None` if no
/// dash precedes the value on its line.
fn column_of_preceding_dash(source: &str, value_start: usize) -> Option<usize> {
    let dash_pos = locate_preceding_dash(source, value_start)?;
    let bytes = source.as_bytes();
    let mut line_start = dash_pos;
    while line_start > 0 && bytes[line_start - 1] != b'\n' {
        line_start -= 1;
    }
    Some(dash_pos - line_start)
}

/// Position of the byte immediately past the next `\n` at or after
/// `pos`. If `pos` already points past a newline, returns `pos`.
/// At end-of-input, returns `source.len()`.
fn end_of_line(source: &str, pos: usize) -> usize {
    let bytes = source.as_bytes();
    let mut i = pos;
    while i < bytes.len() && bytes[i] != b'\n' {
        i += 1;
    }
    if i < bytes.len() {
        i + 1
    } else {
        i
    }
}

fn locate_preceding_dash(source: &str, value_start: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut i = value_start;
    while i > 0 {
        i -= 1;
        match bytes[i] {
            b' ' | b'\t' => {}
            b'-' => return Some(i),
            b'\n' | b'\r' => return None,
            _ => return None,
        }
    }
    None
}

/// Reject ranges that span multiple lines — Phase 2C `remove` only
/// handles entries whose value ends on the same line as the key /
/// dash.
fn require_single_line(source: &str, start: usize, end: usize) -> Result<()> {
    let segment = &source.as_bytes()[start..end];
    if segment.contains(&b'\n') {
        return Err(Error::Parse(
            "remove: multi-line / nested-value entries are not yet supported".into(),
        ));
    }
    Ok(())
}

/// Extend `(start, end)` outward to a full-line byte range:
/// - leftward to the byte after the previous `\n` (or 0).
/// - rightward through the trailing `\n` (or to EOF).
fn line_extent(source: &str, start: usize, end: usize) -> (usize, usize) {
    let bytes = source.as_bytes();
    let mut s = start;
    while s > 0 && bytes[s - 1] != b'\n' {
        s -= 1;
    }
    let mut e = end;
    while e < bytes.len() && bytes[e] != b'\n' {
        e += 1;
    }
    if e < bytes.len() {
        e += 1;
    }
    (s, e)
}

// ── Green-tree leaf lookup ──────────────────────────────────────────

/// Return the [`SyntaxKind`] of the leaf containing byte position
/// `target` in `node`. Walks the green tree recursively with a
/// running offset.
fn leaf_kind_at(node: &GreenNode, target: usize) -> Option<SyntaxKind> {
    let mut pos = 0;
    for child in node.children() {
        let len = child.text_len();
        match child {
            GreenChild::Token { kind, .. } => {
                if pos <= target && target < pos + len {
                    return Some(*kind);
                }
            }
            GreenChild::Node(inner) => {
                if pos <= target && target < pos + len {
                    return leaf_kind_at(inner, target - pos);
                }
            }
        }
        pos += len;
    }
    None
}

/// If the leaf at byte `target` lives inside a `BlockMapping`'s
/// `MappingEntry`, scan the *other* entries' value scalars and
/// return their dominant scalar style — but only when that style is
/// `SingleQuotedScalar` or `DoubleQuotedScalar`. A plain-dominant
/// neighbourhood returns `None` (plain is the default fallback for
/// a plain site, so the caller does not need a hint).
fn sibling_dominant_scalar_kind(node: &GreenNode, target: usize) -> Option<SyntaxKind> {
    let (mapping, entry) = enclosing_mapping_and_entry(node, target, 0)?;
    dominant_sibling_value_kind(mapping, entry)
}

/// Walk the tree and return `(BlockMapping, MappingEntry)` ancestors
/// of the leaf at byte `target`, when both exist. Recursion is
/// linear in the tree height plus the children scanned per level.
fn enclosing_mapping_and_entry<'a>(
    node: &'a GreenNode,
    target: usize,
    base: usize,
) -> Option<(&'a GreenNode, &'a GreenNode)> {
    fn walk<'a>(
        node: &'a GreenNode,
        target: usize,
        base: usize,
        cur_mapping: Option<&'a GreenNode>,
        cur_entry: Option<&'a GreenNode>,
    ) -> Option<(&'a GreenNode, &'a GreenNode)> {
        let mut pos = base;
        for child in node.children() {
            let len = child.text_len();
            if pos <= target && target < pos + len {
                match child {
                    GreenChild::Token { .. } => {
                        if let (Some(m), Some(e)) = (cur_mapping, cur_entry) {
                            return Some((m, e));
                        }
                        return None;
                    }
                    GreenChild::Node(inner) => {
                        let new_mapping = if inner.kind() == SyntaxKind::BlockMapping {
                            Some(inner)
                        } else {
                            cur_mapping
                        };
                        let new_entry = if inner.kind() == SyntaxKind::MappingEntry {
                            Some(inner)
                        } else {
                            cur_entry
                        };
                        if let Some(found) = walk(inner, target, pos, new_mapping, new_entry) {
                            return Some(found);
                        }
                    }
                }
            }
            pos += len;
        }
        None
    }
    walk(node, target, base, None, None)
}

/// Tally value-scalar kinds of every `MappingEntry` child of
/// `mapping` *except* the entry being modified. Return the
/// dominant quoted style if and only if it is uniquely the most
/// frequent and there are at least two siblings vouching for it.
fn dominant_sibling_value_kind(
    mapping: &GreenNode,
    exclude: &GreenNode,
) -> Option<SyntaxKind> {
    let exclude_ptr: *const GreenNode = exclude;
    let mut plain = 0usize;
    let mut single = 0usize;
    let mut double = 0usize;
    for child in mapping.children() {
        if let GreenChild::Node(entry) = child {
            if entry.kind() != SyntaxKind::MappingEntry {
                continue;
            }
            // Cheap pointer-equality check — both come from the same
            // `Arc<[GreenChild]>` storage in this tree, so identity
            // comparison is reliable.
            let entry_ptr: *const GreenNode = entry;
            if core::ptr::eq(entry_ptr, exclude_ptr) {
                continue;
            }
            match entry_value_scalar_kind(entry) {
                Some(SyntaxKind::PlainScalar) => plain += 1,
                Some(SyntaxKind::SingleQuotedScalar) => single += 1,
                Some(SyntaxKind::DoubleQuotedScalar) => double += 1,
                _ => {}
            }
        }
    }
    // Need at least two siblings agreeing on a quoted style and a
    // strict plurality over the other quoted style and over plain.
    if single >= 2 && single > double && single > plain {
        return Some(SyntaxKind::SingleQuotedScalar);
    }
    if double >= 2 && double > single && double > plain {
        return Some(SyntaxKind::DoubleQuotedScalar);
    }
    None
}

/// Within a `MappingEntry`, return the syntax kind of the value
/// scalar (the leaf that follows `:`). `None` if the value is a
/// nested collection or otherwise not a single scalar leaf.
fn entry_value_scalar_kind(entry: &GreenNode) -> Option<SyntaxKind> {
    let mut after_colon = false;
    for child in entry.children() {
        match child {
            GreenChild::Token { kind, .. } => {
                if *kind == SyntaxKind::ColonIndicator {
                    after_colon = true;
                    continue;
                }
                if after_colon {
                    if matches!(
                        kind,
                        SyntaxKind::PlainScalar
                            | SyntaxKind::SingleQuotedScalar
                            | SyntaxKind::DoubleQuotedScalar
                            | SyntaxKind::LiteralScalar
                            | SyntaxKind::FoldedScalar
                    ) {
                        return Some(*kind);
                    }
                    // Skip whitespace / newline / comment leaves.
                }
            }
            GreenChild::Node(_) => {
                if after_colon {
                    // Nested collection — value is not a single scalar.
                    return None;
                }
            }
        }
    }
    None
}

// ── Value → YAML scalar fragment ────────────────────────────────────

/// Context the formatter consults when picking a YAML representation
/// for a replacement value at a particular site.
struct SiteContext {
    /// The existing leaf's syntax kind at the splice site.
    kind: SyntaxKind,
    /// A dominant sibling scalar style, when one is unambiguous.
    /// Only consulted when [`Self::kind`] is `PlainScalar`.
    neighbour: Option<SyntaxKind>,
    /// Column of the first non-whitespace byte on the line that
    /// owns the splice site. Used to decide block-scalar
    /// continuation indent.
    entry_col: usize,
}

fn format_value_for_site(value: &Value, ctx: &SiteContext) -> Result<String> {
    match value {
        Value::Null => Ok("null".to_string()),
        Value::Bool(true) => Ok("true".to_string()),
        Value::Bool(false) => Ok("false".to_string()),
        Value::Number(n) => Ok(format_number(n)),
        Value::String(s) => format_string_for_site(s, ctx),
        Value::Sequence(_) | Value::Mapping(_) => Err(Error::Parse(
            "set_value cannot replace a scalar with a collection (use `set` with a fragment)"
                .into(),
        )),
        Value::Tagged(t) => format_value_for_site(t.value(), ctx),
    }
}

fn format_number(n: &Number) -> String {
    // `Number`'s `Display` matches the YAML 1.2 plain representation
    // for the integer/float variants we emit here.
    n.to_string()
}

fn format_string_for_site(s: &str, ctx: &SiteContext) -> Result<String> {
    // Multi-line string in a block context: prefer a literal block
    // scalar (`|` / `|-`) over `\n`-escaped double quotes — a
    // Renovate-style edit that lifts a one-line value into many
    // lines should look like the rest of the file would have, not
    // an escaped one-liner.
    if s.contains('\n') && can_use_block_literal(s) && is_block_site(ctx.kind) {
        return Ok(format_block_literal(s, ctx.entry_col));
    }

    match ctx.kind {
        SyntaxKind::PlainScalar => {
            // Neighbour preference only kicks in when the current
            // site is plain — i.e. there is no existing quoting
            // intent to preserve. A surrounding mapping that
            // unambiguously prefers one quoted style nudges the new
            // value into the same style.
            match ctx.neighbour {
                Some(SyntaxKind::SingleQuotedScalar) if !s.contains('\n') => {
                    Ok(format_single_quoted(s))
                }
                Some(SyntaxKind::DoubleQuotedScalar) => Ok(format_double_quoted(s)),
                _ => {
                    if is_plain_safe(s) {
                        Ok(s.to_string())
                    } else {
                        Ok(format_double_quoted(s))
                    }
                }
            }
        }
        SyntaxKind::SingleQuotedScalar => Ok(format_single_quoted(s)),
        SyntaxKind::DoubleQuotedScalar => Ok(format_double_quoted(s)),
        SyntaxKind::LiteralScalar | SyntaxKind::FoldedScalar => {
            // Replacing a block scalar with a *single-line* string
            // is a legitimate edit (e.g. truncating a longer note
            // back to one line). Emit the natural plain/quoted
            // shape rather than a one-line block literal.
            if !s.contains('\n') {
                if is_plain_safe(s) {
                    Ok(s.to_string())
                } else {
                    Ok(format_double_quoted(s))
                }
            } else if can_use_block_literal(s) {
                Ok(format_block_literal(s, ctx.entry_col))
            } else {
                Err(Error::Parse(
                    "set_value: existing block scalar can only be replaced with a string \
                     whose content lines do not begin with whitespace or control characters yet"
                        .into(),
                ))
            }
        }
        _ => Err(Error::Parse(
            "set_value: target site is not a scalar leaf".into(),
        )),
    }
}

/// `true` when the existing leaf's syntax kind belongs to a
/// block-context scalar — block mappings/sequences are the only
/// place a literal `|` block scalar makes sense.
fn is_block_site(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::PlainScalar
            | SyntaxKind::SingleQuotedScalar
            | SyntaxKind::DoubleQuotedScalar
            | SyntaxKind::LiteralScalar
            | SyntaxKind::FoldedScalar
    )
}

/// Conservative check: a string is safely representable as a literal
/// block scalar only when none of its lines begin with a horizontal
/// whitespace character (which would require an explicit indent
/// indicator we do not yet emit), it contains no control characters
/// other than `\n`, and its trailing-newline count is zero or one
/// (matched by the `|-` and `|` chomping indicators respectively).
fn can_use_block_literal(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    // Reject control characters except `\n` and `\t` between content.
    for &b in s.as_bytes() {
        if (b < 0x20 && b != b'\n' && b != b'\t') || b == 0x7F {
            return false;
        }
    }
    // Strip up to one trailing newline; reject more than one.
    let trimmed = s.strip_suffix('\n').unwrap_or(s);
    if trimmed.ends_with('\n') {
        return false;
    }
    // No line may start with a space or tab — that requires an
    // explicit indentation indicator we do not emit yet.
    for line in trimmed.split('\n') {
        if line.starts_with(' ') || line.starts_with('\t') {
            return false;
        }
    }
    true
}

/// Format `s` as a literal block scalar (`|` or `|-`) at
/// `entry_col + 2` indent.
fn format_block_literal(s: &str, entry_col: usize) -> String {
    let trailing_nl = s.ends_with('\n');
    let body = if trailing_nl { &s[..s.len() - 1] } else { s };
    let indent_str = " ".repeat(entry_col + 2);

    let mut out = String::with_capacity(s.len() + 8 + indent_str.len() * (body.matches('\n').count() + 1));
    out.push('|');
    if !trailing_nl {
        // Strip chomping indicator removes any trailing newlines, so
        // we can faithfully encode the no-trailing-newline case.
        out.push('-');
    }
    out.push('\n');
    let mut first = true;
    for line in body.split('\n') {
        if !first {
            out.push('\n');
        }
        first = false;
        out.push_str(&indent_str);
        out.push_str(line);
    }
    // `replace_span` pastes the fragment in place of the value
    // bytes only — the trailing line break that separates this
    // entry from the next is already in the surrounding source.
    out
}

/// Compute the column (zero-based) of the first non-whitespace byte
/// on the line that contains `pos` in `source`. For
/// `  version: 0.0.1\n` with `pos` at the value scalar's start,
/// returns 2.
fn entry_indent_column(source: &str, pos: usize) -> usize {
    let bytes = source.as_bytes();
    let mut line_start = pos.min(bytes.len());
    while line_start > 0 && bytes[line_start - 1] != b'\n' {
        line_start -= 1;
    }
    let mut col = line_start;
    while col < bytes.len() && (bytes[col] == b' ' || bytes[col] == b'\t') {
        col += 1;
    }
    col - line_start
}

/// `true` if `s` can be safely emitted as a YAML plain scalar without
/// being misparsed as a different type (bool, null, number) or
/// triggering a structural indicator. Conservative — when in doubt,
/// the caller falls back to a quoted style.
fn is_plain_safe(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    // Reserved scalars that resolve to non-string types.
    if matches!(
        s,
        "null"
            | "Null"
            | "NULL"
            | "~"
            | "true"
            | "True"
            | "TRUE"
            | "false"
            | "False"
            | "FALSE"
            | "yes"
            | "Yes"
            | "YES"
            | "no"
            | "No"
            | "NO"
            | "on"
            | "On"
            | "ON"
            | "off"
            | "Off"
            | "OFF"
    ) {
        return false;
    }
    if looks_like_number(s) {
        return false;
    }
    let bytes = s.as_bytes();
    // Cannot start with structural / flow / quote indicators.
    let first = bytes[0];
    if matches!(
        first,
        b'-' | b'?'
            | b':'
            | b','
            | b'['
            | b']'
            | b'{'
            | b'}'
            | b'#'
            | b'&'
            | b'*'
            | b'!'
            | b'|'
            | b'>'
            | b'\''
            | b'"'
            | b'%'
            | b'@'
            | b'`'
            | b' '
            | b'\t'
    ) {
        return false;
    }
    // Cannot end with whitespace.
    if matches!(*bytes.last().unwrap(), b' ' | b'\t') {
        return false;
    }
    // Disallow line breaks and control characters; disallow `: ` and
    // ` #` which terminate plain scalars in block context.
    let mut prev: u8 = 0;
    for &b in bytes {
        if b < 0x20 || b == 0x7F {
            return false;
        }
        if b == b' ' && prev == b':' {
            return false;
        }
        if b == b'#' && prev == b' ' {
            return false;
        }
        prev = b;
    }
    true
}

fn looks_like_number(s: &str) -> bool {
    // Leading sign or digit makes it a number candidate.
    let mut chars = s.chars();
    let first = match chars.next() {
        Some(c) => c,
        None => return false,
    };
    let candidate = matches!(first, '-' | '+' | '.') || first.is_ascii_digit();
    if !candidate {
        return false;
    }
    // Defer the actual parse to `Number`'s integer/float resolvers via
    // the streaming scalar resolver (which is the source of truth for
    // what the parser would treat as a number).
    let scalar = crate::streaming::resolve_plain(s, false, false);
    matches!(
        scalar,
        crate::streaming::Scalar::Int(_) | crate::streaming::Scalar::Float(_)
    )
}

fn format_single_quoted(s: &str) -> String {
    // YAML 1.2 §7.3.3: single quote is the only escape — `''` for `'`.
    let mut out = String::with_capacity(s.len() + 2);
    out.push('\'');
    for ch in s.chars() {
        if ch == '\'' {
            out.push_str("''");
        } else {
            out.push(ch);
        }
    }
    out.push('\'');
    out
}

fn format_double_quoted(s: &str) -> String {
    // YAML 1.2 §5.7 + §7.3.2: standard JSON-like escapes plus the
    // YAML extras (`\0`, `\a`, `\v`, `\e`, `\N`, `\_`, `\L`, `\P`).
    // For Phase 2B we emit the JSON-compatible subset; the others
    // are unnecessary for round-tripping textual content and would
    // complicate the diff if we surface them.
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\x08' => out.push_str("\\b"),
            '\x0c' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => {
                let _ = write!(&mut out, "\\u{:04X}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
