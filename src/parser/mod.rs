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
pub(crate) use loader::{DuplicateKeyPolicy as InternalDuplicateKeyPolicy, ParseConfig};
pub(crate) use scanner::ScalarStyle;
pub(crate) use scanner::{
    RecordedToken, RecordedTokenKind, ScannedComment, Scanner, TokenKind, Trivia, TriviaKind,
};

/// Returns a default (zero) `Span` for use in synthesized events.
pub(crate) fn scanner_span_default() -> scanner::Span {
    scanner::Span::default()
}

use crate::error::Result;
use crate::prelude::*;
#[cfg(feature = "std")]
use crate::span_context::SpanTree;
use crate::value::Value;

/// Parse a YAML string into a list of `(Value, SpanTree)` documents.
#[cfg(feature = "std")]
pub(crate) fn parse(input: &str, config: &ParseConfig) -> Result<Vec<(Value, SpanTree)>> {
    let mut parser = Parser::new(input);
    loader::load(&mut parser, config, input)
}

/// Parse a single YAML document from a string.
#[cfg(feature = "std")]
pub(crate) fn parse_one(input: &str, config: &ParseConfig) -> Result<(Value, SpanTree)> {
    let mut parser = Parser::new(input);
    loader::load_one(&mut parser, config, input)
}

/// Parse a single YAML document into a `Value` without building a `SpanTree`.
///
/// Used only on `no_std` targets where `SpanTree` construction is
/// unavailable — the `std` build always uses the span-aware path.
#[cfg(not(feature = "std"))]
pub(crate) fn parse_one_value(input: &str, config: &ParseConfig) -> Result<Value> {
    loader::load_one_no_spans(input, config)
}

/// Parse all YAML documents into `Value`s without building `SpanTree`s.
#[cfg(not(feature = "std"))]
pub(crate) fn parse_all_values(input: &str, config: &ParseConfig) -> Result<Vec<Value>> {
    loader::load_all_no_spans(input, config)
}
