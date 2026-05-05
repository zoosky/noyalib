// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! `noyalib-mcp` — Model Context Protocol server exposing
//! noyalib's lossless YAML editing to AI agents.
//!
//! Communicates via newline-delimited JSON-RPC 2.0 over stdio per
//! the MCP 2025-06 spec. Two tools:
//!
//! - `noyalib_get`: read the value at a path in a YAML file.
//! - `noyalib_set`: set the value at a path, preserving every
//!   untouched byte (comments, indentation, sibling entries).
//!
//! # Why this exists
//!
//! AI agents that edit YAML configuration today regex-replace and
//! corrupt comments / formatting. noyalib's CST does the edits
//! losslessly; this server is the protocol shim that lets Claude,
//! Cursor, Zed, and any other MCP-aware client drive that engine
//! safely.
//!
//! # Install + connect
//!
//! ```text
//! cargo install noyalib-mcp
//! claude mcp add noyalib /usr/local/bin/noyalib-mcp
//! ```
//!
//! All dispatch logic lives in the `noyalib_mcp` library crate so
//! it can be exercised by `cargo test` directly. This binary is
//! the stdio transport shim — read a line, hand it to
//! [`noyalib_mcp::handle_message`], write the reply if any.

#![forbid(unsafe_code)]
#![warn(missing_docs)]
// Opt-in coverage exclusion: when `NOYALIB_COVERAGE=1`, the main
// stdio loop is excluded from instrumentation. The loop is the
// integration shim around `noyalib_mcp::handle_message`; its logic
// is covered end-to-end by `tests/protocol.rs` (subprocess-driven
// JSON-RPC) and the I/O failure paths on stdin/stdout require
// broken pipes that pure-Rust unit tests cannot reproduce.
#![cfg_attr(noyalib_coverage, allow(unstable_features))]
#![cfg_attr(noyalib_coverage, feature(coverage_attribute))]

use noyalib_mcp::{handle_message, HandleOutcome};
use std::io::{self, BufRead, Write};

#[cfg_attr(noyalib_coverage, coverage(off))]
fn main() -> io::Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout().lock();
    let mut line = String::new();
    let mut handle = stdin.lock();

    loop {
        line.clear();
        let n = handle.read_line(&mut line)?;
        if n == 0 {
            return Ok(());
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        match handle_message(trimmed) {
            HandleOutcome::Reply(payload) => {
                writeln!(stdout, "{payload}")?;
                stdout.flush()?;
            }
            HandleOutcome::Silent => {}
        }
    }
}
