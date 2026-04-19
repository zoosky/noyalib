//! Multi-document YAML example for noyalib.
//!
//! Demonstrates parsing and working with multiple YAML documents in a single
//! file.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

#[path = "support.rs"]
mod support;

use noyalib::{load_all, load_all_as, try_load_all, Value};
use serde::Deserialize;

fn main() {
    support::header("noyalib -- multi_document");

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

    support::task_with_output("Using load_all to iterate documents", || {
        let mut lines = Vec::new();
        for (i, result) in load_all(yaml).unwrap().enumerate() {
            match result {
                Ok(doc) => {
                    lines.push(format!("Document {}: {doc:?}", i + 1));
                    if let Some(name) = doc.get("name") {
                        lines.push(format!("  Name: {}", name.as_str().unwrap_or("unknown")));
                    }
                }
                Err(e) => lines.push(format!("Error parsing document {}: {e}", i + 1)),
            }
        }
        lines
    });

    support::task_with_output("Using try_load_all for early error detection", || {
        let iter = try_load_all(yaml).unwrap();
        let mut lines = vec![
            format!("Found {} documents", iter.len()),
            format!("Is empty: {}", iter.is_empty()),
        ];

        for value in iter.flatten() {
            lines.push(format!("Document: {value:?}"));
        }
        lines
    });

    support::task_with_output("Using load_all_as with typed structs", || {
        #[derive(Debug, Deserialize)]
        struct Document {
            name: String,
            value: i32,
        }

        let documents: Vec<Document> = load_all_as(yaml).unwrap();
        documents
            .iter()
            .map(|doc| format!("Document: {} = {}", doc.name, doc.value))
            .collect()
    });

    support::task_with_output("Different document types", || {
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

        let mut lines = Vec::new();
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
                lines.push(format!("Document {} is a {type_name}: {doc:?}", i + 1));
            }
        }
        lines
    });

    support::summary(4);
}
