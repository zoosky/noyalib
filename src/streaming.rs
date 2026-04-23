//! Streaming YAML deserializer that operates directly on parser events.
//!
//! Bypasses the intermediate `Value` AST for typed deserialization,
//! eliminating all intermediate allocations. Anchors and aliases are
//! handled natively via event buffering and replay. The common
//! `<<: *anchor` merge-key pattern (merge at first position, single
//! anchor) is also expanded natively; sequence merges, locals-before-
//! merge, and non-mapping merge targets fall back to the Value-based
//! path so correctness is preserved. Tags and `Spanned<T>` still
//! trigger the AST fallback.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use crate::prelude::*;

use rustc_hash::FxHashMap;
use serde::de::{self, DeserializeSeed, IntoDeserializer, MapAccess, SeqAccess, Visitor};
use serde::Deserialize;

use crate::error::{Error, Result};
use crate::parser::{Event, ParseConfig, Parser, ScalarStyle};

/// Sentinel error message used to signal fallback to the Value-based path.
const FALLBACK_SENTINEL: &str = "$__noyalib_streaming_fallback";

/// A simplified, owned copy of a parser event used for anchor replay.
///
/// When the streaming deserializer encounters an anchored subtree, it
/// records all events into a `Vec<BufferedEvent>`. When an alias references
/// that anchor, the buffer is pushed onto a replay stack so the events are
/// re-emitted without re-parsing.
#[derive(Debug, Clone)]
enum BufferedEvent {
    /// A scalar value.
    Scalar {
        /// The resolved scalar text.
        value: String,
        /// The original scalar style.
        style: ScalarStyle,
    },
    /// Start of a sequence.
    SeqStart,
    /// End of a sequence.
    SeqEnd,
    /// Start of a mapping.
    MapStart,
    /// End of a mapping.
    MapEnd,
}

/// Resolved scalar — avoids building a `Value` for the common case.
enum Scalar<'a> {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(Cow<'a, str>),
}

/// Resolve a plain scalar string into a typed `Scalar` without allocating
/// a `Value`. Mirrors the YAML 1.2 Core Schema resolution from the loader.
fn resolve_plain(s: &str, strict_booleans: bool, legacy_booleans: bool) -> Scalar<'_> {
    // YAML 1.1 legacy booleans (yes/no/on/off/y/n).
    if legacy_booleans {
        match s {
            "yes" | "Yes" | "YES" | "on" | "On" | "ON" | "y" | "Y" => {
                return Scalar::Bool(true);
            }
            "no" | "No" | "NO" | "off" | "Off" | "OFF" | "n" | "N" => {
                return Scalar::Bool(false);
            }
            _ => {}
        }
    }

    match s {
        "" | "~" | "null" | "Null" | "NULL" => Scalar::Null,
        "true" => Scalar::Bool(true),
        "false" => Scalar::Bool(false),
        "True" | "TRUE" if !strict_booleans => Scalar::Bool(true),
        "False" | "FALSE" if !strict_booleans => Scalar::Bool(false),
        ".inf" | ".Inf" | ".INF" | "+.inf" | "+.Inf" | "+.INF" => Scalar::Float(f64::INFINITY),
        "-.inf" | "-.Inf" | "-.INF" => Scalar::Float(f64::NEG_INFINITY),
        ".nan" | ".NaN" | ".NAN" => Scalar::Float(f64::NAN),
        _ => {
            let bytes = s.as_bytes();
            let first = bytes[0];
            let could_be_number =
                first.is_ascii_digit() || first == b'+' || first == b'-' || first == b'.';

            if could_be_number {
                // Try integer.
                if let Some(n) = try_parse_integer(s) {
                    return Scalar::Int(n);
                }
                // Try float.
                if let Some(f) = try_parse_float(s) {
                    return Scalar::Float(f);
                }
                // Large integers that overflow i64.
                if looks_like_integer(s) {
                    if let Ok(f) = s.parse::<f64>() {
                        return Scalar::Float(f);
                    }
                }
            }
            Scalar::Str(Cow::Borrowed(s))
        }
    }
}

/// Try to parse an integer (decimal, hex, octal).
fn try_parse_integer(s: &str) -> Option<i64> {
    let bytes = s.as_bytes();
    if bytes.is_empty() {
        return None;
    }
    if bytes.len() > 2 && bytes[0] == b'0' && (bytes[1] == b'x' || bytes[1] == b'X') {
        return i64::from_str_radix(&s[2..], 16).ok();
    }
    if bytes.len() > 2 && bytes[0] == b'0' && (bytes[1] == b'o' || bytes[1] == b'O') {
        return i64::from_str_radix(&s[2..], 8).ok();
    }
    let start = if bytes[0] == b'+' || bytes[0] == b'-' {
        1
    } else {
        0
    };
    if start >= bytes.len() {
        return None;
    }
    if bytes[start..].iter().all(|b| b.is_ascii_digit()) {
        s.parse::<i64>().ok()
    } else {
        None
    }
}

/// Check if a string looks like a decimal integer.
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
    let has_dot = rest.contains(&b'.');
    let has_exp = rest.iter().any(|&b| b == b'e' || b == b'E');
    if !has_dot && !has_exp {
        return None;
    }
    s.parse::<f64>().ok()
}

/// A streaming YAML deserializer that operates directly on parser events.
///
/// Bypasses the intermediate `Value` AST for typed deserialization,
/// eliminating all intermediate allocations.
struct StreamingDeserializer<'a> {
    parser: Parser<'a>,
    /// The current event (peeked but not consumed).
    current: Option<Event<'a>>,
    /// Input string for error reporting and fallback.
    input: &'a str,
    /// Configuration.
    config: ParseConfig,
    /// Current nesting depth for recursion limit enforcement.
    depth: usize,
    /// When true, `deserialize_str` returns the raw scalar text without
    /// YAML type resolution. Set during map key deserialization so that
    /// keys like `true`, `42`, or `null` are passed through as strings.
    raw_str_mode: bool,
    /// Buffered events for anchored subtrees, keyed by anchor name.
    anchor_events: FxHashMap<String, Vec<BufferedEvent>>,
    /// Byte offset of each anchor's definition site, keyed by anchor name.
    /// Used by [`crate::error::Error::UnknownAnchorAt`] to point at the
    /// anchor the user probably meant when suggesting typo fixes.
    anchor_def_spans: FxHashMap<String, usize>,
    /// Stack of event buffers being replayed for alias resolution.
    /// Each entry is a reversed list of events; we pop from the end.
    replay_stack: Vec<Vec<BufferedEvent>>,
    /// When `Some`, we are recording events for an anchored compound
    /// node. The tuple holds (anchor_name, nesting_depth, buffer).
    recording: Option<(String, usize, Vec<BufferedEvent>)>,
}

impl<'a> StreamingDeserializer<'a> {
    fn new(input: &'a str, config: &ParseConfig) -> Self {
        StreamingDeserializer {
            parser: Parser::new(input),
            current: None,
            input,
            config: *config,
            depth: 0,
            raw_str_mode: false,
            anchor_events: FxHashMap::default(),
            anchor_def_spans: FxHashMap::default(),
            replay_stack: Vec::new(),
            recording: None,
        }
    }

    /// Peek at the next event without consuming it.
    fn peek_event(&mut self) -> Result<&Event<'a>> {
        if self.current.is_none() {
            // Check replay stack first.
            if let Some(buf) = self.replay_stack.last_mut() {
                if let Some(be) = buf.pop() {
                    if buf.is_empty() {
                        let _ = self.replay_stack.pop();
                    }
                    self.current = Some(self.buffered_to_event(be));
                    return Ok(self.current.as_ref().expect("internal: current set above"));
                }
                let _ = self.replay_stack.pop();
            }

            let event = self
                .parser
                .next_event()
                .map_err(|e| Error::parse_at(&*e.message, self.input, e.index))?;

            // Handle alias transparently at peek time.
            if let Event::Alias {
                ref anchor,
                ref span,
            } = event
            {
                let start = span.start;
                let resolved = self.resolve_alias(anchor, start)?;
                self.current = Some(resolved);
            } else {
                self.current = Some(event);
            }
        }
        Ok(self
            .current
            .as_ref()
            .expect("internal: current set by peek_event"))
    }

    /// Consume and return the next event.
    ///
    /// First drains from the replay stack (for alias expansion), then
    /// falls through to the peeked event or the underlying parser.
    fn next_event(&mut self) -> Result<Event<'a>> {
        // If we already have a peeked event, use it.
        let mut ev = if let Some(ev) = self.current.take() {
            ev
        } else {
            // Check replay stack.
            if let Some(buf) = self.replay_stack.last_mut() {
                if let Some(be) = buf.pop() {
                    if buf.is_empty() {
                        let _ = self.replay_stack.pop();
                    }
                    return Ok(self.buffered_to_event(be));
                }
                let _ = self.replay_stack.pop();
            }
            self.parser
                .next_event()
                .map_err(|e| Error::parse_at(&*e.message, self.input, e.index))?
        };

        // Handle alias: resolve to buffered events transparently.
        if let Event::Alias {
            ref anchor,
            ref span,
        } = ev
        {
            let start = span.start;
            let resolved = self.resolve_alias(anchor, start)?;
            return Ok(resolved);
        }

        // Start recording if this event introduces an anchor.
        self.handle_anchor(&mut ev);
        // Buffer a copy if we are inside an anchored subtree.
        self.maybe_record(&ev);

        Ok(ev)
    }

    /// Convert a `BufferedEvent` back into an owned `Event` for
    /// consumption by the deserializer.
    fn buffered_to_event(&self, be: BufferedEvent) -> Event<'a> {
        let dummy_span = crate::parser::scanner_span_default();
        match be {
            BufferedEvent::Scalar { value, style } => Event::Scalar {
                value: Cow::Owned(value),
                style,
                anchor: None,
                tag: None,
                span: dummy_span,
            },
            BufferedEvent::SeqStart => Event::SequenceStart {
                anchor: None,
                tag: None,
                span: dummy_span,
            },
            BufferedEvent::SeqEnd => Event::SequenceEnd { span: dummy_span },
            BufferedEvent::MapStart => Event::MappingStart {
                anchor: None,
                tag: None,
                span: dummy_span,
            },
            BufferedEvent::MapEnd => Event::MappingEnd { span: dummy_span },
        }
    }

    /// Handle an anchored event: if the event has an anchor (and no tag),
    /// strip the anchor and start recording. Must be called before
    /// `maybe_record` for the same event.
    fn handle_anchor(&mut self, ev: &mut Event<'_>) {
        // Capture the byte-offset of the anchor definition before we strip
        // it: the recording map is keyed by name, so if the same anchor
        // name is later redefined we still have the most recent location.
        let def_start = match ev {
            Event::Scalar { span, .. }
            | Event::SequenceStart { span, .. }
            | Event::MappingStart { span, .. } => Some(span.start),
            _ => None,
        };
        let anchor_name = Self::strip_anchor_and_record(ev);
        if let Some(name) = anchor_name {
            // Check there is no tag — tags still trigger fallback before we get here.
            let has_tag = match ev {
                Event::Scalar { ref tag, .. }
                | Event::SequenceStart { ref tag, .. }
                | Event::MappingStart { ref tag, .. } => tag.is_some(),
                _ => false,
            };
            if !has_tag {
                if let Some(start) = def_start {
                    let _ = self.anchor_def_spans.insert(name.clone(), start);
                }
                self.start_recording(name);
            }
        }
    }

    /// If we are currently recording an anchored subtree, buffer the event.
    fn maybe_record(&mut self, ev: &Event<'_>) {
        if let Some((_, ref mut depth, ref mut buf)) = self.recording {
            match ev {
                Event::Scalar { value, style, .. } => {
                    buf.push(BufferedEvent::Scalar {
                        value: value.to_string(),
                        style: *style,
                    });
                    if *depth == 0 {
                        // Scalar anchor — recording is done.
                        let (name, _, events) =
                            self.recording.take().expect("internal: recording is Some");
                        let _ = self.anchor_events.insert(name, events);
                    }
                }
                Event::SequenceStart { .. } => {
                    buf.push(BufferedEvent::SeqStart);
                    *depth += 1;
                }
                Event::SequenceEnd { .. } => {
                    buf.push(BufferedEvent::SeqEnd);
                    *depth -= 1;
                    if *depth == 0 {
                        let (name, _, events) =
                            self.recording.take().expect("internal: recording is Some");
                        let _ = self.anchor_events.insert(name, events);
                    }
                }
                Event::MappingStart { .. } => {
                    buf.push(BufferedEvent::MapStart);
                    *depth += 1;
                }
                Event::MappingEnd { .. } => {
                    buf.push(BufferedEvent::MapEnd);
                    *depth -= 1;
                    if *depth == 0 {
                        let (name, _, events) =
                            self.recording.take().expect("internal: recording is Some");
                        let _ = self.anchor_events.insert(name, events);
                    }
                }
                _ => {}
            }
        }
    }

    /// Begin recording events for an anchored node.
    fn start_recording(&mut self, anchor_name: String) {
        self.recording = Some((anchor_name, 0, Vec::new()));
    }

    /// Resolve an alias by pushing its buffered events onto the replay stack.
    /// Returns the first event from the replayed buffer.
    ///
    /// `alias_start` is the byte offset of the alias site used to build a
    /// located [`Error::UnknownAnchorAt`] with an optional typo suggestion.
    fn resolve_alias(&mut self, name: &str, alias_start: usize) -> Result<Event<'a>> {
        let buf = match self.anchor_events.get(name) {
            Some(b) => b.clone(),
            None => return Err(self.build_unknown_anchor(name, alias_start)),
        };
        if buf.is_empty() {
            return Err(self.build_unknown_anchor(name, alias_start));
        }
        // Reverse the buffer so we can pop from the end efficiently.
        let mut reversed = buf;
        reversed.reverse();
        let first = reversed.pop().expect("internal: checked non-empty above");
        if !reversed.is_empty() {
            self.replay_stack.push(reversed);
        }
        Ok(self.buffered_to_event(first))
    }

    /// Inject the inner events of an anchored mapping onto the replay stack
    /// so the caller streams the mapping's contents as if they appeared
    /// inline. Used to expand `<<: *anchor` merge keys natively.
    ///
    /// Returns the fallback sentinel if the target is not a mapping (empty
    /// or scalar/sequence), at which point the caller should propagate the
    /// error to restart deserialization on the AST path.
    fn inject_merge_mapping_contents(&mut self, name: &str, alias_start: usize) -> Result<()> {
        let buf = match self.anchor_events.get(name) {
            Some(b) => b.clone(),
            None => return Err(self.build_unknown_anchor(name, alias_start)),
        };
        // Require the target to be a mapping: [MapStart, ..., MapEnd].
        if buf.len() < 2
            || !matches!(buf.first(), Some(BufferedEvent::MapStart))
            || !matches!(buf.last(), Some(BufferedEvent::MapEnd))
        {
            return Err(self.fallback());
        }
        // Strip outer MapStart/MapEnd; reverse for pop-from-end semantics.
        let mut inner: Vec<BufferedEvent> = buf[1..buf.len() - 1].to_vec();
        if inner.is_empty() {
            return Ok(());
        }
        inner.reverse();
        self.replay_stack.push(inner);
        Ok(())
    }

    /// Build an `UnknownAnchorAt` error, attaching a "did you mean …?"
    /// suggestion when a similar anchor name was defined earlier.
    fn build_unknown_anchor(&self, name: &str, alias_start: usize) -> Error {
        let alias_loc = crate::error::Location::from_index(self.input, alias_start);
        let suggestion =
            crate::error::closest_name(name, self.anchor_def_spans.keys().map(String::as_str))
                .and_then(|sugg| {
                    self.anchor_def_spans.get(sugg).map(|def_start| {
                        (
                            sugg.to_string(),
                            crate::error::Location::from_index(self.input, *def_start),
                        )
                    })
                });
        Error::UnknownAnchorAt {
            name: name.to_owned(),
            location: alias_loc,
            suggestion,
        }
    }

    /// Strip the anchor from an event if present and start recording.
    /// Returns the event without the anchor field set.
    fn strip_anchor_and_record(ev: &mut Event<'_>) -> Option<String> {
        match ev {
            Event::Scalar { ref mut anchor, .. }
            | Event::SequenceStart { ref mut anchor, .. }
            | Event::MappingStart { ref mut anchor, .. } => anchor.take(),
            _ => None,
        }
    }

    /// Skip the next event.
    fn skip_event(&mut self) -> Result<()> {
        let _ = self.next_event()?;
        Ok(())
    }

    /// Skip document wrappers (StreamStart, DocumentStart) to reach content.
    fn skip_to_content(&mut self) -> Result<()> {
        loop {
            let ev = self.peek_event()?;
            match ev {
                Event::StreamStart | Event::DocumentStart => {
                    self.skip_event()?;
                }
                _ => return Ok(()),
            }
        }
    }

    /// Signal that the streaming path cannot handle this input and should
    /// fall back to the Value-based deserializer.
    fn fallback(&self) -> Error {
        Error::Custom(FALLBACK_SENTINEL.to_owned())
    }

    /// Skip over an entire value (scalar, sequence, or mapping) in the
    /// event stream. Used for `deserialize_ignored_any`.
    fn skip_value(&mut self) -> Result<()> {
        let ev = self.next_event()?;
        match ev {
            Event::Scalar { tag, .. } => {
                if tag.is_some() {
                    return Err(self.fallback());
                }
                Ok(())
            }
            Event::Alias { .. } => Err(self.fallback()),
            Event::SequenceStart { tag, .. } => {
                if tag.is_some() {
                    return Err(self.fallback());
                }
                loop {
                    let peek = self.peek_event()?;
                    if matches!(peek, Event::SequenceEnd { .. }) {
                        self.skip_event()?;
                        return Ok(());
                    }
                    self.skip_value()?;
                }
            }
            Event::MappingStart { tag, .. } => {
                if tag.is_some() {
                    return Err(self.fallback());
                }
                loop {
                    let peek = self.peek_event()?;
                    if matches!(peek, Event::MappingEnd { .. }) {
                        self.skip_event()?;
                        return Ok(());
                    }
                    // key
                    self.skip_value()?;
                    // value
                    self.skip_value()?;
                }
            }
            _ => Ok(()),
        }
    }

    /// Resolve a scalar event into a `Scalar` enum (no `Value` allocation).
    fn resolve_scalar<'s>(&self, value: &'s str, style: ScalarStyle) -> Scalar<'s> {
        if style == ScalarStyle::Plain {
            resolve_plain(
                value,
                self.config.strict_booleans,
                self.config.legacy_booleans,
            )
        } else {
            // Quoted scalars are always strings.
            Scalar::Str(Cow::Borrowed(value))
        }
    }
}

impl<'de> de::Deserializer<'de> for &mut StreamingDeserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.skip_to_content()?;
        let ev = self.peek_event()?;
        match ev {
            Event::Scalar { tag, .. } => {
                if tag.is_some() {
                    return Err(self.fallback());
                }
                let ev = self.next_event()?;
                if let Event::Scalar { value, style, .. } = ev {
                    let resolved = self.resolve_scalar(&value, style);
                    match resolved {
                        Scalar::Null => visitor.visit_none(),
                        Scalar::Bool(b) => visitor.visit_bool(b),
                        Scalar::Int(n) => visitor.visit_i64(n),
                        Scalar::Float(f) => visitor.visit_f64(f),
                        Scalar::Str(s) => visitor.visit_str(&s),
                    }
                } else {
                    unreachable!()
                }
            }
            Event::SequenceStart { tag, .. } => {
                if tag.is_some() {
                    return Err(self.fallback());
                }
                self.deserialize_seq(visitor)
            }
            Event::MappingStart { tag, .. } => {
                if tag.is_some() {
                    return Err(self.fallback());
                }
                self.deserialize_map(visitor)
            }
            // Aliases are resolved transparently in peek_event/next_event.
            Event::Alias { .. } => Err(self.fallback()),
            Event::StreamEnd | Event::DocumentEnd => {
                // Empty or comment-only document — resolve to null per YAML spec.
                visitor.visit_none()
            }
            _ => {
                // Skip document markers etc.
                self.skip_event()?;
                self.deserialize_any(visitor)
            }
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.skip_to_content()?;
        let ev = self.next_event()?;
        match ev {
            Event::Scalar {
                ref value,
                style,
                ref tag,
                ..
            } => {
                if tag.is_some() {
                    return Err(self.fallback());
                }
                let resolved = self.resolve_scalar(value, style);
                match resolved {
                    Scalar::Bool(b) => visitor.visit_bool(b),
                    _ => Err(Error::TypeMismatch {
                        expected: "bool",
                        found: "other scalar".to_owned(),
                    }),
                }
            }
            Event::Alias { .. } => Err(self.fallback()),
            _ => Err(Error::TypeMismatch {
                expected: "bool",
                found: "non-scalar".to_owned(),
            }),
        }
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_i64(visitor)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_i64(visitor)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_i64(visitor)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.skip_to_content()?;
        let ev = self.next_event()?;
        match ev {
            Event::Scalar {
                ref value,
                style,
                ref tag,
                ..
            } => {
                if tag.is_some() {
                    return Err(self.fallback());
                }
                let resolved = self.resolve_scalar(value, style);
                match resolved {
                    Scalar::Int(n) => visitor.visit_i64(n),
                    Scalar::Float(n)
                        if n.fract() == 0.0
                            && n >= i64::MIN as f64
                            && n <= i64::MAX as f64
                            && !n.is_nan() =>
                    {
                        visitor.visit_i64(n as i64)
                    }
                    _ => Err(Error::TypeMismatch {
                        expected: "integer",
                        found: "other scalar".to_owned(),
                    }),
                }
            }
            Event::Alias { .. } => Err(self.fallback()),
            _ => Err(Error::TypeMismatch {
                expected: "integer",
                found: "non-scalar".to_owned(),
            }),
        }
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_u64(visitor)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_u64(visitor)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_u64(visitor)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.skip_to_content()?;
        let ev = self.next_event()?;
        match ev {
            Event::Scalar {
                ref value,
                style,
                ref tag,
                ..
            } => {
                if tag.is_some() {
                    return Err(self.fallback());
                }
                let resolved = self.resolve_scalar(value, style);
                match resolved {
                    Scalar::Int(n) if n >= 0 => visitor.visit_u64(n as u64),
                    Scalar::Float(n)
                        if n.fract() == 0.0 && n >= 0.0 && n <= u64::MAX as f64 && !n.is_nan() =>
                    {
                        visitor.visit_u64(n as u64)
                    }
                    _ => Err(Error::TypeMismatch {
                        expected: "unsigned integer",
                        found: "other scalar".to_owned(),
                    }),
                }
            }
            Event::Alias { .. } => Err(self.fallback()),
            _ => Err(Error::TypeMismatch {
                expected: "unsigned integer",
                found: "non-scalar".to_owned(),
            }),
        }
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_f64(visitor)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.skip_to_content()?;
        let ev = self.next_event()?;
        match ev {
            Event::Scalar {
                ref value,
                style,
                ref tag,
                ..
            } => {
                if tag.is_some() {
                    return Err(self.fallback());
                }
                let resolved = self.resolve_scalar(value, style);
                match resolved {
                    Scalar::Float(f) => visitor.visit_f64(f),
                    Scalar::Int(n) => visitor.visit_f64(n as f64),
                    _ => Err(Error::TypeMismatch {
                        expected: "float",
                        found: "other scalar".to_owned(),
                    }),
                }
            }
            Event::Alias { .. } => Err(self.fallback()),
            _ => Err(Error::TypeMismatch {
                expected: "float",
                found: "non-scalar".to_owned(),
            }),
        }
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.skip_to_content()?;
        let ev = self.next_event()?;
        match ev {
            Event::Scalar {
                ref value, ref tag, ..
            } => {
                if tag.is_some() {
                    return Err(self.fallback());
                }
                let s: &str = value;
                if s.chars().count() == 1 {
                    visitor.visit_char(s.chars().next().expect("internal: count verified"))
                } else {
                    Err(Error::TypeMismatch {
                        expected: "char",
                        found: "string".to_owned(),
                    })
                }
            }
            Event::Alias { .. } => Err(self.fallback()),
            _ => Err(Error::TypeMismatch {
                expected: "char",
                found: "non-scalar".to_owned(),
            }),
        }
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.skip_to_content()?;
        let ev = self.next_event()?;
        match ev {
            Event::Scalar {
                ref value,
                style,
                ref tag,
                ..
            } => {
                if tag.is_some() {
                    return Err(self.fallback());
                }
                if self.raw_str_mode {
                    // Map key mode: return raw text without resolution.
                    return visitor.visit_str(value);
                }
                // Resolve first: only string-resolved scalars are valid.
                let resolved = self.resolve_scalar(value, style);
                match resolved {
                    Scalar::Str(s) => visitor.visit_str(&s),
                    _ => Err(Error::TypeMismatch {
                        expected: "string",
                        found: "non-string scalar".to_owned(),
                    }),
                }
            }
            Event::Alias { .. } => Err(self.fallback()),
            _ => Err(Error::TypeMismatch {
                expected: "string",
                found: "non-scalar".to_owned(),
            }),
        }
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.skip_to_content()?;
        let ev = self.next_event()?;
        match ev {
            Event::Scalar {
                ref value,
                style,
                ref tag,
                ..
            } => {
                if tag.is_some() {
                    return Err(self.fallback());
                }
                // Resolve first: only string-resolved scalars are valid for bytes.
                let resolved = self.resolve_scalar(value, style);
                match resolved {
                    Scalar::Str(s) => visitor.visit_bytes(s.as_bytes()),
                    _ => Err(Error::TypeMismatch {
                        expected: "bytes",
                        found: "non-string scalar".to_owned(),
                    }),
                }
            }
            Event::Alias { .. } => Err(self.fallback()),
            _ => Err(Error::TypeMismatch {
                expected: "bytes",
                found: "non-scalar".to_owned(),
            }),
        }
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_bytes(visitor)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.skip_to_content()?;
        let ev = self.peek_event()?;
        match ev {
            Event::Scalar {
                value, style, tag, ..
            } => {
                if tag.is_some() {
                    return Err(self.fallback());
                }
                // Check for null.
                if *style == ScalarStyle::Plain {
                    match &**value {
                        "" | "~" | "null" | "Null" | "NULL" => {
                            self.skip_event()?;
                            return visitor.visit_none();
                        }
                        _ => {}
                    }
                }
                visitor.visit_some(self)
            }
            // Aliases are resolved transparently in peek_event.
            Event::Alias { .. } => Err(self.fallback()),
            _ => visitor.visit_some(self),
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.skip_to_content()?;
        let ev = self.next_event()?;
        match ev {
            Event::Scalar {
                ref value,
                style,
                ref tag,
                ..
            } => {
                if tag.is_some() {
                    return Err(self.fallback());
                }
                let resolved = self.resolve_scalar(value, style);
                match resolved {
                    Scalar::Null => visitor.visit_unit(),
                    _ => Err(Error::TypeMismatch {
                        expected: "null",
                        found: "other scalar".to_owned(),
                    }),
                }
            }
            Event::Alias { .. } => Err(self.fallback()),
            _ => Err(Error::TypeMismatch {
                expected: "null",
                found: "non-scalar".to_owned(),
            }),
        }
    }

    fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.skip_to_content()?;
        let ev = self.next_event()?;
        match ev {
            Event::SequenceStart { tag, .. } => {
                if tag.is_some() {
                    return Err(self.fallback());
                }
                self.depth += 1;
                if self.depth > self.config.max_depth {
                    return Err(Error::RecursionLimitExceeded { depth: self.depth });
                }
                let result = visitor.visit_seq(StreamingSeqAccess {
                    de: self,
                    finished: false,
                })?;
                self.depth = self.depth.saturating_sub(1);
                Ok(result)
            }
            Event::Alias { .. } => Err(self.fallback()),
            _ => Err(Error::TypeMismatch {
                expected: "sequence",
                found: "non-sequence".to_owned(),
            }),
        }
    }

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.skip_to_content()?;
        let ev = self.next_event()?;
        match ev {
            Event::MappingStart { tag, .. } => {
                if tag.is_some() {
                    return Err(self.fallback());
                }
                self.depth += 1;
                if self.depth > self.config.max_depth {
                    return Err(Error::RecursionLimitExceeded { depth: self.depth });
                }
                let result = visitor.visit_map(StreamingMapAccess {
                    de: self,
                    finished: false,
                    has_emitted_key: false,
                })?;
                self.depth = self.depth.saturating_sub(1);
                Ok(result)
            }
            Event::Alias { .. } => Err(self.fallback()),
            _ => Err(Error::TypeMismatch {
                expected: "mapping",
                found: "non-mapping".to_owned(),
            }),
        }
    }

    fn deserialize_struct<V>(
        self,
        name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // Spanned<T> requires the Value-based path with span context.
        if name == crate::spanned::SPANNED_TYPE_NAME {
            return Err(self.fallback());
        }
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.skip_to_content()?;
        let ev = self.peek_event()?;
        match ev {
            Event::Scalar { tag, .. } => {
                if tag.is_some() {
                    return Err(self.fallback());
                }
                // Unit variant: scalar = variant name.
                let ev = self.next_event()?;
                if let Event::Scalar { value, .. } = ev {
                    visitor.visit_enum(value.into_owned().into_deserializer())
                } else {
                    unreachable!()
                }
            }
            Event::MappingStart { tag, .. } => {
                if tag.is_some() {
                    return Err(self.fallback());
                }
                // Newtype/struct/tuple variant: single-entry mapping.
                self.skip_event()?; // consume MappingStart
                let variant_ev = self.next_event()?;
                let variant_name = match variant_ev {
                    Event::Scalar { value, .. } => value.into_owned(),
                    _ => {
                        return Err(Error::TypeMismatch {
                            expected: "string variant name",
                            found: "non-scalar".to_owned(),
                        });
                    }
                };
                visitor.visit_enum(StreamingEnumAccess {
                    de: self,
                    variant: variant_name,
                })
            }
            // Aliases are resolved transparently in peek_event.
            Event::Alias { .. } => Err(self.fallback()),
            _ => Err(Error::TypeMismatch {
                expected: "enum",
                found: "other".to_owned(),
            }),
        }
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // Identifiers (field names, map keys) should return the raw text
        // without YAML type resolution — `true`, `42`, etc. are valid
        // identifier strings.
        self.skip_to_content()?;
        let ev = self.next_event()?;
        match ev {
            Event::Scalar { value, tag, .. } => {
                if tag.is_some() {
                    return Err(self.fallback());
                }
                visitor.visit_str(&value)
            }
            Event::Alias { .. } => Err(self.fallback()),
            _ => Err(Error::TypeMismatch {
                expected: "identifier",
                found: "non-scalar".to_owned(),
            }),
        }
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.skip_to_content()?;
        self.skip_value()?;
        visitor.visit_unit()
    }
}

// ── SeqAccess ───────────────────────────────────────────────────────────

struct StreamingSeqAccess<'a, 'de> {
    de: &'a mut StreamingDeserializer<'de>,
    finished: bool,
}

impl<'de> SeqAccess<'de> for StreamingSeqAccess<'_, 'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: DeserializeSeed<'de>,
    {
        let ev = self.de.peek_event()?;
        if matches!(ev, Event::SequenceEnd { .. }) {
            self.de.skip_event()?;
            self.finished = true;
            return Ok(None);
        }
        seed.deserialize(&mut *self.de).map(Some)
    }
}

impl Drop for StreamingSeqAccess<'_, '_> {
    fn drop(&mut self) {
        if !self.finished {
            // The visitor returned early (e.g., fixed-length tuple).
            // Drain remaining elements and consume SequenceEnd.
            loop {
                match self.de.peek_event() {
                    Ok(Event::SequenceEnd { .. }) => {
                        let _ = self.de.skip_event();
                        break;
                    }
                    Ok(_) => {
                        if self.de.skip_value().is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        }
    }
}

// ── MapAccess ───────────────────────────────────────────────────────────

struct StreamingMapAccess<'a, 'de> {
    de: &'a mut StreamingDeserializer<'de>,
    finished: bool,
    /// Whether this access has surfaced at least one key to the visitor.
    /// Used to decide whether a `<<:` merge can be expanded natively:
    /// native expansion relies on serde's last-wins insertion to honour
    /// "local keys override merged keys", which is only sound when all
    /// local keys follow the merge. If a local key preceded `<<` we fall
    /// back to the AST path so override semantics stay correct.
    has_emitted_key: bool,
}

impl<'de> MapAccess<'de> for StreamingMapAccess<'_, 'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: DeserializeSeed<'de>,
    {
        loop {
            let ev = self.de.peek_event()?;
            if matches!(ev, Event::MappingEnd { .. }) {
                self.de.skip_event()?;
                self.finished = true;
                return Ok(None);
            }
            // Detect merge keys (`<<`). Handle the common `<<: *anchor`
            // pattern natively when it is the first key of this mapping;
            // anything else (sequence merge, non-first position) falls back
            // to the AST path so correctness is preserved.
            if let Event::Scalar {
                value,
                style: ScalarStyle::Plain,
                ..
            } = ev
            {
                if value.as_ref() == "<<" {
                    if self.has_emitted_key {
                        // Locals already emitted — serde's last-wins order
                        // would let merged values override them, violating
                        // YAML "local > merged" semantics. Fall back.
                        return Err(self.de.fallback());
                    }
                    // Consume the `<<` key event and peek the value.
                    self.de.skip_event()?;
                    let next_ev = self.de.peek_event()?;
                    let (anchor_name, alias_start) = if let Event::Alias { anchor, span } = next_ev
                    {
                        (anchor.clone(), span.start)
                    } else {
                        // <<: sequence or inline mapping — fall back.
                        return Err(self.de.fallback());
                    };
                    // Consume the alias event and inject the merge target's
                    // inner events onto the replay stack.
                    self.de.skip_event()?;
                    self.de
                        .inject_merge_mapping_contents(&anchor_name, alias_start)?;
                    // Loop back to pick up the first merged key (or a
                    // post-merge local key if the merged mapping was empty).
                    continue;
                }
            }
            // Enable raw string mode for key deserialization so that
            // non-string scalars (booleans, numbers, null) are passed
            // through as their textual representation, matching the
            // Value-based path's `value_into_key` behavior.
            self.de.raw_str_mode = true;
            let result = seed.deserialize(&mut *self.de).map(Some);
            self.de.raw_str_mode = false;
            if result.is_ok() {
                self.has_emitted_key = true;
            }
            return result;
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: DeserializeSeed<'de>,
    {
        seed.deserialize(&mut *self.de)
    }
}

impl Drop for StreamingMapAccess<'_, '_> {
    fn drop(&mut self) {
        if !self.finished {
            // The visitor returned early. Drain remaining key-value pairs
            // and consume MappingEnd.
            loop {
                match self.de.peek_event() {
                    Ok(Event::MappingEnd { .. }) => {
                        let _ = self.de.skip_event();
                        break;
                    }
                    Ok(_) => {
                        // Skip key.
                        if self.de.skip_value().is_err() {
                            break;
                        }
                        // Skip value.
                        if self.de.skip_value().is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        }
    }
}

// ── EnumAccess ──────────────────────────────────────────────────────────

struct StreamingEnumAccess<'a, 'de> {
    de: &'a mut StreamingDeserializer<'de>,
    variant: String,
}

impl<'a, 'de> de::EnumAccess<'de> for StreamingEnumAccess<'a, 'de> {
    type Error = Error;
    type Variant = StreamingVariantAccess<'a, 'de>;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
    where
        V: DeserializeSeed<'de>,
    {
        use serde::de::value::StringDeserializer;
        let deserializer: StringDeserializer<Error> = self.variant.into_deserializer();
        let variant = seed.deserialize(deserializer)?;
        Ok((variant, StreamingVariantAccess { de: self.de }))
    }
}

struct StreamingVariantAccess<'a, 'de> {
    de: &'a mut StreamingDeserializer<'de>,
}

impl<'de> de::VariantAccess<'de> for StreamingVariantAccess<'_, 'de> {
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        // Consume the MappingEnd for the single-entry enum mapping.
        let ev = self.de.next_event()?;
        if !matches!(ev, Event::MappingEnd { .. }) {
            // The value after the key should be consumed; if it's not a MappingEnd
            // that means there's a value we need to skip, then consume MappingEnd.
            // Actually for unit variants in YAML like `{Variant: null}`, the value
            // is an empty scalar. We need to handle that.
            // Re-think: for `{Variant: ~}`, events are:
            //   MappingStart, Scalar("Variant"), Scalar("~"), MappingEnd
            // For unit_variant, serde expects us to handle just the variant value.
            // We already consumed MappingStart and Scalar("Variant") in deserialize_enum.
            // So here we need to:
            //   1. Consume the value (Scalar("~") or empty) via deserialize_any or skip
            //   2. Consume MappingEnd
            // But wait - `ev` is what we just got. If it's not MappingEnd, it's the value.
            // Let's reconsider: for a unit variant `Variant` as a plain scalar,
            // deserialize_enum already returns before we get here. This path is only
            // for the mapping case `{Variant: ...}`.

            // We got the value event instead of MappingEnd. We need to skip it
            // (put it back) and then skip the value, then get MappingEnd.
            self.de.current = Some(ev);
            self.de.skip_value()?;
            // Now consume MappingEnd
            let end_ev = self.de.next_event()?;
            if !matches!(end_ev, Event::MappingEnd { .. }) {
                return Err(Error::Invalid(
                    "expected mapping end after enum variant".to_owned(),
                ));
            }
        }
        Ok(())
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
    where
        T: DeserializeSeed<'de>,
    {
        let result = seed.deserialize(&mut *self.de)?;
        // Consume MappingEnd
        let ev = self.de.next_event()?;
        if !matches!(ev, Event::MappingEnd { .. }) {
            return Err(Error::Invalid(
                "expected mapping end after enum variant value".to_owned(),
            ));
        }
        Ok(result)
    }

    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let result = de::Deserializer::deserialize_seq(&mut *self.de, visitor)?;
        // Consume MappingEnd
        let ev = self.de.next_event()?;
        if !matches!(ev, Event::MappingEnd { .. }) {
            return Err(Error::Invalid(
                "expected mapping end after enum tuple variant".to_owned(),
            ));
        }
        Ok(result)
    }

    fn struct_variant<V>(self, _fields: &'static [&'static str], visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let result = de::Deserializer::deserialize_map(&mut *self.de, visitor)?;
        // Consume MappingEnd
        let ev = self.de.next_event()?;
        if !matches!(ev, Event::MappingEnd { .. }) {
            return Err(Error::Invalid(
                "expected mapping end after enum struct variant".to_owned(),
            ));
        }
        Ok(result)
    }
}

// ── Public entry point ──────────────────────────────────────────────────

/// Attempt streaming deserialization. If the input contains features that
/// require the Value-based path (tags, merge keys, `Spanned<T>`),
/// returns `None` so the caller can fall back. Anchors and aliases are
/// handled natively via event buffering and replay.
pub(crate) fn from_str_streaming<T>(s: &str, config: &ParseConfig) -> Option<Result<T>>
where
    T: for<'de> Deserialize<'de>,
{
    if s.len() > config.max_document_length {
        return Some(Err(Error::Parse(format!(
            "document exceeds maximum length of {} bytes",
            config.max_document_length
        ))));
    }

    let mut de = StreamingDeserializer::new(s, config);
    let result = T::deserialize(&mut de);

    match result {
        Ok(val) => {
            // Drain remaining events (DocumentEnd, StreamEnd).
            // If there are multiple documents, fall back.
            loop {
                match de.next_event() {
                    Ok(Event::DocumentEnd | Event::StreamEnd) => {}
                    Ok(Event::StreamStart) => {
                        // Already handled during deserialization.
                    }
                    Err(_) => break,
                    Ok(_) => {
                        // Extra events — could be a multi-doc stream or
                        // leftover content. This is fine for the common case.
                        break;
                    }
                }
            }
            Some(Ok(val))
        }
        Err(ref e) => {
            // Check if this is a fallback signal.
            if is_fallback_error(e) {
                None
            } else {
                Some(result)
            }
        }
    }
}

/// Check if an error is the fallback sentinel.
fn is_fallback_error(e: &Error) -> bool {
    match e {
        Error::Custom(msg) => msg == FALLBACK_SENTINEL,
        _ => false,
    }
}
