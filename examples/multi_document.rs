//! Multi-document YAML example for noyalib.
//!
//! Demonstrates parsing and working with multiple YAML documents in a single
//! file.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use noyalib::{load_all, load_all_as, try_load_all, Value};
use serde::Deserialize;

fn main() -> Result<(), noyalib::Error> {
    println!("noyalib multi-document example\n");

    // Multi-document YAML (documents separated by ---)
    let yaml = r#"---
name: document1
value: 100
---
name: document2
value: 200
---
name: document3
value: 300
"#;

    // Using load_all to iterate over documents as Value
    println!("=== Using load_all ===");
    for (i, result) in load_all(yaml).unwrap().enumerate() {
        match result {
            Ok(doc) => {
                println!("Document {}: {:?}", i + 1, doc);
                if let Some(name) = doc.get("name") {
                    println!("  Name: {}", name.as_str().unwrap_or("unknown"));
                }
            }
            Err(e) => println!("Error parsing document {}: {}", i + 1, e),
        }
    }

    // Using try_load_all for early error detection
    println!("\n=== Using try_load_all ===");
    let iter = try_load_all(yaml)?;
    println!("Found {} documents", iter.len());
    println!("Is empty: {}", iter.is_empty());

    for value in iter.flatten() {
        println!("Document: {:?}", value);
    }

    // Using load_all_as for typed deserialization
    println!("\n=== Using load_all_as with typed structs ===");

    #[derive(Debug, Deserialize)]
    struct Document {
        name: String,
        value: i32,
    }

    let documents: Vec<Document> = load_all_as(yaml)?;
    for doc in &documents {
        println!("Document: {} = {}", doc.name, doc.value);
    }

    // Multi-document with different types
    println!("\n=== Different document types ===");
    let mixed_yaml = r#"---
42
---
hello world
---
- item1
- item2
- item3
---
key: value
nested:
  a: 1
  b: 2
"#;

    for (i, result) in load_all(mixed_yaml).unwrap().enumerate() {
        if let Ok(doc) = result {
            let type_name = match &doc {
                Value::Null => "null",
                Value::Bool(_) => "bool",
                Value::Number(_) => "number",
                Value::String(_) => "string",
                Value::Sequence(_) => "sequence",
                Value::Mapping(_) => "mapping",
                Value::Tagged(_) => "tagged",
            };
            println!("Document {} is a {}: {:?}", i + 1, type_name, doc);
        }
    }

    println!("\nMulti-document example completed successfully!");

    Ok(())
}
