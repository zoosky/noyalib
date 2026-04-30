// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Public `Document` handle and parse / mutation entry points.

use core::fmt::Write as _;

use crate::cst::builder::parse_full;
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
    source: String,
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
        self.source = new_source;
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
        let fragment = format_value_for_site(value, kind)?;
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
        source: input.to_owned(),
        green: parsed.green,
        value: parsed.value,
        span_tree: parsed.span_tree,
    })
}

/// Parse a YAML stream and return one [`Document`] per logical
/// document.
///
/// Phase 1: returns a `Vec` containing exactly one `Document` that
/// covers the whole input — multi-document splitting is deferred.
/// The signature is forward-compatible: callers iterating the result
/// today will continue to work when splitting is implemented.
///
/// # Errors
///
/// Same as [`parse_document`].
///
/// # Examples
///
/// ```
/// use noyalib::cst::parse_stream;
///
/// let src = "---\nfoo: 1\n";
/// let docs = parse_stream(src).unwrap();
/// assert_eq!(docs.len(), 1);
/// assert_eq!(docs[0].to_string(), src);
/// ```
pub fn parse_stream(input: &str) -> Result<Vec<Document>> {
    Ok(vec![parse_document(input)?])
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
/// `target` in `node`. Walks the green tree once with a running
/// offset; for the current flat layout the inner recursion never
/// fires, but it is correct for the structural variant that follows.
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

// ── Value → YAML scalar fragment ────────────────────────────────────

fn format_value_for_site(value: &Value, kind: SyntaxKind) -> Result<String> {
    match value {
        Value::Null => Ok("null".to_string()),
        Value::Bool(true) => Ok("true".to_string()),
        Value::Bool(false) => Ok("false".to_string()),
        Value::Number(n) => Ok(format_number(n)),
        Value::String(s) => format_string_for_site(s, kind),
        Value::Sequence(_) | Value::Mapping(_) => Err(Error::Parse(
            "set_value cannot replace a scalar with a collection (use `set` with a fragment)"
                .into(),
        )),
        Value::Tagged(t) => format_value_for_site(t.value(), kind),
    }
}

fn format_number(n: &Number) -> String {
    // `Number`'s `Display` matches the YAML 1.2 plain representation
    // for the integer/float variants we emit here.
    n.to_string()
}

fn format_string_for_site(s: &str, kind: SyntaxKind) -> Result<String> {
    match kind {
        SyntaxKind::PlainScalar => {
            if is_plain_safe(s) {
                Ok(s.to_string())
            } else {
                Ok(format_double_quoted(s))
            }
        }
        SyntaxKind::SingleQuotedScalar => Ok(format_single_quoted(s)),
        SyntaxKind::DoubleQuotedScalar => Ok(format_double_quoted(s)),
        SyntaxKind::LiteralScalar | SyntaxKind::FoldedScalar => Err(Error::Parse(
            "set_value: replacing a block scalar (|, >) is not yet supported".into(),
        )),
        _ => Err(Error::Parse(
            "set_value: target site is not a scalar leaf".into(),
        )),
    }
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
