// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Immutable green-node primitive (zero-copy storage).
//!
//! A `GreenNode` retains the original source string as an
//! `Arc<str>` and stores token leaves as byte ranges into that
//! source — no per-leaf allocation, no copy of the source bytes.
//! Cloning a `GreenNode` is `O(1)` (two `Arc` increments). The
//! concatenation of every descendant leaf's text in document order
//! reproduces the original input byte-for-byte.

use core::ops::Range;

use crate::cst::syntax::SyntaxKind;
use crate::prelude::*;

/// A leaf-or-node child of a [`GreenNode`].
///
/// `Token` variants hold a source byte range rather than an owned
/// string slice — call [`GreenChild::token_text`] (or read from the
/// enclosing [`GreenNode`]'s [`source`](GreenNode::source)) to
/// materialise the text.
///
/// # Examples
///
/// ```
/// use noyalib::cst::{parse_document, GreenChild, SyntaxKind};
///
/// let doc = parse_document("a: 1\n").unwrap();
/// let src = doc.syntax().source();
/// for child in doc.syntax().children() {
///     if let GreenChild::Token { kind, range } = child {
///         let text = &src[range.clone()];
///         assert_eq!(text.is_empty(), false);
///         let _ = (kind, text);
///     }
/// }
/// ```
#[derive(Debug, Clone)]
pub enum GreenChild {
    /// A nested node.
    Node(GreenNode),
    /// A leaf token. `range` indexes the enclosing node's source.
    Token {
        /// Classification of this leaf.
        kind: SyntaxKind,
        /// Byte range into the enclosing [`GreenNode::source`].
        range: Range<usize>,
    },
}

impl GreenChild {
    /// The total byte length of this child's contribution to its
    /// parent's text.
    #[must_use]
    pub fn text_len(&self) -> usize {
        match self {
            Self::Node(n) => n.text_len(),
            Self::Token { range, .. } => range.end - range.start,
        }
    }

    /// Borrow the source text of this leaf, given its enclosing
    /// node's source. Returns `None` for `Node` variants.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::cst::{parse_document, GreenChild, GreenNode};
    ///
    /// fn first_leaf_text<'a>(node: &GreenNode, src: &'a str) -> Option<&'a str> {
    ///     for c in node.children() {
    ///         match c {
    ///             GreenChild::Token { .. } => return c.token_text(src),
    ///             GreenChild::Node(n) => {
    ///                 if let Some(t) = first_leaf_text(n, src) {
    ///                     return Some(t);
    ///                 }
    ///             }
    ///         }
    ///     }
    ///     None
    /// }
    ///
    /// let doc = parse_document("a: 1\n").unwrap();
    /// let src = doc.syntax().source();
    /// assert_eq!(first_leaf_text(doc.syntax(), src), Some("a"));
    /// ```
    #[must_use]
    pub fn token_text<'s>(&self, source: &'s str) -> Option<&'s str> {
        match self {
            Self::Token { range, .. } => Some(&source[range.clone()]),
            Self::Node(_) => None,
        }
    }

    /// Append this child's text into `out`. The caller passes the
    /// enclosing node's source (handed down by
    /// [`GreenNode::write_text`]).
    pub(crate) fn write_text(&self, out: &mut String, source: &str) {
        match self {
            Self::Node(n) => n.write_text(out),
            Self::Token { range, .. } => out.push_str(&source[range.clone()]),
        }
    }
}

/// An immutable, byte-faithful syntax-tree node.
///
/// Every `GreenNode` carries an `Arc<str>` of the original source.
/// Token leaves under it are byte ranges into that source — no
/// per-token allocation. Cloning a `GreenNode` is `O(1)` and safe
/// across threads.
///
/// The text of a node is the concatenation, in document order, of
/// the text of every descendant leaf. For an unmodified parse this
/// equals the original input.
///
/// # Examples
///
/// ```
/// use noyalib::cst::parse_document;
///
/// let src = "key: value\n";
/// let doc = parse_document(src).unwrap();
/// assert_eq!(doc.syntax().text_len(), src.len());
/// assert_eq!(doc.syntax().source(), src);
/// ```
#[derive(Debug, Clone)]
pub struct GreenNode {
    kind: SyntaxKind,
    text_len: usize,
    children: Arc<[GreenChild]>,
    source: Arc<str>,
}

impl GreenNode {
    /// Build a green node from its kind, the original source, and
    /// its children. The total `text_len` is summed from the
    /// children — callers do not need to compute it separately.
    #[must_use]
    pub fn new(kind: SyntaxKind, source: Arc<str>, children: Vec<GreenChild>) -> Self {
        let text_len = children.iter().map(GreenChild::text_len).sum();
        Self {
            kind,
            text_len,
            children: Arc::from(children),
            source,
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

    /// Borrow the underlying source text. All token ranges in
    /// descendants index into this string.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::cst::parse_document;
    ///
    /// let doc = parse_document("foo: 1\n").unwrap();
    /// assert_eq!(doc.syntax().source(), "foo: 1\n");
    /// ```
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Iterate immediate children of this node.
    pub fn children(&self) -> impl Iterator<Item = &GreenChild> {
        self.children.iter()
    }

    /// Concatenation of every descendant leaf's text in document
    /// order. For an unmodified parse, identical to the source input.
    #[must_use]
    pub fn text(&self) -> String {
        let mut out = String::with_capacity(self.text_len);
        self.write_text(&mut out);
        out
    }

    pub(crate) fn write_text(&self, out: &mut String) {
        for child in self.children.iter() {
            child.write_text(out, &self.source);
        }
    }
}
