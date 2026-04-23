// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! YAML Deserialization.

use crate::error::{Error, Result};
use crate::parser::{self};
use crate::span_context;
use crate::value::{Number, Value};
use serde::de::{self, DeserializeSeed, IntoDeserializer, MapAccess, SeqAccess, Visitor};
use serde::Deserialize;
use std::io;

/// Deserialization configuration.
#[derive(Debug, Clone, Copy)]
pub struct ParserConfig {
    /// Maximum recursion depth allowed during parsing (default: 128).
    pub max_depth: usize,
    /// Maximum length of a single YAML document in bytes (default: 64 MB).
    pub max_document_length: usize,
    /// Maximum number of times a single anchor can be expanded (default: 1024).
    pub max_alias_expansions: usize,
    /// Maximum number of keys allowed in a single mapping (default: 64k).
    pub max_mapping_keys: usize,
    /// Maximum number of elements allowed in a single sequence (default: 64k).
    pub max_sequence_length: usize,
    /// How to handle duplicate keys in a mapping (default: Last, per YAML 1.2).
    pub duplicate_key_policy: DuplicateKeyPolicy,
    /// If true, only `true` and `false` (lowercase) are accepted as booleans.
    pub strict_booleans: bool,
    /// If true, accepts YAML 1.1 booleans like `yes`, `no`, `on`, `off`.
    pub legacy_booleans: bool,
}

impl Default for ParserConfig {
    fn default() -> Self {
        ParserConfig {
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

impl ParserConfig {
    /// Create a new configuration with default values.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a strict configuration (YAML 1.2 strict) with tighter
    /// security limits suitable for untrusted input.
    #[must_use]
    pub fn strict() -> Self {
        ParserConfig {
            max_depth: 64,
            max_document_length: 1024 * 1024, // 1 MB
            max_alias_expansions: 100,
            max_mapping_keys: 1024,
            max_sequence_length: 1024,
            strict_booleans: true,
            legacy_booleans: false,
            duplicate_key_policy: DuplicateKeyPolicy::Error,
        }
    }

    /// Set the maximum recursion depth.
    #[must_use]
    pub fn max_depth(mut self, depth: usize) -> Self {
        self.max_depth = depth;
        self
    }

    /// Set the maximum document length.
    #[must_use]
    pub fn max_document_length(mut self, len: usize) -> Self {
        self.max_document_length = len;
        self
    }

    /// Set the maximum alias expansions.
    #[must_use]
    pub fn max_alias_expansions(mut self, expansions: usize) -> Self {
        self.max_alias_expansions = expansions;
        self
    }

    /// Set the maximum number of mapping keys.
    #[must_use]
    pub fn max_mapping_keys(mut self, max: usize) -> Self {
        self.max_mapping_keys = max;
        self
    }

    /// Set the maximum sequence length.
    #[must_use]
    pub fn max_sequence_length(mut self, max: usize) -> Self {
        self.max_sequence_length = max;
        self
    }

    /// Set the duplicate key policy.
    #[must_use]
    pub fn duplicate_key_policy(mut self, policy: DuplicateKeyPolicy) -> Self {
        self.duplicate_key_policy = policy;
        self
    }

    /// Enable or disable strict booleans.
    #[must_use]
    pub fn strict_booleans(mut self, strict: bool) -> Self {
        self.strict_booleans = strict;
        self
    }

    /// Enable or disable legacy booleans.
    #[must_use]
    pub fn legacy_booleans(mut self, legacy: bool) -> Self {
        self.legacy_booleans = legacy;
        self
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

/// Deserialize YAML from a string.
pub fn from_str<T>(s: &str) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    from_str_with_config(s, &ParserConfig::default())
}

/// Deserialize YAML from a string with custom security limits.
pub fn from_str_with_config<T>(s: &str, config: &ParserConfig) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    // Try streaming path first (faster, no intermediate Value AST).
    if let Some(res) = crate::streaming::from_str_streaming(s, &config.into()) {
        return res;
    }

    let parse_config = parser::ParseConfig::from(config);

    #[cfg(feature = "std")]
    {
        let (value, span_tree) = parser::parse_one(s, &parse_config)?;
        let spans = span_context::build_span_map(&value, &span_tree);
        let ctx = span_context::SpanContext {
            spans,
            source: s.into(),
        };
        let _guard = span_context::set_span_context(ctx);
        let de = Deserializer {
            value: &value,
            span_ctx: Some(_guard.as_ref()),
        };
        T::deserialize(de)
    }

    #[cfg(not(feature = "std"))]
    {
        let value = parser::parse_one_value(s, &parse_config)?;
        T::deserialize(Deserializer::new(&value))
    }
}

/// Deserialize YAML from a byte slice.
pub fn from_slice<T>(b: &[u8]) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    let s = std::str::from_utf8(b).map_err(|e| Error::Deserialize(e.to_string()))?;
    from_str(s)
}

/// Deserialize YAML from a byte slice with custom configuration.
pub fn from_slice_with_config<T>(b: &[u8], config: &ParserConfig) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    let s = std::str::from_utf8(b).map_err(|e| Error::Deserialize(e.to_string()))?;
    from_str_with_config(s, config)
}

/// Deserialize YAML from an IO reader.
pub fn from_reader<R, T>(reader: R) -> Result<T>
where
    R: io::Read,
    T: for<'de> Deserialize<'de>,
{
    from_reader_with_config(reader, &ParserConfig::default())
}

/// Deserialize YAML from an IO reader with custom configuration.
pub fn from_reader_with_config<R, T>(mut reader: R, config: &ParserConfig) -> Result<T>
where
    R: io::Read,
    T: for<'de> Deserialize<'de>,
{
    let mut s = String::new();
    let _ = reader.read_to_string(&mut s).map_err(Error::Io)?;
    from_str_with_config(&s, config)
}

/// Deserialize a Value into a Rust type.
pub fn from_value<T>(value: &Value) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    T::deserialize(Deserializer::new(value))
}

/// A YAML deserializer.
#[derive(Debug, Clone, Copy)]
pub struct Deserializer<'de> {
    pub(crate) value: &'de Value,
    pub(crate) span_ctx: Option<&'de span_context::SpanContext>,
}

impl<'de> Deserializer<'de> {
    /// Create a new deserializer from a value.
    #[must_use]
    pub fn new(value: &'de Value) -> Self {
        Deserializer {
            value,
            span_ctx: None,
        }
    }

    /// Create a new deserializer from a value with an associated span context.
    #[must_use]
    pub fn with_span_context(value: &'de Value, span_ctx: &'de span_context::SpanContext) -> Self {
        Deserializer {
            value,
            span_ctx: Some(span_ctx),
        }
    }

    fn wrap_err<T>(&self, res: Result<T>) -> Result<T> {
        match res {
            Err(Error::Deserialize(msg)) => {
                if let Some(ctx) = self.span_ctx {
                    let ptr: *const Value = self.value;
                    let addr = ptr as usize;
                    if let Some(span) = ctx.spans.get(&addr) {
                        return Err(Error::deserialize_at(msg, &ctx.source, span.0));
                    }
                }
                Err(Error::Deserialize(msg))
            }
            _ => res,
        }
    }
}

impl<'de> de::Deserializer<'de> for Deserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::Null => self.wrap_err(visitor.visit_none()),
            Value::Bool(b) => self.wrap_err(visitor.visit_bool(*b)),
            Value::Number(Number::Integer(n)) => self.wrap_err(visitor.visit_i64(*n)),
            Value::Number(Number::Float(n)) => self.wrap_err(visitor.visit_f64(*n)),
            Value::String(s) => self.wrap_err(visitor.visit_str(s)),
            Value::Sequence(_) => self.deserialize_seq(visitor),
            Value::Mapping(_) => self.deserialize_map(visitor),
            Value::Tagged(tagged) => {
                let de = if let Some(ctx) = self.span_ctx {
                    Deserializer::with_span_context(tagged.value(), ctx)
                } else {
                    Deserializer::new(tagged.value())
                };
                de.deserialize_any(visitor)
            }
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::Bool(b) => self.wrap_err(visitor.visit_bool(*b)),
            _ => self.wrap_err(Err(Error::TypeMismatch {
                expected: "bool",
                found: type_name(self.value),
            })),
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
            Value::Number(Number::Integer(n)) => self.wrap_err(visitor.visit_i64(*n)),
            Value::Number(Number::Float(n))
                if n.fract() == 0.0
                    && *n >= i64::MIN as f64
                    && *n <= i64::MAX as f64
                    && !n.is_nan() =>
            {
                self.wrap_err(visitor.visit_i64(*n as i64))
            }
            _ => self.wrap_err(Err(Error::TypeMismatch {
                expected: "integer",
                found: type_name(self.value),
            })),
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
            Value::Number(Number::Integer(n)) if *n >= 0 => {
                self.wrap_err(visitor.visit_u64(*n as u64))
            }
            Value::Number(Number::Float(n))
                if n.fract() == 0.0 && *n >= 0.0 && *n <= u64::MAX as f64 && !n.is_nan() =>
            {
                self.wrap_err(visitor.visit_u64(*n as u64))
            }
            _ => self.wrap_err(Err(Error::TypeMismatch {
                expected: "unsigned integer",
                found: type_name(self.value),
            })),
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
            Value::Number(Number::Float(n)) => self.wrap_err(visitor.visit_f64(*n)),
            Value::Number(Number::Integer(n)) => self.wrap_err(visitor.visit_f64(*n as f64)),
            _ => self.wrap_err(Err(Error::TypeMismatch {
                expected: "float",
                found: type_name(self.value),
            })),
        }
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::String(s) if s.chars().count() == 1 => {
                self.wrap_err(visitor.visit_char(s.chars().next().unwrap()))
            }
            _ => self.wrap_err(Err(Error::TypeMismatch {
                expected: "char",
                found: type_name(self.value),
            })),
        }
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::String(s) => self.wrap_err(visitor.visit_str(s)),
            _ => self.wrap_err(Err(Error::TypeMismatch {
                expected: "string",
                found: type_name(self.value),
            })),
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
            Value::String(s) => self.wrap_err(visitor.visit_bytes(s.as_bytes())),
            _ => self.wrap_err(Err(Error::TypeMismatch {
                expected: "bytes",
                found: type_name(self.value),
            })),
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
            Value::Null => self.wrap_err(visitor.visit_none()),
            _ => self.wrap_err(visitor.visit_some(self)),
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::Null => self.wrap_err(visitor.visit_unit()),
            _ => self.wrap_err(Err(Error::TypeMismatch {
                expected: "null",
                found: type_name(self.value),
            })),
        }
    }

    fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V>(self, name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if name == crate::spanned::SPANNED_TYPE_NAME {
            return visitor.visit_map(SpannedMapAccess::new(self.value, self.span_ctx));
        }
        self.wrap_err(visitor.visit_newtype_struct(self))
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::Sequence(seq) => {
                self.wrap_err(visitor.visit_seq(ValueSeqAccess::new(seq, self.span_ctx)))
            }
            _ => self.wrap_err(Err(Error::TypeMismatch {
                expected: "sequence",
                found: type_name(self.value),
            })),
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
            Value::Mapping(map) => {
                self.wrap_err(visitor.visit_map(ValueMapAccess::new(map, self.span_ctx)))
            }
            _ => self.wrap_err(Err(Error::TypeMismatch {
                expected: "mapping",
                found: type_name(self.value),
            })),
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
        if name == crate::spanned::SPANNED_TYPE_NAME {
            return visitor.visit_map(SpannedMapAccess::new(self.value, self.span_ctx));
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
            Value::String(variant) => {
                let de: de::value::StrDeserializer<'de, Error> =
                    variant.as_str().into_deserializer();
                self.wrap_err(visitor.visit_enum(de))
            }
            Value::Mapping(map) if map.len() == 1 => {
                let (variant, value) = map.iter().next().unwrap();
                self.wrap_err(visitor.visit_enum(EnumAccess {
                    variant,
                    value,
                    span_ctx: self.span_ctx,
                }))
            }
            _ => self.wrap_err(Err(Error::TypeMismatch {
                expected: "string or single-key mapping",
                found: type_name(self.value),
            })),
        }
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::String(s) => self.wrap_err(visitor.visit_str(s)),
            _ => self.deserialize_any(visitor),
        }
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.wrap_err(visitor.visit_unit())
    }
}

pub(crate) struct ValueSeqAccess<'de> {
    iter: std::slice::Iter<'de, Value>,
    span_ctx: Option<&'de span_context::SpanContext>,
}

impl<'de> ValueSeqAccess<'de> {
    pub(crate) fn new(seq: &'de [Value], span_ctx: Option<&'de span_context::SpanContext>) -> Self {
        ValueSeqAccess {
            iter: seq.iter(),
            span_ctx,
        }
    }
}

impl<'de> SeqAccess<'de> for ValueSeqAccess<'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some(value) => {
                let de = if let Some(ctx) = self.span_ctx {
                    Deserializer::with_span_context(value, ctx)
                } else {
                    Deserializer::new(value)
                };
                seed.deserialize(de).map(Some)
            }
            None => Ok(None),
        }
    }
}

pub(crate) struct ValueMapAccess<'de> {
    iter: indexmap::map::Iter<'de, String, Value>,
    value: Option<&'de Value>,
    span_ctx: Option<&'de span_context::SpanContext>,
}

impl<'de> ValueMapAccess<'de> {
    pub(crate) fn new(
        map: &'de crate::value::Mapping,
        span_ctx: Option<&'de span_context::SpanContext>,
    ) -> Self {
        ValueMapAccess {
            iter: map.iter(),
            value: None,
            span_ctx,
        }
    }
}

impl<'de> MapAccess<'de> for ValueMapAccess<'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some((key, value)) => {
                self.value = Some(value);
                let de = if let Some(ctx) = self.span_ctx {
                    Deserializer::with_span_context(value, ctx)
                } else {
                    Deserializer::new(value)
                };
                let key_de: de::value::StrDeserializer<'de, Error> =
                    key.as_str().into_deserializer();
                de.wrap_err(seed.deserialize(key_de).map(Some))
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: DeserializeSeed<'de>,
    {
        match self.value.take() {
            Some(value) => {
                let de = if let Some(ctx) = self.span_ctx {
                    Deserializer::with_span_context(value, ctx)
                } else {
                    Deserializer::new(value)
                };
                let res = seed.deserialize(de);
                de.wrap_err(res)
            }
            None => Err(de::Error::custom("value is missing")),
        }
    }
}

struct EnumAccess<'de> {
    variant: &'de str,
    value: &'de Value,
    span_ctx: Option<&'de span_context::SpanContext>,
}

impl<'de> de::EnumAccess<'de> for EnumAccess<'de> {
    type Error = Error;
    type Variant = VariantAccess<'de>;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
    where
        V: DeserializeSeed<'de>,
    {
        let de: de::value::StrDeserializer<'de, Error> = self.variant.into_deserializer();
        let variant = seed.deserialize(de)?;
        let visitor = VariantAccess {
            value: self.value,
            span_ctx: self.span_ctx,
        };
        Ok((variant, visitor))
    }
}

struct VariantAccess<'de> {
    value: &'de Value,
    span_ctx: Option<&'de span_context::SpanContext>,
}

impl<'de> de::VariantAccess<'de> for VariantAccess<'de> {
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        let de = if let Some(ctx) = self.span_ctx {
            Deserializer::with_span_context(self.value, ctx)
        } else {
            Deserializer::new(self.value)
        };
        Deserialize::deserialize(de)
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
    where
        T: DeserializeSeed<'de>,
    {
        let de = if let Some(ctx) = self.span_ctx {
            Deserializer::with_span_context(self.value, ctx)
        } else {
            Deserializer::new(self.value)
        };
        seed.deserialize(de)
    }

    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let de = if let Some(ctx) = self.span_ctx {
            Deserializer::with_span_context(self.value, ctx)
        } else {
            Deserializer::new(self.value)
        };
        de::Deserializer::deserialize_seq(de, visitor)
    }

    fn struct_variant<V>(self, _fields: &'static [&'static str], visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let de = if let Some(ctx) = self.span_ctx {
            Deserializer::with_span_context(self.value, ctx)
        } else {
            Deserializer::new(self.value)
        };
        de::Deserializer::deserialize_map(de, visitor)
    }
}

pub(crate) struct SpannedMapAccess<'de> {
    value: &'de Value,
    span_ctx: Option<&'de span_context::SpanContext>,
    fields: std::slice::Iter<'static, &'static str>,
}

impl<'de> SpannedMapAccess<'de> {
    pub(crate) fn new(value: &'de Value, span_ctx: Option<&'de span_context::SpanContext>) -> Self {
        SpannedMapAccess {
            value,
            span_ctx,
            fields: crate::spanned::SPANNED_FIELDS.iter(),
        }
    }
}

impl<'de> MapAccess<'de> for SpannedMapAccess<'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: DeserializeSeed<'de>,
    {
        match self.fields.next() {
            Some(field) => {
                use serde::de::value::BorrowedStrDeserializer;
                let de: BorrowedStrDeserializer<'_, Error> = BorrowedStrDeserializer::new(field);
                seed.deserialize(de).map(Some)
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: DeserializeSeed<'de>,
    {
        use crate::spanned::*;
        let last_field = SPANNED_FIELDS[SPANNED_FIELDS.len() - 1 - (self.fields.len())];

        if last_field == SPANNED_FIELD_VALUE {
            let de = if let Some(ctx) = self.span_ctx {
                Deserializer::with_span_context(self.value, ctx)
            } else {
                Deserializer::new(self.value)
            };
            return de.wrap_err(seed.deserialize(de));
        }

        let ptr: *const Value = self.value;
        let addr = ptr as usize;
        let span = self.span_ctx.and_then(|ctx| ctx.spans.get(&addr));
        let loc = if let Some(s) = span {
            crate::error::Location::from_index(&self.span_ctx.unwrap().source, s.0)
        } else {
            crate::error::Location::default()
        };
        let end_loc = if let Some(s) = span {
            crate::error::Location::from_index(&self.span_ctx.unwrap().source, s.1)
        } else {
            crate::error::Location::default()
        };

        let val = match last_field {
            SPANNED_FIELD_START_LINE => loc.line(),
            SPANNED_FIELD_START_COLUMN => loc.column(),
            SPANNED_FIELD_START_INDEX => loc.index(),
            SPANNED_FIELD_END_LINE => end_loc.line(),
            SPANNED_FIELD_END_COLUMN => end_loc.column(),
            SPANNED_FIELD_END_INDEX => end_loc.index(),
            _ => unreachable!(),
        };

        seed.deserialize(val.into_deserializer())
    }
}

fn type_name(value: &Value) -> String {
    match value {
        Value::Null => "null".to_owned(),
        Value::Bool(_) => "bool".to_owned(),
        Value::Number(Number::Integer(_)) => "integer".to_owned(),
        Value::Number(Number::Float(_)) => "float".to_owned(),
        Value::String(_) => "string".to_owned(),
        Value::Sequence(_) => "sequence".to_owned(),
        Value::Mapping(_) => "mapping".to_owned(),
        Value::Tagged(tagged) => format!("tagged value (!{})", tagged.tag().as_str()),
    }
}
