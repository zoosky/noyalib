//! Error types for noyalib.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use crate::prelude::*;

/// A specialized `Result` type for noyalib operations.
pub type Result<T> = core::result::Result<T, Error>;

/// Pick the closest known name to `target`, if any, within a small edit
/// distance. Used by alias-error diagnostics to suggest likely typo fixes
/// ("did you mean `&logger`?"). Returns `None` when no candidate is within
/// the threshold.
///
/// Threshold is length-aware: short names (≤ 3 chars) require distance 1;
/// longer names allow distance 2. This keeps suggestions high-confidence.
pub(crate) fn closest_name<'a, I>(target: &str, candidates: I) -> Option<&'a str>
where
    I: IntoIterator<Item = &'a str>,
{
    let max_dist = if target.len() <= 3 { 1 } else { 2 };
    let mut best: Option<(usize, &'a str)> = None;
    for cand in candidates {
        // Cheap length prefilter: anything differing in length by more than
        // max_dist can't be within edit distance max_dist.
        let lc = cand.len();
        let lt = target.len();
        let diff = lc.abs_diff(lt);
        if diff > max_dist {
            continue;
        }
        let d = levenshtein(target, cand);
        if d <= max_dist && best.map_or(true, |(bd, _)| d < bd) {
            best = Some((d, cand));
        }
    }
    best.map(|(_, name)| name)
}

/// Classic Levenshtein distance (insertions, deletions, substitutions count
/// as one edit each). O(m·n) time, O(min(m,n)) space.
fn levenshtein(a: &str, b: &str) -> usize {
    let (a, b) = if a.len() < b.len() { (b, a) } else { (a, b) };
    let a_bytes = a.as_bytes();
    let b_bytes = b.as_bytes();
    if b_bytes.is_empty() {
        return a_bytes.len();
    }
    let mut prev: Vec<usize> = (0..=b_bytes.len()).collect();
    let mut curr: Vec<usize> = vec![0; b_bytes.len() + 1];
    for (i, &ac) in a_bytes.iter().enumerate() {
        curr[0] = i + 1;
        for (j, &bc) in b_bytes.iter().enumerate() {
            let cost = usize::from(ac != bc);
            curr[j + 1] = (curr[j] + 1) // insertion
                .min(prev[j + 1] + 1) // deletion
                .min(prev[j] + cost); // substitution
        }
        core::mem::swap(&mut prev, &mut curr);
    }
    prev[b_bytes.len()]
}

/// A location within a YAML document.
///
/// This is used to report where in the source document an error occurred.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct Location {
    /// The line number (1-indexed).
    line: usize,
    /// The column number (1-indexed).
    column: usize,
    /// The byte index in the source.
    index: usize,
}

impl Location {
    /// Creates a new location.
    #[must_use]
    pub fn new(line: usize, column: usize, index: usize) -> Self {
        Self {
            line,
            column,
            index,
        }
    }

    /// Returns the line number (1-indexed).
    #[must_use]
    pub fn line(&self) -> usize {
        self.line
    }

    /// Returns the column number (1-indexed).
    #[must_use]
    pub fn column(&self) -> usize {
        self.column
    }

    /// Returns the byte index in the source.
    #[must_use]
    pub fn index(&self) -> usize {
        self.index
    }

    /// Create a location from a byte index in a source string.
    ///
    /// Computes the line and column numbers from the byte index.
    #[must_use]
    pub fn from_index(source: &str, index: usize) -> Self {
        let mut line = 1;
        let mut column = 1;
        let mut current_index = 0;

        for c in source.chars() {
            if current_index >= index {
                break;
            }
            if c == '\n' {
                line += 1;
                column = 1;
            } else {
                column += 1;
            }
            current_index += c.len_utf8();
        }

        Self {
            line,
            column,
            index,
        }
    }
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}, column {}", self.line, self.column)
    }
}

/// Errors that can occur during YAML serialization or deserialization.
///
/// This enum is marked `#[non_exhaustive]` to allow adding new variants
/// in future versions without breaking existing code.
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

    /// Invalid YAML structure.
    #[error("invalid YAML: {0}")]
    Invalid(String),

    /// Type mismatch during deserialization.
    #[error("type mismatch: expected {expected}, found {found}")]
    TypeMismatch {
        /// Expected type.
        expected: &'static str,
        /// Found type.
        found: String,
    },

    /// Missing field during deserialization.
    #[error("missing field: {0}")]
    MissingField(String),

    /// Unknown field during deserialization.
    #[error("unknown field: {0}")]
    UnknownField(String),

    /// Recursion limit exceeded during parsing.
    #[error("recursion limit exceeded at depth {depth}")]
    RecursionLimitExceeded {
        /// The depth at which the limit was exceeded.
        depth: usize,
    },

    /// Repetition limit exceeded during parsing.
    ///
    /// This can occur when the same anchor is referenced too many times,
    /// potentially indicating a denial-of-service attempt.
    #[error("repetition limit exceeded")]
    RepetitionLimitExceeded,

    /// Unknown anchor referenced in YAML.
    ///
    /// This occurs when an alias (`*anchor_name`) references an anchor
    /// that was not defined earlier in the document.
    #[error("unknown anchor: {0}")]
    UnknownAnchor(String),

    /// Unknown anchor with source location and optional typo suggestion.
    ///
    /// Richer counterpart to [`Error::UnknownAnchor`]: carries the byte
    /// location of the failing alias and, when an anchor with a similar name
    /// was defined in the same document, that anchor's name and location.
    /// Used by the miette [`Diagnostic`] impl to render dual-label "did you
    /// mean …?" output.
    ///
    /// [`Diagnostic`]: miette::Diagnostic
    #[error("unknown anchor: {name}")]
    UnknownAnchorAt {
        /// The anchor name the alias tried to reference.
        name: String,
        /// Source location of the alias site.
        location: Location,
        /// An anchor with a similar name that *was* defined, if any, and
        /// where. When present, the diagnostic renders a second label at
        /// this location with label text "defined here".
        suggestion: Option<(String, Location)>,
    },

    /// Scalar value found where a mapping was expected in a merge operation.
    ///
    /// The YAML merge key (`<<`) requires its value to be a mapping or
    /// a sequence of mappings, not a scalar value.
    #[error("expected a mapping or list of mappings for merging, but found scalar")]
    ScalarInMerge,

    /// Tagged value found in a merge operation.
    ///
    /// Tagged values are not supported in merge key operations.
    #[error("unexpected tagged value in merge")]
    TaggedInMerge,

    /// Scalar value found in a merge element.
    ///
    /// When using `<<: [*a, *b]` syntax, each element must be a mapping.
    #[error("expected a mapping for merging, but found scalar")]
    ScalarInMergeElement,

    /// Sequence value found in a merge element.
    ///
    /// When using `<<: [*a, *b]` syntax, each element must be a mapping,
    /// not a nested sequence.
    #[error("expected a mapping for merging, but found sequence")]
    SequenceInMergeElement,

    /// Empty YAML tag.
    ///
    /// YAML tags must have at least one character after the `!`.
    #[error("empty YAML tag is not allowed")]
    EmptyTag,

    /// Failed to parse a number from YAML.
    ///
    /// This occurs when a value that looks like a number cannot be
    /// parsed into the expected numeric type.
    #[error("failed to parse YAML number: {0}")]
    FailedToParseNumber(String),

    /// End of stream reached unexpectedly.
    ///
    /// This occurs when the YAML input ends while still expecting more content.
    #[error("unexpected end of YAML stream")]
    EndOfStream,

    /// More than one document found when only one was expected.
    ///
    /// Use [`load_all`](crate::load_all) or
    /// [`try_load_all`](crate::try_load_all) to handle multi-document YAML
    /// streams.
    #[error("expected a single YAML document, but found multiple documents")]
    MoreThanOneDocument,

    /// Duplicate key found during deserialization.
    ///
    /// This error is returned when
    /// [`DuplicateKeyPolicy::Error`](crate::DuplicateKeyPolicy::Error)
    /// is active and a mapping contains duplicate keys.
    #[error("duplicate key: {0}")]
    DuplicateKey(String),

    /// Custom error message from serde.
    #[error("{0}")]
    Custom(String),

    /// A shared error that can be cloned across thread boundaries.
    ///
    /// This variant wraps an error in an `Arc` to allow sharing the same
    /// error instance across multiple threads. Use [`Error::into_shared()`] to
    /// create a shared error.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::sync::Arc;
    ///
    /// use noyalib::Error;
    ///
    /// let error = Error::Parse("test error".to_string());
    /// let shared: Arc<Error> = error.into_shared();
    ///
    /// // The shared error can be cloned and sent to other threads
    /// let shared2 = Arc::clone(&shared);
    /// ```
    #[error("{0}")]
    Shared(Arc<Error>),
}

impl Error {
    /// Returns the location of the error, if available.
    ///
    /// For shared errors, this delegates to the inner error.
    #[must_use]
    pub fn location(&self) -> Option<Location> {
        match self {
            Error::ParseWithLocation { location, .. } => Some(*location),
            Error::DeserializeWithLocation { location, .. } => Some(*location),
            Error::UnknownAnchorAt { location, .. } => Some(*location),
            Error::Shared(arc) => arc.location(),
            _ => None,
        }
    }

    /// Format the error with source context.
    ///
    /// This produces a formatted error message that includes the relevant
    /// source line with a marker pointing to the error location.
    ///
    /// # Example
    ///
    /// ```rust
    /// use noyalib::{from_str, Value};
    ///
    /// let yaml = "name: test\nport: not_a_number\n";
    /// let result: Result<Value, _> = from_str(yaml);
    ///
    /// if let Err(e) = result {
    ///     let formatted = e.format_with_source(yaml);
    ///     // Output includes the source line and error marker
    /// }
    /// ```
    #[must_use]
    pub fn format_with_source(&self, source: &str) -> String {
        let location = match self.location() {
            Some(loc) => loc,
            None => return self.to_string(),
        };

        let lines: Vec<&str> = source.lines().collect();
        let line_idx = location.line.saturating_sub(1);

        if line_idx >= lines.len() {
            return self.to_string();
        }

        let error_line = lines[line_idx];
        let line_num_width = (location.line).to_string().len().max(3);
        let column = location.column.saturating_sub(1);

        use core::fmt::Write;
        let mut output = String::new();

        // Write directly into the buffer to avoid intermediate String allocations.
        let _ = writeln!(output, "error: {self}");
        let _ = writeln!(
            output,
            "{:>width$}--> line {}:{}",
            "",
            location.line,
            location.column,
            width = line_num_width
        );
        let _ = writeln!(output, "{:>width$} |", "", width = line_num_width);
        let _ = writeln!(
            output,
            "{:>width$} | {}",
            location.line,
            error_line,
            width = line_num_width
        );
        let _ = write!(
            output,
            "{:>width$} | {:>col$}^",
            "",
            "",
            width = line_num_width,
            col = column
        );

        output
    }

    /// Create a parse error with location from a source string and byte index.
    ///
    /// This helper computes line and column from the byte index.
    #[must_use]
    pub fn parse_at(message: impl Into<String>, source: &str, index: usize) -> Self {
        let location = Location::from_index(source, index);
        Error::ParseWithLocation {
            message: message.into(),
            location,
        }
    }

    /// Create a deserialization error with location from a source string and
    /// byte index.
    ///
    /// This helper computes line and column from the byte index.
    #[must_use]
    pub fn deserialize_at(message: impl Into<String>, source: &str, index: usize) -> Self {
        let location = Location::from_index(source, index);
        Error::DeserializeWithLocation {
            message: message.into(),
            location,
        }
    }

    /// Convert this error into a shared error wrapped in an `Arc`.
    ///
    /// This is useful when you need to share the same error across multiple
    /// threads or when the error needs to be cloned multiple times.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::sync::Arc;
    ///
    /// use noyalib::Error;
    ///
    /// let error = Error::Parse("test error".to_string());
    /// let shared: Arc<Error> = error.into_shared();
    ///
    /// // Clone the shared error for use in another thread
    /// let shared2 = Arc::clone(&shared);
    /// std::thread::spawn(move || {
    ///     println!("Error in thread: {}", shared2);
    /// });
    /// ```
    #[must_use]
    pub fn into_shared(self) -> Arc<Error> {
        match self {
            // If already shared, return the inner Arc
            Error::Shared(arc) => arc,
            // Otherwise, wrap in a new Arc
            other => Arc::new(other),
        }
    }

    /// Create an error from a shared error.
    ///
    /// This wraps an `Arc<Error>` in the `Shared` variant.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::sync::Arc;
    ///
    /// use noyalib::Error;
    ///
    /// let original = Error::Parse("test error".to_string());
    /// let shared = original.into_shared();
    ///
    /// // Create a new Error from the shared Arc
    /// let error = Error::from_shared(shared);
    /// ```
    #[must_use]
    pub fn from_shared(arc: Arc<Error>) -> Error {
        Error::Shared(arc)
    }

    /// Returns `true` if this is a shared error.
    ///
    /// # Example
    ///
    /// ```rust
    /// use noyalib::Error;
    ///
    /// let error = Error::Parse("test".to_string());
    /// assert!(!error.is_shared());
    ///
    /// let shared_error = Error::from_shared(error.into_shared());
    /// assert!(shared_error.is_shared());
    /// ```
    #[must_use]
    pub fn is_shared(&self) -> bool {
        matches!(self, Error::Shared(_))
    }

    /// If this is a shared error, returns a reference to the inner error.
    ///
    /// For non-shared errors, returns `None`.
    #[must_use]
    pub fn as_inner(&self) -> Option<&Error> {
        match self {
            Error::Shared(arc) => Some(arc.as_ref()),
            _ => None,
        }
    }
}

impl serde::de::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Error::Custom(msg.to_string())
    }
}

impl serde::ser::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Error::Custom(msg.to_string())
    }
}

// ── miette::Diagnostic integration ──────────────────────────────────────
//
// When the `miette` feature is enabled, noyalib errors participate in the
// standard Rust diagnostics ecosystem. CLI tools using `miette::Report` get
// rich terminal output with source spans for free.

#[cfg(all(feature = "miette", feature = "std"))]
impl miette::Diagnostic for Error {
    fn code<'a>(&'a self) -> Option<Box<dyn fmt::Display + 'a>> {
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
            Error::Shared(inner) => return inner.code(),
            _ => "noyalib::error",
        };
        Some(Box::new(code))
    }

    fn help<'a>(&'a self) -> Option<Box<dyn fmt::Display + 'a>> {
        let help = match self {
            Error::RecursionLimitExceeded { .. } => {
                Some("Increase ParserConfig::max_depth or simplify the document structure")
            }
            Error::RepetitionLimitExceeded => {
                Some("Increase ParserConfig::max_alias_expansions or reduce alias usage")
            }
            Error::DuplicateKey(_) => Some("Use DuplicateKeyPolicy::Last to accept duplicate keys"),
            Error::MoreThanOneDocument => {
                Some("Use noyalib::load_all() to parse multi-document streams")
            }
            Error::UnknownAnchor(_) => Some("Define the anchor (&name) before referencing it"),
            _ => None,
        };
        if let Some(s) = help {
            return Some(Box::new(s));
        }
        // Suggestion-bearing variant: render "did you mean '&name'?" when a
        // similar anchor was defined in the same document.
        if let Error::UnknownAnchorAt { suggestion, .. } = self {
            if let Some((name, _)) = suggestion {
                let msg = std::format!("did you mean '&{name}'?");
                return Some(Box::new(msg));
            }
            return Some(Box::new("Define the anchor (&name) before referencing it"));
        }
        None
    }

    fn labels(&self) -> Option<Box<dyn Iterator<Item = miette::LabeledSpan> + '_>> {
        // Dual-label path: alias site + anchor-definition site when a typo
        // suggestion is present.
        if let Error::UnknownAnchorAt {
            location,
            suggestion,
            name,
            ..
        } = self
        {
            let alias_label = miette::LabeledSpan::at_offset(
                location.index,
                std::format!("unknown anchor '{name}'"),
            );
            if let Some((sugg_name, sugg_loc)) = suggestion {
                let def_label = miette::LabeledSpan::at_offset(
                    sugg_loc.index,
                    std::format!("did you mean this anchor '&{sugg_name}'?"),
                );
                return Some(Box::new([alias_label, def_label].into_iter()));
            }
            return Some(Box::new(core::iter::once(alias_label)));
        }
        let loc = self.location()?;
        // Byte offset → single-character span at the error point.
        let label = miette::LabeledSpan::at_offset(loc.index, "here");
        Some(Box::new(core::iter::once(label)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_into_shared() {
        let error = Error::Parse("test error".to_string());
        let shared = error.into_shared();

        assert_eq!(shared.to_string(), "YAML parse error: test error");
    }

    #[test]
    fn test_error_from_shared() {
        let original = Error::Parse("test error".to_string());
        let shared = original.into_shared();
        let error = Error::from_shared(Arc::clone(&shared));

        assert!(error.is_shared());
        assert_eq!(error.to_string(), "YAML parse error: test error");
    }

    #[test]
    fn test_error_is_shared() {
        let error = Error::Parse("test".to_string());
        assert!(!error.is_shared());

        let shared_error = Error::from_shared(error.into_shared());
        assert!(shared_error.is_shared());
    }

    #[test]
    fn test_error_as_inner() {
        let error = Error::Parse("test".to_string());
        assert!(error.as_inner().is_none());

        let shared_error = Error::from_shared(error.into_shared());
        let inner = shared_error.as_inner().unwrap();
        assert_eq!(inner.to_string(), "YAML parse error: test");
    }

    #[test]
    fn test_shared_error_location() {
        let location = Location::new(10, 5, 100);
        let error = Error::ParseWithLocation {
            message: "test".to_string(),
            location,
        };

        let shared = Error::from_shared(error.into_shared());
        let loc = shared.location().unwrap();

        assert_eq!(loc.line(), 10);
        assert_eq!(loc.column(), 5);
        assert_eq!(loc.index(), 100);
    }

    #[test]
    fn test_double_into_shared() {
        // Converting a shared error to shared should not double-wrap
        let error = Error::Parse("test".to_string());
        let shared1 = error.into_shared();
        let shared_error = Error::from_shared(Arc::clone(&shared1));
        let shared2 = shared_error.into_shared();

        // The Arcs should point to the same underlying data
        assert!(Arc::ptr_eq(&shared1, &shared2));
    }

    #[test]
    fn test_new_error_types() {
        // Test that all new error types can be created and display correctly
        let errors: Vec<Error> = vec![
            Error::RepetitionLimitExceeded,
            Error::UnknownAnchor("test_anchor".to_string()),
            Error::ScalarInMerge,
            Error::TaggedInMerge,
            Error::ScalarInMergeElement,
            Error::SequenceInMergeElement,
            Error::EmptyTag,
            Error::FailedToParseNumber("abc".to_string()),
            Error::EndOfStream,
            Error::MoreThanOneDocument,
        ];

        for error in errors {
            // Ensure they all have non-empty error messages
            let msg = error.to_string();
            assert!(
                !msg.is_empty(),
                "Error message should not be empty: {:?}",
                error
            );
        }
    }

    #[test]
    fn test_location_from_index() {
        let source = "line1\nline2\nline3";

        // First character (l in line1)
        let loc0 = Location::from_index(source, 0);
        assert_eq!(loc0.line(), 1);
        assert_eq!(loc0.column(), 1);

        // Start of line2
        let loc6 = Location::from_index(source, 6);
        assert_eq!(loc6.line(), 2);
        assert_eq!(loc6.column(), 1);

        // Middle of line2
        let loc9 = Location::from_index(source, 9);
        assert_eq!(loc9.line(), 2);
        assert_eq!(loc9.column(), 4);
    }
}
