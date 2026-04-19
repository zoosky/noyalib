//! Native YAML 1.2 parser.
//!
//! This module provides a complete YAML 1.2 Core Schema parser that builds
//! `Value` trees directly, with full control over security limits, duplicate
//! key handling, and alias expansion tracking.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

mod events;
mod loader;
mod scanner;

pub(crate) use events::{Event, Parser};
pub(crate) use loader::ParseConfig;
pub(crate) use scanner::ScalarStyle;

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

/// Parse a single YAML document into a `Value` without building a `SpanTree`.
///
/// This is faster than [`parse_one`] when source-location tracking is not
/// needed (the common case for [`crate::from_str`]).
pub(crate) fn parse_one_value(input: &str, config: &ParseConfig) -> Result<Value> {
    loader::load_one_no_spans(input, config)
}
