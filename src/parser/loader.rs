// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Event-to-Value tree builder with security limits.
//!
//! Converts a stream of [`Event`]s directly into `Vec<(Value, SpanTree)>`.

use crate::error::{Error, Result};
use crate::parser::events::Event;
use crate::span_context::SpanTree;
use crate::value::{Mapping, Number, Tag, Value};
use indexmap::IndexMap;

/// Maximum number of bytes an expanded alias can account for per document.
/// Prevents billion-laughs style attacks.
const MAX_ALIAS_BYTES: usize = 1024 * 1024 * 32; // 32 MB

/// Overhead in bytes accounted for each node in a mapping or sequence.
const NODE_OVERHEAD: usize = 32;

/// The YAML merge key (`<<`).
const MERGE_KEY: &str = "<<";

/// Configuration for the internal parser, mirroring `ParserConfig`.
#[derive(Debug, Clone, Copy)]
pub struct ParseConfig {
    pub max_depth: usize,
    pub max_document_length: usize,
    pub max_alias_expansions: usize,
    pub max_mapping_keys: usize,
    pub max_sequence_length: usize,
    pub duplicate_key_policy: DuplicateKeyPolicy,
    pub strict_booleans: bool,
    pub legacy_booleans: bool,
}

impl Default for ParseConfig {
    fn default() -> Self {
        ParseConfig {
            max_depth: 128,
            max_document_length: 1024 * 1024 * 64, // 64 MB
            max_alias_expansions: 1024,
            max_mapping_keys: 1024 * 64,
            max_sequence_length: 1024 * 64,
            duplicate_key_policy: DuplicateKeyPolicy::default(),
            strict_booleans: false,
            legacy_booleans: false,
        }
    }
}

impl From<&crate::de::ParserConfig> for ParseConfig {
    fn from(c: &crate::de::ParserConfig) -> Self {
        ParseConfig {
            max_depth: c.max_depth,
            max_document_length: c.max_document_length,
            max_alias_expansions: c.max_alias_expansions,
            max_mapping_keys: c.max_mapping_keys,
            max_sequence_length: c.max_sequence_length,
            duplicate_key_policy: match c.duplicate_key_policy {
                crate::de::DuplicateKeyPolicy::First => DuplicateKeyPolicy::First,
                crate::de::DuplicateKeyPolicy::Last => DuplicateKeyPolicy::Last,
                crate::de::DuplicateKeyPolicy::Error => DuplicateKeyPolicy::Error,
            },
            strict_booleans: c.strict_booleans,
            legacy_booleans: c.legacy_booleans,
        }
    }
}

/// Policy for handling duplicate keys in a YAML mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DuplicateKeyPolicy {
    /// Use the first occurrence of the key; ignore subsequent ones.
    First,
    /// Use the last occurrence of the key (YAML 1.2 default).
    #[default]
    Last,
    /// Return an error if a duplicate key is encountered.
    Error,
}

/// Walk a stream of events and return a list of YAML documents.
pub(crate) fn load(
    parser: &mut crate::parser::events::Parser<'_>,
    config: &ParseConfig,
    input: &str,
) -> Result<Vec<(Value, SpanTree)>> {
    let mut loader = Loader::new(config);
    loop {
        match parser.next_event() {
            Ok(Event::StreamEnd) => {
                loader.process_event(Event::StreamEnd, input)?;
                break;
            }
            Ok(event) => loader.process_event(event, input)?,
            Err(e) => return Err(Error::parse_at(&*e.message, input, e.index)),
        }
    }
    Ok(loader.into_docs())
}

/// Load the first document from a YAML stream.
pub(crate) fn load_one(
    parser: &mut crate::parser::events::Parser<'_>,
    config: &ParseConfig,
    input: &str,
) -> Result<(Value, SpanTree)> {
    let docs = load(parser, config, input)?;
    // An empty YAML stream (whitespace, comments, or `---` with no
    // content) is a valid document whose value is `null` per YAML 1.2.
    Ok(docs
        .into_iter()
        .next()
        .unwrap_or((Value::Null, SpanTree::Leaf(0, 0))))
}

/// Stack frame for the tree builder.
#[derive(Debug)]
enum Frame {
    Sequence {
        items: Vec<Value>,
        span_items: Vec<SpanTree>,
        start: usize,
        anchor: Option<String>,
    },
    MappingKey {
        map: Mapping,
        span_entries: Vec<((usize, usize), SpanTree)>,
        start: usize,
        anchor: Option<String>,
        merge_values: Vec<Value>,
    },
    MappingValue {
        map: Mapping,
        span_entries: Vec<((usize, usize), SpanTree)>,
        key: String,
        key_span: (usize, usize),
        start: usize,
        anchor: Option<String>,
        merge_values: Vec<Value>,
    },
}

/// YAML tree builder with security limits and span tracking.
struct Loader<'a> {
    docs: Vec<(Value, SpanTree)>,
    stack: Vec<Frame>,
    anchor_map: IndexMap<String, (Value, SpanTree)>,
    alias_count: usize,
    alias_bytes: usize,
    config: &'a ParseConfig,
    depth: usize,
    in_document: bool,
}

impl<'a> Loader<'a> {
    fn new(config: &'a ParseConfig) -> Self {
        Loader {
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

    fn into_docs(self) -> Vec<(Value, SpanTree)> {
        self.docs
    }

    fn process_event(&mut self, event: Event<'_>, input: &str) -> Result<()> {
        match event {
            Event::StreamStart | Event::StreamEnd => {}
            Event::DocumentStart => {
                self.in_document = true;
                self.anchor_map.clear();
                self.alias_count = 0;
                self.alias_bytes = 0;
            }
            Event::DocumentEnd => {
                self.in_document = false;
            }
            Event::Alias { anchor, span } => {
                if !self.in_document {
                    return Err(Error::parse_at("alias outside document", input, span.start));
                }
                self.alias_count += 1;
                if self.alias_count > self.config.max_alias_expansions {
                    return Err(Error::RepetitionLimitExceeded);
                }

                let (value, span_tree) =
                    self.anchor_map.get(&anchor).cloned().ok_or_else(|| {
                        Error::UnknownAnchorAt {
                            name: anchor.clone(),
                            location: crate::error::Location::from_index(input, span.start),
                            suggestion: None,
                        }
                    })?;

                self.alias_bytes += estimate_value_size(&value);
                // Bound cumulative alias expansion by the document length
                // limit — a classic billion-laughs vector amplifies well
                // beyond the raw input size.
                if self.alias_bytes > self.config.max_document_length
                    || self.alias_bytes > MAX_ALIAS_BYTES
                {
                    return Err(Error::RepetitionLimitExceeded);
                }

                self.push_node(value, span_tree, input)?;
            }
            Event::Scalar {
                value,
                style,
                anchor,
                tag,
                span,
            } => {
                let v = if let Some(t) = tag {
                    resolve_tagged_scalar(&t.0, &t.1, &value)?
                } else if style != crate::parser::ScalarStyle::Plain {
                    // Quoted/literal/folded scalars always resolve as
                    // strings — YAML schema resolution only applies to
                    // plain scalars.
                    Value::String(value.into_owned())
                } else {
                    match crate::streaming::resolve_plain(
                        &value,
                        self.config.strict_booleans,
                        self.config.legacy_booleans,
                    ) {
                        crate::streaming::Scalar::Null => Value::Null,
                        crate::streaming::Scalar::Bool(b) => Value::Bool(b),
                        crate::streaming::Scalar::Int(i) => Value::Number(Number::Integer(i)),
                        crate::streaming::Scalar::Float(f) => Value::Number(Number::Float(f)),
                        crate::streaming::Scalar::Str(s) => Value::String(s.into_owned()),
                    }
                };

                let st = SpanTree::Leaf(span.start, span.end);
                if let Some(name) = anchor {
                    let _ = self.anchor_map.insert(name, (v.clone(), st.clone()));
                }
                self.push_node(v, st, input)?;
            }
            Event::SequenceStart { anchor, span, .. } => {
                self.depth += 1;
                if self.depth > self.config.max_depth {
                    return Err(Error::RecursionLimitExceeded { depth: self.depth });
                }
                self.stack.push(Frame::Sequence {
                    items: Vec::new(),
                    span_items: Vec::new(),
                    start: span.start,
                    anchor,
                });
            }
            Event::SequenceEnd { span } => {
                self.depth = self.depth.saturating_sub(1);
                if let Some(Frame::Sequence {
                    items,
                    span_items,
                    start,
                    anchor,
                }) = self.stack.pop()
                {
                    let v = Value::Sequence(items);
                    let st = SpanTree::Sequence {
                        start,
                        end: span.end,
                        items: span_items,
                    };
                    if let Some(name) = anchor {
                        let _ = self.anchor_map.insert(name, (v.clone(), st.clone()));
                    }
                    self.push_node(v, st, input)?;
                } else {
                    return Err(Error::parse_at(
                        "unexpected sequence end",
                        input,
                        span.start,
                    ));
                }
            }
            Event::MappingStart { anchor, span, .. } => {
                self.depth += 1;
                if self.depth > self.config.max_depth {
                    return Err(Error::RecursionLimitExceeded { depth: self.depth });
                }
                self.stack.push(Frame::MappingKey {
                    map: Mapping::new(),
                    span_entries: Vec::new(),
                    start: span.start,
                    anchor,
                    merge_values: Vec::new(),
                });
            }
            Event::MappingEnd { span } => {
                self.depth = self.depth.saturating_sub(1);
                if let Some(Frame::MappingKey {
                    mut map,
                    span_entries,
                    start,
                    anchor,
                    merge_values,
                }) = self.stack.pop()
                {
                    for mv in merge_values {
                        apply_merge(&mut map, mv)?;
                    }

                    let v = Value::Mapping(map);
                    let st = SpanTree::Mapping {
                        start,
                        end: span.end,
                        entries: span_entries,
                    };
                    if let Some(name) = anchor {
                        let _ = self.anchor_map.insert(name, (v.clone(), st.clone()));
                    }
                    self.push_node(v, st, input)?;
                } else {
                    return Err(Error::parse_at("unexpected mapping end", input, span.start));
                }
            }
        }
        Ok(())
    }

    fn push_node(&mut self, value: Value, span: SpanTree, input: &str) -> Result<()> {
        if self.stack.is_empty() {
            self.docs.push((value, span));
            return Ok(());
        }

        match self.stack.last_mut().unwrap() {
            Frame::Sequence {
                items, span_items, ..
            } => {
                if items.len() >= self.config.max_sequence_length {
                    return Err(Error::Serialize(
                        "sequence length limit exceeded".to_owned(),
                    ));
                }
                items.push(value);
                span_items.push(span);
            }
            Frame::MappingKey {
                map,
                span_entries,
                start,
                anchor,
                merge_values,
            } => {
                // Coerce scalar keys to strings; complex keys (sequences,
                // mappings) are stringified via their YAML serialization
                // so the final `Mapping<String, Value>` can hold them.
                let key_str = match value_to_key_string(value) {
                    Some(k) => k,
                    None => {
                        return Err(Error::parse_at(
                            "mapping key must be a scalar or representable as string",
                            input,
                            0,
                        ))
                    }
                };
                let key_span = if let SpanTree::Leaf(s, e) = span {
                    (s, e)
                } else {
                    (0, 0)
                };
                let old_map = std::mem::replace(map, Mapping::new());
                let old_span_entries = std::mem::replace(span_entries, Vec::new());
                let old_start = *start;
                let old_anchor = anchor.take();
                let old_merge_values = std::mem::replace(merge_values, Vec::new());

                *self.stack.last_mut().unwrap() = Frame::MappingValue {
                    map: old_map,
                    span_entries: old_span_entries,
                    key: key_str,
                    key_span,
                    start: old_start,
                    anchor: old_anchor,
                    merge_values: old_merge_values,
                };
            }
            Frame::MappingValue {
                map,
                span_entries,
                key,
                key_span,
                start,
                anchor,
                merge_values,
            } => {
                if key == MERGE_KEY {
                    merge_values.push(value);
                } else {
                    if map.len() >= self.config.max_mapping_keys {
                        return Err(Error::Serialize("mapping key limit exceeded".to_owned()));
                    }
                    match self.config.duplicate_key_policy {
                        DuplicateKeyPolicy::First => {
                            if !map.contains_key(key) {
                                let _ = map.insert(key.clone(), value);
                                span_entries.push((*key_span, span));
                            }
                        }
                        DuplicateKeyPolicy::Last => {
                            let _ = map.insert(key.clone(), value);
                            span_entries.push((*key_span, span));
                        }
                        DuplicateKeyPolicy::Error => {
                            if map.contains_key(key) {
                                return Err(Error::DuplicateKey(key.clone()));
                            }
                            let _ = map.insert(key.clone(), value);
                            span_entries.push((*key_span, span));
                        }
                    }
                }

                let old_map = std::mem::replace(map, Mapping::new());
                let old_span_entries = std::mem::replace(span_entries, Vec::new());
                let old_start = *start;
                let old_anchor = anchor.take();
                let old_merge_values = std::mem::replace(merge_values, Vec::new());

                *self.stack.last_mut().unwrap() = Frame::MappingKey {
                    map: old_map,
                    span_entries: old_span_entries,
                    start: old_start,
                    anchor: old_anchor,
                    merge_values: old_merge_values,
                };
            }
        }
        Ok(())
    }
}

fn apply_merge(map: &mut Mapping, merge_value: Value) -> Result<()> {
    match merge_value {
        Value::Mapping(m) => {
            for (k, v) in m {
                if !map.contains_key(&k) {
                    let _ = map.insert(k, v);
                }
            }
        }
        Value::Sequence(s) => {
            for v in s {
                apply_merge(map, v)?;
            }
        }
        Value::Null => {}
        _ => return Err(Error::ScalarInMergeElement),
    }
    Ok(())
}

fn estimate_value_size(v: &Value) -> usize {
    match v {
        Value::Null | Value::Bool(_) | Value::Number(_) => NODE_OVERHEAD,
        Value::String(s) => NODE_OVERHEAD + s.len(),
        Value::Sequence(s) => NODE_OVERHEAD + s.iter().map(estimate_value_size).sum::<usize>(),
        Value::Mapping(m) => {
            NODE_OVERHEAD
                + m.iter()
                    .map(|(k, v)| k.len() + estimate_value_size(v))
                    .sum::<usize>()
        }
        Value::Tagged(tagged) => NODE_OVERHEAD + estimate_value_size(tagged.value()),
    }
}

// ── Span-free loader (no_std path) ──────────────────────────────────────
//
// Only compiled when the `std` feature is disabled. The `std` build
// always uses the span-aware loader above so `Spanned<T>` fields are
// populated correctly.

#[cfg(not(feature = "std"))]
pub(crate) fn load_one_no_spans(input: &str, config: &ParseConfig) -> Result<Value> {
    let mut parser = crate::parser::events::Parser::new(input);
    let mut loader = NoSpanLoader::new(config);
    loop {
        match parser.next_event() {
            Ok(Event::StreamEnd) => {
                loader.process_event(Event::StreamEnd, input)?;
                break;
            }
            Ok(event) => loader.process_event(event, input)?,
            Err(e) => return Err(Error::parse_at(&*e.message, input, e.index)),
        }
    }
    Ok(loader.docs.into_iter().next().unwrap_or(Value::Null))
}

#[cfg(not(feature = "std"))]
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

#[cfg(not(feature = "std"))]
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

#[cfg(not(feature = "std"))]
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

    fn process_event(&mut self, event: Event<'_>, input: &str) -> Result<()> {
        match event {
            Event::StreamStart | Event::StreamEnd => {}
            Event::DocumentStart => {
                self.in_document = true;
                self.anchor_map.clear();
            }
            Event::DocumentEnd => {
                self.in_document = false;
            }
            Event::Alias { anchor, span } => {
                self.alias_count += 1;
                if self.alias_count > self.config.max_alias_expansions {
                    return Err(Error::RepetitionLimitExceeded);
                }
                let value = self.anchor_map.get(&anchor).cloned().ok_or_else(|| {
                    Error::UnknownAnchorAt {
                        name: anchor,
                        location: crate::error::Location::from_index(input, span.start),
                        suggestion: None,
                    }
                })?;
                self.alias_bytes += estimate_value_size(&value);
                if self.alias_bytes > MAX_ALIAS_BYTES {
                    return Err(Error::RepetitionLimitExceeded);
                }
                self.push_value(value)?;
            }
            Event::Scalar {
                value,
                style,
                anchor,
                tag,
                ..
            } => {
                let v = if let Some(t) = tag {
                    resolve_tagged_scalar(&t.0, &t.1, &value)?
                } else if style != crate::parser::ScalarStyle::Plain {
                    Value::String(value.into_owned())
                } else {
                    match crate::streaming::resolve_plain(
                        &value,
                        self.config.strict_booleans,
                        self.config.legacy_booleans,
                    ) {
                        crate::streaming::Scalar::Null => Value::Null,
                        crate::streaming::Scalar::Bool(b) => Value::Bool(b),
                        crate::streaming::Scalar::Int(i) => Value::Number(Number::Integer(i)),
                        crate::streaming::Scalar::Float(f) => Value::Number(Number::Float(f)),
                        crate::streaming::Scalar::Str(s) => Value::String(s.into_owned()),
                    }
                };
                if let Some(name) = anchor {
                    let _ = self.anchor_map.insert(name, v.clone());
                }
                self.push_value(v)?;
            }
            Event::SequenceStart { anchor, .. } => {
                self.depth += 1;
                self.stack.push(NoSpanFrame::Sequence {
                    items: Vec::new(),
                    anchor,
                });
            }
            Event::SequenceEnd { .. } => {
                self.depth = self.depth.saturating_sub(1);
                if let Some(NoSpanFrame::Sequence { items, anchor }) = self.stack.pop() {
                    let v = Value::Sequence(items);
                    if let Some(name) = anchor {
                        let _ = self.anchor_map.insert(name, v.clone());
                    }
                    self.push_value(v)?;
                }
            }
            Event::MappingStart { anchor, .. } => {
                self.depth += 1;
                self.stack.push(NoSpanFrame::MappingKey {
                    map: Mapping::new(),
                    anchor,
                    merge_values: Vec::new(),
                });
            }
            Event::MappingEnd { .. } => {
                self.depth = self.depth.saturating_sub(1);
                if let Some(NoSpanFrame::MappingKey {
                    mut map,
                    anchor,
                    merge_values,
                }) = self.stack.pop()
                {
                    for mv in merge_values {
                        apply_merge(&mut map, mv)?;
                    }
                    let v = Value::Mapping(map);
                    if let Some(name) = anchor {
                        let _ = self.anchor_map.insert(name, v.clone());
                    }
                    self.push_value(v)?;
                }
            }
        }
        Ok(())
    }

    fn push_value(&mut self, value: Value) -> Result<()> {
        if self.stack.is_empty() {
            self.docs.push(value);
            return Ok(());
        }
        match self.stack.last_mut().unwrap() {
            NoSpanFrame::Sequence { items, .. } => {
                items.push(value);
            }
            NoSpanFrame::MappingKey {
                map,
                anchor,
                merge_values,
            } => {
                if let Some(key) = value_to_key_string(value) {
                    let old_map = std::mem::replace(map, Mapping::new());
                    let old_anchor = anchor.take();
                    let old_merge_values = std::mem::replace(merge_values, Vec::new());
                    *self.stack.last_mut().unwrap() = NoSpanFrame::MappingValue {
                        map: old_map,
                        key,
                        anchor: old_anchor,
                        merge_values: old_merge_values,
                    };
                }
            }
            NoSpanFrame::MappingValue {
                map,
                key,
                anchor,
                merge_values,
            } => {
                if key == MERGE_KEY {
                    merge_values.push(value);
                } else {
                    let _ = map.insert(key.clone(), value);
                }
                let old_map = std::mem::replace(map, Mapping::new());
                let old_anchor = anchor.take();
                let old_merge_values = std::mem::replace(merge_values, Vec::new());
                *self.stack.last_mut().unwrap() = NoSpanFrame::MappingKey {
                    map: old_map,
                    anchor: old_anchor,
                    merge_values: old_merge_values,
                };
            }
        }
        Ok(())
    }
}

/// Coerce a `Value` into a mapping-key string. Scalars stringify naturally;
/// sequences and mappings use a deterministic inline YAML-like representation
/// so the parser can still build a `Mapping<String, Value>` from YAML with
/// complex keys (common in the official YAML Test Suite).
fn value_to_key_string(value: Value) -> Option<String> {
    use core::fmt::Write as _;
    match value {
        Value::String(s) => Some(s),
        Value::Bool(b) => Some(if b { "true".into() } else { "false".into() }),
        Value::Null => Some("null".into()),
        Value::Number(Number::Integer(n)) => Some(itoa::Buffer::new().format(n).to_owned()),
        Value::Number(Number::Float(n)) => Some(ryu::Buffer::new().format(n).to_owned()),
        Value::Tagged(t) => value_to_key_string(t.value().clone()),
        Value::Sequence(seq) => {
            let mut s = String::from("[");
            for (i, v) in seq.into_iter().enumerate() {
                if i > 0 {
                    s.push_str(", ");
                }
                let _ = write!(s, "{}", value_to_key_string(v).unwrap_or_default());
            }
            s.push("]".chars().next().unwrap());
            Some(s)
        }
        Value::Mapping(m) => {
            let mut s = String::from("{");
            for (i, (k, v)) in m.into_iter().enumerate() {
                if i > 0 {
                    s.push_str(", ");
                }
                s.push_str(&k);
                s.push_str(": ");
                let _ = write!(s, "{}", value_to_key_string(v).unwrap_or_default());
            }
            s.push("}".chars().next().unwrap());
            Some(s)
        }
    }
}

/// Resolve a tagged scalar into a typed `Value`. Handles the YAML 1.2
/// core schema tags (`!!int`, `!!float`, `!!bool`, `!!null`, `!!str`)
/// and any custom tag falls through to the `Tagged` wrapper.
fn resolve_tagged_scalar(handle: &str, suffix: &str, value: &str) -> Result<Value> {
    // Canonicalize tag: handle `!!foo` (secondary) → `tag:yaml.org,2002:foo`.
    let is_core = handle == "!!"
        || handle == "tag:yaml.org,2002:"
        || (handle == "!" && matches!(suffix, "int" | "float" | "bool" | "null" | "str"));
    if is_core {
        match suffix {
            "int" => {
                // Accept decimal, hex (0x), octal (0o), with optional sign.
                let trimmed = value.trim();
                let parsed = if let Some(rest) = trimmed
                    .strip_prefix("0x")
                    .or_else(|| trimmed.strip_prefix("0X"))
                {
                    i64::from_str_radix(rest, 16).ok()
                } else if let Some(rest) = trimmed
                    .strip_prefix("0o")
                    .or_else(|| trimmed.strip_prefix("0O"))
                {
                    i64::from_str_radix(rest, 8).ok()
                } else {
                    trimmed.parse::<i64>().ok()
                };
                parsed
                    .map(|n| Value::Number(Number::Integer(n)))
                    .ok_or_else(|| Error::FailedToParseNumber(format!("!!int {value}")))
            }
            "float" => {
                let trimmed = value.trim();
                match trimmed {
                    ".inf" | ".Inf" | ".INF" | "+.inf" | "+.Inf" | "+.INF" => {
                        Ok(Value::Number(Number::Float(f64::INFINITY)))
                    }
                    "-.inf" | "-.Inf" | "-.INF" => {
                        Ok(Value::Number(Number::Float(f64::NEG_INFINITY)))
                    }
                    ".nan" | ".NaN" | ".NAN" => Ok(Value::Number(Number::Float(f64::NAN))),
                    _ => trimmed
                        .parse::<f64>()
                        .map(|f| Value::Number(Number::Float(f)))
                        .map_err(|_| Error::FailedToParseNumber(format!("!!float {value}"))),
                }
            }
            "bool" => match value.trim() {
                "true" | "True" | "TRUE" => Ok(Value::Bool(true)),
                "false" | "False" | "FALSE" => Ok(Value::Bool(false)),
                _ => Err(Error::FailedToParseNumber(format!("!!bool {value}"))),
            },
            "null" => match value.trim() {
                "" | "~" | "null" | "Null" | "NULL" => Ok(Value::Null),
                _ => Err(Error::FailedToParseNumber(format!("!!null {value}"))),
            },
            "str" => Ok(Value::String(value.to_owned())),
            _ => Ok(Value::Tagged(Box::new(crate::value::TaggedValue::new(
                Tag::new(format!("{handle}{suffix}")),
                Value::String(value.to_owned()),
            )))),
        }
    } else {
        Ok(Value::Tagged(Box::new(crate::value::TaggedValue::new(
            Tag::new(format!("{handle}{suffix}")),
            Value::String(value.to_owned()),
        ))))
    }
}
