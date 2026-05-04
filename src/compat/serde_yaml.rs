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

pub use crate::error::{Error, Result};
pub use crate::value::{Mapping, Number, Sequence, Tag, TaggedValue, Value};

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
}
