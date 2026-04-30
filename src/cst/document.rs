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
