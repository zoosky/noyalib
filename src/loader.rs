//! Multi-document YAML loading.
//!
//! This module provides functionality for parsing YAML documents that contain
//! multiple documents separated by `---`.
//!
//! # Example
//!
//! ```rust
//! use noyalib::loader::load_all;
//!
//! let yaml = "---
//! name: doc1
//! ---
//! name: doc2
//! ";
//!
//! let docs: Vec<_> = load_all(yaml).unwrap().collect();
//! assert_eq!(docs.len(), 2);
//! ```

use crate::de::ParserConfig;
use crate::error::Result;
use crate::parser;
use crate::span_context::{self, SpanTree};
use crate::value::Value;

/// An iterator over YAML documents in a string.
///
/// Created by the [`load_all`] function.
#[derive(Debug)]
pub struct DocumentIterator {
    docs: std::vec::IntoIter<Value>,
    _span_trees: Vec<SpanTree>,
    total: usize,
}

impl DocumentIterator {
    /// Returns the total number of documents parsed.
    #[must_use]
    pub fn len(&self) -> usize {
        self.total
    }

    /// Returns true if there are no documents.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.total == 0
    }
}

impl Iterator for DocumentIterator {
    type Item = Result<Value>;

    fn next(&mut self) -> Option<Self::Item> {
        self.docs.next().map(Ok)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.docs.size_hint()
    }
}

impl ExactSizeIterator for DocumentIterator {}

/// Load all YAML documents from a string.
///
/// This function parses a YAML string that may contain multiple documents
/// separated by `---` markers. Default security limits are applied.
///
/// # Example
///
/// ```rust
/// use noyalib::loader::load_all;
///
/// let yaml = "---
/// first: 1
/// ---
/// second: 2
/// ";
///
/// let docs: Vec<_> = load_all(yaml).unwrap().filter_map(Result::ok).collect();
/// assert_eq!(docs.len(), 2);
/// ```
///
/// # Errors
///
/// Returns an error if the YAML syntax is invalid.
pub fn load_all(input: &str) -> Result<DocumentIterator> {
    load_all_with_config(input, &ParserConfig::default())
}

/// Load all YAML documents from a string with custom security limits.
///
/// # Errors
///
/// Returns an error if the YAML syntax is invalid or the document
/// exceeds the configured limits.
pub fn load_all_with_config(input: &str, config: &ParserConfig) -> Result<DocumentIterator> {
    let parse_config = parser::ParseConfig::from(config);
    let pairs = parser::parse(input, &parse_config)?;
    let (docs, span_trees): (Vec<_>, Vec<_>) = pairs.into_iter().unzip();
    let total = docs.len();
    Ok(DocumentIterator {
        docs: docs.into_iter(),
        _span_trees: span_trees,
        total,
    })
}

/// Load all YAML documents from a string, returning an error if parsing fails.
///
/// This is an alias for [`load_all`] which also returns errors on invalid
/// syntax.
///
/// # Example
///
/// ```rust
/// use noyalib::loader::try_load_all;
///
/// let yaml = "---
/// first: 1
/// ---
/// second: 2
/// ";
///
/// let iter = try_load_all(yaml).unwrap();
/// assert_eq!(iter.len(), 2);
/// ```
///
/// # Errors
///
/// Returns an error if the YAML syntax is invalid.
pub fn try_load_all(input: &str) -> Result<DocumentIterator> {
    load_all(input)
}

/// Load all YAML documents and deserialize them into a typed vector.
///
/// # Example
///
/// ```rust
/// use noyalib::loader::load_all_as;
/// use serde::Deserialize;
///
/// #[derive(Debug, Deserialize, PartialEq)]
/// struct Doc {
///     name: String,
/// }
///
/// let yaml = "---
/// name: first
/// ---
/// name: second
/// ";
///
/// let docs: Vec<Doc> = load_all_as(yaml).unwrap();
/// assert_eq!(docs.len(), 2);
/// assert_eq!(docs[0].name, "first");
/// assert_eq!(docs[1].name, "second");
/// ```
///
/// # Errors
///
/// Returns an error if parsing fails or if any document cannot be
/// deserialized into the target type.
pub fn load_all_as<T>(input: &str) -> Result<Vec<T>>
where
    T: for<'de> serde::Deserialize<'de>,
{
    let parse_config = parser::ParseConfig::from(&ParserConfig::default());
    let pairs = parser::parse(input, &parse_config)?;
    let mut results = Vec::with_capacity(pairs.len());
    let source: std::sync::Arc<str> = input.into();

    for (value, span_tree) in &pairs {
        let spans = span_context::build_span_map(value, span_tree);
        let ctx = span_context::SpanContext {
            spans,
            source: source.clone(),
        };
        let _guard = span_context::set_span_context(ctx);
        let typed: T = crate::from_value(value)?;
        results.push(typed);
    }

    Ok(results)
}
