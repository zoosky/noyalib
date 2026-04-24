//! # noyalib
//!
//! A YAML 1.2 library for Rust. Pure safe code. Full serde integration.
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
/// Zero-copy YAML values that borrow from the input.
pub mod borrowed;
mod comments;
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
mod ser;
#[cfg(feature = "std")]
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
    is_failsafe_compatible, is_json_compatible, validate_core_schema, validate_failsafe_schema,
    validate_json_schema,
};
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
