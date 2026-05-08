#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Noyalib
# SPDX-License-Identifier: MIT OR Apache-2.0
#
# scripts/pgo.sh — Profile-Guided Optimization build pipeline
# for the noyalib library and binaries.
#
# PGO is a compiler technique that builds a release binary in
# two passes:
#
#   Pass 1 (instrument):  compile with `-Cprofile-generate` so
#                         the binary writes per-block hit-count
#                         data to disk while running.
#   Pass 2 (use):         compile with `-Cprofile-use` so the
#                         compiler can lay out hot paths
#                         inline, cold paths off the fast
#                         track, and optimise branch ordering
#                         based on the *actual* execution
#                         profile rather than heuristics.
#
# Typical wins on `from_str::<Value>` and `cst::parse_document`
# are 5–15% on the hot paths (per the LLVM PGO
# documentation; numbers below are project-specific).
#
# The script drives the full pipeline:
#
#   1. Build instrumented binary.
#   2. Run a representative training workload (the
#      `bench_corpus/` fixtures, or a user-supplied YAML file).
#   3. Merge the per-process `*.profraw` files into a single
#      `merged.profdata` via `llvm-profdata`.
#   4. Build the optimised binary with `-Cprofile-use`.
#   5. Report on-disk size + a quick smoke-bench.
#
# Requirements:
#   - rustc 1.75+ (PGO is stable since 1.45 but rustc-llvm-tools
#     are required).
#   - llvm-profdata on PATH (ships with `rustup component add
#     llvm-tools-preview`).
#   - The target `noya-cli` binary must build under `--release`.
#
# Usage:
#   ./scripts/pgo.sh                       # uses the default training corpus
#   ./scripts/pgo.sh path/to/train.yaml    # custom training input
#   PGO_TARGET=noya-cli ./scripts/pgo.sh   # override the trained binary

set -euo pipefail

# ── Configuration ────────────────────────────────────────────────
PGO_TARGET="${PGO_TARGET:-noyafmt}"
PGO_DATA_DIR="${PGO_DATA_DIR:-target/pgo-data}"
TRAIN_INPUT="${1:-}"

# ── Toolchain checks ─────────────────────────────────────────────
if ! command -v llvm-profdata >/dev/null 2>&1; then
  # Try the rustup-shipped variant.
  RUSTC_VER="$(rustc --version | awk '{print $2}')"
  HOST_TRIPLE="$(rustc --print host-tuple)"
  CANDIDATE="$HOME/.rustup/toolchains/${RUSTC_VER}-${HOST_TRIPLE}/lib/rustlib/${HOST_TRIPLE}/bin/llvm-profdata"
  if [ -x "$CANDIDATE" ]; then
    LLVM_PROFDATA="$CANDIDATE"
  else
    echo "✗ llvm-profdata not found on PATH or rustup toolchain"
    echo "  install via:  rustup component add llvm-tools-preview"
    exit 1
  fi
else
  LLVM_PROFDATA="llvm-profdata"
fi

mkdir -p "$PGO_DATA_DIR"

# ── Pass 1: instrument ───────────────────────────────────────────
echo "::group::PGO pass 1 — instrumented build"
rm -rf "$PGO_DATA_DIR"
mkdir -p "$PGO_DATA_DIR"
RUSTFLAGS="-Cprofile-generate=${PWD}/${PGO_DATA_DIR}" \
  cargo build --release --bin "$PGO_TARGET"
echo "::endgroup::"

# ── Train ────────────────────────────────────────────────────────
echo "::group::PGO training run"
TRAIN_BIN="target/release/$PGO_TARGET"
if [ ! -x "$TRAIN_BIN" ]; then
  echo "✗ instrumented binary not found at $TRAIN_BIN"
  exit 1
fi

if [ -n "$TRAIN_INPUT" ] && [ -f "$TRAIN_INPUT" ]; then
  echo "Training on $TRAIN_INPUT"
  "$TRAIN_BIN" "$TRAIN_INPUT" >/dev/null
else
  # Default: drive the binary against every YAML under the test
  # suite + benches/fixtures. Skips files that aren't
  # well-formed (the spec test suite includes negative cases).
  echo "Training on YAML test suite + benches/fixtures (default corpus)"
  for f in \
      crates/noyalib/tests/yaml-test-suite/*.yaml \
      crates/noyalib/benches/fixtures/*.yaml; do
    [ -f "$f" ] || continue
    "$TRAIN_BIN" "$f" >/dev/null 2>&1 || true
  done
fi
echo "::endgroup::"

# ── Merge profile data ───────────────────────────────────────────
echo "::group::Merge .profraw → merged.profdata"
"$LLVM_PROFDATA" merge -o "$PGO_DATA_DIR/merged.profdata" "$PGO_DATA_DIR"
echo "merged: $(ls -la "$PGO_DATA_DIR/merged.profdata")"
echo "::endgroup::"

# ── Pass 2: optimised build ──────────────────────────────────────
echo "::group::PGO pass 2 — optimised build"
RUSTFLAGS="-Cprofile-use=${PWD}/${PGO_DATA_DIR}/merged.profdata" \
  cargo build --release --bin "$PGO_TARGET"
echo "::endgroup::"

# ── Report ───────────────────────────────────────────────────────
FINAL_BIN="target/release/$PGO_TARGET"
echo
echo "✓ PGO build complete"
echo "  binary: $FINAL_BIN"
echo "  size:   $(ls -la "$FINAL_BIN" | awk '{print $5}') bytes"
echo
echo "Compare with a non-PGO release build via:"
echo "  hyperfine --warmup 3 \\"
echo "    'cargo run --release --bin $PGO_TARGET -- crates/noyalib/tests/yaml-test-suite/2AYT.yaml' \\"
echo "    './$FINAL_BIN crates/noyalib/tests/yaml-test-suite/2AYT.yaml'"
