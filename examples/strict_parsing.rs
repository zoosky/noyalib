// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Strict parsing: strict_booleans, duplicate key policies, security limits.
//!
//! Run: `cargo run --example strict_parsing`

#[path = "support.rs"]
mod support;

use noyalib::{from_str_with_config, DuplicateKeyPolicy, ParserConfig, Value};

fn main() {
    support::header("noyalib -- strict_parsing");

    let yaml = "a: True\nb: FALSE\nc: true\nd: false\n";

    support::task("strict_booleans=false: True -> bool", || {
        let lenient = ParserConfig::new().strict_booleans(false);
        let v: Value = from_str_with_config(yaml, &lenient).unwrap();
        assert_eq!(v["a"], Value::Bool(true));
    });

    support::task("strict_booleans=true: True -> string, true -> bool", || {
        let strict = ParserConfig::new().strict_booleans(true);
        let v: Value = from_str_with_config(yaml, &strict).unwrap();
        assert_eq!(v["a"], Value::String("True".to_string()));
        assert_eq!(v["c"], Value::Bool(true));
    });

    let dup_yaml = "key: first\nkey: second\n";

    support::task("DuplicateKeyPolicy::Last", || {
        let last = ParserConfig::new().duplicate_key_policy(DuplicateKeyPolicy::Last);
        let v: Value = from_str_with_config(dup_yaml, &last).unwrap();
        assert_eq!(v["key"], Value::String("second".to_string()));
    });

    support::task("DuplicateKeyPolicy::First", || {
        let first = ParserConfig::new().duplicate_key_policy(DuplicateKeyPolicy::First);
        let v: Value = from_str_with_config(dup_yaml, &first).unwrap();
        assert_eq!(v["key"], Value::String("first".to_string()));
    });

    let _ = support::task_result("DuplicateKeyPolicy::Error (rejected)", || {
        let error = ParserConfig::new().duplicate_key_policy(DuplicateKeyPolicy::Error);
        let result: Result<Value, _> = from_str_with_config(dup_yaml, &error);
        assert!(result.is_err());
        result
    });

    let strict = ParserConfig::strict();

    support::task(
        &format!(
            "ParserConfig::strict (depth={}, doc={}B, aliases={})",
            strict.max_depth, strict.max_document_length, strict.max_alias_expansions
        ),
        || {},
    );

    let _ = support::task_result("depth=40 with max_depth=32: rejected", || {
        let mut deep = String::new();
        for i in 0..40 {
            for _ in 0..i {
                deep.push_str("  ");
            }
            deep.push_str("a:\n");
        }
        let result: Result<Value, _> = from_str_with_config(&deep, &strict);
        assert!(result.is_err());
        result
    });

    support::summary(7);
}
