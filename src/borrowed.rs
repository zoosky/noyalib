// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Zero-copy YAML values that borrow strings from the input.
//!
//! [`BorrowedValue<'a>`] is the zero-copy counterpart of [`Value`](crate::Value).
//! String scalars and mapping keys use `Cow<'a, str>`, borrowing directly from
//! the input buffer when no escape processing was needed. This eliminates heap
//! allocations for the majority of YAML content.
//!
//! # Example
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
use crate::prelude::*;
use indexmap::IndexMap;
use rustc_hash::FxBuildHasher;

/// A zero-copy YAML value that borrows strings from the input.
#[derive(Debug, Clone, PartialEq)]
pub enum BorrowedValue<'a> {
    /// YAML null.
    Null,
    /// YAML boolean.
    Bool(bool),
    /// YAML number.
    Number(crate::value::Number),
    /// YAML string — borrows from input when possible.
    String(Cow<'a, str>),
    /// YAML sequence.
    Sequence(Vec<BorrowedValue<'a>>),
    /// YAML mapping with borrowed keys.
    Mapping(IndexMap<Cow<'a, str>, BorrowedValue<'a>, FxBuildHasher>),
}

impl<'a> BorrowedValue<'a> {
    /// Returns `true` if this is a null value.
    #[must_use]
    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    /// Returns the string value if this is a string.
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }

    /// Returns the i64 value if this is an integer.
    #[must_use]
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Number(n) => n.as_i64(),
            _ => None,
        }
    }

    /// Returns the bool value if this is a boolean.
    #[must_use]
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Returns the sequence if this is a sequence.
    #[must_use]
    pub fn as_sequence(&self) -> Option<&[BorrowedValue<'a>]> {
        match self {
            Self::Sequence(s) => Some(s),
            _ => None,
        }
    }

    /// Returns the mapping if this is a mapping.
    #[must_use]
    pub fn as_mapping(&self) -> Option<&IndexMap<Cow<'a, str>, BorrowedValue<'a>, FxBuildHasher>> {
        match self {
            Self::Mapping(m) => Some(m),
            _ => None,
        }
    }

    /// Convert to an owned `Value`, cloning all borrowed strings.
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

/// Parse YAML into a zero-copy `BorrowedValue` that borrows from the input.
///
/// This is significantly faster than `from_str::<Value>` for large documents
/// because string scalars and mapping keys borrow directly from the input
/// buffer instead of allocating on the heap.
///
/// # Example
///
/// ```rust
/// use noyalib::borrowed::{from_str_borrowed, BorrowedValue};
///
/// let yaml = "host: localhost\nport: 8080\n";
/// let value = from_str_borrowed(yaml).unwrap();
/// assert_eq!(value.as_mapping().unwrap().get("host").unwrap().as_str(), Some("localhost"));
/// ```
pub fn from_str_borrowed(input: &str) -> Result<BorrowedValue<'_>> {
    let config = ParseConfig::from(&crate::ParserConfig::default());
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
