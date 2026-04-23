// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Error handling types.

use crate::prelude::*;
use core::fmt;
use std::sync::Arc;

/// A byte-offset location in a YAML document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct Location {
    index: usize,
    line: usize,
    column: usize,
}

impl Location {
    /// Create a new location from a byte index.
    pub fn from_index(input: &str, index: usize) -> Self {
        let mut line = 1;
        let mut column = 1;
        for (i, c) in input.char_indices() {
            if i >= index {
                break;
            }
            if c == '\n' {
                line += 1;
                column = 1;
            } else {
                column += 1;
            }
        }
        Location {
            index,
            line,
            column,
        }
    }

    /// Create a new location from a byte index (legacy compat).
    pub fn new(line: usize, col: usize, index: usize) -> Self {
        Location {
            index,
            line,
            column: col,
        }
    }

    /// The 0-based byte index.
    pub fn index(&self) -> usize {
        self.index
    }

    /// The 1-based line number.
    pub fn line(&self) -> usize {
        self.line
    }

    /// The 1-based column number.
    pub fn column(&self) -> usize {
        self.column
    }
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}, column {}", self.line, self.column)
    }
}

/// Errors that can occur during YAML serialization or deserialization.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// Error during YAML parsing.
    #[error("YAML parse error: {0}")]
    Parse(String),

    /// Error during YAML parsing with location information.
    #[error("YAML parse error at {location}: {message}")]
    ParseWithLocation {
        /// The error message.
        message: String,
        /// The location in the source where the error occurred.
        location: Location,
    },

    /// Error during serialization.
    #[error("serialization error: {0}")]
    Serialize(String),

    /// Error during deserialization.
    #[error("deserialization error: {0}")]
    Deserialize(String),

    /// Error during deserialization with location information.
    #[error("deserialization error at {location}: {message}")]
    DeserializeWithLocation {
        /// The error message.
        message: String,
        /// The location in the source where the error occurred.
        location: Location,
    },

    /// I/O error (requires std feature).
    #[cfg(feature = "std")]
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Custom error message.
    #[error("{0}")]
    Custom(String),

    /// Error when recursion depth limit is exceeded.
    #[error("recursion depth limit exceeded: {depth}")]
    RecursionLimitExceeded {
        /// The current depth.
        depth: usize,
    },

    /// Error when a duplicate key is encountered.
    #[error("duplicate key: {0}")]
    DuplicateKey(String),

    /// Repetition limit exceeded (security limit).
    #[error("alias expansion limit exceeded")]
    RepetitionLimitExceeded,

    /// Unknown anchor encountered.
    #[error("unknown anchor: {0}")]
    UnknownAnchor(String),

    /// Unknown anchor encountered at a specific location.
    #[error("unknown anchor: {name} at {location}")]
    UnknownAnchorAt {
        /// The anchor name.
        name: String,
        /// The location where it was used.
        location: Location,
        /// Optional suggestion for a similar anchor.
        suggestion: Option<(String, Location)>,
    },

    /// Missing field in a mapping.
    #[error("missing field: {0}")]
    MissingField(String),

    /// Unknown field in a mapping.
    #[error("unknown field: {0}")]
    UnknownField(String),

    /// Scalar encountered where a mapping was expected during merge.
    #[error("scalar in merge element")]
    ScalarInMergeElement,

    /// Sequence encountered where a mapping was expected during merge.
    #[error("sequence in merge element")]
    SequenceInMergeElement,

    /// Tagged value encountered during merge.
    #[error("tagged value in merge")]
    TaggedInMerge,

    /// Generic invalid construct error.
    #[error("invalid YAML: {0}")]
    Invalid(String),

    /// A type mismatch error.
    #[error("type mismatch: expected {expected}, found {found}")]
    TypeMismatch {
        /// The expected type.
        expected: &'static str,
        /// The type that was actually found.
        found: String,
    },

    /// Shared error instance (for anchors).
    #[error("{0}")]
    Shared(Arc<Error>),

    /// End of stream reached unexpectedly.
    #[error("unexpected end of stream")]
    EndOfStream,

    /// More than one document found where one was expected.
    #[error("multiple documents in stream; expected exactly one")]
    MoreThanOneDocument,

    /// Scalar in merge (legacy variant).
    #[error("scalar in merge")]
    ScalarInMerge,

    /// Empty tag encountered.
    #[error("empty tag")]
    EmptyTag,

    /// Failed to parse a number.
    #[error("failed to parse number: {0}")]
    FailedToParseNumber(String),

    /// A message error from Serde (compat variant).
    #[error("serde error: {0}")]
    Message(String, Option<usize>),
}

impl Error {
    /// Get the location of the error, if any.
    pub fn location(&self) -> Option<Location> {
        match self {
            Error::ParseWithLocation { location, .. } => Some(*location),
            Error::DeserializeWithLocation { location, .. } => Some(*location),
            Error::UnknownAnchorAt { location, .. } => Some(*location),
            Error::Shared(arc) => arc.location(),
            _ => None,
        }
    }

    /// Format the error with source context. If the error carries a source
    /// location and the line is in range, the output includes a
    /// `line <n>:<col>` prefix, the offending line, and a caret (`^`)
    /// pointing at the column. Out-of-range lines fall back to plain
    /// `Display`.
    pub fn format_with_source(&self, source: &str) -> String {
        let loc = match self.location() {
            Some(l) => l,
            None => return format!("{self}"),
        };
        let line_no = loc.line();
        let col = loc.column();
        let line_idx = line_no.saturating_sub(1);
        let line = match source.lines().nth(line_idx) {
            Some(l) => l,
            None if line_no == 0 => source.lines().next().unwrap_or(""),
            None => return format!("{self}"),
        };
        let caret_col = col.saturating_sub(1);
        let caret: String = core::iter::repeat(' ')
            .take(caret_col)
            .chain(core::iter::once('^'))
            .collect();
        format!("error: {self}\n  --> line {line_no}:{col}\n  {line}\n  {caret}")
    }

    /// Convert the error into a shared Arc pointer. If the error is
    /// already `Error::Shared`, the inner `Arc` is reused without
    /// double-wrapping.
    pub fn into_shared(self) -> Arc<Self> {
        match self {
            Error::Shared(arc) => arc,
            other => Arc::new(other),
        }
    }

    /// Check if the error is a shared error.
    pub fn is_shared(&self) -> bool {
        matches!(self, Error::Shared(_))
    }

    /// Access the inner error if this is a shared error.
    pub fn as_inner(&self) -> Option<&Self> {
        match self {
            Error::Shared(arc) => Some(&**arc),
            _ => None,
        }
    }

    /// Create a new parse error at the given index.
    pub fn parse_at(message: impl Into<String>, source: &str, index: usize) -> Self {
        Error::ParseWithLocation {
            message: message.into(),
            location: Location::from_index(source, index),
        }
    }

    /// Create a new deserialization error at the given index.
    pub fn deserialize_at(message: impl Into<String>, source: &str, index: usize) -> Self {
        Error::DeserializeWithLocation {
            message: message.into(),
            location: Location::from_index(source, index),
        }
    }

    /// Create a new error from a shared error pointer.
    pub fn from_shared(arc: Arc<Error>) -> Error {
        Error::Shared(arc)
    }
}

impl serde::ser::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Error::Custom(msg.to_string())
    }
}

impl serde::de::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Error::Custom(msg.to_string())
    }

    fn missing_field(field: &'static str) -> Self {
        Error::MissingField(field.to_string())
    }

    fn unknown_field(field: &str, _expected: &'static [&'static str]) -> Self {
        Error::UnknownField(field.to_string())
    }
}

/// A result type where the error is [`Error`].
pub type Result<T> = core::result::Result<T, Error>;

#[cfg(feature = "miette")]
impl miette::Diagnostic for Error {
    fn code<'a>(&'a self) -> Option<Box<dyn fmt::Display + 'a>> {
        if let Error::Shared(arc) = self {
            return arc.code();
        }
        let code = match self {
            Error::Parse(_) | Error::ParseWithLocation { .. } => "noyalib::parse",
            Error::Serialize(_) => "noyalib::serialize",
            Error::Deserialize(_) | Error::DeserializeWithLocation { .. } => "noyalib::deserialize",
            Error::TypeMismatch { .. } => "noyalib::type_mismatch",
            Error::MissingField(_) => "noyalib::missing_field",
            Error::UnknownField(_) => "noyalib::unknown_field",
            Error::RecursionLimitExceeded { .. } => "noyalib::recursion_limit",
            Error::RepetitionLimitExceeded => "noyalib::repetition_limit",
            Error::UnknownAnchor(_) | Error::UnknownAnchorAt { .. } => "noyalib::unknown_anchor",
            Error::DuplicateKey(_) => "noyalib::duplicate_key",
            Error::EndOfStream => "noyalib::eof",
            Error::MoreThanOneDocument => "noyalib::multi_document",
            Error::Io(_) => "noyalib::io",
            _ => "noyalib::error",
        };
        Some(Box::new(code))
    }

    fn help<'a>(&'a self) -> Option<Box<dyn fmt::Display + 'a>> {
        let help: Option<String> = match self {
            Error::UnknownAnchorAt {
                suggestion: Some((name, _)),
                ..
            } => Some(format!("did you mean '&{name}'?")),
            // `UnknownAnchor` (legacy, no location) gets a generic hint;
            // `UnknownAnchorAt` without a similar-name suggestion stays
            // `None` so the dual-label diagnostic speaks for itself.
            Error::UnknownAnchor(_) => {
                Some("define the anchor (&name) before referencing it".into())
            }
            Error::RecursionLimitExceeded { .. } => {
                Some("increase ParserConfig::max_depth or simplify nesting".into())
            }
            Error::RepetitionLimitExceeded => {
                Some("increase ParserConfig::max_alias_expansions or reduce alias usage".into())
            }
            Error::DuplicateKey(_) => {
                Some("use DuplicateKeyPolicy::Last or ::Error to control behaviour".into())
            }
            Error::MoreThanOneDocument => {
                Some("use noyalib::load_all() to parse multi-document streams".into())
            }
            _ => None,
        };
        help.map(|s| -> Box<dyn fmt::Display + 'a> { Box::new(s) })
    }

    fn source_code(&self) -> Option<&dyn miette::SourceCode> {
        if let Error::Shared(arc) = self {
            return arc.source_code();
        }
        None
    }

    fn labels(&self) -> Option<Box<dyn Iterator<Item = miette::LabeledSpan> + '_>> {
        match self {
            Error::ParseWithLocation { message, location } => Some(Box::new(core::iter::once(
                miette::LabeledSpan::new(Some(message.clone()), location.index(), 1),
            ))),
            Error::DeserializeWithLocation { message, location } => {
                Some(Box::new(core::iter::once(miette::LabeledSpan::new(
                    Some(message.clone()),
                    location.index(),
                    1,
                ))))
            }
            Error::TypeMismatch {
                expected: _,
                found: _,
            } => None,
            Error::UnknownAnchorAt {
                name,
                location,
                suggestion,
            } => {
                let mut labels = Vec::new();
                labels.push(miette::LabeledSpan::new(
                    Some(format!("unknown anchor '{name}'")),
                    location.index(),
                    1,
                ));
                if let Some((s_name, s_loc)) = suggestion {
                    labels.push(miette::LabeledSpan::new(
                        Some(format!("did you mean '&{s_name}'?")),
                        s_loc.index(),
                        1,
                    ));
                }
                Some(Box::new(labels.into_iter()))
            }
            Error::Shared(arc) => arc.labels(),
            Error::Message(msg, Some(offset)) => Some(Box::new(core::iter::once(
                miette::LabeledSpan::new(Some(msg.clone()), *offset, 1),
            ))),
            _ => None,
        }
    }
}

pub(crate) fn closest_name<'a>(
    name: &str,
    names: impl Iterator<Item = &'a str>,
) -> Option<&'a str> {
    let mut best_dist = usize::MAX;
    let mut best_name = None;
    for n in names {
        let dist = edit_distance(name, n);
        if dist < best_dist && dist <= 2 {
            best_dist = dist;
            best_name = Some(n);
        }
    }
    best_name
}

fn edit_distance(a: &str, b: &str) -> usize {
    let a_len = a.chars().count();
    let b_len = b.chars().count();
    if a_len == 0 {
        return b_len;
    }
    if b_len == 0 {
        return a_len;
    }
    let mut row: Vec<usize> = (0..=b_len).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut prev = i + 1;
        for (j, cb) in b.chars().enumerate() {
            let mut next = row[j] + (if ca == cb { 0 } else { 1 });
            if i + 1 < row[j + 1] + 1 && i + 1 < next {
                next = i + 1;
            }
            if prev + 1 < next {
                next = prev + 1;
            }
            row[j] = prev;
            prev = next;
        }
        row[b_len] = prev;
    }
    row[b_len]
}
