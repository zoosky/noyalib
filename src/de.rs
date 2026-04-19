//! YAML deserialization.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use std::io::Read;

use serde::de::{self, DeserializeSeed, IntoDeserializer, MapAccess, SeqAccess, Visitor};
use serde::Deserialize;

use crate::error::{Error, Location, Result};
use crate::value::{Mapping, Number, Sequence, Value};
use crate::{parser, span_context, spanned};

/// Policy for handling duplicate keys in YAML mappings.
///
/// By default, the last occurrence of a duplicate key wins (compatible with
/// most YAML parsers). You can make this stricter for untrusted input.
///
/// # Example
///
/// ```rust
/// use noyalib::{
///     from_str_with_config, DuplicateKeyPolicy, ParserConfig, Value,
/// };
///
/// let config =
///     ParserConfig::new().duplicate_key_policy(DuplicateKeyPolicy::Error);
///
/// let yaml = "a: 1\na: 2";
/// let result: Result<Value, _> = from_str_with_config(yaml, &config);
/// assert!(result.is_err());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum DuplicateKeyPolicy {
    /// Reject duplicate keys with an error (strictest).
    Error,
    /// First occurrence wins; later duplicates are silently ignored.
    First,
    /// Last occurrence wins (default, backwards-compatible).
    #[default]
    Last,
}

/// Configuration options for YAML parsing with security limits.
///
/// This struct provides options to limit resource consumption during parsing,
/// protecting against denial-of-service attacks such as the "billion laughs"
/// attack or deeply nested structures.
///
/// # Example
///
/// ```rust
/// use noyalib::{from_str_with_config, ParserConfig, Value};
///
/// let config = ParserConfig::new()
///     .max_depth(50)
///     .max_document_length(1_000_000);
///
/// let yaml = "key: value";
/// let value: Value = from_str_with_config(yaml, &config).unwrap();
/// ```
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct ParserConfig {
    /// Maximum nesting depth allowed (default: 128).
    pub max_depth: usize,
    /// Maximum document length in bytes (default: 64MB).
    pub max_document_length: usize,
    /// Maximum number of alias expansions (default: 1024).
    ///
    /// The "billion laughs" attack uses anchors and aliases to create
    /// exponentially large outputs. This limit protects against such
    /// attacks by counting each alias resolution during loading.
    pub max_alias_expansions: usize,
    /// Maximum number of keys in a single mapping (default: 65536).
    pub max_mapping_keys: usize,
    /// Maximum length of a single sequence (default: 65536).
    pub max_sequence_length: usize,
    /// How to handle duplicate keys in mappings (default: Last).
    pub duplicate_key_policy: DuplicateKeyPolicy,
    /// YAML 1.2 strict boolean mode (default: false).
    ///
    /// When `true`, only exact `"true"` and `"false"` (lowercase) resolve to
    /// booleans. Case variants like `"True"`, `"TRUE"`, `"False"`, `"FALSE"`
    /// are treated as strings. This matches the YAML 1.2 JSON Schema.
    pub strict_booleans: bool,
}

impl Default for ParserConfig {
    fn default() -> Self {
        Self {
            max_depth: 128,
            max_document_length: 64 * 1024 * 1024, // 64MB
            max_alias_expansions: 1024,
            max_mapping_keys: 65536,
            max_sequence_length: 65536,
            duplicate_key_policy: DuplicateKeyPolicy::default(),
            strict_booleans: false,
        }
    }
}

impl ParserConfig {
    /// Create a new parser configuration with default settings.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the maximum nesting depth.
    ///
    /// Deeply nested structures can cause stack overflow. This limit
    /// protects against such attacks.
    #[must_use]
    pub fn max_depth(mut self, depth: usize) -> Self {
        self.max_depth = depth;
        self
    }

    /// Set the maximum document length in bytes.
    ///
    /// Very large documents can cause memory exhaustion. This limit
    /// provides protection against such scenarios.
    #[must_use]
    pub fn max_document_length(mut self, length: usize) -> Self {
        self.max_document_length = length;
        self
    }

    /// Set the maximum number of alias expansions.
    ///
    /// The "billion laughs" attack uses anchors and aliases to create
    /// exponentially large outputs. This limit is enforced during loading
    /// by counting each alias resolution.
    #[must_use]
    pub fn max_alias_expansions(mut self, count: usize) -> Self {
        self.max_alias_expansions = count;
        self
    }

    /// Set the maximum number of keys in a mapping.
    #[must_use]
    pub fn max_mapping_keys(mut self, count: usize) -> Self {
        self.max_mapping_keys = count;
        self
    }

    /// Set the maximum length of a sequence.
    #[must_use]
    pub fn max_sequence_length(mut self, count: usize) -> Self {
        self.max_sequence_length = count;
        self
    }

    /// Create a strict configuration with lower limits.
    ///
    /// Useful for parsing untrusted input. Enables strict booleans
    /// (only `"true"` / `"false"`, not case variants).
    #[must_use]
    pub fn strict() -> Self {
        Self {
            max_depth: 32,
            max_document_length: 1024 * 1024, // 1MB
            max_alias_expansions: 64,
            max_mapping_keys: 1024,
            max_sequence_length: 1024,
            duplicate_key_policy: DuplicateKeyPolicy::Error,
            strict_booleans: true,
        }
    }

    /// Enable or disable strict boolean mode.
    ///
    /// When enabled, only exact `"true"` and `"false"` (lowercase) resolve
    /// to booleans. Case variants are treated as strings.
    #[must_use]
    pub fn strict_booleans(mut self, enabled: bool) -> Self {
        self.strict_booleans = enabled;
        self
    }

    /// Set the duplicate key handling policy.
    #[must_use]
    pub fn duplicate_key_policy(mut self, policy: DuplicateKeyPolicy) -> Self {
        self.duplicate_key_policy = policy;
        self
    }
}

/// Deserialize a YAML string into a Rust type.
///
/// # Errors
///
/// Returns an error if the YAML is invalid or cannot be deserialized
/// into the target type.
///
/// # Example
///
/// ```rust
/// use serde::Deserialize;
///
/// #[derive(Debug, Deserialize)]
/// struct Config {
///     name: String,
///     port: u16,
/// }
///
/// let yaml = "name: myapp\nport: 8080\n";
/// let config: Config = noyalib::from_str(yaml).unwrap();
/// assert_eq!(config.name, "myapp");
/// assert_eq!(config.port, 8080);
/// ```
pub fn from_str<T>(s: &str) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    let config = ParserConfig::default();
    let parse_config = parser::ParseConfig::from(&config);
    // Try the streaming path first — zero intermediate allocations.
    if let Some(result) = crate::streaming::from_str_streaming(s, &parse_config) {
        return result;
    }
    // Fallback: Value-based path (handles anchors, aliases, tags, Spanned<T>).
    from_str_with_config(s, &config)
}

/// Deserialize a YAML string into a Rust type with custom configuration.
///
/// This function allows specifying security limits for parsing untrusted input.
///
/// # Errors
///
/// Returns an error if the YAML is invalid, exceeds configured limits,
/// or cannot be deserialized into the target type.
///
/// # Example
///
/// ```rust
/// use noyalib::{ParserConfig, from_str_with_config, Value};
///
/// let config = ParserConfig::strict(); // Use strict limits for untrusted input
/// let yaml = "key: value";
/// let value: Value = from_str_with_config(yaml, &config).unwrap();
/// ```
pub fn from_str_with_config<T>(s: &str, config: &ParserConfig) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    let parse_config = parser::ParseConfig::from(config);
    let (value, span_tree) = parser::parse_one(s, &parse_config)?;
    let spans = span_context::build_span_map(&value, &span_tree);
    let ctx = span_context::SpanContext {
        spans,
        source: s.into(),
    };
    let _guard = span_context::set_span_context(ctx);
    T::deserialize(Deserializer::new(&value))
}

/// Deserialize a YAML byte slice into a Rust type.
///
/// # Errors
///
/// Returns an error if the bytes are not valid UTF-8, the YAML is invalid,
/// or the data cannot be deserialized into the target type.
pub fn from_slice<T>(slice: &[u8]) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    let s = std::str::from_utf8(slice).map_err(|e| Error::Parse(e.to_string()))?;
    let parse_config = parser::ParseConfig::from(&ParserConfig::default());
    // Try the streaming path first.
    if let Some(result) = crate::streaming::from_str_streaming(s, &parse_config) {
        return result;
    }
    // Fallback: Value-based path.
    let value = parser::parse_one_value(s, &parse_config)?;
    T::deserialize(Deserializer::new(&value))
}

/// Deserialize a YAML byte slice with custom security limits.
///
/// # Errors
///
/// Returns an error if the bytes are not valid UTF-8, security limits
/// are exceeded, or the data cannot be deserialized.
pub fn from_slice_with_config<T>(slice: &[u8], config: &ParserConfig) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    let s = std::str::from_utf8(slice).map_err(|e| Error::Parse(e.to_string()))?;
    from_str_with_config(s, config)
}

/// Deserialize YAML from a reader into a Rust type.
///
/// # Errors
///
/// Returns an error if reading fails, the YAML is invalid,
/// or the data cannot be deserialized into the target type.
pub fn from_reader<T, R>(mut reader: R) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
    R: Read,
{
    let mut s = String::new();
    let _ = reader.read_to_string(&mut s)?;
    let parse_config = parser::ParseConfig::from(&ParserConfig::default());
    // Try the streaming path first.
    if let Some(result) = crate::streaming::from_str_streaming(&s, &parse_config) {
        return result;
    }
    // Fallback: Value-based path.
    let value = parser::parse_one_value(&s, &parse_config)?;
    T::deserialize(Deserializer::new(&value))
}

/// Deserialize YAML from a reader with custom security limits.
///
/// This function reads the entire input into memory, then parses it using
/// the provided configuration limits to prevent denial-of-service attacks.
///
/// # Errors
///
/// Returns an error if reading fails, security limits are exceeded,
/// or the data cannot be deserialized.
///
/// # Example
///
/// ```rust
/// use std::io::Cursor;
///
/// use noyalib::{from_reader_with_config, ParserConfig, Value};
///
/// let yaml = "key: value\nnumber: 42";
/// let reader = Cursor::new(yaml);
/// let config = ParserConfig::strict();
///
/// let value: Value = from_reader_with_config(reader, &config).unwrap();
/// ```
pub fn from_reader_with_config<T, R>(reader: R, config: &ParserConfig) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
    R: Read,
{
    // Read with size limit
    let mut buffer = Vec::new();
    let bytes_read = reader
        .take(config.max_document_length as u64 + 1)
        .read_to_end(&mut buffer)?;

    if bytes_read > config.max_document_length {
        return Err(Error::Parse(format!(
            "document exceeds maximum length of {} bytes",
            config.max_document_length
        )));
    }

    let s = std::str::from_utf8(&buffer).map_err(|e| Error::Parse(e.to_string()))?;
    from_str_with_config(s, config)
}

/// Deserialize a `Value` into a Rust type.
///
/// # Errors
///
/// Returns an error if the value cannot be deserialized into the target type.
pub fn from_value<T>(value: &Value) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    T::deserialize(Deserializer::new(value))
}

/// A YAML deserializer.
#[derive(Debug)]
pub struct Deserializer<'de> {
    value: &'de Value,
}

impl<'de> Deserializer<'de> {
    /// Create a new deserializer from a value.
    #[must_use]
    pub fn new(value: &'de Value) -> Self {
        Deserializer { value }
    }
}

impl<'de> de::Deserializer<'de> for Deserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::Null => visitor.visit_none(),
            Value::Bool(b) => visitor.visit_bool(*b),
            Value::Number(Number::Integer(n)) => visitor.visit_i64(*n),
            Value::Number(Number::Float(n)) => visitor.visit_f64(*n),
            Value::String(s) => visitor.visit_str(s),
            Value::Sequence(_) => self.deserialize_seq(visitor),
            Value::Mapping(_) => self.deserialize_map(visitor),
            Value::Tagged(tagged) => {
                // Deserialize the inner value
                Deserializer::new(tagged.value()).deserialize_any(visitor)
            }
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::Bool(b) => visitor.visit_bool(*b),
            _ => Err(Error::TypeMismatch {
                expected: "bool",
                found: type_name(self.value),
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
        match self.value {
            Value::Number(Number::Integer(n)) => visitor.visit_i64(*n),
            Value::Number(Number::Float(n))
                if n.fract() == 0.0
                    && *n >= i64::MIN as f64
                    && *n <= i64::MAX as f64
                    && !n.is_nan() =>
            {
                visitor.visit_i64(*n as i64)
            }
            _ => Err(Error::TypeMismatch {
                expected: "integer",
                found: type_name(self.value),
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
        match self.value {
            Value::Number(Number::Integer(n)) if *n >= 0 => visitor.visit_u64(*n as u64),
            Value::Number(Number::Float(n))
                if n.fract() == 0.0 && *n >= 0.0 && *n <= u64::MAX as f64 && !n.is_nan() =>
            {
                visitor.visit_u64(*n as u64)
            }
            _ => Err(Error::TypeMismatch {
                expected: "unsigned integer",
                found: type_name(self.value),
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
        match self.value {
            Value::Number(Number::Float(n)) => visitor.visit_f64(*n),
            Value::Number(Number::Integer(n)) => visitor.visit_f64(*n as f64),
            _ => Err(Error::TypeMismatch {
                expected: "float",
                found: type_name(self.value),
            }),
        }
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::String(s) if s.chars().count() == 1 => {
                // SAFETY: count() == 1 guarantees next() returns Some.
                visitor.visit_char(s.chars().next().expect("internal: count verified"))
            }
            _ => Err(Error::TypeMismatch {
                expected: "char",
                found: type_name(self.value),
            }),
        }
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::String(s) => visitor.visit_str(s),
            _ => Err(Error::TypeMismatch {
                expected: "string",
                found: type_name(self.value),
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
        match self.value {
            Value::String(s) => visitor.visit_bytes(s.as_bytes()),
            _ => Err(Error::TypeMismatch {
                expected: "bytes",
                found: type_name(self.value),
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
        match self.value {
            Value::Null => visitor.visit_none(),
            _ => visitor.visit_some(self),
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::Null => visitor.visit_unit(),
            _ => Err(Error::TypeMismatch {
                expected: "null",
                found: type_name(self.value),
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
        match self.value {
            Value::Sequence(seq) => visitor.visit_seq(SeqDeserializer::new(seq)),
            _ => Err(Error::TypeMismatch {
                expected: "sequence",
                found: type_name(self.value),
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
        match self.value {
            Value::Mapping(map) => visitor.visit_map(MapDeserializer::new(map)),
            _ => Err(Error::TypeMismatch {
                expected: "mapping",
                found: type_name(self.value),
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
        if name == spanned::SPANNED_TYPE_NAME {
            let p: *const Value = self.value;
            let ptr = p as usize;
            let (start, end) = span_context::lookup_span(ptr)
                .unwrap_or((Location::default(), Location::default()));
            return visitor.visit_map(SpannedMapAccess::new(start, end, self.value));
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
        match self.value {
            Value::String(s) => visitor.visit_enum(s.as_str().into_deserializer()),
            Value::Mapping(map) if map.len() == 1 => {
                // SAFETY: len() == 1 guarantees next() returns Some.
                let (key, value) = map.iter().next().expect("internal: len verified");
                visitor.visit_enum(EnumDeserializer {
                    variant: key,
                    value,
                })
            }
            _ => Err(Error::TypeMismatch {
                expected: "enum",
                found: type_name(self.value),
            }),
        }
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }
}

struct SeqDeserializer<'de> {
    iter: std::slice::Iter<'de, Value>,
}

impl<'de> SeqDeserializer<'de> {
    fn new(seq: &'de Sequence) -> Self {
        SeqDeserializer { iter: seq.iter() }
    }
}

impl<'de> SeqAccess<'de> for SeqDeserializer<'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some(value) => seed.deserialize(Deserializer::new(value)).map(Some),
            None => Ok(None),
        }
    }
}

struct MapDeserializer<'de> {
    iter: indexmap::map::Iter<'de, String, Value>,
    value: Option<&'de Value>,
}

impl<'de> MapDeserializer<'de> {
    fn new(map: &'de Mapping) -> Self {
        MapDeserializer {
            iter: map.iter(),
            value: None,
        }
    }
}

impl<'de> MapAccess<'de> for MapDeserializer<'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some((key, value)) => {
                self.value = Some(value);
                seed.deserialize(key.as_str().into_deserializer()).map(Some)
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: DeserializeSeed<'de>,
    {
        match self.value.take() {
            Some(value) => seed.deserialize(Deserializer::new(value)),
            None => Err(Error::Invalid("missing value in map".to_string())),
        }
    }
}

/// Feeds the virtual `Spanned<T>` fields (start_line, start_column, …, value)
/// to the `SpannedVisitor`.
pub(crate) struct SpannedMapAccess<'de> {
    start: Location,
    end: Location,
    value: &'de Value,
    state: SpannedFieldState,
}

#[derive(Debug, Clone, Copy)]
enum SpannedFieldState {
    StartLine,
    StartColumn,
    StartIndex,
    EndLine,
    EndColumn,
    EndIndex,
    Value,
    Done,
}

impl<'de> SpannedMapAccess<'de> {
    pub(crate) fn new(start: Location, end: Location, value: &'de Value) -> Self {
        SpannedMapAccess {
            start,
            end,
            value,
            state: SpannedFieldState::StartLine,
        }
    }
}

impl<'de> MapAccess<'de> for SpannedMapAccess<'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: DeserializeSeed<'de>,
    {
        let field = match self.state {
            SpannedFieldState::StartLine => spanned::SPANNED_FIELD_START_LINE,
            SpannedFieldState::StartColumn => spanned::SPANNED_FIELD_START_COLUMN,
            SpannedFieldState::StartIndex => spanned::SPANNED_FIELD_START_INDEX,
            SpannedFieldState::EndLine => spanned::SPANNED_FIELD_END_LINE,
            SpannedFieldState::EndColumn => spanned::SPANNED_FIELD_END_COLUMN,
            SpannedFieldState::EndIndex => spanned::SPANNED_FIELD_END_INDEX,
            SpannedFieldState::Value => spanned::SPANNED_FIELD_VALUE,
            SpannedFieldState::Done => return Ok(None),
        };
        seed.deserialize(de::value::BorrowedStrDeserializer::new(field))
            .map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: DeserializeSeed<'de>,
    {
        match self.state {
            SpannedFieldState::StartLine => {
                self.state = SpannedFieldState::StartColumn;
                seed.deserialize(self.start.line().into_deserializer())
            }
            SpannedFieldState::StartColumn => {
                self.state = SpannedFieldState::StartIndex;
                seed.deserialize(self.start.column().into_deserializer())
            }
            SpannedFieldState::StartIndex => {
                self.state = SpannedFieldState::EndLine;
                seed.deserialize(self.start.index().into_deserializer())
            }
            SpannedFieldState::EndLine => {
                self.state = SpannedFieldState::EndColumn;
                seed.deserialize(self.end.line().into_deserializer())
            }
            SpannedFieldState::EndColumn => {
                self.state = SpannedFieldState::EndIndex;
                seed.deserialize(self.end.column().into_deserializer())
            }
            SpannedFieldState::EndIndex => {
                self.state = SpannedFieldState::Value;
                seed.deserialize(self.end.index().into_deserializer())
            }
            SpannedFieldState::Value => {
                self.state = SpannedFieldState::Done;
                seed.deserialize(Deserializer::new(self.value))
            }
            SpannedFieldState::Done => Err(Error::Invalid("no more fields".to_string())),
        }
    }
}

struct EnumDeserializer<'de> {
    variant: &'de str,
    value: &'de Value,
}

impl<'de> de::EnumAccess<'de> for EnumDeserializer<'de> {
    type Error = Error;
    type Variant = VariantDeserializer<'de>;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
    where
        V: DeserializeSeed<'de>,
    {
        use serde::de::value::StrDeserializer;
        let deserializer: StrDeserializer<'_, Error> = self.variant.into_deserializer();
        let variant = seed.deserialize(deserializer)?;
        Ok((variant, VariantDeserializer { value: self.value }))
    }
}

struct VariantDeserializer<'de> {
    value: &'de Value,
}

impl<'de> de::VariantAccess<'de> for VariantDeserializer<'de> {
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        Ok(())
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
    where
        T: DeserializeSeed<'de>,
    {
        seed.deserialize(Deserializer::new(self.value))
    }

    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        de::Deserializer::deserialize_seq(Deserializer::new(self.value), visitor)
    }

    fn struct_variant<V>(self, _fields: &'static [&'static str], visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        de::Deserializer::deserialize_map(Deserializer::new(self.value), visitor)
    }
}

fn type_name(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(_) => "boolean".to_string(),
        Value::Number(Number::Integer(_)) => "integer".to_string(),
        Value::Number(Number::Float(_)) => "float".to_string(),
        Value::String(_) => "string".to_string(),
        Value::Sequence(_) => "sequence".to_string(),
        Value::Mapping(_) => "mapping".to_string(),
        Value::Tagged(t) => format!("tagged({})", t.tag()),
    }
}
