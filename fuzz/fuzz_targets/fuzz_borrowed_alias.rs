//! Fuzz target: borrowed-path alias resolution.
//!
//! Exercises the eager alias-resolution code introduced for v0.0.1
//! on the borrowed-`Value` path. Forces inputs likely to contain
//! anchors / aliases by prepending a small fixed prefix to every
//! arbitrary input, so libfuzzer's coverage feedback steers toward
//! the new code rather than getting lost in unrelated paths.
//!
//! Targets every documented branch:
//! - Scalar / sequence / mapping anchor capture
//! - Alias resolution into already-built tree
//! - Alias used as mapping key (string coercion)
//! - Non-scalar key-alias rejection
//! - Anchor-namespace reset between documents
//! - Bomb defence via `max_alias_expansions`
//!
//! Panics are bugs; errors are expected.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

#![no_main]

use libfuzzer_sys::fuzz_target;
use noyalib::borrowed::from_str_borrowed_with_config;
use noyalib::ParserConfig;

const ALIAS_PREFIX: &str = "anchors:\n  - &x ";

fuzz_target!(|data: &[u8]| {
    let Ok(suffix) = std::str::from_utf8(data) else {
        return;
    };

    // Tight bomb cap so anything trying to amplify hits the limit
    // instead of running for minutes. The goal is to exercise the
    // path, not measure throughput.
    let cfg = ParserConfig::new().max_alias_expansions(64);

    // Direct: parse the input as-is (covers the no-anchor path).
    let _ = from_str_borrowed_with_config(suffix, &cfg);

    // Spiked: inject an anchor + alias structure so libfuzzer's
    // coverage feedback can find the alias-resolution branches.
    let mut spiked = String::with_capacity(ALIAS_PREFIX.len() + suffix.len() + 32);
    spiked.push_str(ALIAS_PREFIX);
    spiked.push_str(suffix);
    spiked.push_str("\n  - *x\n");
    let _ = from_str_borrowed_with_config(&spiked, &cfg);
});
