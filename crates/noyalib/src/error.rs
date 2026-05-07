// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Error handling types.

use crate::prelude::*;
use core::fmt;

/// A `(line, column, byte index)` location in a YAML document.
///
/// `line` and `column` are **1-based** for any [`Location`]
/// produced by parsing or by [`Location::from_index`] /
/// [`Location::new`]. The single exception is
/// [`Location::default()`] (and the `Spanned::new` constructor it
/// powers), which yields `0/0/0` as a sentinel for "not yet
/// populated by a parser pass." User code that only ever sees a
/// [`Location`] returned from a parser may treat both axes as
/// strictly ≥ 1.
///
/// `index` is always **0-based** and counts UTF-8 bytes from the
/// start of the document.
///
/// # Examples
///
/// ```
/// use noyalib::Location;
/// let loc = Location::from_index("a: 1\nb: 2\n", 5);
/// assert_eq!(loc.line(), 2);
/// assert_eq!(loc.column(), 1);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct Location {
    index: usize,
    line: usize,
    column: usize,
}

impl Location {
    /// Create a new location from a byte index.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::Location;
    /// let loc = Location::from_index("hello\nworld", 6);
    /// assert_eq!(loc.line(), 2);
    /// ```
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

    /// Create a new location from line, column, and byte index.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::Location;
    /// let loc = Location::new(1, 1, 0);
    /// assert_eq!(loc.line(), 1);
    /// ```
    pub fn new(line: usize, col: usize, index: usize) -> Self {
        Location {
            index,
            line,
            column: col,
        }
    }

    /// The 0-based byte index.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::Location;
    /// let loc = Location::from_index("abc", 2);
    /// assert_eq!(loc.index(), 2);
    /// ```
    pub fn index(&self) -> usize {
        self.index
    }

    /// The 1-based line number.
    ///
    /// Returns `0` only for a [`Location::default()`] that has not
    /// been populated by a parser pass; any [`Location`] produced
    /// by [`Location::from_index`], [`Location::new`], or returned
    /// from a parser is `≥ 1`.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::Location;
    /// let loc = Location::from_index("a\nb", 2);
    /// assert_eq!(loc.line(), 2);
    /// ```
    pub fn line(&self) -> usize {
        self.line
    }

    /// The 1-based column number.
    ///
    /// Returns `0` only for a [`Location::default()`] that has not
    /// been populated by a parser pass; any [`Location`] produced
    /// by [`Location::from_index`], [`Location::new`], or returned
    /// from a parser is `≥ 1`.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::Location;
    /// let loc = Location::from_index("abcd", 3);
    /// assert_eq!(loc.column(), 4);
    /// ```
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
///
/// # Examples
///
/// ```
/// use noyalib::{from_str, Error, Value};
/// let err = from_str::<Value>("a: [unclosed").unwrap_err();
/// assert!(matches!(err, Error::Parse(_) | Error::ParseWithLocation { .. }));
/// ```
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// Error during YAML parsing.
    ///
    /// # Examples
    ///
    /// ```
    /// let _e = noyalib::Error::Parse("unexpected token".into());
    /// ```
    Parse(String),

    /// Error during YAML parsing with location information.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{Error, Location};
    /// let _e = Error::ParseWithLocation {
    ///     message: "bad token".into(),
    ///     location: Location::from_index("a: [", 3),
    /// };
    /// ```
    ParseWithLocation {
        /// The error message.
        ///
        /// # Examples
        ///
        /// ```
        /// use noyalib::{Error, Location};
        /// let e = Error::ParseWithLocation {
        ///     message: "bad".into(),
        ///     location: Location::default(),
        /// };
        /// if let Error::ParseWithLocation { message, .. } = e {
        ///     assert_eq!(message, "bad");
        /// }
        /// ```
        message: String,
        /// The location in the source where the error occurred.
        ///
        /// # Examples
        ///
        /// ```
        /// use noyalib::{Error, Location};
        /// let e = Error::ParseWithLocation {
        ///     message: "x".into(),
        ///     location: Location::from_index("abc", 1),
        /// };
        /// if let Error::ParseWithLocation { location, .. } = e {
        ///     assert_eq!(location.column(), 2);
        /// }
        /// ```
        location: Location,
    },

    /// Error during serialization.
    ///
    /// # Examples
    ///
    /// ```
    /// let _e = noyalib::Error::Serialize("bad value".into());
    /// ```
    Serialize(String),

    /// Error during deserialization.
    ///
    /// # Examples
    ///
    /// ```
    /// let _e = noyalib::Error::Deserialize("type mismatch".into());
    /// ```
    Deserialize(String),

    /// Error during deserialization with location information.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{Error, Location};
    /// let _e = Error::DeserializeWithLocation {
    ///     message: "expected int".into(),
    ///     location: Location::default(),
    /// };
    /// ```
    DeserializeWithLocation {
        /// The error message.
        ///
        /// # Examples
        ///
        /// ```
        /// use noyalib::{Error, Location};
        /// let e = Error::DeserializeWithLocation {
        ///     message: "m".into(),
        ///     location: Location::default(),
        /// };
        /// if let Error::DeserializeWithLocation { message, .. } = e {
        ///     assert_eq!(message, "m");
        /// }
        /// ```
        message: String,
        /// The location in the source where the error occurred.
        ///
        /// # Examples
        ///
        /// ```
        /// use noyalib::{Error, Location};
        /// let e = Error::DeserializeWithLocation {
        ///     message: "m".into(),
        ///     location: Location::from_index("ab", 1),
        /// };
        /// if let Error::DeserializeWithLocation { location, .. } = e {
        ///     assert_eq!(location.column(), 2);
        /// }
        /// ```
        location: Location,
    },

    /// I/O error (requires std feature).
    ///
    /// # Examples
    ///
    /// ```
    /// let ioe = std::io::Error::new(std::io::ErrorKind::Other, "nope");
    /// let _e = noyalib::Error::Io(ioe);
    /// ```
    #[cfg(feature = "std")]
    Io(std::io::Error),

    /// Custom error message.
    ///
    /// # Examples
    ///
    /// ```
    /// let _e = noyalib::Error::Custom("whatever".into());
    /// ```
    Custom(String),

    /// Error when recursion depth limit is exceeded.
    ///
    /// # Examples
    ///
    /// ```
    /// let _e = noyalib::Error::RecursionLimitExceeded { depth: 64 };
    /// ```
    RecursionLimitExceeded {
        /// The current depth.
        ///
        /// # Examples
        ///
        /// ```
        /// use noyalib::Error;
        /// if let Error::RecursionLimitExceeded { depth } =
        ///     (Error::RecursionLimitExceeded { depth: 10 })
        /// {
        ///     assert_eq!(depth, 10);
        /// }
        /// ```
        depth: usize,
    },

    /// Error when a duplicate key is encountered.
    ///
    /// # Examples
    ///
    /// ```
    /// let _e = noyalib::Error::DuplicateKey("name".into());
    /// ```
    DuplicateKey(String),

    /// Repetition limit exceeded (security limit against billion-laughs).
    ///
    /// # Examples
    ///
    /// ```
    /// let _e = noyalib::Error::RepetitionLimitExceeded;
    /// ```
    RepetitionLimitExceeded,

    /// Unknown anchor encountered.
    ///
    /// # Examples
    ///
    /// ```
    /// let _e = noyalib::Error::UnknownAnchor("missing".into());
    /// ```
    UnknownAnchor(String),

    /// Unknown anchor encountered at a specific location.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{Error, Location};
    /// let _e = Error::UnknownAnchorAt {
    ///     name: "x".into(),
    ///     location: Location::default(),
    ///     suggestion: None,
    /// };
    /// ```
    UnknownAnchorAt {
        /// The anchor name.
        ///
        /// # Examples
        ///
        /// ```
        /// use noyalib::{Error, Location};
        /// let e = Error::UnknownAnchorAt {
        ///     name: "x".into(),
        ///     location: Location::default(),
        ///     suggestion: None,
        /// };
        /// if let Error::UnknownAnchorAt { name, .. } = e {
        ///     assert_eq!(name, "x");
        /// }
        /// ```
        name: String,
        /// The location where it was used.
        ///
        /// # Examples
        ///
        /// ```
        /// use noyalib::{Error, Location};
        /// let e = Error::UnknownAnchorAt {
        ///     name: "x".into(),
        ///     location: Location::from_index("ab", 1),
        ///     suggestion: None,
        /// };
        /// if let Error::UnknownAnchorAt { location, .. } = e {
        ///     assert_eq!(location.column(), 2);
        /// }
        /// ```
        location: Location,
        /// Optional suggestion for a similar anchor.
        ///
        /// # Examples
        ///
        /// ```
        /// use noyalib::{Error, Location};
        /// let e = Error::UnknownAnchorAt {
        ///     name: "x".into(),
        ///     location: Location::default(),
        ///     suggestion: Some(("y".into(), Location::default())),
        /// };
        /// if let Error::UnknownAnchorAt { suggestion: Some((s, _)), .. } = e {
        ///     assert_eq!(s, "y");
        /// }
        /// ```
        suggestion: Option<(String, Location)>,
    },

    /// Missing field in a mapping.
    ///
    /// # Examples
    ///
    /// ```
    /// let _e = noyalib::Error::MissingField("name".into());
    /// ```
    MissingField(String),

    /// Unknown field in a mapping (with `deny_unknown_fields`).
    ///
    /// # Examples
    ///
    /// ```
    /// let _e = noyalib::Error::UnknownField("extra".into());
    /// ```
    UnknownField(String),

    /// Scalar encountered where a mapping was expected during merge.
    ///
    /// # Examples
    ///
    /// ```
    /// let _e = noyalib::Error::ScalarInMergeElement;
    /// ```
    ScalarInMergeElement,

    /// Sequence encountered where a mapping was expected during merge.
    ///
    /// # Examples
    ///
    /// ```
    /// let _e = noyalib::Error::SequenceInMergeElement;
    /// ```
    SequenceInMergeElement,

    /// Tagged value encountered during merge.
    ///
    /// # Examples
    ///
    /// ```
    /// let _e = noyalib::Error::TaggedInMerge;
    /// ```
    TaggedInMerge,

    /// Generic invalid construct error.
    ///
    /// # Examples
    ///
    /// ```
    /// let _e = noyalib::Error::Invalid("bad construct".into());
    /// ```
    Invalid(String),

    /// A type mismatch error.
    ///
    /// # Examples
    ///
    /// ```
    /// let _e = noyalib::Error::TypeMismatch {
    ///     expected: "integer",
    ///     found: "string".into(),
    /// };
    /// ```
    TypeMismatch {
        /// The expected type.
        ///
        /// # Examples
        ///
        /// ```
        /// use noyalib::Error;
        /// let e = Error::TypeMismatch { expected: "int", found: "str".into() };
        /// if let Error::TypeMismatch { expected, .. } = e {
        ///     assert_eq!(expected, "int");
        /// }
        /// ```
        expected: &'static str,
        /// The type that was actually found.
        ///
        /// # Examples
        ///
        /// ```
        /// use noyalib::Error;
        /// let e = Error::TypeMismatch { expected: "int", found: "str".into() };
        /// if let Error::TypeMismatch { found, .. } = e {
        ///     assert_eq!(found, "str");
        /// }
        /// ```
        found: String,
    },

    /// Shared error instance (Arc-wrapped for cloning).
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::Arc;
    /// let _e = noyalib::Error::Shared(Arc::new(noyalib::Error::EndOfStream));
    /// ```
    Shared(Arc<Error>),

    /// End of stream reached unexpectedly.
    ///
    /// # Examples
    ///
    /// ```
    /// let _e = noyalib::Error::EndOfStream;
    /// ```
    EndOfStream,

    /// More than one document found where one was expected.
    ///
    /// # Examples
    ///
    /// ```
    /// let _e = noyalib::Error::MoreThanOneDocument;
    /// ```
    MoreThanOneDocument,

    /// Scalar in merge (legacy variant).
    ///
    /// # Examples
    ///
    /// ```
    /// let _e = noyalib::Error::ScalarInMerge;
    /// ```
    ScalarInMerge,

    /// Empty tag encountered.
    ///
    /// # Examples
    ///
    /// ```
    /// let _e = noyalib::Error::EmptyTag;
    /// ```
    EmptyTag,

    /// Failed to parse a number.
    ///
    /// # Examples
    ///
    /// ```
    /// let _e = noyalib::Error::FailedToParseNumber("not-a-number".into());
    /// ```
    FailedToParseNumber(String),

    /// A message error from Serde (compat variant).
    ///
    /// # Examples
    ///
    /// ```
    /// let _e = noyalib::Error::Message("oops".into(), Some(42));
    /// ```
    Message(String, Option<usize>),
}

// ── Manual `Display` + `Error` impls ───────────────────────────────────
//
// noyalib does not depend on `thiserror` so the proc-macro
// expansion cost stays out of every downstream crate's compile.
// These impls reproduce the exact format strings the previous
// `#[error("...")]` attributes generated, so the user-visible
// `Display` output is byte-stable across the migration.

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Parse(msg) => write!(f, "YAML parse error: {msg}"),
            Error::ParseWithLocation { message, location } => {
                write!(f, "YAML parse error at {location}: {message}")
            }
            Error::Serialize(msg) => write!(f, "serialization error: {msg}"),
            Error::Deserialize(msg) => write!(f, "deserialization error: {msg}"),
            Error::DeserializeWithLocation { message, location } => {
                write!(f, "deserialization error at {location}: {message}")
            }
            #[cfg(feature = "std")]
            Error::Io(e) => write!(f, "I/O error: {e}"),
            Error::Custom(msg) => f.write_str(msg),
            Error::RecursionLimitExceeded { depth } => {
                write!(f, "recursion depth limit exceeded: {depth}")
            }
            Error::DuplicateKey(name) => write!(f, "duplicate key: {name}"),
            Error::RepetitionLimitExceeded => f.write_str("alias expansion limit exceeded"),
            Error::UnknownAnchor(name) => write!(f, "unknown anchor: {name}"),
            Error::UnknownAnchorAt { name, location, .. } => {
                write!(f, "unknown anchor: {name} at {location}")
            }
            Error::MissingField(name) => write!(f, "missing field: {name}"),
            Error::UnknownField(name) => write!(f, "unknown field: {name}"),
            Error::ScalarInMergeElement => f.write_str("scalar in merge element"),
            Error::SequenceInMergeElement => f.write_str("sequence in merge element"),
            Error::TaggedInMerge => f.write_str("tagged value in merge"),
            Error::Invalid(msg) => write!(f, "invalid YAML: {msg}"),
            Error::TypeMismatch { expected, found } => {
                write!(f, "type mismatch: expected {expected}, found {found}")
            }
            Error::Shared(arc) => fmt::Display::fmt(arc.as_ref(), f),
            Error::EndOfStream => f.write_str("unexpected end of stream"),
            Error::MoreThanOneDocument => {
                f.write_str("multiple documents in stream; expected exactly one")
            }
            Error::ScalarInMerge => f.write_str("scalar in merge"),
            Error::EmptyTag => f.write_str("empty tag"),
            Error::FailedToParseNumber(msg) => write!(f, "failed to parse number: {msg}"),
            Error::Message(msg, _) => write!(f, "serde error: {msg}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(e) => Some(e),
            Error::Shared(arc) => Some(arc.as_ref()),
            _ => None,
        }
    }
}

#[cfg(feature = "std")]
impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

impl Error {
    /// Get the location of the error, if any.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{from_str, Value};
    /// let err = from_str::<Value>("a: [unclosed").unwrap_err();
    /// let _ = err.location();
    /// ```
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
    ///
    /// For rustc-style multi-line context with surrounding lines, use
    /// [`Self::format_with_source_radius`].
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{from_str, Value};
    /// let source = "a: [unclosed";
    /// let err = from_str::<Value>(source).unwrap_err();
    /// let formatted = err.format_with_source(source);
    /// assert!(formatted.contains("error"));
    /// ```
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

    /// Format the error with `radius` lines of context above and
    /// below the offending line — rustc-style. Each line gets a line
    /// number on the left; the caret line under the offending column
    /// is unnumbered. The output is byte-for-byte stable across
    /// minor releases (no terminal escape codes, no
    /// platform-conditional whitespace).
    ///
    /// Out-of-range locations fall back to plain `Display` (no
    /// snippet) — same contract as [`Self::format_with_source`].
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{from_str, Value};
    /// // Indentation-mismatch error — carries a concrete `(line,
    /// // column)` location, so the snippet renderer engages.
    /// let source = "\
    /// header: ok
    /// service:
    ///    nested: x
    ///   bad: y
    /// trailer: ok
    /// ";
    /// let e = from_str::<Value>(source).unwrap_err();
    /// let formatted = e.format_with_source_radius(source, 1);
    /// // Output includes the offending line plus a single line
    /// // of context above and below.
    /// assert!(formatted.contains("|"));
    /// assert!(formatted.contains("bad: y"));
    /// ```
    pub fn format_with_source_radius(&self, source: &str, radius: usize) -> String {
        let loc = match self.location() {
            Some(l) => l,
            None => return format!("{self}"),
        };
        let line_no = loc.line();
        let col = loc.column();
        let line_idx = line_no.saturating_sub(1);

        let lines: Vec<&str> = source.lines().collect();
        if lines.is_empty() {
            return format!("{self}");
        }
        let target = match lines.get(line_idx) {
            Some(_) => line_idx,
            None if line_no == 0 => 0,
            None => return format!("{self}"),
        };

        let lo = target.saturating_sub(radius);
        let hi = (target + radius).min(lines.len().saturating_sub(1));
        // Width of the highest line number we'll print, for column
        // alignment of the gutter.
        let gutter_w = (hi + 1).to_string().len();
        let caret_col = col.saturating_sub(1);

        let mut out = format!("error: {self}\n");
        out.push_str(&format!(
            "  --> line {line_no}:{col}\n",
            line_no = line_no,
            col = col,
        ));

        // Top spacer
        out.push_str(&format!("{:>w$} |\n", "", w = gutter_w));
        for (i, idx) in (lo..=hi).enumerate() {
            let n = idx + 1;
            let line_text = lines[idx];
            out.push_str(&format!(
                "{n:>w$} | {line_text}\n",
                n = n,
                w = gutter_w,
                line_text = line_text,
            ));
            if idx == target {
                // Caret line — gutter is blank, then `|`, then
                // spaces up to the column, then `^`.
                let pad = " ".repeat(caret_col);
                out.push_str(&format!("{:>w$} | {pad}^\n", "", w = gutter_w, pad = pad,));
            }
            let _ = i;
        }
        // Bottom spacer
        out.push_str(&format!("{:>w$} |\n", "", w = gutter_w));
        out
    }

    /// Format the error with source context, capped at `max_chars`
    /// **ASCII characters** — the bridged-channel-friendly variant
    /// of [`Self::format_with_source`]. Use when the diagnostic is
    /// destined for a Slack message, a Sentry tag, a structured
    /// log field, or any sink with a hard length budget.
    ///
    /// # Truncation contract
    ///
    /// 1. The output is plain ASCII (the renderer already emits no
    ///    ANSI escapes, so this is a no-op for that axis).
    /// 2. If the rendered string is `<= max_chars`, returns it
    ///    unchanged.
    /// 3. Otherwise truncates at a UTF-8 character boundary
    ///    `<= max_chars - 3` and appends an `...` ellipsis so the
    ///    final length is at most `max_chars`.
    /// 4. `max_chars` smaller than 3 keeps as much of the prefix
    ///    as fits and drops the ellipsis (so a `max_chars = 2`
    ///    yields exactly two characters of the message).
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{from_str, Value};
    /// let source = "a: [unclosed";
    /// let err = from_str::<Value>(source).unwrap_err();
    /// let short = err.format_with_source_truncated(source, 60);
    /// assert!(short.len() <= 60);
    /// // Untrimmed output is the same as `format_with_source`:
    /// let full = err.format_with_source(source);
    /// let unbounded = err.format_with_source_truncated(source, full.len() + 100);
    /// assert_eq!(unbounded, full);
    /// ```
    #[must_use]
    pub fn format_with_source_truncated(&self, source: &str, max_chars: usize) -> String {
        let full = self.format_with_source(source);
        truncate_with_ellipsis(full, max_chars)
    }

    /// Format the error with multi-line `radius` context, capped
    /// at `max_chars`. Same truncation contract as
    /// [`Self::format_with_source_truncated`].
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{from_str, Value};
    /// let source = "a:\n  b:\n    c: [unclosed";
    /// let err = from_str::<Value>(source).unwrap_err();
    /// let s = err.format_with_source_radius_truncated(source, 1, 80);
    /// assert!(s.len() <= 80);
    /// ```
    #[must_use]
    pub fn format_with_source_radius_truncated(
        &self,
        source: &str,
        radius: usize,
        max_chars: usize,
    ) -> String {
        let full = self.format_with_source_radius(source, radius);
        truncate_with_ellipsis(full, max_chars)
    }

    /// Convert the error into a shared Arc pointer. If the error is
    /// already `Error::Shared`, the inner `Arc` is reused without
    /// double-wrapping.
    ///
    /// # Examples
    ///
    /// ```
    /// let shared = noyalib::Error::EndOfStream.into_shared();
    /// assert!(matches!(&*shared, noyalib::Error::EndOfStream));
    /// ```
    pub fn into_shared(self) -> Arc<Self> {
        match self {
            Error::Shared(arc) => arc,
            other => Arc::new(other),
        }
    }

    /// Check if the error is a shared error.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::Arc;
    /// let e = noyalib::Error::Shared(Arc::new(noyalib::Error::EndOfStream));
    /// assert!(e.is_shared());
    /// ```
    pub fn is_shared(&self) -> bool {
        matches!(self, Error::Shared(_))
    }

    /// Access the inner error if this is a shared error.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::Arc;
    /// let e = noyalib::Error::Shared(Arc::new(noyalib::Error::EndOfStream));
    /// assert!(e.as_inner().is_some());
    /// ```
    pub fn as_inner(&self) -> Option<&Self> {
        match self {
            Error::Shared(arc) => Some(&**arc),
            _ => None,
        }
    }

    /// Create a new parse error at the given index.
    ///
    /// # Examples
    ///
    /// ```
    /// let e = noyalib::Error::parse_at("bad", "a: x", 3);
    /// assert!(matches!(e, noyalib::Error::ParseWithLocation { .. }));
    /// ```
    pub fn parse_at(message: impl Into<String>, source: &str, index: usize) -> Self {
        Error::ParseWithLocation {
            message: message.into(),
            location: Location::from_index(source, index),
        }
    }

    /// Create a new deserialization error at the given index.
    ///
    /// # Examples
    ///
    /// ```
    /// let e = noyalib::Error::deserialize_at("bad", "a: x", 3);
    /// assert!(matches!(e, noyalib::Error::DeserializeWithLocation { .. }));
    /// ```
    pub fn deserialize_at(message: impl Into<String>, source: &str, index: usize) -> Self {
        Error::DeserializeWithLocation {
            message: message.into(),
            location: Location::from_index(source, index),
        }
    }

    /// Create a new error from a shared error pointer.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::Arc;
    /// let e = noyalib::Error::from_shared(Arc::new(noyalib::Error::EndOfStream));
    /// assert!(e.is_shared());
    /// ```
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

/// Truncate `s` to at most `max_chars` characters, replacing the
/// dropped suffix with an ASCII `...` ellipsis. Used by the
/// `*_truncated` formatters to fit error reports into bounded
/// log / message-bus channels.
///
/// Truncation always lands on a UTF-8 character boundary so the
/// output is a valid `String`. When `max_chars < 3` the ellipsis
/// is dropped and the function returns whatever prefix fits.
fn truncate_with_ellipsis(s: String, max_chars: usize) -> String {
    let len = s.chars().count();
    if len <= max_chars {
        return s;
    }
    if max_chars < 3 {
        // No room for `...` — return the longest character-aligned
        // prefix that fits.
        let end = s
            .char_indices()
            .nth(max_chars)
            .map(|(i, _)| i)
            .unwrap_or(s.len());
        return s[..end].to_owned();
    }
    let keep_chars = max_chars - 3;
    let end = s
        .char_indices()
        .nth(keep_chars)
        .map(|(i, _)| i)
        .unwrap_or(s.len());
    let mut out = String::with_capacity(end + 3);
    out.push_str(&s[..end]);
    out.push_str("...");
    out
}

/// A result type where the error is [`Error`].
///
/// # Examples
///
/// ```
/// use noyalib::Result;
/// fn parse() -> Result<noyalib::Value> {
///     noyalib::from_str("k: 1")
/// }
/// assert!(parse().is_ok());
/// ```
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

/// Panic helper for invariants the type system cannot express but
/// which the implementation has proved hold. Intended to replace
/// inline `unreachable!()` arms so the panic site is a single
/// `coverage(off)`-annotated function rather than an
/// arm-by-arm region in the coverage report.
///
/// `msg` is the human-readable invariant statement — quoted
/// verbatim into the panic message when the impossible happens
/// (e.g. when a future refactor accidentally breaks the
/// invariant). The tail-call shape lets call sites use it in any
/// position that expects a divergent expression.
#[track_caller]
#[cold]
#[inline(never)]
#[cfg_attr(noyalib_coverage, coverage(off))]
pub(crate) fn invariant_violated(msg: &'static str) -> ! {
    unreachable!("invariant violated: {msg}")
}

#[cfg(test)]
mod truncate_tests {
    use super::truncate_with_ellipsis;

    #[test]
    fn under_budget_passthrough() {
        assert_eq!(truncate_with_ellipsis("hello".into(), 10), "hello");
        assert_eq!(truncate_with_ellipsis("hello".into(), 5), "hello");
    }

    #[test]
    fn over_budget_truncates_with_ellipsis() {
        assert_eq!(truncate_with_ellipsis("hello world".into(), 8), "hello...");
        assert_eq!(truncate_with_ellipsis("0123456789".into(), 5), "01...");
    }

    #[test]
    fn tiny_budget_drops_ellipsis() {
        assert_eq!(truncate_with_ellipsis("hello".into(), 0), "");
        assert_eq!(truncate_with_ellipsis("hello".into(), 1), "h");
        assert_eq!(truncate_with_ellipsis("hello".into(), 2), "he");
    }

    #[test]
    fn utf8_aligned_at_char_boundary() {
        // Multi-byte chars — truncation must not split codepoints.
        let s = "café au lait — décaféiné".to_string();
        let t = truncate_with_ellipsis(s, 10);
        assert!(t.is_char_boundary(t.len()));
        assert_eq!(t.chars().count(), 10);
    }
}
