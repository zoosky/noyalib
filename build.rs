// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Build script: detect whether `rustc` is a nightly toolchain and
//! expose that as a `cfg(noyalib_nightly)` flag for the rest of
//! the crate. Used to gate `nightly-simd` so a user passing
//! `--all-features` on stable does not get a hard compile error
//! from the unstable `feature(portable_simd)` attribute.

fn main() {
    // Inform Cargo that the cfg names below are known —
    // suppresses the `unexpected_cfgs` lint introduced by Cargo 1.79.
    println!("cargo:rustc-check-cfg=cfg(noyalib_nightly)");
    println!("cargo:rustc-check-cfg=cfg(noyalib_coverage)");

    let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    if let Ok(output) = std::process::Command::new(rustc).arg("--version").output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        // `rustc --version` on nightly looks like:
        //   `rustc 1.94.0-nightly (abcdef0 2026-04-15)`
        if stdout.contains("nightly") || stdout.contains("-dev") {
            println!("cargo:rustc-cfg=noyalib_nightly");
        }
    }

    // Opt-in coverage annotations: when `NOYALIB_COVERAGE` is set
    // (typically by `cargo +nightly llvm-cov --cfg=noyalib_coverage`),
    // items annotated with `#[cfg_attr(noyalib_coverage,
    // coverage(off))]` are excluded from coverage instrumentation.
    // The flag is opt-in so non-coverage builds (which compile on
    // stable rustc) never see the unstable `coverage_attribute`
    // feature flag.
    if std::env::var_os("NOYALIB_COVERAGE").is_some() {
        println!("cargo:rustc-cfg=noyalib_coverage");
    }
    println!("cargo:rerun-if-env-changed=NOYALIB_COVERAGE");
}
