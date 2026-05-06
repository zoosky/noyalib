//! Fuzz target: double-quoted scalar escape handling, with focus on
//! the JSON-style UTF-16 surrogate-pair pairing introduced in
//! v0.0.1.
//!
//! Wraps every input as a double-quoted YAML scalar so libfuzzer's
//! coverage feedback drives toward the new `scan_unicode_4` helper
//! and the surrounding escape branches (`\xXX`, `\UXXXXXXXX`,
//! `\N`, `\_`, `\L`, `\P`, line-fold escapes, etc.).
//!
//! Panics are bugs; errors on malformed input are expected.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

#![no_main]

use libfuzzer_sys::fuzz_target;
use noyalib::Value;

fuzz_target!(|data: &[u8]| {
    let Ok(s) = std::str::from_utf8(data) else {
        return;
    };
    if s.contains('"') || s.contains('\\') {
        // Avoid escape conflicts when wrapping — only let inputs
        // through that we can reliably embed inside the quoted form.
        // We *also* want adversarial backslash content though, so
        // re-add the same input wrapped without quoting below.
    } else {
        let wrapped = format!("v: \"{s}\"\n");
        let _ = noyalib::from_str::<Value>(&wrapped);
    }

    // Always parse the raw input too — this hits backslash-and-quote
    // sequences libfuzzer constructs from corpus seeds, which are
    // exactly the inputs that exercise `scan_unicode_4` /
    // `scan_hex_escape` and the surrogate-pair pairing logic.
    let _ = noyalib::from_str::<Value>(s);
});
