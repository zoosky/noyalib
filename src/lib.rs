//! # noyalib
//!
//! A YAML 1.2 library for Rust. Pure safe code. Full serde integration.
//!
//! ## Two APIs, one parser
//!
//! noyalib exposes two complementary surfaces over the same scanner
//! and strictness rules. Pick the one that matches your job:
//!
//! - **Data binding** — [`from_str`], [`to_string`], [`Value`],
//!   [`StreamingDeserializer`], [`borrowed::BorrowedValue`]. Read
//!   YAML into typed Rust data, write Rust data back to YAML. The
//!   round-trip travels through a `Value`/struct, so comments,
//!   blank lines, and the original whitespace are not preserved.
//!   Use this for config loaders, RPC payloads, and the 95% of YAML
//!   workloads that just want data.
//!
//! - **Tooling / automation** — [`cst::parse_document`],
//!   [`cst::parse_stream`], [`cst::Document`]. Read YAML into a
//!   side-table CST that reproduces the source byte-for-byte,
//!   targeted edits via `doc.set("path", "fragment")` rewrite only
//!   the touched span — comments, formatting, and sibling entries
//!   are left untouched. Use this when *what the user wrote* matters
//!   (Renovate-style version bumps, Kubernetes manifest patchers,
//!   formatters, schema-driven linters). See `examples/lossless_edit.rs`.
//!
//! ## Quick Start
//!
//! ```rust
//! use noyalib::{from_str, to_string};
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Debug, Serialize, Deserialize, PartialEq)]
//! struct Config {
//!     name: String,
//!     port: u16,
//!     features: Vec<String>,
//! }
//!
//! let yaml = "name: myapp\nport: 8080\nfeatures:\n  - auth\n  - api\n";
//! let config: Config = from_str(yaml).unwrap();
//! assert_eq!(config.name, "myapp");
//! assert_eq!(config.port, 8080);
//!
//! let output = to_string(&config).unwrap();
//! let roundtrip: Config = from_str(&output).unwrap();
//! assert_eq!(config, roundtrip);
//! ```
//!
//! ## Deserialization
//!
//! ```rust,no_run
//! # use noyalib::Value;
//! # let yaml = "key: value";
//! # let bytes = b"key: value";
//! # let file = std::io::Cursor::new(yaml);
//! # let value = Value::Null;
//! // From string, byte slice, reader, or Value
//! let v: Value = noyalib::from_str(yaml).unwrap();
//! let v: Value = noyalib::from_slice(bytes).unwrap();
//! let v: Value = noyalib::from_reader(file).unwrap();
//! let v: Value = noyalib::from_value(&value).unwrap();
//!
//! // With security limits
//! let config = noyalib::ParserConfig::strict();
//! let v: Value = noyalib::from_str_with_config(yaml, &config).unwrap();
//! ```
//!
//! ## Serialization
//!
//! ```rust,no_run
//! # use noyalib::Value;
//! # let value = Value::Null;
//! // To string, writer, or fmt::Write
//! let yaml: String = noyalib::to_string(&value).unwrap();
//! let mut buf = Vec::new();
//! noyalib::to_writer(&mut buf, &value).unwrap();
//! let mut s = String::new();
//! noyalib::to_fmt_writer(&mut s, &value).unwrap();
//!
//! // With custom config
//! let config = noyalib::SerializerConfig::new()
//!     .indent(4)
//!     .quote_all(true);
//! let yaml = noyalib::to_string_with_config(&value, &config).unwrap();
//! ```
//!
//! ## Highlights
//!
//! - **Pure Rust** — native YAML 1.2 scanner and parser. No C bindings. No FFI.
//! - **Zero `unsafe`** — `#![forbid(unsafe_code)]` enforced at compile time.
//! - **Fast** — 75% faster serialization, 50% faster deserialization than
//!   serde\_yaml\_ng. Streaming deserializer bypasses the Value AST.
//! - **Serde-native** — serialize and deserialize any `Serialize` /
//!   `Deserialize` type.
//! - **Ordered mappings** — [`IndexMap`](indexmap::IndexMap)-backed. Insertion
//!   order preserved.
//! - **Source spans** — [`Spanned<T>`] tracks exact line, column, and byte
//!   offset.
//! - **Hardened** — configurable depth, size, and alias limits. Billion-laughs
//!   safe.
//! - **100% YAML Test Suite** — 392/392 official test cases pass.
//! - **Zero-copy** — [`borrowed::BorrowedValue`] borrows strings from input.
//! - **Path queries** — `value.query("items[*].name")` with wildcards.
//! - **`no_std`** — works with `alloc` only (`default-features = false`).
//! - **`miette`** — optional rich terminal diagnostics (`--features miette`).

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

/// Internal prelude for no_std compatibility.
/// Provides String, Vec, Box, etc. from alloc when std is absent.
#[cfg(not(feature = "std"))]
pub(crate) mod prelude {
    pub(crate) use alloc::borrow::{Cow, ToOwned};
    pub(crate) use alloc::boxed::Box;
    pub(crate) use alloc::format;
    pub(crate) use alloc::string::{String, ToString};
    pub(crate) use alloc::sync::Arc;
    pub(crate) use alloc::vec;
    pub(crate) use alloc::vec::Vec;
    pub(crate) use core::fmt;
}

/// Internal prelude for std compatibility.
#[cfg(feature = "std")]
pub(crate) mod prelude {
    pub(crate) use std::borrow::{Cow, ToOwned};
    pub(crate) use std::boxed::Box;
    pub(crate) use std::fmt;
    pub(crate) use std::format;
    pub(crate) use std::string::{String, ToString};
    pub(crate) use std::sync::Arc;
    pub(crate) use std::vec;
    pub(crate) use std::vec::Vec;
}

mod anchors;
/// Internal RFC 4648 base64 codec for `!!binary` scalars.
mod base64;
/// Zero-copy YAML values that borrow from the input.
pub mod borrowed;
mod comments;
/// Drop-in compatibility shims for upstream YAML crates. Each shim
/// is gated behind its own feature flag so unused migration paths
/// add zero compile cost. See [`compat::serde_yaml`] for the
/// `serde_yaml` 0.9 surface.
pub mod compat;
/// Side-table CST for byte-faithful round-tripping with typed
/// path-targeted edits.
///
/// See `docs/design/green-tree.md` for the architectural plan. The
/// `Document` API depends on the parser's `SpanTree`, which lives
/// under the `std` feature.
#[cfg(feature = "std")]
pub mod cst;
mod de;
/// Spanned-to-miette diagnostic bridge (requires `miette` feature).
#[cfg(feature = "miette")]
#[cfg_attr(docsrs, doc(cfg(feature = "miette")))]
pub mod diagnostic;
/// Multi-document loading and iteration.
pub mod document;
mod error;
/// Formatting wrappers for per-value YAML output style control.
pub mod fmt;
mod parser;
mod path;
/// Robotics and scientific numeric types (requires `robotics` feature).
#[cfg(feature = "robotics")]
#[cfg_attr(docsrs, doc(cfg(feature = "robotics")))]
pub mod robotics;
mod schema;
/// JSON Schema codegen via [`schemars`] — derive
/// [`schemars::JsonSchema`] for a Rust type and call
/// [`schema_for`] / [`schema_for_yaml`] to obtain the schema as a
/// [`crate::Value`] or as YAML text. Requires the `schema` feature.
#[cfg(feature = "schema")]
#[cfg_attr(docsrs, doc(cfg(feature = "schema")))]
mod schema_codegen;
/// Schema *validation* — enforce a JSON Schema 2020-12 contract
/// against a parsed [`Value`]. Pairs with [`schema_codegen`].
/// Requires the `validate-schema` feature (which implies `schema`).
#[cfg(feature = "validate-schema")]
#[cfg_attr(docsrs, doc(cfg(feature = "validate-schema")))]
mod schema_validate;
mod ser;
/// SIMD-friendly multi-byte search primitives — Phase 4. Off by
/// default; enable the `simd` feature to expose `noyalib::simd`.
/// Future hot-path integrations route through this module so
/// non-SIMD environments stay on the byte-by-byte baseline.
#[cfg(feature = "simd")]
#[cfg_attr(docsrs, doc(cfg(feature = "simd")))]
pub mod simd;
pub(crate) mod span_context;
pub(crate) mod spanned;
mod streaming;
pub mod tag_registry;
/// Declarative post-deserialise validation via [`garde`] or [`validator`]
/// (requires the corresponding feature).
#[cfg(any(feature = "garde", feature = "validator"))]
#[cfg_attr(docsrs, doc(cfg(any(feature = "garde", feature = "validator"))))]
pub mod validated;
mod value;
pub mod with;

pub use anchors::{
    AnchorRegistry, ArcAnchor, ArcAnchorRegistry, ArcWeakAnchor, RcAnchor, RcWeakAnchor,
};
pub use comments::{load_comments, Comment, CommentKind};
#[cfg(feature = "std")]
pub use de::{from_reader, from_reader_with_config};
pub use de::{
    from_slice, from_slice_with_config, from_str, from_str_with_config, from_value, Deserializer,
    DuplicateKeyPolicy, ParserConfig,
};
pub use document::{load_all, load_all_as, load_all_with_config, try_load_all};
pub use error::{Error, Location, Result};
pub use fmt::{Commented, FlowMap, FlowSeq, FoldStr, FoldString, LitStr, LitString, SpaceAfter};
pub use path::Path;
pub use schema::{
    is_yaml_failsafe_compatible, is_yaml_json_compatible, validate_yaml_core_schema,
    validate_yaml_failsafe_schema, validate_yaml_json_schema,
};
#[cfg(feature = "schema")]
#[cfg_attr(docsrs, doc(cfg(feature = "schema")))]
pub use schema_codegen::{schema_for, schema_for_yaml, JsonSchema};
#[cfg(feature = "validate-schema")]
#[cfg_attr(docsrs, doc(cfg(feature = "validate-schema")))]
pub use schema_validate::{validate_against_schema, validate_against_schema_str};
pub use ser::{
    to_fmt_writer, to_fmt_writer_with_config, to_string, to_string_multi,
    to_string_multi_with_config, to_string_with_config, to_value, FlowStyle, ScalarStyle,
    Serializer, SerializerConfig,
};
#[cfg(feature = "std")]
pub use ser::{
    to_string_tracking_shared, to_string_tracking_shared_with_config, to_writer_tracking_shared,
    to_writer_tracking_shared_with_config,
};
#[cfg(feature = "std")]
pub use ser::{to_writer, to_writer_multi, to_writer_multi_with_config, to_writer_with_config};
pub use spanned::Spanned;
pub use streaming::StreamingDeserializer;
pub use tag_registry::TagRegistry;
#[cfg(feature = "garde")]
#[cfg_attr(docsrs, doc(cfg(feature = "garde")))]
pub use validated::Validated;
#[cfg(feature = "validator")]
#[cfg_attr(docsrs, doc(cfg(feature = "validator")))]
pub use validated::ValidatedValidator;
pub use value::{
    check_for_tag, nobang, Mapping, MappingAny, MaybeTag, Number, ParseNumberError, Sequence, Tag,
    TaggedValue, Value, ValueIndex,
};
