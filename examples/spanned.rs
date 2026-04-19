//! Demonstrates `Spanned<T>` for tracking source locations.
//!
//! Run with: `cargo run --example spanned`

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

#[path = "support.rs"]
mod support;

use noyalib::{from_str, Spanned};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Config {
    host: Spanned<String>,
    port: Spanned<u16>,
}

fn main() {
    support::header("noyalib -- spanned");

    support::task_with_output("Parse and inspect spanned values", || {
        let yaml = "host: localhost\nport: 8080\n";

        let config: Config = from_str(yaml).unwrap();

        vec![
            format!("host = {:?}", *config.host),
            format!(
                "  location: line {}, column {}, byte {}",
                config.host.start.line(),
                config.host.start.column(),
                config.host.start.index(),
            ),
            format!("port = {}", *config.port),
            format!(
                "  location: line {}, column {}, byte {}",
                config.port.start.line(),
                config.port.start.column(),
                config.port.start.index(),
            ),
        ]
    });

    support::summary(1);
}
