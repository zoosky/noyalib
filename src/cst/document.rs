// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Public `Document` handle and parse entry points.

use crate::cst::builder::build_document;
use crate::cst::green::GreenNode;
use crate::error::Result;
use crate::prelude::*;

/// A YAML document with byte-faithful source preservation.
///
/// `Document` wraps an immutable [`GreenNode`] that retains every
/// byte of the input — content, whitespace, comments, line breaks.
/// [`Document::to_string`] returns the original bytes verbatim for
/// any input that parses successfully.
///
/// Phase 1 of the green-tree migration is read-only: there is no
/// mutation API yet. The typed `get` / `set` / `replace_span` surface
/// described in `docs/design/green-tree.md` follows in a later phase.
///
/// # Examples
///
/// ```
/// use noyalib::cst::parse_document;
///
/// let src = "name: noyalib  # the project\nversion: 0.0.1\n";
/// let doc = parse_document(src).unwrap();
/// assert_eq!(doc.to_string(), src);
/// ```
#[derive(Debug, Clone)]
pub struct Document {
    green: GreenNode,
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
/// Phase 1: the returned `Document` covers the entire input as a
/// single byte-faithful tree. Per-document splitting (one `Document`
/// per `---` / `...` boundary) lands in a later phase along with the
/// typed mutation API; until then, multi-document inputs round-trip
/// correctly but are not split.
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
    let green = build_document(input)?;
    Ok(Document { green })
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
