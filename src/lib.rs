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

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]

mod anchors;
mod de;
/// Multi-document loading and iteration.
pub mod document;
mod error;
/// Formatting wrappers for per-value YAML output style control.
pub mod fmt;
mod parser;
mod path;
mod schema;
mod ser;
pub(crate) mod span_context;
pub(crate) mod spanned;
mod streaming;
mod value;
pub mod with;

pub use anchors::{ArcAnchor, ArcWeakAnchor, RcAnchor, RcWeakAnchor};
pub use de::{
    from_reader, from_reader_with_config, from_slice, from_slice_with_config, from_str,
    from_str_with_config, from_value, Deserializer, DuplicateKeyPolicy, ParserConfig,
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
    to_string_multi_with_config, to_string_with_config, to_value, to_writer, to_writer_multi,
    to_writer_multi_with_config, to_writer_with_config, FlowStyle, ScalarStyle, Serializer,
    SerializerConfig,
};
pub use spanned::Spanned;
pub use value::{
    check_for_tag, nobang, Mapping, MappingAny, MaybeTag, Number, ParseNumberError, Sequence, Tag,
    TaggedValue, Value, ValueIndex,
};
