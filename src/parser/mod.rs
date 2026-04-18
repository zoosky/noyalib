//! Native YAML 1.2 parser.
//!
//! This module provides a complete YAML 1.2 Core Schema parser that builds
//! `Value` trees directly, with full control over security limits, duplicate
//! key handling, and alias expansion tracking.

mod events;
mod loader;
mod scanner;

pub(crate) use loader::ParseConfig;

use crate::error::Result;
use crate::span_context::SpanTree;
use crate::value::Value;

/// Parse a YAML string into a list of `(Value, SpanTree)` documents.
pub(crate) fn parse(input: &str, config: &ParseConfig) -> Result<Vec<(Value, SpanTree)>> {
    loader::load(input, config)
}

/// Parse a single YAML document from a string.
pub(crate) fn parse_one(input: &str, config: &ParseConfig) -> Result<(Value, SpanTree)> {
    loader::load_one(input, config)
}
