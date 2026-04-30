// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Side-table CST (concrete syntax tree) for lossless round-tripping.
//!
//! This module is the implementation of the design described in
//! `docs/design/green-tree.md`. It exposes a `Document` type that
//! parses YAML byte-faithfully — every byte of the input is retained
//! as a green-tree leaf — so that
//! `parse_document(s).unwrap().to_string()` is byte-identical to `s`
//! for any input the parser accepts.
//!
//! The `Value` API (`from_str`, `to_string`, `StreamingDeserializer`)
//! is unchanged. Trivia capture is enabled only on this path; the
//! fast path pays no extra cost.
//!
//! # Phase 1 scope
//!
//! Read-only. The green tree is a flat sequence of leaves under a
//! single `Document` parent — sufficient to satisfy the round-trip
//! property and to expose source bytes for inspection. Hierarchical
//! nesting (per-mapping / per-sequence parent nodes) and the typed
//! mutation API (`get` / `set` / `replace_span`) are deferred to a
//! follow-up phase per the design doc.
//!
//! # Examples
//!
//! ```
//! use noyalib::cst::parse_document;
//!
//! let src = "name: noyalib  # the project\nversion: 0.0.1\n";
//! let doc = parse_document(src).unwrap();
//! assert_eq!(doc.to_string(), src);
//! ```
//!
//! # Multi-document streams
//!
//! Use [`parse_stream`] for inputs containing `---` / `...` separators:
//!
//! ```
//! use noyalib::cst::parse_stream;
//!
//! let src = "---\nfoo: 1\n...\n---\nbar: 2\n";
//! let docs = parse_stream(src).unwrap();
//! assert_eq!(docs.len(), 1);  // one stream, byte-faithfully captured
//! assert_eq!(docs[0].to_string(), src);
//! ```

mod builder;
mod document;
mod green;
mod syntax;

pub use document::{parse_document, parse_stream, Document};
pub use green::{GreenChild, GreenNode};
pub use syntax::SyntaxKind;
