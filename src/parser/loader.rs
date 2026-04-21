//! Event-to-Value tree builder with security limits.
//!
//! Converts a stream of [`Event`]s directly into `Vec<(Value, SpanTree)>`.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use crate::prelude::*;
use indexmap::IndexMap;

use super::events::{Event, Parser};
use super::scanner::ScalarStyle;
use crate::de::{DuplicateKeyPolicy, ParserConfig};
use crate::error::{Error, Result};
#[cfg(feature = "std")]
use crate::span_context::SpanTree;
use crate::value::{Mapping, Number, Value};

/// The YAML merge key (`<<`).
const MERGE_KEY: &str = "<<";

/// Configuration for the internal parser, mirroring `ParserConfig`.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ParseConfig {
    pub max_depth: usize,
    pub max_document_length: usize,
    pub max_alias_expansions: usize,
    pub max_mapping_keys: usize,
    pub max_sequence_length: usize,
    pub duplicate_key_policy: DuplicateKeyPolicy,
    pub strict_booleans: bool,
    pub legacy_booleans: bool,
}

impl From<&ParserConfig> for ParseConfig {
    fn from(c: &ParserConfig) -> Self {
        ParseConfig {
            max_depth: c.max_depth,
            max_document_length: c.max_document_length,
            max_alias_expansions: c.max_alias_expansions,
            max_mapping_keys: c.max_mapping_keys,
            max_sequence_length: c.max_sequence_length,
            duplicate_key_policy: c.duplicate_key_policy,
            strict_booleans: c.strict_booleans,
            legacy_booleans: c.legacy_booleans,
        }
    }
}

#[cfg(feature = "std")]
/// Build a `Vec<(Value, SpanTree)>` from a YAML input string.
pub(crate) fn load(input: &str, config: &ParseConfig) -> Result<Vec<(Value, SpanTree)>> {
    // Check document length limit.
    if input.len() > config.max_document_length {
        return Err(Error::Parse(format!(
            "document exceeds maximum length of {} bytes",
            config.max_document_length
        )));
    }

    let mut parser = Parser::new(input);
    let mut loader = Loader::new(config);

    loop {
        let event = parser
            .next_event()
            .map_err(|e| Error::parse_at(&*e.message, input, e.index))?;

        match loader.process_event(event, input)? {
            LoaderState::Continue => {}
            LoaderState::Done => break,
        }
    }

    Ok(loader.into_docs())
}

#[cfg(feature = "std")]
/// Build a single `(Value, SpanTree)` from a YAML input string.
pub(crate) fn load_one(input: &str, config: &ParseConfig) -> Result<(Value, SpanTree)> {
    let docs = load(input, config)?;

    if docs.is_empty() {
        // Empty/comment-only documents resolve to Null per YAML spec.
        return Ok((Value::Null, SpanTree::Leaf(0, 0)));
    }

    Ok(docs
        .into_iter()
        .next()
        .expect("internal: docs verified non-empty"))
}

enum LoaderState {
    Continue,
    Done,
}

#[cfg(feature = "std")]
/// Stack frame for the tree builder.
#[derive(Debug)]
enum Frame {
    Sequence {
        items: Vec<Value>,
        span_items: Vec<SpanTree>,
        anchor: Option<String>,
        start_offset: usize,
    },
    MappingKey {
        map: Mapping,
        span_entries: Vec<((usize, usize), SpanTree)>,
        anchor: Option<String>,
        merge_values: Vec<Value>,
        start_offset: usize,
    },
    MappingValue {
        map: Mapping,
        span_entries: Vec<((usize, usize), SpanTree)>,
        key: String,
        key_span: (usize, usize),
        anchor: Option<String>,
        merge_values: Vec<Value>,
        start_offset: usize,
    },
}

#[cfg(feature = "std")]
/// YAML tree builder with security limits.
struct Loader<'a> {
    docs: Vec<(Value, SpanTree)>,
    stack: Vec<Frame>,
    anchor_map: IndexMap<String, Value>,
    anchor_span_map: IndexMap<String, SpanTree>,
    alias_count: usize,
    alias_bytes: usize,
    config: &'a ParseConfig,
    depth: usize,
    in_document: bool,
}

#[cfg(feature = "std")]
impl<'a> Loader<'a> {
    fn new(config: &'a ParseConfig) -> Self {
        Loader {
            docs: Vec::new(),
            stack: Vec::new(),
            anchor_map: IndexMap::new(),
            anchor_span_map: IndexMap::new(),
            alias_count: 0,
            alias_bytes: 0,
            config,
            depth: 0,
            in_document: false,
        }
    }

    fn into_docs(self) -> Vec<(Value, SpanTree)> {
        self.docs
    }

    fn process_event(&mut self, event: Event<'_>, input: &str) -> Result<LoaderState> {
        match event {
            Event::StreamStart => Ok(LoaderState::Continue),
            Event::StreamEnd => Ok(LoaderState::Done),
            Event::DocumentStart => {
                self.in_document = true;
                self.anchor_map.clear();
                self.anchor_span_map.clear();
                self.alias_count = 0;
                self.alias_bytes = 0;
                Ok(LoaderState::Continue)
            }
            Event::DocumentEnd => {
                self.in_document = false;
                Ok(LoaderState::Continue)
            }
            Event::Scalar {
                value,
                style,
                anchor,
                tag,
                span,
            } => {
                let resolved = if style == ScalarStyle::Plain {
                    resolve_plain_scalar(
                        value,
                        &tag,
                        self.config.strict_booleans,
                        self.config.legacy_booleans,
                    )
                } else {
                    resolve_quoted_scalar(value, &tag)
                };
                let resolved = match resolved {
                    Ok(v) => v,
                    Err(msg) => return Err(Error::parse_at(msg, input, span.start)),
                };

                let span_tree = SpanTree::Leaf(span.start, span.end);

                if let Some(name) = anchor {
                    let _ = self.anchor_map.insert(name.clone(), resolved.clone());
                    let _ = self.anchor_span_map.insert(name, span_tree.clone());
                }
                self.push_value(resolved, span_tree)?;
                Ok(LoaderState::Continue)
            }
            Event::Alias { anchor, span } => {
                self.alias_count += 1;
                if self.alias_count > self.config.max_alias_expansions {
                    return Err(Error::RepetitionLimitExceeded);
                }
                // Clone is intentional here — alias expansion must produce an
                // independent copy of the anchored value.
                let value = self.anchor_map.get(&anchor).cloned().ok_or_else(|| {
                    Error::parse_at(format!("unknown anchor '{anchor}'"), input, span.start)
                })?;
                // Track cumulative bytes cloned via aliases to prevent
                // billion-laughs attacks where a large value is aliased many times.
                self.alias_bytes = self.alias_bytes.saturating_add(estimate_value_size(&value));
                if self.alias_bytes > self.config.max_document_length {
                    return Err(Error::RepetitionLimitExceeded);
                }
                // Use the alias usage site's span, not the original anchor's span.
                let span_tree = SpanTree::Leaf(span.start, span.end);
                self.push_value(value, span_tree)?;
                Ok(LoaderState::Continue)
            }
            Event::SequenceStart { anchor, span, .. } => {
                self.depth += 1;
                if self.depth > self.config.max_depth {
                    return Err(Error::RecursionLimitExceeded { depth: self.depth });
                }
                self.stack.push(Frame::Sequence {
                    items: Vec::new(),
                    span_items: Vec::new(),
                    anchor,
                    start_offset: span.start,
                });
                Ok(LoaderState::Continue)
            }
            Event::SequenceEnd { span } => {
                self.depth = self.depth.saturating_sub(1);
                match self.stack.pop() {
                    Some(Frame::Sequence {
                        items,
                        span_items,
                        anchor,
                        start_offset,
                    }) => {
                        if items.len() > self.config.max_sequence_length {
                            return Err(Error::Parse(format!(
                                "sequence exceeds maximum length of {} elements",
                                self.config.max_sequence_length
                            )));
                        }
                        let value = Value::Sequence(items);
                        let span_tree = SpanTree::Sequence {
                            start: start_offset,
                            end: span.end,
                            items: span_items,
                        };
                        if let Some(name) = anchor {
                            let _ = self.anchor_map.insert(name.clone(), value.clone());
                            let _ = self.anchor_span_map.insert(name, span_tree.clone());
                        }
                        self.push_value(value, span_tree)?;
                    }
                    _ => return Err(Error::Parse("unexpected sequence end".to_string())),
                }
                Ok(LoaderState::Continue)
            }
            Event::MappingStart { anchor, span, .. } => {
                self.depth += 1;
                if self.depth > self.config.max_depth {
                    return Err(Error::RecursionLimitExceeded { depth: self.depth });
                }
                self.stack.push(Frame::MappingKey {
                    map: Mapping::new(),
                    span_entries: Vec::new(),
                    anchor,
                    merge_values: Vec::new(),
                    start_offset: span.start,
                });
                Ok(LoaderState::Continue)
            }
            Event::MappingEnd { span } => {
                self.depth = self.depth.saturating_sub(1);
                match self.stack.pop() {
                    Some(Frame::MappingKey {
                        mut map,
                        span_entries,
                        anchor,
                        merge_values,
                        start_offset,
                    }) => {
                        // Apply merge keys (first source takes precedence).
                        for merge_val in merge_values {
                            match merge_val {
                                Value::Mapping(merge_map) => {
                                    for (k, v) in merge_map {
                                        if !map.contains_key(&k) {
                                            let _ = map.insert(k, v);
                                        }
                                    }
                                }
                                Value::Sequence(seq) => {
                                    for item in seq {
                                        if let Value::Mapping(merge_map) = item {
                                            for (k, v) in merge_map {
                                                if !map.contains_key(&k) {
                                                    let _ = map.insert(k, v);
                                                }
                                            }
                                        } else {
                                            return Err(Error::ScalarInMergeElement);
                                        }
                                    }
                                }
                                _ => return Err(Error::ScalarInMergeElement),
                            }
                        }

                        if map.len() > self.config.max_mapping_keys {
                            return Err(Error::Parse(format!(
                                "mapping exceeds maximum of {} keys",
                                self.config.max_mapping_keys
                            )));
                        }
                        let value = Value::Mapping(map);
                        let span_tree = SpanTree::Mapping {
                            start: start_offset,
                            end: span.end,
                            entries: span_entries,
                        };
                        if let Some(name) = anchor {
                            let _ = self.anchor_map.insert(name.clone(), value.clone());
                            let _ = self.anchor_span_map.insert(name, span_tree.clone());
                        }
                        self.push_value(value, span_tree)?;
                    }
                    Some(Frame::MappingValue { .. }) => {
                        return Err(Error::Parse(
                            "unexpected mapping end while expecting value".to_string(),
                        ));
                    }
                    _ => return Err(Error::Parse("unexpected mapping end".to_string())),
                }
                Ok(LoaderState::Continue)
            }
        }
    }

    /// Push a fully-resolved value + span tree onto the current stack frame.
    fn push_value(&mut self, value: Value, span_tree: SpanTree) -> Result<()> {
        match self.stack.last_mut() {
            None => {
                // Top-level document value.
                self.docs.push((value, span_tree));
            }
            Some(Frame::Sequence {
                items, span_items, ..
            }) => {
                items.push(value);
                span_items.push(span_tree);
            }
            Some(Frame::MappingKey {
                map,
                span_entries,
                anchor,
                merge_values,
                start_offset,
            }) => {
                // This value is a mapping key.
                let key = value_into_key(value)?;
                let key_span = match &span_tree {
                    SpanTree::Leaf(s, e) => (*s, *e),
                    SpanTree::Sequence { start, end, .. } => (*start, *end),
                    SpanTree::Mapping { start, end, .. } => (*start, *end),
                };

                // Check for merge key.
                if key == MERGE_KEY {
                    let map = core::mem::replace(map, Mapping::new());
                    let span_entries = core::mem::take(span_entries);
                    let anchor = anchor.clone();
                    let merge_values = core::mem::take(merge_values);
                    let start_offset = *start_offset;
                    let frame = Frame::MappingValue {
                        map,
                        span_entries,
                        key,
                        key_span,
                        anchor,
                        merge_values,
                        start_offset,
                    };
                    // SAFETY: stack is non-empty during event processing — pushed at Document/Sequence/Mapping start.
                    *self
                        .stack
                        .last_mut()
                        .expect("internal: stack non-empty during event processing") = frame;
                    return Ok(());
                }

                // Duplicate key policy check.
                if self.config.duplicate_key_policy == DuplicateKeyPolicy::Error
                    && map.contains_key(&key)
                {
                    return Err(Error::DuplicateKey(key));
                }

                let map = core::mem::replace(map, Mapping::new());
                let span_entries = core::mem::take(span_entries);
                let anchor = anchor.clone();
                let merge_values = core::mem::take(merge_values);
                let start_offset = *start_offset;
                let frame = Frame::MappingValue {
                    map,
                    span_entries,
                    key,
                    key_span,
                    anchor,
                    merge_values,
                    start_offset,
                };
                // SAFETY: stack is non-empty during event processing — pushed at Document/Sequence/Mapping start.
                *self
                    .stack
                    .last_mut()
                    .expect("internal: stack non-empty during event processing") = frame;
            }
            Some(Frame::MappingValue {
                map,
                span_entries,
                key,
                key_span,
                anchor,
                merge_values,
                start_offset,
            }) => {
                // This value is a mapping value.
                if key == MERGE_KEY {
                    // Store merge value for later processing (span dropped for merges).
                    merge_values.push(value);
                } else {
                    match self.config.duplicate_key_policy {
                        DuplicateKeyPolicy::First => {
                            if !map.contains_key(key.as_str()) {
                                let _ = map.insert(key.clone(), value);
                                span_entries.push((*key_span, span_tree));
                            }
                        }
                        DuplicateKeyPolicy::Last | DuplicateKeyPolicy::Error => {
                            if let Some(old_idx) = map.keys().position(|k| k == key.as_str()) {
                                // Replace the span entry at the same position as
                                // the old key so spans stay aligned with map entries.
                                if old_idx < span_entries.len() {
                                    span_entries[old_idx] = (*key_span, span_tree);
                                }
                            } else {
                                span_entries.push((*key_span, span_tree));
                            }
                            let _ = map.insert(key.clone(), value);
                        }
                    }
                }

                // Transition back to MappingKey.
                let map = core::mem::replace(map, Mapping::new());
                let span_entries = core::mem::take(span_entries);
                let anchor = anchor.clone();
                let merge_values = core::mem::take(merge_values);
                let start_offset = *start_offset;
                let frame = Frame::MappingKey {
                    map,
                    span_entries,
                    anchor,
                    merge_values,
                    start_offset,
                };
                // SAFETY: stack is non-empty during event processing — pushed at Document/Sequence/Mapping start.
                *self
                    .stack
                    .last_mut()
                    .expect("internal: stack non-empty during event processing") = frame;
            }
        }
        Ok(())
    }
}

/// Convert a Value into a string key for mappings.
///
/// Takes ownership of the value to avoid cloning `Value::String`.
fn value_into_key(value: Value) -> Result<String> {
    match value {
        Value::String(s) => Ok(s),
        Value::Number(Number::Integer(n)) => Ok(itoa::Buffer::new().format(n).to_owned()),
        Value::Number(Number::Float(n)) => Ok(ryu::Buffer::new().format(n).to_owned()),
        Value::Bool(b) => Ok(if b { "true" } else { "false" }.to_owned()),
        Value::Null => Ok("null".to_owned()),
        _ => Err(Error::Invalid("non-scalar key in mapping".to_owned())),
    }
}

/// Resolve a plain (unquoted) scalar according to YAML 1.2 Core Schema.
///
/// Accepts `Cow<str>` so that borrowed scalars (from the scanner fast path)
/// avoid allocation when they resolve to a non-String type (null, bool, number).
pub(crate) fn resolve_plain_scalar(
    value: Cow<'_, str>,
    tag: &Option<(String, String)>,
    strict_booleans: bool,
    legacy_booleans: bool,
) -> core::result::Result<Value, String> {
    // If there's a tag, handle it.
    if let Some((handle, suffix)) = tag {
        return resolve_tagged_scalar(&value, handle, suffix);
    }

    // YAML 1.1 legacy booleans (yes/no/on/off/y/n).
    if legacy_booleans {
        match &*value {
            "yes" | "Yes" | "YES" | "on" | "On" | "ON" | "y" | "Y" => {
                return Ok(Value::Bool(true));
            }
            "no" | "No" | "NO" | "off" | "Off" | "OFF" | "n" | "N" => {
                return Ok(Value::Bool(false));
            }
            _ => {}
        }
    }

    // YAML 1.2 Core Schema resolution for plain scalars.
    match &*value {
        // Null values.
        "" | "~" | "null" | "Null" | "NULL" => Ok(Value::Null),

        // Boolean values — strict mode only accepts lowercase.
        "true" => Ok(Value::Bool(true)),
        "false" => Ok(Value::Bool(false)),
        "True" | "TRUE" if !strict_booleans => Ok(Value::Bool(true)),
        "False" | "FALSE" if !strict_booleans => Ok(Value::Bool(false)),

        // Special float values.
        ".inf" | ".Inf" | ".INF" | "+.inf" | "+.Inf" | "+.INF" => {
            Ok(Value::Number(Number::Float(f64::INFINITY)))
        }
        "-.inf" | "-.Inf" | "-.INF" => Ok(Value::Number(Number::Float(f64::NEG_INFINITY))),
        ".nan" | ".NaN" | ".NAN" => Ok(Value::Number(Number::Float(f64::NAN))),

        _ => {
            // Fast reject: if the first byte isn't a digit, sign, or dot,
            // this can't be a number — skip numeric parsing entirely.
            let first = value.as_bytes()[0];
            let could_be_number =
                first.is_ascii_digit() || first == b'+' || first == b'-' || first == b'.';

            // Try integer patterns.
            if could_be_number {
                if let Some(n) = try_parse_integer(&value) {
                    return Ok(Value::Number(Number::Integer(n)));
                }
            }

            // Try float patterns.
            if could_be_number {
                if let Some(f) = try_parse_float(&value) {
                    return Ok(Value::Number(Number::Float(f)));
                }
            }

            // Large integers that overflow i64 — store as float (matches YAML convention).
            if could_be_number && looks_like_integer(&value) {
                if let Ok(f) = value.parse::<f64>() {
                    return Ok(Value::Number(Number::Float(f)));
                }
            }

            // Default: string — into_owned() avoids allocation if already Owned.
            Ok(Value::String(value.into_owned()))
        }
    }
}

/// Resolve a quoted scalar.
pub(crate) fn resolve_quoted_scalar(
    value: Cow<'_, str>,
    tag: &Option<(String, String)>,
) -> core::result::Result<Value, String> {
    if let Some((handle, suffix)) = tag {
        return resolve_tagged_scalar(&value, handle, suffix);
    }
    // Quoted scalars are always strings.
    Ok(Value::String(value.into_owned()))
}

/// Resolve a scalar with an explicit tag.
fn resolve_tagged_scalar(
    value: &str,
    handle: &str,
    suffix: &str,
) -> core::result::Result<Value, String> {
    // Build full tag URI.
    let full_tag = match handle {
        "!!" => format!("tag:yaml.org,2002:{suffix}"),
        "!" => {
            if suffix.is_empty() {
                return Ok(Value::String(value.to_string()));
            }
            format!("!{suffix}")
        }
        _ => format!("{handle}{suffix}"),
    };

    match full_tag.as_str() {
        "tag:yaml.org,2002:null" => Ok(Value::Null),
        "tag:yaml.org,2002:bool" => match value {
            "true" | "True" | "TRUE" => Ok(Value::Bool(true)),
            "false" | "False" | "FALSE" => Ok(Value::Bool(false)),
            _ => Err(format!("invalid boolean: {value}")),
        },
        "tag:yaml.org,2002:int" => {
            if let Some(n) = try_parse_integer(value) {
                Ok(Value::Number(Number::Integer(n)))
            } else {
                Err(format!("invalid integer: {value}"))
            }
        }
        "tag:yaml.org,2002:float" => match value {
            ".inf" | ".Inf" | ".INF" | "+.inf" | "+.Inf" | "+.INF" => {
                Ok(Value::Number(Number::Float(f64::INFINITY)))
            }
            "-.inf" | "-.Inf" | "-.INF" => Ok(Value::Number(Number::Float(f64::NEG_INFINITY))),
            ".nan" | ".NaN" | ".NAN" => Ok(Value::Number(Number::Float(f64::NAN))),
            _ => {
                if let Some(f) = try_parse_float(value) {
                    Ok(Value::Number(Number::Float(f)))
                } else {
                    Err(format!("invalid float: {value}"))
                }
            }
        },
        "tag:yaml.org,2002:str" => Ok(Value::String(value.to_string())),
        _ => {
            // Unknown tag: treat as tagged value using our Tag type.
            use crate::value::{Tag, TaggedValue};
            let tag_obj = Tag::new(full_tag);
            let inner = if let Some(n) = try_parse_integer(value) {
                Value::Number(Number::Integer(n))
            } else if let Some(f) = try_parse_float(value) {
                Value::Number(Number::Float(f))
            } else {
                Value::String(value.to_string())
            };
            Ok(Value::Tagged(Box::new(TaggedValue::new(tag_obj, inner))))
        }
    }
}

/// Try to parse an integer (decimal, hex, octal).
fn try_parse_integer(s: &str) -> Option<i64> {
    let bytes = s.as_bytes();
    if bytes.is_empty() {
        return None;
    }

    // Hex: 0x...
    if bytes.len() > 2 && bytes[0] == b'0' && (bytes[1] == b'x' || bytes[1] == b'X') {
        return i64::from_str_radix(&s[2..], 16).ok();
    }

    // Octal: 0o...
    if bytes.len() > 2 && bytes[0] == b'0' && (bytes[1] == b'o' || bytes[1] == b'O') {
        return i64::from_str_radix(&s[2..], 8).ok();
    }

    // Decimal integer: optional sign followed by digits.
    let start = if bytes[0] == b'+' || bytes[0] == b'-' {
        1
    } else {
        0
    };

    if start >= bytes.len() {
        return None;
    }

    // All remaining chars must be digits.
    if bytes[start..].iter().all(|b| b.is_ascii_digit()) {
        s.parse::<i64>().ok()
    } else {
        None
    }
}

/// Check if a string looks like a decimal integer (digits with optional leading
/// sign). Used to detect i64-overflowing integers that should be stored as
/// floats.
fn looks_like_integer(s: &str) -> bool {
    let bytes = s.as_bytes();
    if bytes.is_empty() {
        return false;
    }
    let start = if bytes[0] == b'+' || bytes[0] == b'-' {
        1
    } else {
        0
    };
    start < bytes.len() && bytes[start..].iter().all(|b| b.is_ascii_digit())
}

/// Try to parse a float.
fn try_parse_float(s: &str) -> Option<f64> {
    let bytes = s.as_bytes();
    if bytes.is_empty() {
        return None;
    }

    let start = if bytes[0] == b'+' || bytes[0] == b'-' {
        1
    } else {
        0
    };

    if start >= bytes.len() {
        return None;
    }

    let rest = &bytes[start..];

    // Must contain a dot or 'e'/'E' to be a float.
    let has_dot = rest.contains(&b'.');
    let has_exp = rest.iter().any(|&b| b == b'e' || b == b'E');

    if !has_dot && !has_exp {
        return None;
    }

    // Validate: digits, dots, and exponent parts.
    // Let Rust's parser handle the actual validation.
    s.parse::<f64>().ok()
}

// ── Span-free loader ────────────────────────────────────────────────────
// Used by `from_str` (the common path) to avoid building a SpanTree that
// is immediately discarded for non-Spanned types.

/// Build a single `Value` from YAML input without constructing a `SpanTree`.
pub(crate) fn load_one_no_spans(input: &str, config: &ParseConfig) -> Result<Value> {
    if input.len() > config.max_document_length {
        return Err(Error::Parse(format!(
            "document exceeds maximum length of {} bytes",
            config.max_document_length
        )));
    }

    let mut parser = Parser::new(input);
    let mut loader = NoSpanLoader::new(config);

    loop {
        let event = parser
            .next_event()
            .map_err(|e| Error::parse_at(&*e.message, input, e.index))?;

        match loader.process_event(event, input)? {
            LoaderState::Continue => {}
            LoaderState::Done => break,
        }
    }

    let mut docs = loader.into_docs();
    if docs.is_empty() {
        // Empty/comment-only documents resolve to Null per YAML spec.
        return Ok(Value::Null);
    }
    Ok(docs.swap_remove(0))
}

/// Build all `Value` documents from YAML input without constructing `SpanTree`s.
#[cfg(not(feature = "std"))]
pub(crate) fn load_all_no_spans(input: &str, config: &ParseConfig) -> Result<Vec<Value>> {
    if input.len() > config.max_document_length {
        return Err(Error::Parse(format!(
            "document exceeds maximum length of {} bytes",
            config.max_document_length
        )));
    }

    let mut parser = Parser::new(input);
    let mut loader = NoSpanLoader::new(config);

    loop {
        let event = parser
            .next_event()
            .map_err(|e| Error::parse_at(&*e.message, input, e.index))?;

        match loader.process_event(event, input)? {
            LoaderState::Continue => {}
            LoaderState::Done => break,
        }
    }

    Ok(loader.into_docs())
}

/// Stack frame for the span-free tree builder.
#[derive(Debug)]
enum NoSpanFrame {
    Sequence {
        items: Vec<Value>,
        anchor: Option<String>,
    },
    MappingKey {
        map: Mapping,
        anchor: Option<String>,
        merge_values: Vec<Value>,
    },
    MappingValue {
        map: Mapping,
        key: String,
        anchor: Option<String>,
        merge_values: Vec<Value>,
    },
}

/// Span-free YAML tree builder — identical logic to `Loader` but without any
/// `SpanTree` tracking.
struct NoSpanLoader<'a> {
    docs: Vec<Value>,
    stack: Vec<NoSpanFrame>,
    anchor_map: IndexMap<String, Value>,
    alias_count: usize,
    alias_bytes: usize,
    config: &'a ParseConfig,
    depth: usize,
    in_document: bool,
}

impl<'a> NoSpanLoader<'a> {
    fn new(config: &'a ParseConfig) -> Self {
        NoSpanLoader {
            docs: Vec::new(),
            stack: Vec::new(),
            anchor_map: IndexMap::new(),
            alias_count: 0,
            alias_bytes: 0,
            config,
            depth: 0,
            in_document: false,
        }
    }

    fn into_docs(self) -> Vec<Value> {
        self.docs
    }

    fn process_event(&mut self, event: Event<'_>, input: &str) -> Result<LoaderState> {
        match event {
            Event::StreamStart => Ok(LoaderState::Continue),
            Event::StreamEnd => Ok(LoaderState::Done),
            Event::DocumentStart => {
                self.in_document = true;
                self.anchor_map.clear();
                self.alias_count = 0;
                self.alias_bytes = 0;
                Ok(LoaderState::Continue)
            }
            Event::DocumentEnd => {
                self.in_document = false;
                Ok(LoaderState::Continue)
            }
            Event::Scalar {
                value,
                style,
                anchor,
                tag,
                span,
            } => {
                let resolved = if style == ScalarStyle::Plain {
                    resolve_plain_scalar(
                        value,
                        &tag,
                        self.config.strict_booleans,
                        self.config.legacy_booleans,
                    )
                } else {
                    resolve_quoted_scalar(value, &tag)
                };
                let resolved = match resolved {
                    Ok(v) => v,
                    Err(msg) => return Err(Error::parse_at(msg, input, span.start)),
                };
                if let Some(name) = anchor {
                    let _ = self.anchor_map.insert(name, resolved.clone());
                }
                self.push_value(resolved)?;
                Ok(LoaderState::Continue)
            }
            Event::Alias { anchor, span } => {
                self.alias_count += 1;
                if self.alias_count > self.config.max_alias_expansions {
                    return Err(Error::RepetitionLimitExceeded);
                }
                let value = self.anchor_map.get(&anchor).cloned().ok_or_else(|| {
                    Error::parse_at(format!("unknown anchor '{anchor}'"), input, span.start)
                })?;
                self.alias_bytes = self.alias_bytes.saturating_add(estimate_value_size(&value));
                if self.alias_bytes > self.config.max_document_length {
                    return Err(Error::RepetitionLimitExceeded);
                }
                self.push_value(value)?;
                Ok(LoaderState::Continue)
            }
            Event::SequenceStart { anchor, .. } => {
                self.depth += 1;
                if self.depth > self.config.max_depth {
                    return Err(Error::RecursionLimitExceeded { depth: self.depth });
                }
                self.stack.push(NoSpanFrame::Sequence {
                    items: Vec::new(),
                    anchor,
                });
                Ok(LoaderState::Continue)
            }
            Event::SequenceEnd { .. } => {
                self.depth = self.depth.saturating_sub(1);
                match self.stack.pop() {
                    Some(NoSpanFrame::Sequence { items, anchor }) => {
                        if items.len() > self.config.max_sequence_length {
                            return Err(Error::Parse(format!(
                                "sequence exceeds maximum length of {} elements",
                                self.config.max_sequence_length
                            )));
                        }
                        let value = Value::Sequence(items);
                        if let Some(name) = anchor {
                            let _ = self.anchor_map.insert(name, value.clone());
                        }
                        self.push_value(value)?;
                    }
                    _ => return Err(Error::Parse("unexpected sequence end".to_owned())),
                }
                Ok(LoaderState::Continue)
            }
            Event::MappingStart { anchor, .. } => {
                self.depth += 1;
                if self.depth > self.config.max_depth {
                    return Err(Error::RecursionLimitExceeded { depth: self.depth });
                }
                self.stack.push(NoSpanFrame::MappingKey {
                    map: Mapping::new(),
                    anchor,
                    merge_values: Vec::new(),
                });
                Ok(LoaderState::Continue)
            }
            Event::MappingEnd { .. } => {
                self.depth = self.depth.saturating_sub(1);
                match self.stack.pop() {
                    Some(NoSpanFrame::MappingKey {
                        mut map,
                        anchor,
                        merge_values,
                    }) => {
                        for merge_val in merge_values {
                            match merge_val {
                                Value::Mapping(merge_map) => {
                                    for (k, v) in merge_map {
                                        if !map.contains_key(&k) {
                                            let _ = map.insert(k, v);
                                        }
                                    }
                                }
                                Value::Sequence(seq) => {
                                    for item in seq {
                                        if let Value::Mapping(merge_map) = item {
                                            for (k, v) in merge_map {
                                                if !map.contains_key(&k) {
                                                    let _ = map.insert(k, v);
                                                }
                                            }
                                        } else {
                                            return Err(Error::ScalarInMergeElement);
                                        }
                                    }
                                }
                                _ => return Err(Error::ScalarInMergeElement),
                            }
                        }

                        if map.len() > self.config.max_mapping_keys {
                            return Err(Error::Parse(format!(
                                "mapping exceeds maximum of {} keys",
                                self.config.max_mapping_keys
                            )));
                        }
                        let value = Value::Mapping(map);
                        if let Some(name) = anchor {
                            let _ = self.anchor_map.insert(name, value.clone());
                        }
                        self.push_value(value)?;
                    }
                    Some(NoSpanFrame::MappingValue { .. }) => {
                        return Err(Error::Parse(
                            "unexpected mapping end while expecting value".to_owned(),
                        ));
                    }
                    _ => return Err(Error::Parse("unexpected mapping end".to_owned())),
                }
                Ok(LoaderState::Continue)
            }
        }
    }

    fn push_value(&mut self, value: Value) -> Result<()> {
        match self.stack.last_mut() {
            None => {
                self.docs.push(value);
            }
            Some(NoSpanFrame::Sequence { items, .. }) => {
                items.push(value);
            }
            Some(NoSpanFrame::MappingKey {
                map,
                anchor,
                merge_values,
            }) => {
                let key = value_into_key(value)?;

                if key == MERGE_KEY {
                    let map = core::mem::replace(map, Mapping::new());
                    let anchor = anchor.clone();
                    let merge_values = core::mem::take(merge_values);
                    *self
                        .stack
                        .last_mut()
                        .expect("internal: stack non-empty during event processing") =
                        NoSpanFrame::MappingValue {
                            map,
                            key,
                            anchor,
                            merge_values,
                        };
                    return Ok(());
                }

                if self.config.duplicate_key_policy == DuplicateKeyPolicy::Error
                    && map.contains_key(&key)
                {
                    return Err(Error::DuplicateKey(key));
                }

                let map = core::mem::replace(map, Mapping::new());
                let anchor = anchor.clone();
                let merge_values = core::mem::take(merge_values);
                *self
                    .stack
                    .last_mut()
                    .expect("internal: stack non-empty during event processing") =
                    NoSpanFrame::MappingValue {
                        map,
                        key,
                        anchor,
                        merge_values,
                    };
            }
            Some(NoSpanFrame::MappingValue {
                map,
                key,
                anchor,
                merge_values,
            }) => {
                if key == MERGE_KEY {
                    merge_values.push(value);
                } else {
                    match self.config.duplicate_key_policy {
                        DuplicateKeyPolicy::First => {
                            if !map.contains_key(key.as_str()) {
                                let _ = map.insert(key.clone(), value);
                            }
                        }
                        DuplicateKeyPolicy::Last | DuplicateKeyPolicy::Error => {
                            let _ = map.insert(key.clone(), value);
                        }
                    }
                }

                let map = core::mem::replace(map, Mapping::new());
                let anchor = anchor.clone();
                let merge_values = core::mem::take(merge_values);
                *self
                    .stack
                    .last_mut()
                    .expect("internal: stack non-empty during event processing") =
                    NoSpanFrame::MappingKey {
                        map,
                        anchor,
                        merge_values,
                    };
            }
        }
        Ok(())
    }
}

/// Estimate the in-memory size of a Value tree (in bytes).
///
/// This is used to bound cumulative alias expansion and prevent
/// billion-laughs attacks where a large anchored value is cloned many times.
fn estimate_value_size(value: &Value) -> usize {
    // Fixed per-node overhead (enum discriminant + heap pointers).
    const NODE_OVERHEAD: usize = 64;
    match value {
        Value::Null | Value::Bool(_) => NODE_OVERHEAD,
        Value::Number(_) => NODE_OVERHEAD,
        Value::String(s) => NODE_OVERHEAD + s.len(),
        Value::Sequence(seq) => NODE_OVERHEAD + seq.iter().map(estimate_value_size).sum::<usize>(),
        Value::Mapping(map) => {
            NODE_OVERHEAD
                + map
                    .iter()
                    .map(|(k, v)| k.len() + estimate_value_size(v))
                    .sum::<usize>()
        }
        Value::Tagged(tagged) => NODE_OVERHEAD + estimate_value_size(tagged.value()),
    }
}
