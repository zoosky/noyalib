//! Fuzz target: parse arbitrary bytes as YAML.
//!
//! Exercises the scanner, event parser, and loader on untrusted input.
//! Panics are bugs; errors are expected.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

#![no_main]

use libfuzzer_sys::fuzz_target;
use noyalib::Value;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        // Parse with default config — should never panic.
        let _ = noyalib::from_str::<Value>(s);
    }
});
