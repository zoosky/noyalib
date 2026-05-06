//! Fuzz target: YAML 1.1 resolver mode.
//!
//! Exercises the legacy resolution table that
//! `ParserConfig::version(YamlVersion::V1_1)` activates as a bundle:
//! - `legacy_booleans` — `yes` / `no` / `on` / `off` → bool
//! - `legacy_octal_numbers` — bare `0`-prefix `0644` → octal
//! - `legacy_sexagesimal` — colon-separated `60:00` → base-60 int
//!
//! These resolution paths are not exercised by the default 1.2
//! corpus. Panics are bugs; errors are expected.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

#![no_main]

use libfuzzer_sys::fuzz_target;
use noyalib::{from_str_with_config, ParserConfig, Value, YamlVersion};

fuzz_target!(|data: &[u8]| {
    let Ok(s) = std::str::from_utf8(data) else {
        return;
    };

    // Whole-bundle 1.1 mode.
    let cfg_v11 = ParserConfig::new().version(YamlVersion::V1_1);
    let _ = from_str_with_config::<Value>(s, &cfg_v11);

    // Each individual legacy flag in isolation — overrides after
    // the version preset, exercising the fine-grained branches.
    let cfg_only_bools = ParserConfig::new()
        .version(YamlVersion::V1_1)
        .legacy_octal_numbers(false)
        .legacy_sexagesimal(false);
    let _ = from_str_with_config::<Value>(s, &cfg_only_bools);

    let cfg_only_octal = ParserConfig::new()
        .version(YamlVersion::V1_1)
        .legacy_booleans(false)
        .legacy_sexagesimal(false);
    let _ = from_str_with_config::<Value>(s, &cfg_only_octal);

    let cfg_only_sexa = ParserConfig::new()
        .version(YamlVersion::V1_1)
        .legacy_booleans(false)
        .legacy_octal_numbers(false);
    let _ = from_str_with_config::<Value>(s, &cfg_only_sexa);
});
