//! Demonstrates `ParserConfig` for security limits.
//!
//! Run with: `cargo run --example parser_config`

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

#[path = "support.rs"]
mod support;

use noyalib::{from_str_with_config, DuplicateKeyPolicy, ParserConfig, Value};

fn main() {
    support::header("noyalib -- parser_config");

    support::task_with_output("Strict preset for untrusted input", || {
        let strict = ParserConfig::strict();
        let safe_yaml = "key: value\n";
        match from_str_with_config::<Value>(safe_yaml, &strict) {
            Ok(v) => vec![format!("Strict mode accepted: {:?}", v.get("key"))],
            Err(e) => vec![format!("Strict mode rejected: {e}")],
        }
    });

    support::task_with_output("Custom limits with max depth", || {
        let config = ParserConfig::new()
            .max_depth(4)
            .max_document_length(1024)
            .max_alias_expansions(10)
            .max_mapping_keys(100)
            .max_sequence_length(100)
            .duplicate_key_policy(DuplicateKeyPolicy::Error);

        let deep = "a:\n  b:\n    c:\n      d:\n        e: too deep\n";
        match from_str_with_config::<Value>(deep, &config) {
            Ok(_) => vec!["Accepted deep nesting".to_string()],
            Err(e) => vec![format!("Rejected deep nesting: {e}")],
        }
    });

    support::task_with_output("Reject duplicate keys", || {
        let config = ParserConfig::new()
            .max_depth(4)
            .max_document_length(1024)
            .max_alias_expansions(10)
            .max_mapping_keys(100)
            .max_sequence_length(100)
            .duplicate_key_policy(DuplicateKeyPolicy::Error);

        let dupes = "name: first\nname: second\n";
        match from_str_with_config::<Value>(dupes, &config) {
            Ok(_) => vec!["Accepted duplicate keys".to_string()],
            Err(e) => vec![format!("Rejected duplicate keys: {e}")],
        }
    });

    support::summary(3);
}
