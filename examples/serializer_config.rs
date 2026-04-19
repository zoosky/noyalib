// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Serializer configuration: quote_all, flow styles, document markers.
//!
//! Run: `cargo run --example serializer_config`

use noyalib::{from_str, to_string_with_config, FlowStyle, ScalarStyle, SerializerConfig, Value};

fn done(msg: &str) {
    println!("  \x1b[32m+\x1b[0m {msg}");
}

fn main() {
    println!("\n  \x1b[1mnoyalib serializer config\x1b[0m\n");

    let yaml = "name: noyalib\ntags:\n  - yaml\n  - rust\n  - fast\n";
    let value: Value = from_str(yaml).unwrap();

    let _ = to_string_with_config(&value, &SerializerConfig::new()).unwrap();
    done("default (indent=2, block)");

    let _ = to_string_with_config(&value, &SerializerConfig::new().indent(4)).unwrap();
    done("indent=4");

    let _ = to_string_with_config(&value, &SerializerConfig::new().flow_style(FlowStyle::Flow))
        .unwrap();
    done("flow_style=Flow (JSON-like)");

    let _ = to_string_with_config(
        &value,
        &SerializerConfig::new().scalar_style(ScalarStyle::DoubleQuoted),
    )
    .unwrap();
    done("scalar_style=DoubleQuoted");

    let _ = to_string_with_config(&value, &SerializerConfig::new().quote_all(true)).unwrap();
    done("quote_all=true");

    let _ = to_string_with_config(
        &value,
        &SerializerConfig::new()
            .document_start(true)
            .document_end(true),
    )
    .unwrap();
    done("document_start + document_end markers");

    let docs = vec![Value::from(1), Value::from(2), Value::from(3)];
    let _ = noyalib::to_string_multi(&docs).unwrap();
    done("to_string_multi (3 documents)");

    println!("\n  \x1b[90mAll serializer configurations verified.\x1b[0m\n");
}
