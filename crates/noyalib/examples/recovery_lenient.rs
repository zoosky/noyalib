// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! `recovery::parse_lenient` — best-effort parsing for LSP / IDE
//! partial-document scenarios.
//!
//! Run with: `cargo run --example recovery_lenient --features recovery`

#[cfg(feature = "recovery")]
fn main() {
    use noyalib::recovery::parse_lenient;

    let half_typed = "\
config:
  name: noyalib
  version: 0.0.6
  features: [recovery, std
  # ^^ user is mid-typing — the flow sequence is unclosed
";

    let result = parse_lenient(half_typed);

    println!("is_complete: {}", result.is_complete);
    println!("errors:      {}", result.errors.len());
    for (i, err) in result.errors.iter().enumerate() {
        println!("  [{i}] {err}");
    }
    println!("recovered tree: {:?}", result.value);
}

#[cfg(not(feature = "recovery"))]
fn main() {
    eprintln!("This example requires the `recovery` feature.");
    eprintln!("Run with: cargo run --example recovery_lenient --features recovery");
}
