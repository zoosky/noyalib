// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Zero-copy YAML values that borrow strings from the input.
//!
//! [`BorrowedValue<'a>`] is the zero-copy counterpart of [`Value`](crate::Value).
//! String scalars and mapping keys use `Cow<'a, str>`, borrowing directly from
//! the input buffer when no escape processing was needed. This eliminates heap
//! allocations for the majority of YAML content.
//!
//! # Examples
//!
//! ```rust
//! use noyalib::borrowed::{from_str_borrowed, BorrowedValue};
//!
//! let yaml = "name: noyalib\nversion: 1\n";
//! let value: BorrowedValue<'_> = from_str_borrowed(yaml).unwrap();
//! assert_eq!(value.as_mapping().unwrap().get("name").unwrap().as_str(), Some("noyalib"));
//! ```

use crate::error::{Error, Result};
use crate::parser::{Event, ParseConfig, Parser, ScalarStyle};
use crate::path::{parse_query_path, QuerySegment};
use crate::prelude::*;
use core::hash::{Hash, Hasher};
use indexmap::IndexMap;
use rustc_hash::FxBuildHasher;
use serde::Serialize;

/// A zero-copy YAML value that borrows strings from the input.
///
/// # Examples
///
/// ```
/// use noyalib::borrowed::{from_str_borrowed, BorrowedValue};
/// let v: BorrowedValue<'_> = from_str_borrowed("k: 1\n").unwrap();
/// assert!(v.as_mapping().is_some());
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BorrowedValue<'a> {
    /// YAML null.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::borrowed::BorrowedValue;
    /// assert!(BorrowedValue::Null.is_null());
    /// ```
    Null,
    /// YAML boolean.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::borrowed::BorrowedValue;
    /// let v = BorrowedValue::Bool(true);
    /// assert_eq!(v.as_bool(), Some(true));
    /// ```
    Bool(bool),
    /// YAML number.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{borrowed::BorrowedValue, Number};
    /// let v = BorrowedValue::Number(Number::Integer(42));
    /// assert_eq!(v.as_i64(), Some(42));
    /// ```
    Number(crate::value::Number),
    /// YAML string — borrows from input when possible.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::borrow::Cow;
    /// use noyalib::borrowed::BorrowedValue;
    /// let v = BorrowedValue::String(Cow::Borrowed("hi"));
    /// assert_eq!(v.as_str(), Some("hi"));
    /// ```
    String(Cow<'a, str>),
    /// YAML sequence.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::borrowed::BorrowedValue;
    /// let v = BorrowedValue::Sequence(vec![BorrowedValue::Null]);
    /// assert_eq!(v.as_sequence().unwrap().len(), 1);
    /// ```
    Sequence(Vec<BorrowedValue<'a>>),
    /// YAML mapping with borrowed keys.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::borrowed::{from_str_borrowed, BorrowedValue};
    /// let v = from_str_borrowed("k: 1\n").unwrap();
    /// assert!(matches!(v, BorrowedValue::Mapping(_)));
    /// ```
    Mapping(IndexMap<Cow<'a, str>, BorrowedValue<'a>, FxBuildHasher>),
}

impl<'a> BorrowedValue<'a> {
    /// Returns `true` if this is a null value.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::borrowed::BorrowedValue;
    /// assert!(BorrowedValue::Null.is_null());
    /// ```
    #[must_use]
    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    /// Returns the string value if this is a string.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::borrow::Cow;
    /// use noyalib::borrowed::BorrowedValue;
    /// let v = BorrowedValue::String(Cow::Borrowed("hi"));
    /// assert_eq!(v.as_str(), Some("hi"));
    /// ```
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }

    /// Returns the i64 value if this is an integer.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{borrowed::BorrowedValue, Number};
    /// let v = BorrowedValue::Number(Number::Integer(42));
    /// assert_eq!(v.as_i64(), Some(42));
    /// ```
    #[must_use]
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Number(n) => n.as_i64(),
            _ => None,
        }
    }

    /// Returns the bool value if this is a boolean.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::borrowed::BorrowedValue;
    /// assert_eq!(BorrowedValue::Bool(true).as_bool(), Some(true));
    /// ```
    #[must_use]
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Returns the sequence if this is a sequence.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::borrowed::BorrowedValue;
    /// let v = BorrowedValue::Sequence(vec![BorrowedValue::Null]);
    /// assert_eq!(v.as_sequence().unwrap().len(), 1);
    /// ```
    #[must_use]
    pub fn as_sequence(&self) -> Option<&[BorrowedValue<'a>]> {
        match self {
            Self::Sequence(s) => Some(s),
            _ => None,
        }
    }

    /// Returns the mapping if this is a mapping.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::borrowed::from_str_borrowed;
    /// let v = from_str_borrowed("k: 1\n").unwrap();
    /// assert!(v.as_mapping().is_some());
    /// ```
    #[must_use]
    pub fn as_mapping(&self) -> Option<&IndexMap<Cow<'a, str>, BorrowedValue<'a>, FxBuildHasher>> {
        match self {
            Self::Mapping(m) => Some(m),
            _ => None,
        }
    }

    /// Query nested values using an extended path expression.
    ///
    /// Returns all matching values. Supports dot notation, bracket indexing,
    /// wildcards (`*`), and recursive descent (`..`).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use noyalib::borrowed::from_str_borrowed;
    ///
    /// let yaml = "items:\n  - name: a\n  - name: b\n";
    /// let v = from_str_borrowed(yaml).unwrap();
    /// let names = v.query("items[*].name");
    /// assert_eq!(names.len(), 2);
    /// ```
    #[must_use]
    pub fn query(&self, path: &str) -> Vec<&BorrowedValue<'a>> {
        let segments = parse_query_path(path);
        let mut results = Vec::new();
        borrowed_query_recursive(self, &segments, 0, &mut results);
        results
    }

    /// Access a nested value via a dotted path.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::borrowed::from_str_borrowed;
    /// let v = from_str_borrowed("a:\n  b: 2\n").unwrap();
    /// assert_eq!(v.get_path("a.b").unwrap().as_i64(), Some(2));
    /// ```
    #[must_use]
    pub fn get_path(&self, path: &str) -> Option<&BorrowedValue<'a>> {
        let segments = parse_query_path(path);
        let mut current = self;
        for seg in &segments {
            current = match seg {
                QuerySegment::Key(key) => {
                    if let Self::Mapping(m) = current {
                        m.get(key.as_str())?
                    } else {
                        return None;
                    }
                }
                QuerySegment::Index(idx) => {
                    if let Self::Sequence(s) = current {
                        s.get(*idx)?
                    } else {
                        return None;
                    }
                }
                QuerySegment::Wildcard | QuerySegment::RecursiveDescent => {
                    return self.query(path).into_iter().next();
                }
            };
        }
        Some(current)
    }

    /// Convert to an owned `Value`, cloning all borrowed strings.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::borrowed::from_str_borrowed;
    /// let v = from_str_borrowed("k: 1\n").unwrap();
    /// let owned = v.into_owned();
    /// assert!(owned.as_mapping().is_some());
    /// ```
    #[must_use]
    pub fn into_owned(self) -> crate::Value {
        match self {
            Self::Null => crate::Value::Null,
            Self::Bool(b) => crate::Value::Bool(b),
            Self::Number(n) => crate::Value::Number(n),
            Self::String(s) => crate::Value::String(s.into_owned()),
            Self::Sequence(seq) => {
                crate::Value::Sequence(seq.into_iter().map(|v| v.into_owned()).collect())
            }
            Self::Mapping(map) => {
                let mut m = crate::Mapping::with_capacity(map.len());
                for (k, v) in map {
                    let _ = m.insert(k.into_owned(), v.into_owned());
                }
                crate::Value::Mapping(m)
            }
        }
    }
}

impl Hash for BorrowedValue<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
        match self {
            Self::Null => {}
            Self::Bool(b) => b.hash(state),
            Self::Number(n) => n.hash(state),
            Self::String(s) => s.hash(state),
            Self::Sequence(seq) => seq.hash(state),
            Self::Mapping(map) => {
                state.write_usize(map.len());
                for (k, v) in map {
                    k.hash(state);
                    v.hash(state);
                }
            }
        }
    }
}

impl Serialize for BorrowedValue<'_> {
    fn serialize<S>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::Null => serializer.serialize_none(),
            Self::Bool(b) => serializer.serialize_bool(*b),
            Self::Number(n) => match n {
                crate::value::Number::Integer(i) => serializer.serialize_i64(*i),
                crate::value::Number::Float(f) => serializer.serialize_f64(*f),
            },
            Self::String(s) => serializer.serialize_str(s),
            Self::Sequence(seq) => seq.serialize(serializer),
            Self::Mapping(map) => {
                use serde::ser::SerializeMap;
                let mut m = serializer.serialize_map(Some(map.len()))?;
                for (k, v) in map {
                    m.serialize_entry(k.as_ref(), v)?;
                }
                m.end()
            }
        }
    }
}

impl PartialOrd for BorrowedValue<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for BorrowedValue<'_> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        use core::cmp::Ordering;
        let rank = |v: &Self| -> u8 {
            match v {
                Self::Null => 0,
                Self::Bool(_) => 1,
                Self::Number(_) => 2,
                Self::String(_) => 3,
                Self::Sequence(_) => 4,
                Self::Mapping(_) => 5,
            }
        };
        let r = rank(self).cmp(&rank(other));
        if r != Ordering::Equal {
            return r;
        }
        match (self, other) {
            (Self::Null, Self::Null) => Ordering::Equal,
            (Self::Bool(a), Self::Bool(b)) => a.cmp(b),
            (Self::Number(a), Self::Number(b)) => a.cmp(b),
            (Self::String(a), Self::String(b)) => a.cmp(b),
            (Self::Sequence(a), Self::Sequence(b)) => a.cmp(b),
            (Self::Mapping(a), Self::Mapping(b)) => a.len().cmp(&b.len()),
            _ => Ordering::Equal,
        }
    }
}

/// Recursively query a `BorrowedValue` tree.
fn borrowed_query_recursive<'a, 'b>(
    value: &'b BorrowedValue<'a>,
    segments: &[QuerySegment],
    depth: usize,
    results: &mut Vec<&'b BorrowedValue<'a>>,
) {
    if depth >= segments.len() {
        results.push(value);
        return;
    }
    match &segments[depth] {
        QuerySegment::Key(key) => {
            if let BorrowedValue::Mapping(m) = value {
                if let Some(child) = m.get(key.as_str()) {
                    borrowed_query_recursive(child, segments, depth + 1, results);
                }
            }
        }
        QuerySegment::Index(idx) => {
            if let BorrowedValue::Sequence(s) = value {
                if let Some(child) = s.get(*idx) {
                    borrowed_query_recursive(child, segments, depth + 1, results);
                }
            }
        }
        QuerySegment::Wildcard => match value {
            BorrowedValue::Sequence(seq) => {
                for item in seq {
                    borrowed_query_recursive(item, segments, depth + 1, results);
                }
            }
            BorrowedValue::Mapping(map) => {
                for (_, v) in map {
                    borrowed_query_recursive(v, segments, depth + 1, results);
                }
            }
            _ => {}
        },
        QuerySegment::RecursiveDescent => {
            let remaining = &segments[depth + 1..];
            if !remaining.is_empty() {
                borrowed_query_recursive(value, segments, depth + 1, results);
                match value {
                    BorrowedValue::Sequence(seq) => {
                        for item in seq {
                            borrowed_query_recursive(item, segments, depth, results);
                        }
                    }
                    BorrowedValue::Mapping(map) => {
                        for (_, v) in map {
                            borrowed_query_recursive(v, segments, depth, results);
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

/// Parse YAML into a zero-copy `BorrowedValue` that borrows from the input.
///
/// This is significantly faster than `from_str::<Value>` for large documents
/// because string scalars and mapping keys borrow directly from the input
/// buffer instead of allocating on the heap.
///
/// # Examples
///
/// ```rust
/// use noyalib::borrowed::{from_str_borrowed, BorrowedValue};
///
/// let yaml = "host: localhost\nport: 8080\n";
/// let value = from_str_borrowed(yaml).unwrap();
/// assert_eq!(value.as_mapping().unwrap().get("host").unwrap().as_str(), Some("localhost"));
/// ```
pub fn from_str_borrowed(input: &str) -> Result<BorrowedValue<'_>> {
    from_str_borrowed_with_config(input, &crate::ParserConfig::default())
}

/// Parse YAML into a zero-copy `BorrowedValue` with custom security limits.
///
/// Same as [`from_str_borrowed`] but accepts a [`crate::ParserConfig`]
/// so callers can tighten `max_document_length`, `max_depth`, and other
/// limits for untrusted input.
///
/// # Errors
///
/// Returns an error when the input exceeds `max_document_length`, when
/// the parser encounters invalid YAML, or when an unsupported construct
/// (anchors/aliases on the borrowed path) is seen.
///
/// # Examples
///
/// ```
/// use noyalib::{borrowed::from_str_borrowed_with_config, ParserConfig};
/// let cfg = ParserConfig::strict();
/// let v = from_str_borrowed_with_config("k: 1\n", &cfg).unwrap();
/// assert!(v.as_mapping().is_some());
/// ```
pub fn from_str_borrowed_with_config<'a>(
    input: &'a str,
    user_config: &crate::ParserConfig,
) -> Result<BorrowedValue<'a>> {
    let config = ParseConfig::from(user_config);
    if input.len() > config.max_document_length {
        return Err(Error::Parse(format!(
            "document exceeds maximum length of {} bytes",
            config.max_document_length
        )));
    }

    let mut parser = Parser::new(input);
    let mut builder = BorrowedBuilder::new(&config);

    loop {
        let event = parser
            .next_event()
            .map_err(|e| Error::parse_at(&*e.message, input, e.index))?;
        match builder.process(event, input)? {
            BuilderState::Continue => {}
            BuilderState::Done => break,
        }
    }

    Ok(builder.into_value())
}

enum BuilderState {
    Continue,
    Done,
}

enum Frame<'a> {
    Sequence(Vec<BorrowedValue<'a>>),
    MappingKey(IndexMap<Cow<'a, str>, BorrowedValue<'a>, FxBuildHasher>),
    MappingValue(
        IndexMap<Cow<'a, str>, BorrowedValue<'a>, FxBuildHasher>,
        Cow<'a, str>,
    ),
}

struct BorrowedBuilder<'a> {
    stack: Vec<Frame<'a>>,
    result: Option<BorrowedValue<'a>>,
    max_depth: usize,
    depth: usize,
    in_document: bool,
}

impl<'a> BorrowedBuilder<'a> {
    fn new(config: &ParseConfig) -> Self {
        Self {
            stack: Vec::new(),
            result: None,
            max_depth: config.max_depth,
            depth: 0,
            in_document: false,
        }
    }

    fn into_value(self) -> BorrowedValue<'a> {
        self.result.unwrap_or(BorrowedValue::Null)
    }

    fn resolve_scalar(&self, value: Cow<'a, str>, style: ScalarStyle) -> BorrowedValue<'a> {
        if style != ScalarStyle::Plain {
            return BorrowedValue::String(value);
        }

        match &*value {
            "" | "~" | "null" | "Null" | "NULL" => BorrowedValue::Null,
            "true" | "True" | "TRUE" => BorrowedValue::Bool(true),
            "false" | "False" | "FALSE" => BorrowedValue::Bool(false),
            ".inf" | ".Inf" | ".INF" | "+.inf" | "+.Inf" | "+.INF" => {
                BorrowedValue::Number(crate::value::Number::Float(f64::INFINITY))
            }
            "-.inf" | "-.Inf" | "-.INF" => {
                BorrowedValue::Number(crate::value::Number::Float(f64::NEG_INFINITY))
            }
            ".nan" | ".NaN" | ".NAN" => {
                BorrowedValue::Number(crate::value::Number::Float(f64::NAN))
            }
            s => {
                let bytes = s.as_bytes();
                if !bytes.is_empty() {
                    let first = bytes[0];
                    if first.is_ascii_digit() || first == b'+' || first == b'-' || first == b'.' {
                        if let Ok(n) = s.parse::<i64>() {
                            return BorrowedValue::Number(crate::value::Number::Integer(n));
                        }
                        if let Ok(f) = s.parse::<f64>() {
                            return BorrowedValue::Number(crate::value::Number::Float(f));
                        }
                    }
                }
                BorrowedValue::String(value)
            }
        }
    }

    fn push_value(&mut self, value: BorrowedValue<'a>) {
        match self.stack.last_mut() {
            Some(Frame::Sequence(seq)) => seq.push(value),
            Some(Frame::MappingValue(map, key)) => {
                let k = core::mem::replace(key, Cow::Borrowed(""));
                let _ = map.insert(k, value);
                // Transition back to key state
                let map = match self.stack.pop() {
                    Some(Frame::MappingValue(m, _)) => m,
                    _ => unreachable!(),
                };
                self.stack.push(Frame::MappingKey(map));
            }
            Some(Frame::MappingKey(_)) => {
                // This shouldn't happen — keys should transition to MappingValue
            }
            None => {
                self.result = Some(value);
            }
        }
    }

    fn process(&mut self, event: Event<'a>, _input: &str) -> Result<BuilderState> {
        match event {
            Event::StreamStart => Ok(BuilderState::Continue),
            Event::StreamEnd => Ok(BuilderState::Done),
            Event::DocumentStart => {
                self.in_document = true;
                Ok(BuilderState::Continue)
            }
            Event::DocumentEnd => {
                self.in_document = false;
                Ok(BuilderState::Continue)
            }
            Event::Scalar { value, style, .. } => {
                // Check if this is a mapping key
                if let Some(Frame::MappingKey(_)) = self.stack.last_mut() {
                    let key = value;
                    let map = match self.stack.pop() {
                        Some(Frame::MappingKey(m)) => m,
                        _ => unreachable!(),
                    };
                    self.stack.push(Frame::MappingValue(map, key));
                    return Ok(BuilderState::Continue);
                }

                let resolved = self.resolve_scalar(value, style);
                self.push_value(resolved);
                Ok(BuilderState::Continue)
            }
            Event::SequenceStart { .. } => {
                self.depth += 1;
                if self.depth > self.max_depth {
                    return Err(Error::RecursionLimitExceeded { depth: self.depth });
                }
                self.stack.push(Frame::Sequence(Vec::with_capacity(4)));
                Ok(BuilderState::Continue)
            }
            Event::SequenceEnd { .. } => {
                self.depth = self.depth.saturating_sub(1);
                let seq = match self.stack.pop() {
                    Some(Frame::Sequence(s)) => s,
                    _ => return Err(Error::Invalid("unexpected sequence end".to_string())),
                };
                self.push_value(BorrowedValue::Sequence(seq));
                Ok(BuilderState::Continue)
            }
            Event::MappingStart { .. } => {
                self.depth += 1;
                if self.depth > self.max_depth {
                    return Err(Error::RecursionLimitExceeded { depth: self.depth });
                }
                self.stack
                    .push(Frame::MappingKey(IndexMap::with_capacity_and_hasher(
                        4,
                        FxBuildHasher,
                    )));
                Ok(BuilderState::Continue)
            }
            Event::MappingEnd { .. } => {
                self.depth = self.depth.saturating_sub(1);
                let map = match self.stack.pop() {
                    Some(Frame::MappingKey(m)) => m,
                    Some(Frame::MappingValue(m, _)) => m,
                    _ => return Err(Error::Invalid("unexpected mapping end".to_string())),
                };
                self.push_value(BorrowedValue::Mapping(map));
                Ok(BuilderState::Continue)
            }
            Event::Alias { .. } => {
                // Aliases not supported in borrowed mode — would require cloning
                Err(Error::Invalid(
                    "aliases not supported in borrowed mode; use from_str::<Value> instead"
                        .to_string(),
                ))
            }
        }
    }
}
