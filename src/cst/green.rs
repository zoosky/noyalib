// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Immutable green-node primitive.
//!
//! A `GreenNode` is shared via `Arc`; cloning is cheap. Concatenating
//! every descendant leaf's text in document order reproduces the
//! original input byte-for-byte (the round-trip property).

use crate::cst::syntax::SyntaxKind;
use crate::prelude::*;

/// A leaf-or-node child of a [`GreenNode`].
#[derive(Debug, Clone)]
pub enum GreenChild {
    /// A nested node.
    Node(GreenNode),
    /// A leaf token. `text` is a verbatim source slice; `kind`
    /// classifies what kind of leaf it is.
    Token {
        /// Classification of this leaf.
        kind: SyntaxKind,
        /// Verbatim source bytes.
        text: Box<str>,
    },
}

impl GreenChild {
    /// The total byte length of this child's contribution to its
    /// parent's text.
    #[must_use]
    pub fn text_len(&self) -> usize {
        match self {
            Self::Node(n) => n.text_len(),
            Self::Token { text, .. } => text.len(),
        }
    }

    /// Append this child's verbatim text into `out`.
    pub(crate) fn write_text(&self, out: &mut String) {
        match self {
            Self::Node(n) => n.write_text(out),
            Self::Token { text, .. } => out.push_str(text),
        }
    }
}

/// An immutable, byte-faithful syntax-tree node.
///
/// `GreenNode` is the building block of the side-table CST. Every
/// node owns its children via an `Arc`-shared slice so that cloning
/// a `GreenNode` is `O(1)` and safe across threads.
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
    /// order. For an unmodified parse, identical to the source input.
    #[must_use]
    pub fn text(&self) -> String {
        let mut out = String::with_capacity(self.text_len);
        self.write_text(&mut out);
        out
    }

    pub(crate) fn write_text(&self, out: &mut String) {
        for child in self.children.iter() {
            child.write_text(out);
        }
    }
}
