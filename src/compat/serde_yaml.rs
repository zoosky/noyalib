// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Drop-in API surface compatible with `serde_yaml` 0.9.
//!
//! The upstream `serde_yaml` 0.9 crate is unmaintained. This
//! module exposes a name-for-name surface so existing codebases
//! can migrate by editing two lines:
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
//! # Zero legacy dependencies
//!
//! The shim deliberately **does not depend on the unmaintained
//! `serde_yaml` 0.9 crate**. Every type the shim exposes is a
//! noyalib-native type re-exported under the `serde_yaml` name —
//! you migrate *off* the legacy crate, not into a vendored copy
//! of it. Downstream `cargo audit` / `cargo deny` runs do not
//! pick up the archived advisory chain.
//!
//! # Known surface differences
//!
//! - **`Mapping` is string-keyed.** noyalib's [`Mapping`] uses
//!   `String` keys; `serde_yaml::Mapping` allowed any [`Value`] as
//!   a key. The 99 % case (configuration files, RPC payloads) is
//!   string-keyed and works unchanged. If your code constructs a
//!   `Mapping` with non-string keys, switch to noyalib's
//!   [`crate::MappingAny`] directly — the shim does not re-export
//!   it under the `Mapping` name to keep type errors localised.
//!
//! # Migrating an in-flight `serde_yaml::Value`
//!
//! Mid-migration codebases sometimes still hold an upstream
//! `::serde_yaml::Value` produced by an un-migrated module. The
//! Serde data model is the universal translator: every
//! Serde-compatible value can be funnelled through
//! [`crate::from_value`] / [`crate::to_value`] without depending
//! on the upstream library.
//!
//! ```rust,ignore
//! // Upstream value in flight (un-migrated module hands you one).
//! let upstream: ::serde_yaml::Value = legacy_call();
//!
//! // Lower it into noyalib::Value via the Serde bridge — works
//! // because both ASTs implement `Serialize` / `Deserialize`.
//! let lowered: noyalib::Value = noyalib::to_value(&upstream)?;
//!
//! // Or go straight to a typed struct, skipping the Value AST.
//! let cfg: MyConfig = noyalib::from_value(&noyalib::to_value(&upstream)?)?;
//! ```
//!
//! Going the other direction is just as direct:
//!
//! ```rust,ignore
//! let lifted: ::serde_yaml::Value =
//!     ::serde_yaml::to_value(&my_noyalib_value)?;
//! ```
//!
//! Both directions cost one Serde round-trip — the same wall-clock
//! cost as a hand-written `From` impl on a representative
//! `Value` shape. The benefit: zero dependency on the archived
//! crate.
//!
//! # `Error` parity
//!
//! The `Error::location() -> Option<Location>` /
//! `Location::line()` / `Location::column()` / `Location::index()`
//! chain matches `serde_yaml`'s shape byte-for-byte (1-indexed
//! line and column, 0-indexed byte offset), so existing
//! diagnostic-emitting code that destructures these compiles
//! unchanged.
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

// ── Deserialization ──────────────────────────────────────────────────

/// Deserialize a YAML document into the target type.
///
/// Direct re-export of [`crate::from_str`] — same signature,
/// same behaviour.
///
/// # Examples
///
/// ```
/// use noyalib::compat::serde_yaml as syml;
/// let n: i32 = syml::from_str("42").unwrap();
/// assert_eq!(n, 42);
/// ```
pub fn from_str<T>(s: &str) -> Result<T>
where
    T: DeserializeOwned,
{
    crate::from_str(s)
}

/// Deserialize a YAML document from a byte slice.
///
/// # Examples
///
/// ```
/// use noyalib::compat::serde_yaml as syml;
/// let n: i32 = syml::from_slice(b"7").unwrap();
/// assert_eq!(n, 7);
/// ```
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
///
/// # Examples
///
/// ```
/// use noyalib::compat::serde_yaml as syml;
/// let bytes: &[u8] = b"port: 8080\n";
/// let m: std::collections::BTreeMap<String, u16> =
///     syml::from_reader(bytes).unwrap();
/// assert_eq!(m["port"], 8080);
/// ```
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
///
/// # Examples
///
/// ```
/// use noyalib::compat::serde_yaml as syml;
/// let v = syml::Value::Number(syml::Number::Integer(42));
/// let n: i32 = syml::from_value(v).unwrap();
/// assert_eq!(n, 42);
/// ```
pub fn from_value<T>(value: Value) -> Result<T>
where
    T: DeserializeOwned,
{
    crate::from_value(&value)
}

// ── Serialization ────────────────────────────────────────────────────

/// Serialize a typed value to a YAML string.
///
/// # Examples
///
/// ```
/// use noyalib::compat::serde_yaml as syml;
/// let s = syml::to_string(&42_i32).unwrap();
/// assert!(s.contains("42"));
/// ```
pub fn to_string<T>(value: &T) -> Result<String>
where
    T: Serialize,
{
    crate::to_string(value)
}

/// Serialize a typed value to any [`std::io::Write`] sink.
///
/// # Examples
///
/// ```
/// use noyalib::compat::serde_yaml as syml;
/// let mut buf: Vec<u8> = Vec::new();
/// syml::to_writer(&mut buf, &42_i32).unwrap();
/// assert!(!buf.is_empty());
/// ```
#[cfg(feature = "std")]
pub fn to_writer<W, T>(writer: W, value: &T) -> Result<()>
where
    W: std::io::Write,
    T: Serialize,
{
    crate::to_writer(writer, value)
}

/// Serialize a typed value to a [`Value`].
///
/// `serde_yaml::to_value` takes by value; noyalib takes by
/// reference. Accepts the `serde_yaml` shape and forwards.
///
/// # Examples
///
/// ```
/// use noyalib::compat::serde_yaml as syml;
/// let v = syml::to_value(42_i32).unwrap();
/// assert_eq!(v.as_i64(), Some(42));
/// ```
pub fn to_value<T>(value: T) -> Result<Value>
where
    T: Serialize,
{
    crate::to_value(&value)
}

// ── Multi-document streams ───────────────────────────────────────────
//
// `serde_yaml` exposed `Deserializer::from_str(s).into_iter::<T>()`
// for multi-document parsing. noyalib's nearest equivalent is
// `load_all_as`. We expose it under the `serde_yaml` name pattern.

/// Iterate every YAML document in a multi-document stream and
/// deserialize each into `T`. Mirrors the
/// `Deserializer::from_str` chained with `into_iter::<T>()` —
/// the typical `serde_yaml` multi-document idiom.
///
/// # Examples
///
/// ```
/// use noyalib::compat::serde_yaml as syml;
/// let yaml = "1\n---\n2\n---\n3\n";
/// let docs: Vec<i32> = syml::from_str_multi(yaml).unwrap();
/// assert_eq!(docs, vec![1, 2, 3]);
/// ```
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

    #[test]
    fn error_exposes_location_line_column() {
        // The `serde_yaml::Error::location()` → `Location::line()`
        // / `Location::column()` chain is the contract that any
        // diagnostic-emitting migrant relies on. noyalib's `Error`
        // exposes the exact same shape, 1-indexed.
        let err = from_str::<Value>("a: [unclosed").unwrap_err();
        let loc = err.location().expect("parse error must carry a location");
        assert!(loc.line() >= 1);
        assert!(loc.column() >= 1);
        let _: usize = loc.index();
    }
}
