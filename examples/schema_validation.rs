//! Demonstrates YAML schema validation.
//!
//! Run with: `cargo run --example schema_validation`

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use noyalib::{from_str, validate_core_schema, validate_json_schema, Value};

fn main() -> Result<(), noyalib::Error> {
    let yaml = r#"
name: noyalib
version: 1
enabled: true
tags:
  - yaml
  - serde
"#;

    let value: Value = from_str(yaml)?;

    match validate_core_schema(&value) {
        Ok(()) => println!("Core schema: valid"),
        Err(e) => println!("Core schema: {e}"),
    }

    match validate_json_schema(&value) {
        Ok(()) => println!("JSON schema: valid"),
        Err(e) => println!("JSON schema: {e}"),
    }

    // NaN is valid in core but not in JSON schema
    let nan_yaml = "value: .nan\n";
    let nan_value: Value = from_str(nan_yaml)?;

    match validate_json_schema(&nan_value) {
        Ok(()) => println!("NaN in JSON schema: valid (unexpected)"),
        Err(e) => println!("NaN in JSON schema: rejected — {e}"),
    }

    Ok(())
}
