// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Serializer configuration: quote_all, flow styles, document markers.
//!
//! Run: `cargo run --example serializer_config`

#[path = "support.rs"]
mod support;

use noyalib::{from_str, to_string_with_config, FlowStyle, ScalarStyle, SerializerConfig, Value};

fn main() {
    support::header("noyalib -- serializer_config");

    let yaml = "name: noyalib\ntags:\n  - yaml\n  - rust\n  - fast\n";
    let value: Value = from_str(yaml).unwrap();

    support::task("default (indent=2, block)", || {
        let _ = to_string_with_config(&value, &SerializerConfig::new()).unwrap();
    });

    support::task("indent=4", || {
        let _ = to_string_with_config(&value, &SerializerConfig::new().indent(4)).unwrap();
    });

    support::task("flow_style=Flow (JSON-like)", || {
        let _ = to_string_with_config(&value, &SerializerConfig::new().flow_style(FlowStyle::Flow))
            .unwrap();
    });

    support::task("scalar_style=DoubleQuoted", || {
        let _ = to_string_with_config(
            &value,
            &SerializerConfig::new().scalar_style(ScalarStyle::DoubleQuoted),
        )
        .unwrap();
    });

    support::task("quote_all=true", || {
        let _ = to_string_with_config(&value, &SerializerConfig::new().quote_all(true)).unwrap();
    });

    support::task("document_start + document_end markers", || {
        let _ = to_string_with_config(
            &value,
            &SerializerConfig::new()
                .document_start(true)
                .document_end(true),
        )
        .unwrap();
    });

    support::task("to_string_multi (3 documents)", || {
        let docs = vec![Value::from(1), Value::from(2), Value::from(3)];
        let _ = noyalib::to_string_multi(&docs).unwrap();
    });

    support::summary(7);
}
