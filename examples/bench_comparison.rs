//! Benchmark comparison example for noyalib.
//!
//! This example demonstrates how to perform simple benchmarking
//! of noyalib's parsing and serialization performance.
//!
//! Run with: cargo run --example benchmarks-comparison --release

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

#![allow(unused_results)]

use std::time::{Duration, Instant};

use noyalib::{from_str, to_string, Value};

/// Simple benchmark runner that measures execution time.
fn benchmark<F>(name: &str, iterations: usize, mut f: F) -> Duration
where
    F: FnMut(),
{
    // Warmup
    for _ in 0..iterations / 10 {
        f();
    }

    // Actual benchmark
    let start = Instant::now();
    for _ in 0..iterations {
        f();
    }
    let elapsed = start.elapsed();

    let per_op = elapsed / iterations as u32;
    println!(
        "{}: {} iterations in {:?} ({:?}/op)",
        name, iterations, elapsed, per_op
    );

    elapsed
}

fn main() {
    println!("=== noyalib Benchmark Comparison ===\n");

    // Test data of varying complexity
    let simple_yaml = r#"
name: test
version: 1
enabled: true
"#;

    let nested_yaml = r#"
server:
  host: localhost
  port: 8080
  ssl:
    enabled: true
    cert: /path/to/cert
    key: /path/to/key
database:
  host: db.example.com
  port: 5432
  credentials:
    username: admin
    password: secret
"#;

    let sequence_yaml = r#"
items:
  - name: item1
    value: 100
    tags:
      - tag1
      - tag2
  - name: item2
    value: 200
    tags:
      - tag3
  - name: item3
    value: 300
    tags:
      - tag4
      - tag5
      - tag6
"#;

    let large_mapping_yaml = (0..50)
        .map(|i| format!("key{i}: value{i}\n"))
        .collect::<String>();

    // Parsing benchmarks
    println!("--- Parsing Benchmarks ---\n");

    benchmark("Parse simple YAML", 10000, || {
        let _: Value = from_str(simple_yaml).unwrap();
    });

    benchmark("Parse nested YAML", 10000, || {
        let _: Value = from_str(nested_yaml).unwrap();
    });

    benchmark("Parse sequence YAML", 10000, || {
        let _: Value = from_str(sequence_yaml).unwrap();
    });

    benchmark("Parse large mapping YAML", 5000, || {
        let _: Value = from_str(&large_mapping_yaml).unwrap();
    });

    // Serialization benchmarks
    println!("\n--- Serialization Benchmarks ---\n");

    let simple_value: Value = from_str(simple_yaml).unwrap();
    let nested_value: Value = from_str(nested_yaml).unwrap();
    let sequence_value: Value = from_str(sequence_yaml).unwrap();
    let large_mapping_value: Value = from_str(&large_mapping_yaml).unwrap();

    benchmark("Serialize simple value", 10000, || {
        let _ = to_string(&simple_value).unwrap();
    });

    benchmark("Serialize nested value", 10000, || {
        let _ = to_string(&nested_value).unwrap();
    });

    benchmark("Serialize sequence value", 10000, || {
        let _ = to_string(&sequence_value).unwrap();
    });

    benchmark("Serialize large mapping", 5000, || {
        let _ = to_string(&large_mapping_value).unwrap();
    });

    // Roundtrip benchmarks
    println!("\n--- Roundtrip Benchmarks ---\n");

    benchmark("Roundtrip simple YAML", 5000, || {
        let value: Value = from_str(simple_yaml).unwrap();
        let _ = to_string(&value).unwrap();
    });

    benchmark("Roundtrip nested YAML", 5000, || {
        let value: Value = from_str(nested_yaml).unwrap();
        let _ = to_string(&value).unwrap();
    });

    // Value operations benchmarks
    println!("\n--- Value Operations Benchmarks ---\n");

    let nested: Value = from_str(nested_yaml).unwrap();

    benchmark("Value.get() single level", 100000, || {
        let _ = nested.get("server");
    });

    benchmark("Value.get_path() deep", 100000, || {
        let _ = nested.get_path("server.ssl.enabled");
    });

    // Memory efficiency demonstration
    println!("\n--- Memory Efficiency Demo ---\n");

    let start = Instant::now();
    let mut values: Vec<Value> = Vec::new();
    for i in 0..1000 {
        let yaml = format!("id: {i}\nname: item_{i}\nvalue: {}", i * 100);
        values.push(from_str(&yaml).unwrap());
    }
    println!("Parsed 1000 YAML documents in {:?}", start.elapsed());

    let start = Instant::now();
    let mut outputs: Vec<String> = Vec::new();
    for value in &values {
        outputs.push(to_string(value).unwrap());
    }
    println!("Serialized 1000 values in {:?}", start.elapsed());

    println!("\n=== Benchmark Complete ===");
    println!("\nNote: For proper benchmarking, use `cargo bench` with criterion.");
    println!("These simple benchmarks provide a quick performance overview.");
}
