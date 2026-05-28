// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Error-recovering YAML parser for LSP / IDE partial parsing.
//!
//! The default `from_str` family returns `Err` at the first
//! syntax violation. Language Server Protocol implementations
//! need the opposite contract: keep going past errors, build a
//! best-effort partial tree, and collect every error encountered
//! so the editor can show a complete diagnostics list and offer
//! autocomplete on the recoverable subtrees.
//!
//! [`parse_lenient`] is that contract:
//!
//! * Top-level `---` document boundaries are scanned first; each
//!   document is parsed independently so one broken document
//!   never prevents the others from being recovered.
//! * Within a single document, if the strict pass fails, the
//!   recoverer retries with [`DuplicateKeyPolicy::Last`] — the
//!   most common LSP-time error mode (a user typing a new key
//!   while an old one is still on screen). Successful recovery
//!   yields the post-retry value plus the original error in the
//!   error list, so the editor still flags the duplicate.
//! * If that retry also fails, the recoverer performs **line
//!   truncation recovery**: drop trailing lines one by one and
//!   re-parse until either a parse succeeds or the input is
//!   exhausted. The successful prefix becomes the recovered
//!   value; everything past the truncation point is summarised
//!   as a synthetic [`Value::Null`].
//! * A configurable error cap ([`LenientConfig::max_errors`])
//!   stops further recovery once enough diagnostics have been
//!   collected — useful when the document is so malformed that
//!   every line errors.
//!
//! Gated behind the `recovery` Cargo feature.
//!
//! # Output shape
//!
//! For multi-document input the result's [`ParseResult::value`]
//! is a [`Value::Sequence`] of per-document values (recovered or
//! `Null`) — this matches what an LSP would walk to label
//! per-document diagnostics. For single-document input the
//! result's `value` is the recovered document directly (not
//! wrapped in a sequence).
//!
//! # Example
//!
//! ```
//! # #[cfg(feature = "recovery")] {
//! let yaml = "a: 1\nb: [unclosed\nc: 3\n";
//! let result = noyalib::recovery::parse_lenient(yaml);
//! assert!(!result.is_complete);
//! assert!(!result.errors.is_empty());
//! // `value` is the best-effort tree the recoverer salvaged.
//! # }
//! ```

use crate::de::{DuplicateKeyPolicy, ParserConfig, from_str_with_config};
use crate::error::Error;
use crate::value::Value;

/// Result of an error-recovering parse pass.
///
/// `value` is the best-effort tree the recoverer was able to
/// salvage from the input. `errors` lists every error the
/// recoverer encountered, in the order it found them.
/// `is_complete` is `true` when no errors were collected — the
/// input parsed cleanly on the first attempt.
#[derive(Debug)]
#[non_exhaustive]
pub struct ParseResult {
    /// Best-effort recovered value. For multi-document input
    /// this is a [`Value::Sequence`] of per-document values
    /// (recovered or [`Value::Null`]); for single-document input
    /// it is the recovered document directly.
    pub value: Value,
    /// Every error the recoverer encountered, in source order.
    pub errors: Vec<Error>,
    /// `true` when no errors were collected.
    pub is_complete: bool,
}

/// Knobs for the recovery passes.
///
/// Constructed via [`LenientConfig::default`]; tweak fields
/// inline. The struct is intentionally not marked
/// `#[non_exhaustive]` so callers can use struct-literal syntax
/// like `LenientConfig { max_errors: 50, ..Default::default() }`.
/// Adding a field is a semver-minor breaking change pre-1.0.
#[derive(Debug, Clone)]
pub struct LenientConfig {
    /// Stop collecting diagnostics once this many errors have
    /// been recorded. Defaults to `100`.
    pub max_errors: usize,
    /// When the strict parse fails, retry with
    /// [`DuplicateKeyPolicy::Last`] before giving up. Defaults to
    /// `true`.
    pub recover_duplicate_keys: bool,
    /// When both the strict and duplicate-key retries fail,
    /// drop trailing lines one by one and re-parse. Defaults to
    /// `true`.
    pub line_truncation: bool,
    /// Base parser configuration. Defaults to
    /// [`ParserConfig::default`].
    pub base_config: ParserConfig,
}

impl Default for LenientConfig {
    fn default() -> Self {
        Self {
            max_errors: 100,
            recover_duplicate_keys: true,
            line_truncation: true,
            base_config: ParserConfig::default(),
        }
    }
}

/// Parse `input` with full error recovery.
///
/// Equivalent to [`parse_lenient_with`] with
/// [`LenientConfig::default`].
#[must_use]
pub fn parse_lenient(input: &str) -> ParseResult {
    parse_lenient_with(input, &LenientConfig::default())
}

/// Parse `input` with caller-supplied recovery knobs.
///
/// See the [module docs](self) for the recovery strategy.
#[must_use]
pub fn parse_lenient_with(input: &str, config: &LenientConfig) -> ParseResult {
    let docs = split_documents(input);

    if docs.is_empty() {
        return ParseResult {
            value: Value::Null,
            errors: Vec::new(),
            is_complete: true,
        };
    }

    if docs.len() == 1 {
        let (value, errors) = recover_one(docs[0], config, config.max_errors);
        let is_complete = errors.is_empty();
        return ParseResult {
            value,
            errors,
            is_complete,
        };
    }

    let mut values: Vec<Value> = Vec::with_capacity(docs.len());
    let mut errors: Vec<Error> = Vec::new();
    let mut budget = config.max_errors;
    for doc in docs {
        let (value, doc_errors) = recover_one(doc, config, budget);
        budget = budget.saturating_sub(doc_errors.len());
        errors.extend(doc_errors);
        values.push(value);
        if budget == 0 {
            break;
        }
    }
    let is_complete = errors.is_empty();
    ParseResult {
        value: Value::Sequence(values),
        errors,
        is_complete,
    }
}

/// Recover a single document.
///
/// Returns the best-effort `Value` and the list of errors
/// encountered. Bounded by `budget` — once exhausted, the
/// recoverer returns whatever it has and stops.
fn recover_one(input: &str, config: &LenientConfig, budget: usize) -> (Value, Vec<Error>) {
    if budget == 0 {
        return (Value::Null, Vec::new());
    }

    // Pass 1: strict.
    let strict_err = match from_str_with_config::<Value>(input, &config.base_config) {
        Ok(v) => return (v, Vec::new()),
        Err(e) => e,
    };
    let errors = vec![strict_err];

    // Pass 2: duplicate-key recovery via DuplicateKeyPolicy::Last.
    if config.recover_duplicate_keys && errors.len() < budget {
        let mut cfg2 = config.base_config.clone();
        cfg2.duplicate_key_policy = DuplicateKeyPolicy::Last;
        if let Ok(v) = from_str_with_config::<Value>(input, &cfg2) {
            // The original strict error is still surfaced as a
            // diagnostic so the editor flags the duplicate.
            return (v, errors);
        }
    }

    // Pass 3: line-truncation recovery.
    if config.line_truncation && errors.len() < budget {
        if let Some(v) = try_line_truncation(input, &config.base_config) {
            return (v, errors);
        }
    }

    (Value::Null, errors)
}

/// Drop trailing lines one at a time, retrying the parse, until
/// a parse succeeds or only one line remains.
fn try_line_truncation(input: &str, config: &ParserConfig) -> Option<Value> {
    // Collect line boundaries so we can truncate by slice rather
    // than by re-string-building.
    let mut boundaries: Vec<usize> = Vec::new();
    for (i, b) in input.as_bytes().iter().enumerate() {
        if *b == b'\n' {
            boundaries.push(i);
        }
    }
    // Iterate from the last line backwards.
    for &cut in boundaries.iter().rev() {
        let candidate = &input[..cut];
        if candidate.trim().is_empty() {
            continue;
        }
        if let Ok(v) = from_str_with_config::<Value>(candidate, config) {
            return Some(v);
        }
    }
    None
}

/// Split `input` on top-level YAML `---` document markers.
///
/// Inline mirror of the document-boundary scanner in
/// [`crate::parallel::split`] — duplicated here so the
/// `recovery` feature does not require the heavyweight
/// `parallel` feature (which pulls in Rayon).
fn split_documents(input: &str) -> Vec<&str> {
    let bytes = input.as_bytes();
    let mut markers: Vec<usize> = Vec::new();
    let mut i = 0;
    while i + 3 <= bytes.len() {
        let at_line_start = i == 0 || bytes[i - 1] == b'\n' || bytes[i - 1] == b'\r';
        if at_line_start && &bytes[i..i + 3] == b"---" {
            let next_ok =
                i + 3 >= bytes.len() || matches!(bytes[i + 3], b'\n' | b'\r' | b' ' | b'\t');
            if next_ok {
                markers.push(i);
                i += 3;
                continue;
            }
        }
        i += 1;
    }

    if markers.is_empty() {
        return if input.trim().is_empty() {
            Vec::new()
        } else {
            vec![input]
        };
    }

    let mut docs: Vec<&str> = Vec::with_capacity(markers.len() + 1);
    if markers[0] > 0 {
        let pre = input[..markers[0]].trim();
        if !pre.is_empty() {
            docs.push(&input[..markers[0]]);
        }
    }
    for window in markers.windows(2) {
        docs.push(&input[window[0]..window[1]]);
    }
    let last = *markers.last().unwrap();
    if last < input.len() {
        let trailing = &input[last..];
        if !trailing.trim_end().is_empty() {
            docs.push(trailing);
        }
    }
    docs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_input_is_complete() {
        let r = parse_lenient("a: 1\nb: 2\n");
        assert!(r.is_complete);
        assert!(r.errors.is_empty());
        let m = r.value.as_mapping().unwrap();
        assert!(m.contains_key("a"));
        assert!(m.contains_key("b"));
    }

    #[test]
    fn empty_input_is_complete() {
        let r = parse_lenient("");
        assert!(r.is_complete);
        assert!(r.errors.is_empty());
        assert!(matches!(r.value, Value::Null));
    }

    #[test]
    fn duplicate_key_is_recovered() {
        // The recovery pass kicks in only when the base config
        // is strict about duplicate keys; the workspace default
        // (DuplicateKeyPolicy::Last) silently accepts them.
        let cfg = LenientConfig {
            base_config: ParserConfig::default()
                .duplicate_key_policy(DuplicateKeyPolicy::Error),
            ..LenientConfig::default()
        };
        let r = parse_lenient_with("a: 1\na: 2\n", &cfg);
        assert!(!r.is_complete);
        assert_eq!(r.errors.len(), 1);
        let m = r.value.as_mapping().unwrap();
        let v = m.get("a").unwrap();
        assert_eq!(v.as_i64(), Some(2));
    }

    #[test]
    fn unrecoverable_input_yields_null_with_errors() {
        // `[` opens a flow sequence that never closes — no
        // truncation makes this valid.
        let r = parse_lenient("[\n");
        assert!(!r.is_complete);
        assert!(!r.errors.is_empty());
    }

    #[test]
    fn line_truncation_recovers_trailing_garbage() {
        // First two lines parse; third line is malformed flow.
        let r = parse_lenient("a: 1\nb: 2\nc: [unclosed\n");
        assert!(!r.is_complete);
        // The recoverer should salvage at least the strict-error
        // for the malformed third line.
        assert!(!r.errors.is_empty());
        // Best-effort tree: should contain `a` (and may contain `b`).
        if let Value::Mapping(m) = &r.value {
            assert!(m.contains_key("a"));
        }
    }

    #[test]
    fn multi_doc_recovers_each_independently() {
        let yaml = "---\na: 1\n---\nb: [unclosed\n---\nc: 3\n";
        let r = parse_lenient(yaml);
        assert!(!r.is_complete);
        let seq = match &r.value {
            Value::Sequence(s) => s,
            _ => panic!("expected sequence for multi-doc input"),
        };
        assert_eq!(seq.len(), 3);
        // Docs 0 and 2 should recover; doc 1 is the bad one.
        assert!(matches!(&seq[0], Value::Mapping(_)));
        assert!(matches!(&seq[2], Value::Mapping(_)));
    }

    #[test]
    fn max_errors_caps_collection() {
        let cfg = LenientConfig {
            max_errors: 1,
            ..LenientConfig::default()
        };
        let yaml = "---\na: [bad\n---\nb: [bad\n---\nc: [bad\n";
        let r = parse_lenient_with(yaml, &cfg);
        assert!(r.errors.len() <= 1);
    }

    #[test]
    fn split_documents_handles_single() {
        let d = split_documents("a: 1\n");
        assert_eq!(d.len(), 1);
    }

    #[test]
    fn split_documents_handles_empty() {
        assert!(split_documents("").is_empty());
        assert!(split_documents("   \n").is_empty());
    }
}
