//! YAML serialization.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use std::fmt::Write as FmtWrite;
use std::io::Write;

use serde::ser::{self, Serialize};

use crate::error::{Error, Result};
use crate::fmt;
use crate::value::{Mapping, Number, Sequence, Tag, TaggedValue, Value};

/// Flow style preference for collections.
///
/// Controls whether sequences and mappings should use inline (flow) or
/// multi-line (block) style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FlowStyle {
    /// Always use block style (multi-line).
    #[default]
    Block,
    /// Always use flow style (inline, JSON-like).
    Flow,
    /// Automatic: use flow for small collections, block for larger ones.
    Auto,
}

/// Scalar style preference for strings.
///
/// Controls how string values should be quoted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScalarStyle {
    /// Automatic quoting based on content.
    #[default]
    Auto,
    /// Always use double quotes.
    DoubleQuoted,
    /// Always use single quotes.
    SingleQuoted,
    /// Use literal block style (|) for multiline.
    Literal,
    /// Use folded block style (>) for multiline.
    Folded,
    /// Plain (unquoted) style when possible.
    Plain,
}

/// Configuration options for YAML serialization.
///
/// # Example
///
/// ```rust
/// use noyalib::{FlowStyle, ScalarStyle, SerializerConfig};
///
/// let config = SerializerConfig::new()
///     .indent(4)
///     .flow_style(FlowStyle::Auto)
///     .scalar_style(ScalarStyle::DoubleQuoted)
///     .document_start(true);
/// ```
#[derive(Debug, Clone, Copy)]
pub struct SerializerConfig {
    /// Number of spaces per indentation level (default: 2).
    pub indent: usize,
    /// Whether to include document start marker `---` (default: false).
    pub document_start: bool,
    /// Whether to include document end marker `...` (default: false).
    pub document_end: bool,
    /// Whether to use block style for multiline strings (default: true).
    pub block_scalars: bool,
    /// Minimum number of newlines to trigger block scalar style (default: 1).
    pub block_scalar_threshold: usize,
    /// Flow style preference for collections (default: Block).
    pub flow_style: FlowStyle,
    /// Scalar style preference for strings (default: Auto).
    pub scalar_style: ScalarStyle,
    /// Maximum number of items in a collection to use flow style in Auto mode
    /// (default: 4).
    pub flow_threshold: usize,
    /// Force-quote all string scalars regardless of content (default: false).
    pub quote_all: bool,
    /// Compact list indentation under mapping keys (default: false).
    ///
    /// When `true`, sequence items under a mapping key align with the key
    /// instead of being indented an extra level.
    pub compact_list_indent: bool,
    /// Line width for folded block scalars (default: 80).
    pub folded_wrap_chars: usize,
    /// Minimum string length before block scalar style is considered (default: 80).
    pub min_fold_chars: usize,
}

impl Default for SerializerConfig {
    fn default() -> Self {
        Self {
            indent: 2,
            document_start: false,
            document_end: false,
            block_scalars: true,
            block_scalar_threshold: 1,
            flow_style: FlowStyle::Block,
            scalar_style: ScalarStyle::Auto,
            flow_threshold: 4,
            quote_all: false,
            compact_list_indent: false,
            folded_wrap_chars: 80,
            min_fold_chars: 80,
        }
    }
}

impl SerializerConfig {
    /// Create a new configuration with default settings.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the indentation width.
    #[must_use]
    pub fn indent(mut self, spaces: usize) -> Self {
        self.indent = spaces;
        self
    }

    /// Enable or disable document start marker `---`.
    #[must_use]
    pub fn document_start(mut self, enabled: bool) -> Self {
        self.document_start = enabled;
        self
    }

    /// Enable or disable document end marker `...`.
    #[must_use]
    pub fn document_end(mut self, enabled: bool) -> Self {
        self.document_end = enabled;
        self
    }

    /// Enable or disable block scalar style for multiline strings.
    #[must_use]
    pub fn block_scalars(mut self, enabled: bool) -> Self {
        self.block_scalars = enabled;
        self
    }

    /// Set minimum newlines to trigger block scalar style.
    #[must_use]
    pub fn block_scalar_threshold(mut self, count: usize) -> Self {
        self.block_scalar_threshold = count;
        self
    }

    /// Set the flow style preference for collections.
    ///
    /// - `FlowStyle::Block`: Always use multi-line block style
    /// - `FlowStyle::Flow`: Always use inline flow style
    /// - `FlowStyle::Auto`: Use flow for small collections
    #[must_use]
    pub fn flow_style(mut self, style: FlowStyle) -> Self {
        self.flow_style = style;
        self
    }

    /// Set the scalar style preference for strings.
    ///
    /// - `ScalarStyle::Auto`: Quote only when necessary
    /// - `ScalarStyle::DoubleQuoted`: Always use double quotes
    /// - `ScalarStyle::SingleQuoted`: Always use single quotes
    /// - `ScalarStyle::Literal`: Use `|` for multiline
    /// - `ScalarStyle::Folded`: Use `>` for multiline
    /// - `ScalarStyle::Plain`: Unquoted when possible
    #[must_use]
    pub fn scalar_style(mut self, style: ScalarStyle) -> Self {
        self.scalar_style = style;
        self
    }

    /// Set the threshold for automatic flow style.
    ///
    /// Collections with this many or fewer items will use flow style
    /// when `flow_style` is set to `Auto`.
    #[must_use]
    pub fn flow_threshold(mut self, threshold: usize) -> Self {
        self.flow_threshold = threshold;
        self
    }

    /// Force-quote all string scalars regardless of content.
    #[must_use]
    pub fn quote_all(mut self, enabled: bool) -> Self {
        self.quote_all = enabled;
        self
    }

    /// Enable compact list indentation under mapping keys.
    ///
    /// When enabled, sequence items align with the key rather than
    /// being indented an extra level.
    #[must_use]
    pub fn compact_list_indent(mut self, enabled: bool) -> Self {
        self.compact_list_indent = enabled;
        self
    }

    /// Set the line width for folded block scalars.
    #[must_use]
    pub fn folded_wrap_chars(mut self, chars: usize) -> Self {
        self.folded_wrap_chars = chars;
        self
    }

    /// Set the minimum string length for block scalar style.
    ///
    /// Strings shorter than this threshold will not use block scalar
    /// (`|` / `>`) style, even if they contain newlines.
    #[must_use]
    pub fn min_fold_chars(mut self, chars: usize) -> Self {
        self.min_fold_chars = chars;
        self
    }
}

/// Serialize a Rust type to a YAML string.
///
/// # Errors
///
/// Returns an error if the type cannot be serialized to YAML.
///
/// # Example
///
/// ```rust
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct Config {
///     name: String,
///     port: u16,
/// }
///
/// let config = Config {
///     name: "myapp".to_string(),
///     port: 8080,
/// };
///
/// let yaml = noyalib::to_string(&config).unwrap();
/// assert!(yaml.contains("name: myapp"));
/// assert!(yaml.contains("port: 8080"));
/// ```
pub fn to_string<T>(value: &T) -> Result<String>
where
    T: ?Sized + Serialize,
{
    let v = to_value(value)?;
    value_to_string(&v, &SerializerConfig::default())
}

/// Serialize a Rust type to a YAML string with custom configuration.
///
/// # Errors
///
/// Returns an error if the type cannot be serialized to YAML.
///
/// # Example
///
/// ```rust
/// use noyalib::SerializerConfig;
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct Config {
///     name: String,
///     port: u16,
/// }
///
/// let config = Config {
///     name: "myapp".to_string(),
///     port: 8080,
/// };
///
/// let yaml = noyalib::to_string_with_config(
///     &config,
///     &SerializerConfig::new().indent(4).document_start(true),
/// )
/// .unwrap();
/// assert!(yaml.starts_with("---"));
/// ```
pub fn to_string_with_config<T>(value: &T, config: &SerializerConfig) -> Result<String>
where
    T: ?Sized + Serialize,
{
    let v = to_value(value)?;
    value_to_string(&v, config)
}

/// Serialize a Rust type to a YAML writer.
///
/// # Errors
///
/// Returns an error if the type cannot be serialized or writing fails.
pub fn to_writer<W, T>(writer: W, value: &T) -> Result<()>
where
    W: Write,
    T: ?Sized + Serialize,
{
    to_writer_with_config(writer, value, &SerializerConfig::default())
}

/// Serialize a Rust type to a YAML writer with custom configuration.
///
/// # Errors
///
/// Returns an error if the type cannot be serialized or writing fails.
pub fn to_writer_with_config<W, T>(writer: W, value: &T, config: &SerializerConfig) -> Result<()>
where
    W: Write,
    T: ?Sized + Serialize,
{
    let s = to_string_with_config(value, config)?;
    let mut writer = writer;
    writer.write_all(s.as_bytes())?;
    Ok(())
}

/// Serialize a Rust type to a `fmt::Write` destination.
///
/// # Errors
///
/// Returns an error if the type cannot be serialized or writing fails.
pub fn to_fmt_writer<W, T>(writer: &mut W, value: &T) -> Result<()>
where
    W: std::fmt::Write,
    T: ?Sized + Serialize,
{
    to_fmt_writer_with_config(writer, value, &SerializerConfig::default())
}

/// Serialize a Rust type to a `fmt::Write` destination with custom configuration.
///
/// # Errors
///
/// Returns an error if the type cannot be serialized or writing fails.
pub fn to_fmt_writer_with_config<W, T>(
    writer: &mut W,
    value: &T,
    config: &SerializerConfig,
) -> Result<()>
where
    W: std::fmt::Write,
    T: ?Sized + Serialize,
{
    let s = to_string_with_config(value, config)?;
    writer
        .write_str(&s)
        .map_err(|e| Error::Serialize(e.to_string()))
}

/// Serialize a Rust type to a `Value`.
///
/// # Errors
///
/// Returns an error if the type cannot be serialized.
pub fn to_value<T>(value: &T) -> Result<Value>
where
    T: ?Sized + Serialize,
{
    value.serialize(Serializer)
}

fn value_to_string(value: &Value, config: &SerializerConfig) -> Result<String> {
    let mut output = String::with_capacity(estimate_yaml_size(value));
    if config.document_start {
        output.push_str("---\n");
    }
    write_value(&mut output, value, 0, true, config)?;
    if config.document_end {
        output.push_str("\n...");
    }
    Ok(output)
}

fn estimate_yaml_size(value: &Value) -> usize {
    match value {
        Value::Null => 4,
        Value::Bool(_) => 5,
        Value::Number(_) => 12,
        Value::String(s) => s.len() + 4,
        Value::Sequence(seq) => 4 + seq.iter().map(|v| estimate_yaml_size(v) + 4).sum::<usize>(),
        Value::Mapping(map) => {
            4 + map
                .iter()
                .map(|(k, v)| k.len() + estimate_yaml_size(v) + 6)
                .sum::<usize>()
        }
        Value::Tagged(t) => 20 + estimate_yaml_size(t.value()),
    }
}

/// Write `total_spaces` space characters to `output` without heap allocation.
#[inline]
fn write_indent(output: &mut String, total_spaces: usize) {
    const SPACES: &str = "                                                                ";
    // 64 spaces - covers indent up to depth 32 with indent=2
    let mut remaining = total_spaces;
    while remaining > 0 {
        let n = remaining.min(SPACES.len());
        output.push_str(&SPACES[..n]);
        remaining -= n;
    }
}

fn write_value(
    output: &mut String,
    value: &Value,
    indent: usize,
    is_root: bool,
    config: &SerializerConfig,
) -> Result<()> {
    match value {
        Value::Null => output.push_str("null"),
        Value::Bool(b) => output.push_str(if *b { "true" } else { "false" }),
        Value::Number(Number::Integer(n)) => {
            let mut buf = itoa::Buffer::new();
            output.push_str(buf.format(*n));
        }
        Value::Number(Number::Float(n)) => {
            if n.is_nan() {
                output.push_str(".nan");
            } else if n.is_infinite() {
                if *n > 0.0 {
                    output.push_str(".inf");
                } else {
                    output.push_str("-.inf");
                }
            } else {
                let mut buf = ryu::Buffer::new();
                output.push_str(buf.format(*n));
            }
        }
        Value::String(s) => write_string(output, s, indent, config),
        Value::Sequence(seq) => write_sequence(output, seq, indent, is_root, config)?,
        Value::Mapping(map) => write_mapping(output, map, indent, is_root, config)?,
        Value::Tagged(tagged) => {
            let tag_str = tagged.tag().as_str();
            if tag_str.starts_with("__noya_") {
                write_internal_tag(output, tag_str, tagged.value(), indent, is_root, config)?;
            } else {
                // Write tag followed by value
                output.push_str(tag_str);
                output.push(' ');
                write_value(output, tagged.value(), indent, false, config)?;
            }
        }
    }
    Ok(())
}

/// Fast check whether a plain scalar would be interpreted as a number by a YAML
/// parser.  This is intentionally over-inclusive to ensure roundtrip safety —
/// it's cheaper to quote a few extra strings than to lose data.
fn looks_like_number(s: &str) -> bool {
    let bytes = s.as_bytes();
    if bytes.is_empty() {
        return false;
    }

    // YAML special float literals (case variants)
    if matches!(
        s,
        ".inf"
            | ".Inf"
            | ".INF"
            | "+.inf"
            | "+.Inf"
            | "+.INF"
            | "-.inf"
            | "-.Inf"
            | "-.INF"
            | ".nan"
            | ".NaN"
            | ".NAN"
    ) {
        return true;
    }

    // Skip any leading signs (yaml-rust2 is permissive with e.g. "++1")
    let mut i = 0;
    while i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        i += 1;
    }
    if i >= bytes.len() {
        return false;
    }

    let rest = &bytes[i..];

    // Anything starting with a digit could be interpreted as numeric.
    if rest[0].is_ascii_digit() {
        return true;
    }
    // "." followed by digit: floats like .5
    if rest[0] == b'.' && rest.len() > 1 && rest[1].is_ascii_digit() {
        return true;
    }

    false
}

/// Lookup table: true if the byte requires the string to be quoted.
/// Covers: control chars (except tab), colon, hash, newline, etc.
static NEEDS_QUOTE_BYTE: [bool; 128] = {
    let mut t = [false; 128];
    // Control characters (except tab 0x09)
    let mut i = 0u8;
    while i < 0x20 {
        if i != b'\t' {
            t[i as usize] = true;
        }
        i += 1;
    }
    // YAML structural characters
    t[b':' as usize] = true;
    t[b'#' as usize] = true;
    t[b'\n' as usize] = true;
    t[b'\r' as usize] = true;
    t[b'\0' as usize] = true;
    t
};

/// Characters that require quoting when they appear as the first character.
static FIRST_CHAR_QUOTE: [bool; 128] = {
    let mut t = [false; 128];
    t[b' ' as usize] = true;
    t[b'-' as usize] = true;
    t[b'&' as usize] = true;
    t[b'*' as usize] = true;
    t[b'!' as usize] = true;
    t[b'|' as usize] = true;
    t[b'>' as usize] = true;
    t[b'%' as usize] = true;
    t[b'@' as usize] = true;
    t[b'`' as usize] = true;
    t[b'{' as usize] = true;
    t[b'}' as usize] = true;
    t[b'[' as usize] = true;
    t[b']' as usize] = true;
    t[b',' as usize] = true;
    t[b'?' as usize] = true;
    t[b'\'' as usize] = true;
    t[b'"' as usize] = true;
    t
};

fn write_string(output: &mut String, s: &str, indent: usize, config: &SerializerConfig) {
    let bytes = s.as_bytes();

    // Empty string must be quoted
    if bytes.is_empty() {
        output.push_str("\"\"");
        return;
    }

    // Force-quote all strings when configured
    if config.quote_all {
        write_single_quoted(output, s);
        return;
    }

    // Fast path: short ASCII strings that are clearly safe as plain scalars.
    // Avoids the full lookup table scan for the majority of mapping keys.
    if bytes.len() <= 64
        && bytes[0].is_ascii_alphanumeric()
        && bytes[bytes.len() - 1].is_ascii_alphanumeric()
        && !config.block_scalars
        || bytes.iter().all(|&b| b != b'\n')
    {
        let safe = bytes.iter().all(|&b| {
            b.is_ascii_alphanumeric() || b == b'_' || b == b'-' || b == b'.' || b == b'/'
        });
        if safe
            && !matches!(
                s,
                "true"
                    | "false"
                    | "null"
                    | "~"
                    | "True"
                    | "False"
                    | "TRUE"
                    | "FALSE"
                    | "Null"
                    | "NULL"
            )
            && !looks_like_number(s)
        {
            output.push_str(s);
            return;
        }
    }

    // Block scalar for multiline strings
    if config.block_scalars {
        let newlines = bytes.iter().filter(|&&b| b == b'\n').count();
        if newlines >= config.block_scalar_threshold {
            write_block_scalar(output, s, indent, config);
            return;
        }
    }

    // Single-pass quoting decision
    let mut needs_quotes = false;
    let mut has_control = false;

    // Check first character
    if bytes[0] < 128 && FIRST_CHAR_QUOTE[bytes[0] as usize] {
        needs_quotes = true;
    }

    // Check last character (trailing space)
    if bytes[bytes.len() - 1] == b' ' {
        needs_quotes = true;
    }

    // Reserved words
    if !needs_quotes {
        needs_quotes = matches!(
            s,
            "true" | "false" | "null" | "~" | "True" | "False" | "TRUE" | "FALSE" | "Null" | "NULL"
        ) || looks_like_number(s);
    }

    // Single pass through interior bytes
    if !needs_quotes {
        for &b in bytes {
            if b < 128 && NEEDS_QUOTE_BYTE[b as usize] {
                if b < 0x20 && b != b'\t' {
                    has_control = true;
                }
                needs_quotes = true;
                // Don't break - we need to know if there are control chars
            }
        }
    }

    if !needs_quotes {
        // Plain scalar - zero-copy output
        output.push_str(s);
        return;
    }

    // Use double quotes for all quoted strings
    let _ = has_control;
    write_double_quoted(output, s);
}

/// Write a single-quoted string, escaping embedded single quotes.
fn write_single_quoted(output: &mut String, s: &str) {
    output.push('\'');
    for c in s.chars() {
        if c == '\'' {
            output.push_str("''");
        } else {
            output.push(c);
        }
    }
    output.push('\'');
}

/// Write a double-quoted string with bulk-copy between escape points.
fn write_double_quoted(output: &mut String, s: &str) {
    output.push('"');
    let bytes = s.as_bytes();
    let mut start = 0;
    for (i, &b) in bytes.iter().enumerate() {
        let esc = match b {
            b'"' => "\\\"",
            b'\\' => "\\\\",
            b'\n' => "\\n",
            b'\r' => "\\r",
            b'\t' => "\\t",
            b'\0' => "\\0",
            c if c < 0x20 && c != b'\t' => {
                // Other control characters: flush and write hex escape
                output.push_str(&s[start..i]);
                let _ = write!(output, "\\x{c:02X}");
                start = i + 1;
                continue;
            }
            _ => continue,
        };
        output.push_str(&s[start..i]);
        output.push_str(esc);
        start = i + 1;
    }
    output.push_str(&s[start..]);
    output.push('"');
}

/// Write a string using YAML literal block scalar style (|).
fn write_block_scalar(output: &mut String, s: &str, indent: usize, config: &SerializerConfig) {
    // Determine chomping indicator based on trailing newlines
    let chomping = if s.ends_with('\n') {
        if s.ends_with("\n\n") {
            "+" // Keep all trailing newlines
        } else {
            "" // Keep single trailing newline (default)
        }
    } else {
        "-" // Strip trailing newlines
    };

    output.push('|');
    output.push_str(chomping);

    for line in s.lines() {
        output.push('\n');
        write_indent(output, config.indent * (indent + 1));
        output.push_str(line);
    }

    // s.lines() does not yield trailing empty lines, so we must emit them
    // for the "keep" (+) chomping mode to roundtrip correctly.
    if s.ends_with('\n') {
        // Count trailing newlines
        let trailing = s.len() - s.trim_end_matches('\n').len();
        // lines() already omits one trailing newline in default mode,
        // so for "+" mode we need to emit all trailing newlines explicitly.
        for _ in 0..trailing {
            output.push('\n');
        }
    }
}

fn write_sequence(
    output: &mut String,
    seq: &Sequence,
    indent: usize,
    is_root: bool,
    config: &SerializerConfig,
) -> Result<()> {
    if seq.is_empty() {
        output.push_str("[]");
        return Ok(());
    }

    for (i, value) in seq.iter().enumerate() {
        if i > 0 || !is_root {
            output.push('\n');
            write_indent(output, config.indent * indent);
        }
        output.push_str("- ");

        match value {
            Value::Mapping(m) if !m.is_empty() => {
                // Write first key-value on same line as dash
                let mut iter = m.iter();
                if let Some((k, v)) = iter.next() {
                    write_string(output, k, indent + 1, config);
                    output.push_str(": ");
                    if matches!(v, Value::Mapping(_) | Value::Sequence(_)) {
                        write_value(output, v, indent + 2, false, config)?;
                    } else {
                        write_value(output, v, indent + 1, false, config)?;
                    }
                }
                // Write remaining key-values
                for (k, v) in iter {
                    output.push('\n');
                    write_indent(output, config.indent * (indent + 1));
                    write_string(output, k, indent + 1, config);
                    output.push_str(": ");
                    if matches!(v, Value::Mapping(_) | Value::Sequence(_)) {
                        write_value(output, v, indent + 2, false, config)?;
                    } else {
                        write_value(output, v, indent + 1, false, config)?;
                    }
                }
            }
            Value::Sequence(_) => {
                write_value(output, value, indent + 1, false, config)?;
            }
            _ => {
                write_value(output, value, indent + 1, false, config)?;
            }
        }
    }
    Ok(())
}

fn write_mapping(
    output: &mut String,
    map: &Mapping,
    indent: usize,
    is_root: bool,
    config: &SerializerConfig,
) -> Result<()> {
    if map.is_empty() {
        output.push_str("{}");
        return Ok(());
    }

    for (i, (key, value)) in map.iter().enumerate() {
        if i > 0 || !is_root {
            output.push('\n');
            write_indent(output, config.indent * indent);
        }
        write_string(output, key, indent, config);

        match value {
            Value::Mapping(m) if !m.is_empty() => {
                output.push(':');
                write_value(output, value, indent + 1, false, config)?;
            }
            Value::Sequence(s) if !s.is_empty() => {
                output.push(':');
                write_value(output, value, indent + 1, false, config)?;
            }
            _ => {
                output.push_str(": ");
                write_value(output, value, indent, false, config)?;
            }
        }
    }
    Ok(())
}

fn write_internal_tag(
    output: &mut String,
    tag: &str,
    value: &Value,
    indent: usize,
    is_root: bool,
    config: &SerializerConfig,
) -> Result<()> {
    match tag {
        fmt::MAGIC_FLOW_SEQ => {
            if let Value::Sequence(seq) = value {
                write_flow_sequence(output, seq, config)?;
            } else {
                write_value(output, value, indent, is_root, config)?;
            }
        }
        fmt::MAGIC_FLOW_MAP => {
            if let Value::Mapping(map) = value {
                write_flow_mapping(output, map, config)?;
            } else {
                write_value(output, value, indent, is_root, config)?;
            }
        }
        fmt::MAGIC_LIT_STR => {
            if let Value::String(s) = value {
                write_literal_block(output, s, indent, config);
            } else {
                write_value(output, value, indent, is_root, config)?;
            }
        }
        fmt::MAGIC_FOLD_STR => {
            if let Value::String(s) = value {
                write_folded_block(output, s, indent, config);
            } else {
                write_value(output, value, indent, is_root, config)?;
            }
        }
        fmt::MAGIC_COMMENTED => {
            // value is a sequence [inner_value, comment_string]
            if let Value::Sequence(seq) = value {
                if seq.len() == 2 {
                    write_value(output, &seq[0], indent, is_root, config)?;
                    if let Value::String(comment) = &seq[1] {
                        output.push_str(" # ");
                        output.push_str(comment);
                    }
                } else {
                    write_value(output, value, indent, is_root, config)?;
                }
            } else {
                write_value(output, value, indent, is_root, config)?;
            }
        }
        fmt::MAGIC_SPACE_AFTER => {
            write_value(output, value, indent, is_root, config)?;
            output.push('\n');
        }
        _ => {
            // Unknown internal tag — fall through to regular output
            write_value(output, value, indent, is_root, config)?;
        }
    }
    Ok(())
}

fn write_flow_sequence(
    output: &mut String,
    seq: &Sequence,
    config: &SerializerConfig,
) -> Result<()> {
    output.push('[');
    for (i, value) in seq.iter().enumerate() {
        if i > 0 {
            output.push_str(", ");
        }
        write_value(output, value, 0, false, config)?;
    }
    output.push(']');
    Ok(())
}

fn write_flow_mapping(output: &mut String, map: &Mapping, config: &SerializerConfig) -> Result<()> {
    output.push('{');
    for (i, (key, value)) in map.iter().enumerate() {
        if i > 0 {
            output.push_str(", ");
        }
        write_string(output, key, 0, config);
        output.push_str(": ");
        write_value(output, value, 0, false, config)?;
    }
    output.push('}');
    Ok(())
}

fn write_literal_block(output: &mut String, s: &str, indent: usize, config: &SerializerConfig) {
    let chomping = if s.ends_with('\n') {
        if s.ends_with("\n\n") {
            "+"
        } else {
            ""
        }
    } else {
        "-"
    };

    output.push('|');
    output.push_str(chomping);

    for line in s.lines() {
        output.push('\n');
        write_indent(output, config.indent * (indent + 1));
        output.push_str(line);
    }
}

fn write_folded_block(output: &mut String, s: &str, indent: usize, config: &SerializerConfig) {
    let chomping = if s.ends_with('\n') {
        if s.ends_with("\n\n") {
            "+"
        } else {
            ""
        }
    } else {
        "-"
    };

    output.push('>');
    output.push_str(chomping);

    for line in s.lines() {
        output.push('\n');
        write_indent(output, config.indent * (indent + 1));
        output.push_str(line);
    }
}

/// Serialize multiple values as a multi-document YAML string.
///
/// Each value is separated by `---` document start markers.
///
/// # Errors
///
/// Returns an error if any value cannot be serialized.
///
/// # Example
///
/// ```rust
/// let docs = vec![1, 2, 3];
/// let yaml = noyalib::to_string_multi(&docs).unwrap();
/// assert!(yaml.contains("---"));
/// ```
pub fn to_string_multi<T: Serialize>(values: &[T]) -> Result<String> {
    to_string_multi_with_config(values, &SerializerConfig::default())
}

/// Serialize multiple values as a multi-document YAML string with custom
/// configuration.
///
/// # Errors
///
/// Returns an error if any value cannot be serialized.
pub fn to_string_multi_with_config<T: Serialize>(
    values: &[T],
    config: &SerializerConfig,
) -> Result<String> {
    let mut output = String::new();
    for (i, value) in values.iter().enumerate() {
        if i > 0 {
            output.push('\n');
        }
        output.push_str("---\n");
        let v = to_value(value)?;
        write_value(&mut output, &v, 0, true, config)?;
        output.push('\n');
    }
    Ok(output)
}

/// Serialize multiple values as multi-document YAML to a writer.
///
/// # Errors
///
/// Returns an error if any value cannot be serialized or writing fails.
pub fn to_writer_multi<W: Write, T: Serialize>(writer: W, values: &[T]) -> Result<()> {
    to_writer_multi_with_config(writer, values, &SerializerConfig::default())
}

/// Serialize multiple values as multi-document YAML to a writer with custom
/// configuration.
///
/// # Errors
///
/// Returns an error if any value cannot be serialized or writing fails.
pub fn to_writer_multi_with_config<W: Write, T: Serialize>(
    writer: W,
    values: &[T],
    config: &SerializerConfig,
) -> Result<()> {
    let s = to_string_multi_with_config(values, config)?;
    let mut writer = writer;
    writer.write_all(s.as_bytes())?;
    Ok(())
}

/// A YAML serializer.
#[derive(Debug, Copy, Clone)]
pub struct Serializer;

impl ser::Serializer for Serializer {
    type Ok = Value;
    type Error = Error;

    type SerializeSeq = SerializeSeq;
    type SerializeTuple = SerializeSeq;
    type SerializeTupleStruct = SerializeSeq;
    type SerializeTupleVariant = SerializeTupleVariant;
    type SerializeMap = SerializeMap;
    type SerializeStruct = SerializeMap;
    type SerializeStructVariant = SerializeStructVariant;

    fn serialize_bool(self, v: bool) -> Result<Value> {
        Ok(Value::Bool(v))
    }

    fn serialize_i8(self, v: i8) -> Result<Value> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_i16(self, v: i16) -> Result<Value> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_i32(self, v: i32) -> Result<Value> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_i64(self, v: i64) -> Result<Value> {
        Ok(Value::Number(Number::Integer(v)))
    }

    fn serialize_u8(self, v: u8) -> Result<Value> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_u16(self, v: u16) -> Result<Value> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_u32(self, v: u32) -> Result<Value> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_u64(self, v: u64) -> Result<Value> {
        if v <= i64::MAX as u64 {
            Ok(Value::Number(Number::Integer(v as i64)))
        } else {
            Err(Error::Serialize(format!(
                "u64 value {v} exceeds i64::MAX and cannot be represented losslessly"
            )))
        }
    }

    fn serialize_f32(self, v: f32) -> Result<Value> {
        self.serialize_f64(f64::from(v))
    }

    fn serialize_f64(self, v: f64) -> Result<Value> {
        Ok(Value::Number(Number::Float(v)))
    }

    fn serialize_char(self, v: char) -> Result<Value> {
        self.serialize_str(&v.to_string())
    }

    fn serialize_str(self, v: &str) -> Result<Value> {
        Ok(Value::String(v.to_owned()))
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Value> {
        match String::from_utf8(v.to_vec()) {
            Ok(s) => Ok(Value::String(s)),
            Err(_) => Err(Error::Serialize(
                "bytes contain invalid UTF-8; YAML strings must be valid UTF-8".into(),
            )),
        }
    }

    fn serialize_none(self) -> Result<Value> {
        Ok(Value::Null)
    }

    fn serialize_some<T>(self, value: &T) -> Result<Value>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Value> {
        Ok(Value::Null)
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Value> {
        self.serialize_unit()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Value> {
        self.serialize_str(variant)
    }

    fn serialize_newtype_struct<T>(self, name: &'static str, value: &T) -> Result<Value>
    where
        T: ?Sized + Serialize,
    {
        // Intercept formatting hint magic names
        match name {
            fmt::MAGIC_FLOW_SEQ
            | fmt::MAGIC_FLOW_MAP
            | fmt::MAGIC_LIT_STR
            | fmt::MAGIC_FOLD_STR
            | fmt::MAGIC_SPACE_AFTER => {
                let inner = value.serialize(Serializer)?;
                Ok(Value::Tagged(Box::new(TaggedValue::new(
                    Tag::new(name),
                    inner,
                ))))
            }
            fmt::MAGIC_COMMENTED => {
                // value is a tuple (inner_value, comment_string)
                let inner = value.serialize(Serializer)?;
                Ok(Value::Tagged(Box::new(TaggedValue::new(
                    Tag::new(name),
                    inner,
                ))))
            }
            _ => value.serialize(self),
        }
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Value>
    where
        T: ?Sized + Serialize,
    {
        let mut map = Mapping::new();
        let _ = map.insert(variant.to_owned(), value.serialize(Serializer)?);
        Ok(Value::Mapping(map))
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq> {
        Ok(SerializeSeq {
            vec: Vec::with_capacity(len.unwrap_or(0)),
        })
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        Ok(SerializeTupleVariant {
            name: variant.to_owned(),
            vec: Vec::with_capacity(len),
        })
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        Ok(SerializeMap {
            map: Mapping::new(),
            key: None,
        })
    }

    fn serialize_struct(self, _name: &'static str, len: usize) -> Result<Self::SerializeStruct> {
        self.serialize_map(Some(len))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        Ok(SerializeStructVariant {
            name: variant.to_owned(),
            map: Mapping::new(),
        })
    }
}

/// Serializer for sequences.
#[derive(Debug)]
pub struct SerializeSeq {
    vec: Vec<Value>,
}

impl ser::SerializeSeq for SerializeSeq {
    type Ok = Value;
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.vec.push(value.serialize(Serializer)?);
        Ok(())
    }

    fn end(self) -> Result<Value> {
        Ok(Value::Sequence(self.vec))
    }
}

impl ser::SerializeTuple for SerializeSeq {
    type Ok = Value;
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Value> {
        ser::SerializeSeq::end(self)
    }
}

impl ser::SerializeTupleStruct for SerializeSeq {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Value> {
        ser::SerializeSeq::end(self)
    }
}

/// Serializer for tuple variants.
#[derive(Debug)]
pub struct SerializeTupleVariant {
    name: String,
    vec: Vec<Value>,
}

impl ser::SerializeTupleVariant for SerializeTupleVariant {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.vec.push(value.serialize(Serializer)?);
        Ok(())
    }

    fn end(self) -> Result<Value> {
        let mut map = Mapping::new();
        let _ = map.insert(self.name, Value::Sequence(self.vec));
        Ok(Value::Mapping(map))
    }
}

/// Serializer for maps.
#[derive(Debug)]
pub struct SerializeMap {
    map: Mapping,
    key: Option<String>,
}

impl ser::SerializeMap for SerializeMap {
    type Ok = Value;
    type Error = Error;

    fn serialize_key<T>(&mut self, key: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        let key_value = key.serialize(Serializer)?;
        let key_str = match key_value {
            Value::String(s) => s,
            Value::Number(Number::Integer(n)) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            _ => return Err(Error::Serialize("map key must be a string".to_string())),
        };
        self.key = Some(key_str);
        Ok(())
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        let key = self
            .key
            .take()
            .ok_or_else(|| Error::Serialize("missing key".to_string()))?;
        let _ = self.map.insert(key, value.serialize(Serializer)?);
        Ok(())
    }

    fn end(self) -> Result<Value> {
        Ok(Value::Mapping(self.map))
    }
}

impl ser::SerializeStruct for SerializeMap {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        let _ = self
            .map
            .insert(key.to_owned(), value.serialize(Serializer)?);
        Ok(())
    }

    fn end(self) -> Result<Value> {
        Ok(Value::Mapping(self.map))
    }
}

/// Serializer for struct variants.
#[derive(Debug)]
pub struct SerializeStructVariant {
    name: String,
    map: Mapping,
}

impl ser::SerializeStructVariant for SerializeStructVariant {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        let _ = self
            .map
            .insert(key.to_owned(), value.serialize(Serializer)?);
        Ok(())
    }

    fn end(self) -> Result<Value> {
        let mut map = Mapping::new();
        let _ = map.insert(self.name, Value::Mapping(self.map));
        Ok(Value::Mapping(map))
    }
}
