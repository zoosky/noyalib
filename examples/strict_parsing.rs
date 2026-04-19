// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Strict parsing: strict_booleans, duplicate key policies, security limits.
//!
//! Run: `cargo run --example strict_parsing`

use noyalib::{from_str_with_config, DuplicateKeyPolicy, ParserConfig, Value};

fn done(msg: &str) {
    println!("  \x1b[32m+\x1b[0m {msg}");
}

fn main() {
    println!("\n  \x1b[1mnoyalib strict parsing\x1b[0m\n");

    let yaml = "a: True\nb: FALSE\nc: true\nd: false\n";

    let lenient = ParserConfig::new().strict_booleans(false);
    let v: Value = from_str_with_config(yaml, &lenient).unwrap();
    assert_eq!(v["a"], Value::Bool(true));
    done("strict_booleans=false: True -> bool");

    let strict = ParserConfig::new().strict_booleans(true);
    let v: Value = from_str_with_config(yaml, &strict).unwrap();
    assert_eq!(v["a"], Value::String("True".to_string()));
    assert_eq!(v["c"], Value::Bool(true));
    done("strict_booleans=true: True -> string, true -> bool");

    let yaml = "key: first\nkey: second\n";

    let last = ParserConfig::new().duplicate_key_policy(DuplicateKeyPolicy::Last);
    let v: Value = from_str_with_config(yaml, &last).unwrap();
    assert_eq!(v["key"], Value::String("second".to_string()));
    done("DuplicateKeyPolicy::Last");

    let first = ParserConfig::new().duplicate_key_policy(DuplicateKeyPolicy::First);
    let v: Value = from_str_with_config(yaml, &first).unwrap();
    assert_eq!(v["key"], Value::String("first".to_string()));
    done("DuplicateKeyPolicy::First");

    let error = ParserConfig::new().duplicate_key_policy(DuplicateKeyPolicy::Error);
    let result: Result<Value, _> = from_str_with_config(yaml, &error);
    assert!(result.is_err());
    done("DuplicateKeyPolicy::Error (rejected)");

    let strict = ParserConfig::strict();
    done(&format!(
        "ParserConfig::strict (depth={}, doc={}B, aliases={})",
        strict.max_depth, strict.max_document_length, strict.max_alias_expansions
    ));

    let mut deep = String::new();
    for i in 0..40 {
        for _ in 0..i {
            deep.push_str("  ");
        }
        deep.push_str("a:\n");
    }
    let result: Result<Value, _> = from_str_with_config(&deep, &strict);
    assert!(result.is_err());
    done("depth=40 with max_depth=32: rejected");

    println!("\n  \x1b[90mAll strict parsing modes verified.\x1b[0m\n");
}
