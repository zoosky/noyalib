//! Serialization configuration example for noyalib.
//!
//! Demonstrates using SerializerConfig to customize YAML output.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use noyalib::{to_string, to_string_with_config, SerializerConfig, Value};
use serde::Serialize;

#[derive(Debug, Serialize)]
struct ServerConfig {
    host: String,
    port: u16,
    description: String,
    features: Vec<String>,
}

fn main() -> Result<(), noyalib::Error> {
    println!("noyalib serialization config example\n");

    let config = ServerConfig {
        host: "localhost".to_string(),
        port: 8080,
        description:
            "This is a long description\nthat spans multiple lines\nfor demonstration purposes."
                .to_string(),
        features: vec![
            "authentication".to_string(),
            "rate-limiting".to_string(),
            "caching".to_string(),
        ],
    };

    // Default serialization
    println!("=== Default serialization ===");
    let yaml = to_string(&config)?;
    println!("{}", yaml);

    // With document markers
    println!("\n=== With document start/end markers ===");
    let config_with_markers = SerializerConfig::default()
        .document_start(true)
        .document_end(true);
    let yaml = to_string_with_config(&config, &config_with_markers)?;
    println!("{}", yaml);

    // With custom indent
    println!("\n=== With 4-space indent ===");
    let config_4_indent = SerializerConfig::default().indent(4);
    let yaml = to_string_with_config(&config, &config_4_indent)?;
    println!("{}", yaml);

    // With block scalars for multiline strings
    println!("\n=== With block scalars enabled ===");
    let config_block = SerializerConfig::default()
        .block_scalars(true)
        .block_scalar_threshold(1);
    let yaml = to_string_with_config(&config, &config_block)?;
    println!("{}", yaml);

    // Combined configuration
    println!("\n=== Combined configuration ===");
    let full_config = SerializerConfig::new()
        .indent(2)
        .document_start(true)
        .document_end(true)
        .block_scalars(true)
        .block_scalar_threshold(2);
    let yaml = to_string_with_config(&config, &full_config)?;
    println!("{}", yaml);

    // Using builder pattern for configuration
    println!("\n=== Using builder pattern ===");
    let builder_config = SerializerConfig::default().indent(3).document_start(true);
    let yaml = to_string_with_config(&config, &builder_config)?;
    println!("{}", yaml);

    // Demonstrating Value serialization with config
    println!("\n=== Value with custom config ===");
    let value = Value::from("A string with special chars: *anchor, &alias, #comment");
    let yaml = to_string_with_config(&value, &SerializerConfig::default())?;
    println!("Value: {}", yaml);

    println!("\nSerialization config example completed successfully!");

    Ok(())
}
