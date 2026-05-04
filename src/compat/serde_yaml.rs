// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Drop-in API surface compatible with `serde_yaml` 0.9.
//!
//! The upstream `serde_yaml` crate is unmaintained. This module
//! exposes a name-for-name surface so existing codebases can
//! migrate by editing two lines:
//!
//! ```toml
//! # Cargo.toml — before
//! serde_yaml = "0.9"
//! # Cargo.toml — after
//! noyalib = { version = "0.0.1", features = ["compat-serde-yaml"] }
//! ```
//!
//! ```rust,ignore
//! // anywhere in the codebase
//! - use serde_yaml::{from_str, to_string, Value};
//! + use noyalib::compat::serde_yaml::{from_str, to_string, Value};
//! ```
//!
//! Every function delegates to the underlying noyalib engine — no
//! double-parsing, no extra allocations, no parser fork. Where
//! `serde_yaml`'s signature differs from noyalib's (the most common
//! case is taking a `Value` by value vs. by reference), this shim
//! provides a thin adapter; everything else is a re-export.
//!
//! # Known surface differences
//!
//! - **`Mapping` is string-keyed.** noyalib's [`Mapping`] uses
//!   `String` keys; `serde_yaml::Mapping` allowed any [`Value`] as a
//!   key. The 99% case (configuration files, RPC payloads) is
//!   string-keyed and works unchanged. If your code constructs a
//!   `Mapping` with non-string keys, switch to noyalib's
//!   [`crate::MappingAny`] directly — the shim does not re-export
//!   it under the `Mapping` name to keep type errors localised.
//!
//! # `Value` conversions during migration
//!
//! Codebases mid-migration sometimes hold a `serde_yaml::Value` in
//! flight (e.g. as a function parameter on the boundary between
//! migrated and un-migrated modules). The shim provides
//! `From<noyalib::Value> for serde_yaml::Value` and
//! `TryFrom<serde_yaml::Value> for noyalib::Value` so those values
//! flow across the boundary without re-parsing through a string.
//!
//! ```rust,ignore
//! use noyalib::compat::serde_yaml as syml;
//!
//! // Lift a noyalib::Value into the upstream serde_yaml::Value.
//! // Total — never fails.
//! let upstream: ::serde_yaml::Value = my_noyalib_value.into();
//!
//! // Lower an upstream serde_yaml::Value into noyalib::Value.
//! // Fallible: a non-string mapping key returns
//! // `SerdeYamlConversionError::NonStringKey`.
//! let lowered: syml::Value = upstream.try_into()?;
//! ```
//!
//! # `Error` parity
//!
//! The `Error::location() -> Option<Location>` /
//! `Location::line()` / `Location::column()` / `Location::index()`
//! chain matches `serde_yaml`'s shape byte-for-byte (1-indexed line
//! and column, 0-indexed byte offset), so existing diagnostic-emitting
//! code that destructures these compiles unchanged.
//!
//! # Migration cookbook
//!
//! ```rust
//! use noyalib::compat::serde_yaml as syml;
//!
//! #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
//! struct Config { name: String, port: u16 }
//!
//! let yaml = "name: noyalib\nport: 8080\n";
//! let cfg: Config = syml::from_str(yaml).unwrap();
//! assert_eq!(cfg, Config { name: "noyalib".into(), port: 8080 });
//!
//! let back = syml::to_string(&cfg).unwrap();
//! let round: Config = syml::from_str(&back).unwrap();
//! assert_eq!(cfg, round);
//! ```

use crate::prelude::*;
use serde::de::DeserializeOwned;
use serde::Serialize;

// ── Types — re-exported under the serde_yaml names ───────────────────

pub use crate::error::{Error, Location, Result};
pub use crate::value::{Mapping, Number, Sequence, Tag, TaggedValue, Value};

// ── Conversions between noyalib::Value and ::serde_yaml::Value ───────
//
// The two `Value` types are structurally compatible *almost*
// everywhere. The one asymmetry is in `Mapping`:
//
//   - noyalib's `Mapping` is `String → Value` (the 99 % case for
//     real-world configuration / RPC payloads).
//   - `serde_yaml`'s `Mapping` is `Value → Value` — keys can be any
//     YAML node, including sequences and other mappings.
//
// That means `noyalib::Value → serde_yaml::Value` is *total* (a
// `String` key trivially fits where any `Value` is allowed) — `From`
// is the right trait. The reverse direction is *fallible* — a
// `serde_yaml` value with non-string keys cannot be lowered into
// noyalib's `Value` without information loss — so it goes through
// `TryFrom`. Both shapes match the standard Rust idiom: `From` for
// rich-from-simple, `TryFrom` for simple-from-rich.

/// Errors produced when converting a `serde_yaml::Value` into a
/// `noyalib::Value`. Both variants are rare in practice — most
/// real-world YAML uses string-keyed mappings and `i64`/`f64`-shaped
/// numbers — but they are real spec corners that the type system
/// makes explicit rather than papering over.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SerdeYamlConversionError {
    /// The source `serde_yaml::Value::Mapping` contained a non-string
    /// key. noyalib's `Mapping` is string-keyed by design (see
    /// `noyalib::MappingAny` for the value-keyed alternative).
    NonStringKey,
    /// The source `serde_yaml::Number` was a `u64` whose value does
    /// not fit in noyalib's `i64` integer representation.
    UnrepresentableNumber,
}

impl core::fmt::Display for SerdeYamlConversionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NonStringKey => f.write_str(
                "serde_yaml::Value: mapping with non-string key cannot lower into \
                 noyalib::Value (use noyalib::MappingAny for value-keyed maps)",
            ),
            Self::UnrepresentableNumber => f.write_str(
                "serde_yaml::Number: value out of range for noyalib::Number (i64/f64)",
            ),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for SerdeYamlConversionError {}

impl From<Number> for ::serde_yaml::Number {
    fn from(n: Number) -> Self {
        match n {
            Number::Integer(i) => i.into(),
            Number::Float(f) => f.into(),
        }
    }
}

impl From<Value> for ::serde_yaml::Value {
    fn from(v: Value) -> Self {
        match v {
            Value::Null => ::serde_yaml::Value::Null,
            Value::Bool(b) => ::serde_yaml::Value::Bool(b),
            Value::Number(n) => ::serde_yaml::Value::Number(n.into()),
            Value::String(s) => ::serde_yaml::Value::String(s),
            Value::Sequence(seq) => {
                ::serde_yaml::Value::Sequence(seq.into_iter().map(Into::into).collect())
            }
            Value::Mapping(m) => {
                let mut out = ::serde_yaml::Mapping::with_capacity(m.len());
                for (k, v) in m {
                    let _ = out.insert(::serde_yaml::Value::String(k), v.into());
                }
                ::serde_yaml::Value::Mapping(out)
            }
            Value::Tagged(boxed) => {
                let tagged: TaggedValue = *boxed;
                ::serde_yaml::Value::Tagged(Box::new(::serde_yaml::value::TaggedValue {
                    tag: ::serde_yaml::value::Tag::new(tagged.tag().as_str()),
                    value: tagged.value().clone().into(),
                }))
            }
        }
    }
}

impl TryFrom<::serde_yaml::Number> for Number {
    type Error = SerdeYamlConversionError;

    fn try_from(n: ::serde_yaml::Number) -> core::result::Result<Self, Self::Error> {
        if let Some(i) = n.as_i64() {
            Ok(Number::Integer(i))
        } else if let Some(f) = n.as_f64() {
            Ok(Number::Float(f))
        } else {
            Err(SerdeYamlConversionError::UnrepresentableNumber)
        }
    }
}

impl TryFrom<::serde_yaml::Value> for Value {
    type Error = SerdeYamlConversionError;

    fn try_from(v: ::serde_yaml::Value) -> core::result::Result<Self, Self::Error> {
        match v {
            ::serde_yaml::Value::Null => Ok(Value::Null),
            ::serde_yaml::Value::Bool(b) => Ok(Value::Bool(b)),
            ::serde_yaml::Value::Number(n) => Ok(Value::Number(n.try_into()?)),
            ::serde_yaml::Value::String(s) => Ok(Value::String(s)),
            ::serde_yaml::Value::Sequence(seq) => {
                let mut out = Vec::with_capacity(seq.len());
                for item in seq {
                    out.push(Value::try_from(item)?);
                }
                Ok(Value::Sequence(out))
            }
            ::serde_yaml::Value::Mapping(m) => {
                let mut out = Mapping::with_capacity(m.len());
                for (k, val) in m {
                    let key = match k {
                        ::serde_yaml::Value::String(s) => s,
                        _ => return Err(SerdeYamlConversionError::NonStringKey),
                    };
                    let _ = out.insert(key, Value::try_from(val)?);
                }
                Ok(Value::Mapping(out))
            }
            ::serde_yaml::Value::Tagged(boxed) => {
                // serde_yaml::Tag stringifies to `!Thing` (with the
                // leading `!`); noyalib::Tag::new tolerates either
                // form (`!foo` and `foo` produce equal tags).
                let tag_str = boxed.tag.to_string();
                let inner = Value::try_from(boxed.value)?;
                Ok(Value::Tagged(Box::new(TaggedValue::new(
                    Tag::new(tag_str),
                    inner,
                ))))
            }
        }
    }
}

// ── Deserialization ──────────────────────────────────────────────────

/// Deserialize a YAML document into the target type.
///
/// Direct re-export of [`crate::from_str`] — same signature,
/// same behaviour.
pub fn from_str<T>(s: &str) -> Result<T>
where
    T: DeserializeOwned,
{
    crate::from_str(s)
}

/// Deserialize a YAML document from a byte slice.
pub fn from_slice<T>(bytes: &[u8]) -> Result<T>
where
    T: DeserializeOwned,
{
    crate::from_slice(bytes)
}

/// Deserialize a YAML document from any [`std::io::Read`] source.
///
/// `serde_yaml::from_reader` and noyalib's `from_reader` have
/// identical signatures, so this is a direct re-export.
#[cfg(feature = "std")]
pub fn from_reader<R, T>(reader: R) -> Result<T>
where
    R: std::io::Read,
    T: DeserializeOwned,
{
    crate::from_reader(reader)
}

/// Deserialize a typed value from a [`Value`].
///
/// `serde_yaml::from_value` takes the [`Value`] by *value*; noyalib
/// takes it by reference. This adapter accepts the
/// `serde_yaml`-style by-value form so call sites do not need to
/// add an `&` during migration.
pub fn from_value<T>(value: Value) -> Result<T>
where
    T: DeserializeOwned,
{
    crate::from_value(&value)
}

// ── Serialization ────────────────────────────────────────────────────

/// Serialize a typed value to a YAML string.
pub fn to_string<T>(value: &T) -> Result<String>
where
    T: Serialize,
{
    crate::to_string(value)
}

/// Serialize a typed value to any [`std::io::Write`] sink.
#[cfg(feature = "std")]
pub fn to_writer<W, T>(writer: W, value: &T) -> Result<()>
where
    W: std::io::Write,
    T: Serialize,
{
    crate::to_writer(writer, value)
}

/// Serialize a typed value to a [`Value`].
pub fn to_value<T>(value: T) -> Result<Value>
where
    T: Serialize,
{
    // `serde_yaml::to_value` takes by value; noyalib takes by
    // reference. Accept the `serde_yaml` shape and forward.
    crate::to_value(&value)
}

// ── Multi-document streams ───────────────────────────────────────────
//
// `serde_yaml` exposed `Deserializer::from_str(s).into_iter::<T>()`
// for multi-document parsing. noyalib's nearest equivalent is
// `load_all_as`. We expose it under the `serde_yaml` name pattern.

/// Iterate every YAML document in a multi-document stream and
/// deserialize each into `T`. Mirrors the `Deserializer::from_str`
/// + `into_iter::<T>()` chain that is the typical `serde_yaml`
/// multi-document idiom.
pub fn from_str_multi<T>(s: &str) -> Result<Vec<T>>
where
    T: DeserializeOwned,
{
    crate::load_all_as::<T>(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Config {
        name: String,
        port: u16,
    }

    #[test]
    fn from_str_round_trips_typed() {
        let yaml = "name: noyalib\nport: 8080\n";
        let cfg: Config = from_str(yaml).unwrap();
        assert_eq!(
            cfg,
            Config {
                name: "noyalib".into(),
                port: 8080
            }
        );
    }

    #[test]
    fn from_slice_typed() {
        let bytes = b"name: noyalib\nport: 8080\n";
        let cfg: Config = from_slice(bytes).unwrap();
        assert_eq!(cfg.port, 8080);
    }

    #[test]
    fn from_value_takes_by_value_like_serde_yaml() {
        let mut m = Mapping::new();
        let _ = m.insert("name", Value::String("noyalib".into()));
        let _ = m.insert("port", Value::Number(Number::Integer(8080)));
        let v = Value::Mapping(m);
        // Note: by *value*, no `&`. This is the `serde_yaml` shape.
        let cfg: Config = from_value(v).unwrap();
        assert_eq!(cfg.port, 8080);
    }

    #[test]
    fn to_value_takes_by_value_like_serde_yaml() {
        let cfg = Config {
            name: "noyalib".into(),
            port: 8080,
        };
        // By value, no `&`.
        let v = to_value(cfg).unwrap();
        match v {
            Value::Mapping(m) => {
                assert_eq!(m.get("name"), Some(&Value::String("noyalib".into())));
            }
            _ => panic!("expected Mapping"),
        }
    }

    #[test]
    fn round_trip_via_to_string_from_str() {
        let cfg = Config {
            name: "noyalib".into(),
            port: 8080,
        };
        let s = to_string(&cfg).unwrap();
        let back: Config = from_str(&s).unwrap();
        assert_eq!(cfg, back);
    }

    #[test]
    fn multi_doc_stream() {
        let yaml = "name: a\nport: 1\n---\nname: b\nport: 2\n";
        let docs: Vec<Config> = from_str_multi(yaml).unwrap();
        assert_eq!(docs.len(), 2);
        assert_eq!(docs[0].port, 1);
        assert_eq!(docs[1].port, 2);
    }

    #[test]
    fn error_type_re_export_is_noyalib_error() {
        // Compile-time check: `serde_yaml::Error` and
        // `noyalib::Error` are the same type, so callers' existing
        // error-handling code keeps working.
        #[allow(unused_qualifications)]
        fn _identity(e: super::Error) -> crate::error::Error {
            e
        }
    }

    // ── Error parity: location() / line() / column() ─────────────

    #[test]
    fn error_exposes_location_line_column() {
        // The `serde_yaml::Error::location()` → `Location::line()`
        // / `Location::column()` chain is the contract that any
        // diagnostic-emitting migrant relies on. noyalib's `Error`
        // exposes the exact same shape, 1-indexed.
        let err = from_str::<Value>("a: [unclosed").unwrap_err();
        let loc = err
            .location()
            .expect("parse error must carry a location");
        assert!(loc.line() >= 1);
        assert!(loc.column() >= 1);
        // Index is the 0-based byte offset, matching serde_yaml.
        let _: usize = loc.index();
    }

    // ── Value conversions ────────────────────────────────────────

    #[test]
    fn noyalib_to_serde_yaml_roundtrip_via_string() {
        // Build a noyalib::Value, lift to serde_yaml, emit, re-parse
        // through noyalib — the JSON-equivalent shape must survive.
        let mut m = Mapping::new();
        let _ = m.insert("name", Value::String("noyalib".into()));
        let _ = m.insert("port", Value::Number(Number::Integer(8080)));
        let _ = m.insert(
            "features",
            Value::Sequence(vec![
                Value::String("auth".into()),
                Value::String("api".into()),
            ]),
        );
        let original = Value::Mapping(m);

        let lifted: ::serde_yaml::Value = original.clone().into();
        let yaml = ::serde_yaml::to_string(&lifted).unwrap();
        let reparsed: Value = from_str(&yaml).unwrap();
        assert_eq!(original, reparsed);
    }

    #[test]
    fn serde_yaml_to_noyalib_simple_mapping() {
        // The 99 % case: string-keyed mapping. Conversion must succeed.
        let yaml = "name: noyalib\nport: 8080\nflags:\n  - a\n  - b\n";
        let upstream: ::serde_yaml::Value =
            ::serde_yaml::from_str(yaml).unwrap();
        let lowered: Value = upstream.try_into().unwrap();
        match lowered {
            Value::Mapping(m) => {
                assert_eq!(m.get("name"), Some(&Value::String("noyalib".into())));
                assert_eq!(
                    m.get("port"),
                    Some(&Value::Number(Number::Integer(8080)))
                );
            }
            _ => panic!("expected Mapping"),
        }
    }

    #[test]
    fn serde_yaml_to_noyalib_rejects_non_string_keys() {
        // YAML allows any node as a key. noyalib's `Mapping` is
        // string-keyed by design — `TryFrom` surfaces the mismatch
        // as a typed error rather than papering over it.
        let yaml = "[1, 2]: nested-key\nstring-key: value\n";
        let upstream: ::serde_yaml::Value =
            ::serde_yaml::from_str(yaml).unwrap();
        let result: core::result::Result<Value, _> = upstream.try_into();
        match result {
            Err(SerdeYamlConversionError::NonStringKey) => (),
            other => panic!("expected NonStringKey error, got {other:?}"),
        }
    }

    #[test]
    fn serde_yaml_to_noyalib_preserves_tagged() {
        let yaml = "!Custom value\n";
        let upstream: ::serde_yaml::Value =
            ::serde_yaml::from_str(yaml).unwrap();
        let lowered: Value = upstream.try_into().unwrap();
        match lowered {
            Value::Tagged(boxed) => {
                // serde_yaml normalises to a leading `!`; noyalib's
                // Tag::new tolerates either form, so equality is
                // by content.
                assert!(boxed.tag().as_str().contains("Custom"));
                assert_eq!(*boxed.value(), Value::String("value".into()));
            }
            other => panic!("expected Tagged, got {other:?}"),
        }
    }

    #[test]
    fn number_conversions_are_lossless_for_i64_and_f64() {
        // i64 round-trip
        let n_in = Number::Integer(-42);
        let lifted: ::serde_yaml::Number = n_in.into();
        let lowered: Number = lifted.try_into().unwrap();
        assert!(matches!(lowered, Number::Integer(-42)));

        // f64 round-trip
        let n_in = Number::Float(3.125);
        let lifted: ::serde_yaml::Number = n_in.into();
        let lowered: Number = lifted.try_into().unwrap();
        assert!(matches!(lowered, Number::Float(f) if (f - 3.125).abs() < f64::EPSILON));
    }

    #[test]
    fn full_serde_yaml_to_noyalib_roundtrip_via_actual_files() {
        // Migration scenario: a downstream codebase still has an
        // upstream `serde_yaml::Value` in flight. They convert it
        // into `noyalib::Value` to take advantage of noyalib's
        // streaming, querying, or CST APIs without re-parsing.
        let yaml = r#"
server:
  host: localhost
  port: 8080
  ssl: true
features:
  - auth
  - api
limits:
  rps: 1000
  burst: 50
"#;
        let upstream: ::serde_yaml::Value =
            ::serde_yaml::from_str(yaml).unwrap();
        let lowered: Value = upstream.clone().try_into().unwrap();
        let direct: Value = from_str(yaml).unwrap();
        assert_eq!(
            lowered, direct,
            "serde_yaml→noyalib conversion must agree with direct noyalib parse"
        );
    }
}
