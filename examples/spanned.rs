//! Demonstrates `Spanned<T>` for tracking source locations.
//!
//! Run with: `cargo run --example spanned`

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use noyalib::{from_str, Spanned};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Config {
    host: Spanned<String>,
    port: Spanned<u16>,
}

fn main() -> Result<(), noyalib::Error> {
    let yaml = "host: localhost\nport: 8080\n";

    let config: Config = from_str(yaml)?;

    println!("host = {:?}", *config.host);
    println!(
        "  location: line {}, column {}, byte {}",
        config.host.start.line(),
        config.host.start.column(),
        config.host.start.index(),
    );

    println!("port = {}", *config.port);
    println!(
        "  location: line {}, column {}, byte {}",
        config.port.start.line(),
        config.port.start.column(),
        config.port.start.index(),
    );

    Ok(())
}
