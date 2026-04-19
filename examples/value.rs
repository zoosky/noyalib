//! Value type example for noyalib.
//!
//! Demonstrates working with the dynamic Value type.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use noyalib::{from_str, to_string, Value};

fn main() -> Result<(), noyalib::Error> {
    println!("noyalib Value type example\n");

    // Parse YAML into a Value
    let yaml = r#"
name: my-config
version: 1
enabled: true
ratio: 3.125
tags:
  - alpha
  - beta
  - gamma
settings:
  timeout: 30
  retries: 3
  debug: false
"#;

    let value: Value = from_str(yaml)?;
    println!("Parsed Value: {:?}\n", value);

    // Access fields dynamically
    if let Some(name) = value.get("name") {
        println!("Name: {}", name.as_str().unwrap_or("unknown"));
    }

    if let Some(version) = value.get("version") {
        println!("Version: {}", version.as_i64().unwrap_or(0));
    }

    if let Some(enabled) = value.get("enabled") {
        println!("Enabled: {}", enabled.as_bool().unwrap_or(false));
    }

    if let Some(ratio) = value.get("ratio") {
        println!("Ratio: {}", ratio.as_f64().unwrap_or(0.0));
    }

    // Access nested values
    if let Some(tags) = value.get("tags") {
        println!("\nTags:");
        if let Some(seq) = tags.as_sequence() {
            for (i, tag) in seq.iter().enumerate() {
                println!("  [{}]: {}", i, tag.as_str().unwrap_or(""));
            }
        }
    }

    if let Some(settings) = value.get("settings") {
        println!("\nSettings:");
        if let Some(timeout) = settings.get("timeout") {
            println!("  timeout: {}", timeout.as_i64().unwrap_or(0));
        }
        if let Some(retries) = settings.get("retries") {
            println!("  retries: {}", retries.as_i64().unwrap_or(0));
        }
    }

    // Serialize Value back to YAML
    println!("\nSerialized back to YAML:");
    let output = to_string(&value)?;
    println!("{}", output);

    println!("Value type test passed!");

    Ok(())
}
