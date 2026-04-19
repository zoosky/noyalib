//! Anchors and merge keys example for noyalib.
//!
//! Demonstrates parsing YAML with anchors (&), aliases (*), and merge keys
//! (<<).

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use std::collections::BTreeMap;

use noyalib::{from_str, Value};
use serde::Deserialize;

fn main() -> Result<(), noyalib::Error> {
    println!("noyalib anchors and merge keys example\n");

    // Example 1: Simple anchor and alias
    println!("=== Simple anchor and alias ===");
    let yaml = r#"
first: &value 42
second: *value
third: *value
"#;

    let parsed: BTreeMap<String, i32> = from_str(yaml)?;
    println!("Parsed: {:?}", parsed);
    assert_eq!(parsed.get("first"), Some(&42));
    assert_eq!(parsed.get("second"), Some(&42));
    assert_eq!(parsed.get("third"), Some(&42));

    // Example 2: Anchor with mapping
    println!("\n=== Anchor with mapping ===");
    let yaml = r#"
defaults: &defaults
  timeout: 30
  retries: 3
  debug: false

server1:
  <<: *defaults
  host: server1.example.com

server2:
  <<: *defaults
  host: server2.example.com
  timeout: 60
"#;

    let value: Value = from_str(yaml)?;

    println!("Server 1:");
    if let Some(s1) = value.get("server1") {
        println!("  host: {:?}", s1.get("host").and_then(|v| v.as_str()));
        println!(
            "  timeout: {:?}",
            s1.get("timeout").and_then(|v| v.as_i64())
        );
        println!(
            "  retries: {:?}",
            s1.get("retries").and_then(|v| v.as_i64())
        );
    }

    println!("\nServer 2:");
    if let Some(s2) = value.get("server2") {
        println!("  host: {:?}", s2.get("host").and_then(|v| v.as_str()));
        println!(
            "  timeout: {:?}",
            s2.get("timeout").and_then(|v| v.as_i64())
        ); // Overridden!
        println!(
            "  retries: {:?}",
            s2.get("retries").and_then(|v| v.as_i64())
        );
    }

    // Example 3: Typed deserialization with merge keys
    println!("\n=== Typed deserialization with merge keys ===");

    #[derive(Debug, Deserialize)]
    #[allow(dead_code)]
    struct ServerConfig {
        host: String,
        timeout: u32,
        retries: u32,
        #[serde(default)]
        debug: bool,
    }

    #[derive(Debug, Deserialize)]
    #[allow(dead_code)]
    struct Config {
        server1: ServerConfig,
        server2: ServerConfig,
    }

    // Note: We need to skip the 'defaults' key for typed deserialization
    let yaml = r#"
server1:
  host: server1.example.com
  timeout: 30
  retries: 3
  debug: false

server2:
  host: server2.example.com
  timeout: 60
  retries: 3
  debug: true
"#;

    let config: Config = from_str(yaml)?;
    println!("Server 1: {:?}", config.server1);
    println!("Server 2: {:?}", config.server2);

    // Example 4: Anchor with sequence
    println!("\n=== Anchor with sequence ===");
    let yaml = r#"
base_items: &items
  - apple
  - banana
  - cherry

list1: *items
list2: *items
"#;

    let value: Value = from_str(yaml)?;
    println!("Base items: {:?}", value.get("base_items"));
    println!("List 1: {:?}", value.get("list1"));
    println!("List 2: {:?}", value.get("list2"));

    // Example 5: Multiple merge sources
    println!("\n=== Multiple merge sources ===");
    let yaml = r#"
source1: &s1
  a: 1
  b: 2

source2: &s2
  c: 3
  d: 4

combined:
  <<: [*s1, *s2]
  e: 5
"#;

    let value: Value = from_str(yaml)?;
    if let Some(combined) = value.get("combined") {
        println!("Combined mapping:");
        println!("  a: {:?}", combined.get("a").and_then(|v| v.as_i64()));
        println!("  b: {:?}", combined.get("b").and_then(|v| v.as_i64()));
        println!("  c: {:?}", combined.get("c").and_then(|v| v.as_i64()));
        println!("  d: {:?}", combined.get("d").and_then(|v| v.as_i64()));
        println!("  e: {:?}", combined.get("e").and_then(|v| v.as_i64()));
    }

    // Example 6: Merge key precedence
    println!("\n=== Merge key precedence ===");
    let yaml = r#"
first: &first
  key: from_first

second: &second
  key: from_second

merged:
  <<: [*first, *second]
"#;

    let value: Value = from_str(yaml)?;
    if let Some(merged) = value.get("merged") {
        // First source in the array takes precedence
        println!(
            "Merged key value: {:?}",
            merged.get("key").and_then(|v| v.as_str())
        );
    }

    // Example 7: Nested merge
    println!("\n=== Nested merge ===");
    let yaml = r#"
base1: &base1
  level1:
    a: 1
    b: 2

base2: &base2
  <<: *base1
  level2: extra

final:
  <<: *base2
  level3: more
"#;

    let value: Value = from_str(yaml)?;
    if let Some(final_val) = value.get("final") {
        println!("Final has level1: {}", final_val.get("level1").is_some());
        println!(
            "Final level2: {:?}",
            final_val.get("level2").and_then(|v| v.as_str())
        );
        println!(
            "Final level3: {:?}",
            final_val.get("level3").and_then(|v| v.as_str())
        );
    }

    println!("\nAnchors and merge keys example completed successfully!");

    Ok(())
}
