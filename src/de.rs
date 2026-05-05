// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! YAML Deserialization.
//!
//! # Examples
//!
//! ```
//! use noyalib::from_str;
//! use std::collections::BTreeMap;
//! let m: BTreeMap<String, i32> = from_str("a: 1\nb: 2\n").unwrap();
//! assert_eq!(m.get("a"), Some(&1));
//! ```

use crate::error::{Error, Result};
use crate::parser::{self};
use crate::prelude::*;
use crate::span_context;
use crate::value::{Number, Value};
use serde::de::{self, DeserializeSeed, IntoDeserializer, MapAccess, SeqAccess, Visitor};
use serde::Deserialize;
#[cfg(feature = "std")]
use std::io;

/// Deserialization configuration.
///
/// # Examples
///
/// ```
/// use noyalib::ParserConfig;
/// let cfg = ParserConfig::new().max_depth(64);
/// assert_eq!(cfg.max_depth, 64);
/// ```
#[derive(Debug, Clone)]
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
    /// Optional registry of custom tags to strip on the streaming path.
    ///
    /// See [`TagRegistry`](crate::TagRegistry) for the full rationale.
    /// `None` (default) preserves the legacy behaviour of routing every
    /// custom-tagged value through the AST fallback.
    pub tag_registry: Option<Arc<crate::TagRegistry>>,
    /// How the YAML merge key (`<<`) should be handled.
    ///
    /// See [`MergeKeyPolicy`] for the available policies. The
    /// default is [`MergeKeyPolicy::Auto`] — the YAML 1.2 spec
    /// behaviour where `<<:` triggers automatic mapping merge.
    pub merge_key_policy: MergeKeyPolicy,
    /// When `true`, plain scalars are *never* resolved to
    /// `null` / `bool` / `int` / `float` — every plain scalar
    /// becomes a string. Useful for schema-strict pipelines that
    /// require the user to quote intent explicitly. Default
    /// `false`.
    pub no_schema: bool,
    /// When `true`, accept YAML 1.1-style bare `0`-prefix octal
    /// literals (e.g. `0644` parsed as 420) in addition to the
    /// YAML 1.2 `0o644` form. Default `false` to honour the YAML
    /// 1.2 schema.
    pub legacy_octal_numbers: bool,
    /// When `true`, deserializing `!!binary "ABCD"` into a
    /// [`String`] target yields the literal base64 source string
    /// (`"ABCD"`) rather than rejecting on tag mismatch. The
    /// canonical bytes path (`Vec<u8>`,
    /// `serde_bytes::ByteBuf`) still decodes the base64 payload
    /// either way. Useful for migrations from Python pyyaml-style
    /// applications that treat the tag as advisory. Default
    /// `false`.
    pub ignore_binary_tag_for_string: bool,
    /// When `true`, accept YAML 1.1-style **sexagesimal** numbers
    /// (`60:00`, `1:30:00`) as integers. The colon-separated
    /// digits are interpreted in base 60: each component is
    /// multiplied by an increasing power of 60, summed left to
    /// right. `60:00` → 3 600; `1:30:00` → 5 400. Negative values
    /// (`-1:30:00`) and partial signs are honoured.
    ///
    /// Off by default to honour the YAML 1.2 schema. Useful for
    /// migrations from YAML 1.1 / Ruby / pyyaml configs that use
    /// the legacy time-of-day notation.
    pub legacy_sexagesimal: bool,
    /// Pluggable "Safe YAML" policies, run during parsing.
    ///
    /// Each [`Policy`](crate::policy::Policy) inspects parser
    /// events and the post-parse [`Value`] tree; any policy
    /// returning `Err(...)` aborts the parse with that diagnostic.
    /// Empty by default.
    ///
    /// Use [`ParserConfig::with_policy`] to register a policy.
    /// When at least one policy is present the streaming fast-path
    /// is bypassed automatically so the policy contract holds for
    /// every code path.
    pub policies: Vec<Arc<dyn crate::policy::Policy>>,
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
            tag_registry: None,
            merge_key_policy: MergeKeyPolicy::default(),
            no_schema: false,
            legacy_octal_numbers: false,
            ignore_binary_tag_for_string: false,
            legacy_sexagesimal: false,
            policies: Vec::new(),
        }
    }
}

impl ParserConfig {
    /// Create a new configuration with default values.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::ParserConfig;
    /// let cfg = ParserConfig::new();
    /// assert_eq!(cfg.max_depth, 128);
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a strict configuration (YAML 1.2 strict) with tighter
    /// security limits suitable for untrusted input.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::ParserConfig;
    /// let cfg = ParserConfig::strict();
    /// assert_eq!(cfg.max_depth, 64);
    /// ```
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
            tag_registry: None,
            merge_key_policy: MergeKeyPolicy::default(),
            no_schema: false,
            legacy_octal_numbers: false,
            ignore_binary_tag_for_string: false,
            legacy_sexagesimal: false,
            policies: Vec::new(),
        }
    }

    /// Register a [`Policy`](crate::policy::Policy) to enforce
    /// during parsing.
    ///
    /// Multiple policies may be registered; they all run in
    /// registration order, and the first error short-circuits the
    /// parse. When any policy is present the streaming fast-path
    /// is bypassed so the policy contract is enforced uniformly.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{from_str_with_config, ParserConfig, Value};
    /// use noyalib::policy::DenyAnchors;
    ///
    /// let cfg = ParserConfig::new().with_policy(DenyAnchors);
    /// let res: Result<Value, _> =
    ///     from_str_with_config("a: &x 1\nb: *x\n", &cfg);
    /// assert!(res.is_err());
    /// ```
    #[must_use]
    pub fn with_policy<P>(mut self, policy: P) -> Self
    where
        P: crate::policy::Policy + 'static,
    {
        self.policies.push(Arc::new(policy));
        self
    }

    /// Set the maximum recursion depth.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::ParserConfig;
    /// let cfg = ParserConfig::new().max_depth(32);
    /// assert_eq!(cfg.max_depth, 32);
    /// ```
    #[must_use]
    pub fn max_depth(mut self, depth: usize) -> Self {
        self.max_depth = depth;
        self
    }

    /// Set the maximum document length.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::ParserConfig;
    /// let cfg = ParserConfig::new().max_document_length(1024);
    /// assert_eq!(cfg.max_document_length, 1024);
    /// ```
    #[must_use]
    pub fn max_document_length(mut self, len: usize) -> Self {
        self.max_document_length = len;
        self
    }

    /// Set the maximum alias expansions.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::ParserConfig;
    /// let cfg = ParserConfig::new().max_alias_expansions(50);
    /// assert_eq!(cfg.max_alias_expansions, 50);
    /// ```
    #[must_use]
    pub fn max_alias_expansions(mut self, expansions: usize) -> Self {
        self.max_alias_expansions = expansions;
        self
    }

    /// Set the maximum number of mapping keys.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::ParserConfig;
    /// let cfg = ParserConfig::new().max_mapping_keys(100);
    /// assert_eq!(cfg.max_mapping_keys, 100);
    /// ```
    #[must_use]
    pub fn max_mapping_keys(mut self, max: usize) -> Self {
        self.max_mapping_keys = max;
        self
    }

    /// Set the maximum sequence length.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::ParserConfig;
    /// let cfg = ParserConfig::new().max_sequence_length(100);
    /// assert_eq!(cfg.max_sequence_length, 100);
    /// ```
    #[must_use]
    pub fn max_sequence_length(mut self, max: usize) -> Self {
        self.max_sequence_length = max;
        self
    }

    /// Set the duplicate key policy.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{DuplicateKeyPolicy, ParserConfig};
    /// let cfg = ParserConfig::new().duplicate_key_policy(DuplicateKeyPolicy::Error);
    /// assert_eq!(cfg.duplicate_key_policy, DuplicateKeyPolicy::Error);
    /// ```
    #[must_use]
    pub fn duplicate_key_policy(mut self, policy: DuplicateKeyPolicy) -> Self {
        self.duplicate_key_policy = policy;
        self
    }

    /// Enable or disable strict booleans.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::ParserConfig;
    /// let cfg = ParserConfig::new().strict_booleans(true);
    /// assert!(cfg.strict_booleans);
    /// ```
    #[must_use]
    pub fn strict_booleans(mut self, strict: bool) -> Self {
        self.strict_booleans = strict;
        self
    }

    /// Enable or disable legacy booleans.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::ParserConfig;
    /// let cfg = ParserConfig::new().legacy_booleans(true);
    /// assert!(cfg.legacy_booleans);
    /// ```
    #[must_use]
    pub fn legacy_booleans(mut self, legacy: bool) -> Self {
        self.legacy_booleans = legacy;
        self
    }

    /// Attach a [`TagRegistry`](crate::TagRegistry) so the streaming
    /// deserializer strips listed custom tags instead of routing them
    /// through the AST.
    ///
    /// See the [`tag_registry`](crate::tag_registry) module
    /// documentation for when to use this versus `#[serde(rename)]`.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{ParserConfig, TagRegistry};
    /// use std::sync::Arc;
    /// let reg = Arc::new(TagRegistry::new().with("!Celsius"));
    /// let cfg = ParserConfig::new().tag_registry(Arc::clone(&reg));
    /// assert!(cfg.tag_registry.is_some());
    /// ```
    #[must_use]
    pub fn tag_registry(mut self, registry: Arc<crate::TagRegistry>) -> Self {
        self.tag_registry = Some(registry);
        self
    }

    /// Set the policy for handling the YAML merge key (`<<`).
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{MergeKeyPolicy, ParserConfig};
    /// let cfg = ParserConfig::new().merge_key_policy(MergeKeyPolicy::AsOrdinary);
    /// assert_eq!(cfg.merge_key_policy, MergeKeyPolicy::AsOrdinary);
    /// ```
    #[must_use]
    pub fn merge_key_policy(mut self, policy: MergeKeyPolicy) -> Self {
        self.merge_key_policy = policy;
        self
    }

    /// Toggle schema-free plain-scalar resolution. When `true`,
    /// every plain scalar becomes a string regardless of whether
    /// it would normally resolve to `null`, `bool`, integer, or
    /// float.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::ParserConfig;
    /// let cfg = ParserConfig::new().no_schema(true);
    /// assert!(cfg.no_schema);
    /// ```
    #[must_use]
    pub fn no_schema(mut self, no_schema: bool) -> Self {
        self.no_schema = no_schema;
        self
    }

    /// Toggle YAML 1.1-style bare `0`-prefix octal parsing
    /// (e.g. `0644` → 420). Off by default; YAML 1.2 requires the
    /// `0o` prefix.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::ParserConfig;
    /// let cfg = ParserConfig::new().legacy_octal_numbers(true);
    /// assert!(cfg.legacy_octal_numbers);
    /// ```
    #[must_use]
    pub fn legacy_octal_numbers(mut self, on: bool) -> Self {
        self.legacy_octal_numbers = on;
        self
    }

    /// Toggle the migration-helper behaviour where
    /// `!!binary "ABCD"` deserializes into a [`String`] target as
    /// the literal base64 source string. The canonical bytes
    /// path (`Vec<u8>`, `serde_bytes::ByteBuf`) is unaffected —
    /// it always decodes the base64 payload.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::ParserConfig;
    /// let cfg = ParserConfig::new().ignore_binary_tag_for_string(true);
    /// assert!(cfg.ignore_binary_tag_for_string);
    /// ```
    #[must_use]
    pub fn ignore_binary_tag_for_string(mut self, on: bool) -> Self {
        self.ignore_binary_tag_for_string = on;
        self
    }

    /// Toggle YAML 1.1-style sexagesimal number parsing
    /// (`60:00` → 3 600). Off by default; YAML 1.2 dropped the
    /// sexagesimal schema, so plain `1:30:00` would otherwise
    /// surface as a string.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::ParserConfig;
    /// let cfg = ParserConfig::new().legacy_sexagesimal(true);
    /// assert!(cfg.legacy_sexagesimal);
    /// ```
    #[must_use]
    pub fn legacy_sexagesimal(mut self, on: bool) -> Self {
        self.legacy_sexagesimal = on;
        self
    }
}

/// Policy for handling the YAML merge key (`<<`) during parsing.
///
/// YAML 1.2 §10.2 defines `<<` as a "merge key" that, when used as
/// a mapping key, splices the value's mapping (or sequence of
/// mappings) into the enclosing mapping. The variants below let
/// callers opt out of that behaviour.
///
/// # Examples
///
/// ```
/// use noyalib::MergeKeyPolicy;
/// assert_eq!(MergeKeyPolicy::default(), MergeKeyPolicy::Auto);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MergeKeyPolicy {
    /// Apply the YAML 1.2 merge-key semantics — `<<:` keys trigger
    /// automatic merge of the value into the enclosing mapping.
    /// Default.
    #[default]
    Auto,
    /// Treat `<<` as an ordinary string key. The mapping retains a
    /// literal `<<` entry whose value is whatever the YAML
    /// document supplied. Useful when round-tripping configuration
    /// files that happen to contain a `<<` key for non-merge
    /// reasons.
    AsOrdinary,
    /// Reject any document that contains a `<<` key with
    /// [`crate::Error::Custom`]. Useful for schema-strict pipelines
    /// where merge keys are forbidden.
    Error,
}

/// Policy for handling duplicate keys in a YAML mapping.
///
/// # Examples
///
/// ```
/// use noyalib::DuplicateKeyPolicy;
/// assert_eq!(DuplicateKeyPolicy::default(), DuplicateKeyPolicy::Last);
/// ```
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
///
/// # Examples
///
/// ```
/// let n: i32 = noyalib::from_str("42").unwrap();
/// assert_eq!(n, 42);
/// ```
pub fn from_str<T>(s: &str) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    from_str_with_config(s, &ParserConfig::default())
}

/// Deserialize YAML from a string with custom security limits.
///
/// # Examples
///
/// ```
/// use noyalib::{from_str_with_config, ParserConfig};
/// let cfg = ParserConfig::strict();
/// let n: i32 = from_str_with_config("7", &cfg).unwrap();
/// assert_eq!(n, 7);
/// ```
pub fn from_str_with_config<T>(s: &str, config: &ParserConfig) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    // Try streaming path first (faster, no intermediate Value AST).
    // The streaming path bakes in YAML 1.2 semantics:
    // - `<<: *alias` merges natively;
    // - `!!binary` is propagated as a typed tag.
    // When the caller asked for a non-default behaviour on either
    // axis, route through the AST loader so the relevant toggle
    // takes effect.
    let stream_eligible = config.merge_key_policy == MergeKeyPolicy::Auto
        && !config.ignore_binary_tag_for_string
        && config.policies.is_empty();
    if stream_eligible {
        if let Some(res) = crate::streaming::from_str_streaming(s, config) {
            return res;
        }
    }

    let parse_config = parser::ParseConfig::from(config);

    #[cfg(feature = "std")]
    {
        let (value, span_tree) = parser::parse_one(s, &parse_config)?;
        for p in &config.policies {
            p.check_value(&value)?;
        }
        let spans = span_context::build_span_map(&value, &span_tree);
        let ctx = span_context::SpanContext {
            spans,
            source: s.into(),
        };
        let _guard = span_context::set_span_context(ctx);
        let de = Deserializer::with_options(
            &value,
            Some(_guard.as_ref()),
            config.ignore_binary_tag_for_string,
        );
        T::deserialize(de)
    }

    #[cfg(not(feature = "std"))]
    {
        let value = parser::parse_one_value(s, &parse_config)?;
        T::deserialize(Deserializer::with_options(
            &value,
            None,
            config.ignore_binary_tag_for_string,
        ))
    }
}

/// Deserialize YAML from a byte slice.
///
/// # Examples
///
/// ```
/// let n: i32 = noyalib::from_slice(b"42").unwrap();
/// assert_eq!(n, 42);
/// ```
pub fn from_slice<T>(b: &[u8]) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    let s = core::str::from_utf8(b).map_err(|e| Error::Deserialize(e.to_string()))?;
    from_str(s)
}

/// Deserialize YAML from a byte slice with custom configuration.
///
/// # Examples
///
/// ```
/// use noyalib::{from_slice_with_config, ParserConfig};
/// let cfg = ParserConfig::new();
/// let n: i32 = from_slice_with_config(b"7", &cfg).unwrap();
/// assert_eq!(n, 7);
/// ```
pub fn from_slice_with_config<T>(b: &[u8], config: &ParserConfig) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    let s = core::str::from_utf8(b).map_err(|e| Error::Deserialize(e.to_string()))?;
    from_str_with_config(s, config)
}

/// Deserialize YAML from an IO reader.
///
/// # Examples
///
/// ```
/// let yaml = b"42".to_vec();
/// let n: i32 = noyalib::from_reader(&yaml[..]).unwrap();
/// assert_eq!(n, 42);
/// ```
#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
pub fn from_reader<R, T>(reader: R) -> Result<T>
where
    R: io::Read,
    T: for<'de> Deserialize<'de>,
{
    from_reader_with_config(reader, &ParserConfig::default())
}

/// Deserialize YAML from an IO reader with custom configuration.
///
/// # Examples
///
/// ```
/// use noyalib::{from_reader_with_config, ParserConfig};
/// let cfg = ParserConfig::new();
/// let bytes = b"7".to_vec();
/// let n: i32 = from_reader_with_config(&bytes[..], &cfg).unwrap();
/// assert_eq!(n, 7);
/// ```
#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
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
///
/// # Examples
///
/// ```
/// use noyalib::{from_value, Value};
/// let v = Value::from(42_i64);
/// let n: i32 = from_value(&v).unwrap();
/// assert_eq!(n, 42);
/// ```
pub fn from_value<T>(value: &Value) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    T::deserialize(Deserializer::new(value))
}

/// A YAML deserializer.
///
/// # Examples
///
/// ```
/// use noyalib::{Deserializer, Value};
/// use serde::Deserialize;
/// let v = Value::from(42_i64);
/// let de = Deserializer::new(&v);
/// let n: i32 = Deserialize::deserialize(de).unwrap();
/// assert_eq!(n, 42);
/// ```
#[derive(Debug, Clone, Copy)]
pub struct Deserializer<'de> {
    pub(crate) value: &'de Value,
    pub(crate) span_ctx: Option<&'de span_context::SpanContext>,
    /// Per-call flag mirroring
    /// [`ParserConfig::ignore_binary_tag_for_string`]. When `true`,
    /// `!!binary "ABCD"` deserializes into `String` as the literal
    /// `"ABCD"` (no base64 decode). Default `false` preserves YAML
    /// 1.2 semantics.
    pub(crate) ignore_binary_tag_for_string: bool,
}

impl<'de> Deserializer<'de> {
    /// Create a new deserializer from a value.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{Deserializer, Value};
    /// let v = Value::from(1_i64);
    /// let _de = Deserializer::new(&v);
    /// ```
    #[must_use]
    pub fn new(value: &'de Value) -> Self {
        Deserializer {
            value,
            span_ctx: None,
            ignore_binary_tag_for_string: false,
        }
    }

    /// Create a new deserializer from a value with an associated span context.
    ///
    /// The span context carries source-location information used to attach
    /// line/column details to errors and `Spanned<T>` fields. This
    /// constructor is primarily used internally by `from_str`; most callers
    /// should prefer [`Deserializer::new`].
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // Constructed internally by from_str — external callers use Deserializer::new.
    /// use noyalib::Deserializer;
    /// # let value = unimplemented!();
    /// # let span_ctx = unimplemented!();
    /// let _de = Deserializer::with_span_context(value, span_ctx);
    /// ```
    #[must_use]
    pub fn with_span_context(value: &'de Value, span_ctx: &'de span_context::SpanContext) -> Self {
        Deserializer {
            value,
            span_ctx: Some(span_ctx),
            ignore_binary_tag_for_string: false,
        }
    }

    /// Pass-through constructor for the
    /// [`crate::ParserConfig::ignore_binary_tag_for_string`] flag.
    /// Used internally by [`from_str_with_config`] when the caller
    /// has opted in to the migration helper.
    pub(crate) fn with_options(
        value: &'de Value,
        span_ctx: Option<&'de span_context::SpanContext>,
        ignore_binary_tag_for_string: bool,
    ) -> Self {
        Deserializer {
            value,
            span_ctx,
            ignore_binary_tag_for_string,
        }
    }

    /// Construct a child deserializer for `value`, propagating the
    /// span context and every per-call config toggle from `self`.
    /// Used by every descent site (struct field, sequence element,
    /// tagged inner value) so the toggles survive the walk.
    pub(crate) fn descend(&self, value: &'de Value) -> Self {
        Deserializer {
            value,
            span_ctx: self.span_ctx,
            ignore_binary_tag_for_string: self.ignore_binary_tag_for_string,
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
                let de = self.descend(tagged.value());
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
            // Migration helper: when the source declared
            // `!!binary "ABCD"` and the caller opted in to
            // `ignore_binary_tag_for_string`, surface the literal
            // source string rather than rejecting on tag mismatch.
            // The base64 encoding stays as the user-facing value;
            // the application layer can decode (or not) as it
            // sees fit.
            Value::Tagged(boxed)
                if self.ignore_binary_tag_for_string && is_binary_tag(boxed.tag().as_str()) =>
            {
                match boxed.value() {
                    Value::String(s) => self.wrap_err(visitor.visit_str(s)),
                    other => self.wrap_err(Err(Error::TypeMismatch {
                        expected: "string-shaped !!binary content",
                        found: type_name(other),
                    })),
                }
            }
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
            // YAML 1.2.2 §10.4: `!!binary` carries an RFC 4648
            // base64-encoded payload. Decode on demand when a serde
            // target asks for bytes / a byte buffer (Vec<u8>,
            // serde_bytes::ByteBuf, &[u8] via owned visit).
            Value::Tagged(boxed) if is_binary_tag(boxed.tag().as_str()) => match boxed.value() {
                Value::String(s) => match crate::base64::decode(s) {
                    Ok(bytes) => self.wrap_err(visitor.visit_byte_buf(bytes)),
                    Err(why) => self.wrap_err(Err(Error::Deserialize(format!("!!binary: {why}")))),
                },
                other => self.wrap_err(Err(Error::TypeMismatch {
                    expected: "string-shaped !!binary content",
                    found: type_name(other),
                })),
            },
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
                self.wrap_err(visitor.visit_seq(ValueSeqAccess::from_de(&self, seq)))
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
                self.wrap_err(visitor.visit_map(ValueMapAccess::from_de(&self, map)))
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
    iter: core::slice::Iter<'de, Value>,
    span_ctx: Option<&'de span_context::SpanContext>,
    ignore_binary_tag_for_string: bool,
}

impl<'de> ValueSeqAccess<'de> {
    pub(crate) fn from_de(de: &Deserializer<'de>, seq: &'de [Value]) -> Self {
        ValueSeqAccess {
            iter: seq.iter(),
            span_ctx: de.span_ctx,
            ignore_binary_tag_for_string: de.ignore_binary_tag_for_string,
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
                let de = Deserializer::with_options(
                    value,
                    self.span_ctx,
                    self.ignore_binary_tag_for_string,
                );
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
    ignore_binary_tag_for_string: bool,
}

impl<'de> ValueMapAccess<'de> {
    pub(crate) fn from_de(de: &Deserializer<'de>, map: &'de crate::value::Mapping) -> Self {
        ValueMapAccess {
            iter: map.iter(),
            value: None,
            span_ctx: de.span_ctx,
            ignore_binary_tag_for_string: de.ignore_binary_tag_for_string,
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
                let de = Deserializer::with_options(
                    value,
                    self.span_ctx,
                    self.ignore_binary_tag_for_string,
                );
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
                let de = Deserializer::with_options(
                    value,
                    self.span_ctx,
                    self.ignore_binary_tag_for_string,
                );
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
    fields: core::slice::Iter<'static, &'static str>,
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
            _ => crate::error::invariant_violated(
                "spanned-field index outside the SPANNED_FIELDS array",
            ),
        };

        seed.deserialize(val.into_deserializer())
    }
}

/// True if `tag` names the YAML 1.2 binary tag, in any of the forms
/// the scanner / loader may produce: shorthand `!!binary`, suffix
/// `binary` (post-handle-stripping), or the canonical full URI
/// `tag:yaml.org,2002:binary`. Stripping the leading `!` on the
/// shorthand keeps `Tag::new("!!binary") == Tag::new("binary")` —
/// which noyalib's `Tag` already considers equal — both matching.
pub(crate) fn is_binary_tag(tag: &str) -> bool {
    matches!(
        tag,
        "!!binary" | "binary" | "tag:yaml.org,2002:binary" | "!<tag:yaml.org,2002:binary>"
    )
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
