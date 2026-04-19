//! Demonstrates YAML schema validation.
//!
//! Run with: `cargo run --example schema_validation`

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

#[path = "support.rs"]
mod support;

use noyalib::{from_str, validate_core_schema, validate_json_schema, Value};

fn main() {
    support::header("noyalib -- schema_validation");

    support::task_with_output("Validate against core schema", || {
        let yaml = r#"
name: noyalib
version: 1
enabled: true
tags:
  - yaml
  - serde
"#;

        let value: Value = from_str(yaml).unwrap();

        match validate_core_schema(&value) {
            Ok(()) => vec!["Core schema: valid".to_string()],
            Err(e) => vec![format!("Core schema: {e}")],
        }
    });

    support::task_with_output("Validate against JSON schema", || {
        let yaml = r#"
name: noyalib
version: 1
enabled: true
tags:
  - yaml
  - serde
"#;

        let value: Value = from_str(yaml).unwrap();

        match validate_json_schema(&value) {
            Ok(()) => vec!["JSON schema: valid".to_string()],
            Err(e) => vec![format!("JSON schema: {e}")],
        }
    });

    support::task_with_output("NaN rejected by JSON schema", || {
        let nan_yaml = "value: .nan\n";
        let nan_value: Value = from_str(nan_yaml).unwrap();

        match validate_json_schema(&nan_value) {
            Ok(()) => vec!["NaN in JSON schema: valid (unexpected)".to_string()],
            Err(e) => vec![format!("NaN in JSON schema: rejected -- {e}")],
        }
    });

    support::summary(3);
}
