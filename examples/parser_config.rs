//! Demonstrates `ParserConfig` for security limits.
//!
//! Run with: `cargo run --example parser_config`

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use noyalib::{from_str_with_config, DuplicateKeyPolicy, ParserConfig, Value};

fn main() {
    // Strict preset: reduced limits for untrusted input
    let strict = ParserConfig::strict();
    let safe_yaml = "key: value\n";
    match from_str_with_config::<Value>(safe_yaml, &strict) {
        Ok(v) => println!("Strict mode accepted: {:?}", v.get("key")),
        Err(e) => println!("Strict mode rejected: {e}"),
    }

    // Custom limits
    let config = ParserConfig::new()
        .max_depth(4)
        .max_document_length(1024)
        .max_alias_expansions(10)
        .max_mapping_keys(100)
        .max_sequence_length(100)
        .duplicate_key_policy(DuplicateKeyPolicy::Error);

    // Reject deeply nested input
    let deep = "a:\n  b:\n    c:\n      d:\n        e: too deep\n";
    match from_str_with_config::<Value>(deep, &config) {
        Ok(_) => println!("Accepted deep nesting"),
        Err(e) => println!("Rejected deep nesting: {e}"),
    }

    // Reject duplicate keys
    let dupes = "name: first\nname: second\n";
    match from_str_with_config::<Value>(dupes, &config) {
        Ok(_) => println!("Accepted duplicate keys"),
        Err(e) => println!("Rejected duplicate keys: {e}"),
    }
}
