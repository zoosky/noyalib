// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Run all noyalib examples in sequence.
//!
//! Usage: `cargo run --example run_all`

use std::process::Command;
use std::time::Instant;

const EXAMPLES: &[&str] = &[
    // Core
    "hello",
    "std",
    "variants",
    "deep",
    "dynamic",
    "modify",
    "tags", // Spec
    "alias",
    "smart",
    "overlay",
    "inherit",
    "stream",
    "types",
    "binary",
    // Logic & Security
    "strict",
    "secure",
    "schema",
    "env", // DX
    "errors",
    "trace",
    "source",
    "style", // Advanced
    "emit",
    "rename",
    "flatten",
    "bridge",
    "pipes",
    "global",
    // Future-Proof
    "portable",
    "mask",
    "patch",
    "suggest",
    "schema_ext",
    // Deep Rust
    "untagged",
    "borrow",
    "transcode",
    "comments",
    // Platform
    "diagnostic",
    "nostd",
    "preserve",
    // Competitive Features
    "replay",
    "registry",
    "scientific",
    "validation",
    "anchor_shared",
    // Final
    "async_io",
    "recursive",
    // Bench (last — longest)
    "bench",
];

fn main() {
    println!("\n  \x1b[1mnoyalib examples\x1b[0m\n");

    let start = Instant::now();
    let mut passed = 0;
    let mut failed = 0;

    for name in EXAMPLES {
        print!("  \x1b[90m{name:<28}\x1b[0m");

        let result = Command::new("cargo")
            .args(["run", "--example", name, "--quiet"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();

        match result {
            Ok(status) if status.success() => {
                println!("\x1b[32mdone\x1b[0m");
                passed += 1;
            }
            _ => {
                println!("\x1b[31mfail\x1b[0m");
                failed += 1;
            }
        }
    }

    let elapsed = start.elapsed();
    println!();
    if failed == 0 {
        println!(
            "  \x1b[1;32m{passed} examples passed\x1b[0m \x1b[90m({:.1}s)\x1b[0m\n",
            elapsed.as_secs_f64()
        );
    } else {
        println!(
            "  \x1b[1;31m{failed} failed\x1b[0m, {passed} passed \x1b[90m({:.1}s)\x1b[0m\n",
            elapsed.as_secs_f64()
        );
        std::process::exit(1);
    }
}
