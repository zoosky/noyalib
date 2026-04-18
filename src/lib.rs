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
//!     version: u32,
//! }
//!
//! let yaml = "name: myapp\nversion: 1\n";
//! let config: Config = from_str(yaml).unwrap();
//! assert_eq!(config.name, "myapp");
//!
//! let output = to_string(&config).unwrap();
//! assert!(output.contains("name: myapp"));
//! ```
//!
//! ## Highlights
//!
//! - **Pure Rust** — native YAML 1.2 scanner and parser. No C bindings. No FFI.
//! - **Zero `unsafe`** — `#![forbid(unsafe_code)]` enforced at compile time.
//! - **Serde-native** — serialize and deserialize any `Serialize` /
//!   `Deserialize` type.
//! - **Ordered mappings** — [`IndexMap`](indexmap::IndexMap)-backed. Insertion
//!   order preserved.
//! - **Source spans** — [`Spanned<T>`] tracks exact line, column, and byte
//!   offset.
//! - **Hardened** — configurable depth, size, and alias limits. Billion-laughs
//!   safe.
//! - **Three dependencies** — [`serde`], [`indexmap`], [`thiserror`]. That's
//!   it.

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]

mod anchors;
mod de;
mod error;
/// Formatting wrappers for per-value YAML output style control.
pub mod fmt;
pub mod loader;
mod parser;
mod path;
mod schema;
mod ser;
pub(crate) mod span_context;
pub(crate) mod spanned;
mod value;
pub mod with;

pub use anchors::{ArcAnchor, ArcWeakAnchor, RcAnchor, RcWeakAnchor};
pub use de::{
    from_reader, from_reader_with_config, from_slice, from_str, from_str_with_config, from_value,
    Deserializer, DuplicateKeyPolicy, ParserConfig,
};
pub use error::{Error, Location, Result};
pub use fmt::{Commented, FlowMap, FlowSeq, FoldStr, FoldString, LitStr, LitString, SpaceAfter};
pub use loader::{load_all, load_all_as, load_all_with_config, try_load_all};
pub use path::Path;
pub use schema::{
    is_failsafe_compatible, is_json_compatible, validate_core_schema, validate_failsafe_schema,
    validate_json_schema,
};
pub use ser::{
    to_string, to_string_multi, to_string_multi_with_config, to_string_with_config, to_value,
    to_writer, to_writer_multi, to_writer_multi_with_config, to_writer_with_config, FlowStyle,
    ScalarStyle, Serializer, SerializerConfig,
};
pub use spanned::Spanned;
pub use value::{
    check_for_tag, nobang, Mapping, MappingAny, MaybeTag, Number, ParseNumberError, Sequence, Tag,
    TaggedValue, Value, ValueIndex,
};
