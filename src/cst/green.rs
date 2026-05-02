// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Immutable green-node primitive (Phase B: relative-len leaves).
//!
//! A `GreenNode` is purely structural — it stores `SyntaxKind` plus
//! children, and tracks `text_len` (the sum of its descendants'
//! byte lengths). Token leaves carry only their `len`, not an
//! absolute byte range. The actual source text lives once, on the
//! [`crate::cst::Document`] that owns the tree; every text-bearing
//! API takes a `source` argument.
//!
//! This shape is what makes incremental edits cheap: a splice only
//! rewrites the path from the root down to the spliced node's
//! parent. Pre- and post-splice subtrees are reused via cheap
//! `Arc<[GreenChild]>` clones — no per-leaf range arithmetic.
//!
//! # Migrating from the absolute-range API
//!
//! Phase A's `GreenChild::Token { kind, range }` is now
//! `{ kind, len }`. To recover an absolute byte position, walk
//! the tree from the root accumulating offsets — see the doctest
//! on [`GreenChild::token_text`].

use crate::cst::syntax::SyntaxKind;
use crate::prelude::*;

/// A leaf-or-node child of a [`GreenNode`].
///
/// Token leaves carry only their byte length within their parent.
/// To materialise text, walk the tree from the root and pass the
/// running offset down (see [`GreenChild::token_text`]).
///
/// # Examples
///
/// ```
/// use noyalib::cst::{parse_document, GreenChild, SyntaxKind};
///
/// let doc = parse_document("a: 1\n").unwrap();
/// let src = doc.source();
/// // Walk children, tracking byte offset, to materialise leaf text.
/// let mut offset = 0;
/// for child in doc.syntax().children() {
///     if let GreenChild::Token { kind, len } = child {
///         let text = &src[offset..offset + len];
///         assert_eq!(text.is_empty(), false);
///         let _ = (kind, text);
///     }
///     offset += child.text_len();
/// }
/// ```
#[derive(Debug, Clone)]
pub enum GreenChild {
    /// A nested node.
    Node(GreenNode),
    /// A leaf token. `len` is its byte length in the source — its
    /// absolute position depends on the running offset accumulated
    /// while walking from the root.
    Token {
        /// Classification of this leaf.
        kind: SyntaxKind,
        /// Byte length of this leaf in the source.
        len: usize,
    },
}

impl GreenChild {
    /// Total byte length of this child's contribution to its
    /// parent's text. For nodes this is `text_len()`; for tokens
    /// it is `len`.
    #[must_use]
    pub fn text_len(&self) -> usize {
        match self {
            Self::Node(n) => n.text_len(),
            Self::Token { len, .. } => *len,
        }
    }

    /// Borrow the source text of this leaf, given `source` (the
    /// document's source) and `offset` (the running byte position
    /// at which this child begins). Returns `None` for `Node`
    /// variants — recurse into them with `offset + 0` as the new
    /// base for their children.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::cst::{parse_document, GreenChild, GreenNode};
    ///
    /// fn first_leaf_text<'a>(node: &GreenNode, src: &'a str, base: usize) -> Option<&'a str> {
    ///     let mut offset = base;
    ///     for c in node.children() {
    ///         match c {
    ///             GreenChild::Token { .. } => return c.token_text(src, offset),
    ///             GreenChild::Node(n) => {
    ///                 if let Some(t) = first_leaf_text(n, src, offset) {
    ///                     return Some(t);
    ///                 }
    ///             }
    ///         }
    ///         offset += c.text_len();
    ///     }
    ///     None
    /// }
    ///
    /// let doc = parse_document("a: 1\n").unwrap();
    /// assert_eq!(first_leaf_text(doc.syntax(), doc.source(), 0), Some("a"));
    /// ```
    #[must_use]
    pub fn token_text<'s>(&self, source: &'s str, offset: usize) -> Option<&'s str> {
        match self {
            Self::Token { len, .. } => Some(&source[offset..offset + len]),
            Self::Node(_) => None,
        }
    }

    /// Append this child's text into `out`. The caller passes the
    /// document source and the running byte offset at which this
    /// child begins. Returns the offset past the child's last byte.
    pub(crate) fn write_text(&self, out: &mut String, source: &str, offset: usize) -> usize {
        match self {
            Self::Node(n) => n.write_text(out, source, offset),
            Self::Token { len, .. } => {
                out.push_str(&source[offset..offset + len]);
                offset + len
            }
        }
    }
}

/// An immutable, byte-faithful syntax-tree node.
///
/// Phase B: a `GreenNode` is purely structural — it carries `kind`,
/// `text_len`, and an `Arc<[GreenChild]>` of children. The actual
/// source text lives elsewhere (on the owning [`crate::cst::Document`]),
/// and every text-bearing API takes the source as an argument.
///
/// Cloning a `GreenNode` is `O(1)` — three `Arc` increments at most.
///
/// The text of a node is the concatenation, in document order, of
/// the text of every descendant leaf. For an unmodified parse this
/// equals the input.
///
/// # Examples
///
/// ```
/// use noyalib::cst::parse_document;
///
/// let src = "key: value\n";
/// let doc = parse_document(src).unwrap();
/// assert_eq!(doc.syntax().text_len(), src.len());
/// assert_eq!(doc.syntax().text(src), src);
/// ```
#[derive(Debug, Clone)]
pub struct GreenNode {
    kind: SyntaxKind,
    text_len: usize,
    children: Arc<[GreenChild]>,
}

impl GreenNode {
    /// Build a green node from its kind and children. The total
    /// `text_len` is summed from the children — callers do not need
    /// to compute it separately.
    #[must_use]
    pub fn new(kind: SyntaxKind, children: Vec<GreenChild>) -> Self {
        let text_len = children.iter().map(GreenChild::text_len).sum();
        Self {
            kind,
            text_len,
            children: Arc::from(children),
        }
    }

    /// Classification of this node.
    #[must_use]
    pub fn kind(&self) -> SyntaxKind {
        self.kind
    }

    /// Total byte length of this node's text.
    #[must_use]
    pub fn text_len(&self) -> usize {
        self.text_len
    }

    /// Iterate immediate children of this node.
    pub fn children(&self) -> impl Iterator<Item = &GreenChild> {
        self.children.iter()
    }

    /// Concatenation of every descendant leaf's text in document
    /// order, given the source string the leaves index into. For
    /// an unmodified parse this is identical to the input. For a
    /// post-edit document call this with [`crate::cst::Document::source`].
    #[must_use]
    pub fn text(&self, source: &str) -> String {
        let mut out = String::with_capacity(self.text_len);
        let _ = self.write_text(&mut out, source, 0);
        out
    }

    /// Append the descendant text into `out`. Used by
    /// [`Self::text`] and by the document-level `Display` impl.
    pub(crate) fn write_text(&self, out: &mut String, source: &str, offset: usize) -> usize {
        let mut pos = offset;
        for child in self.children.iter() {
            pos = child.write_text(out, source, pos);
        }
        pos
    }
}
