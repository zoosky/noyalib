# POSIX-compatible Makefile for noyalib
# Works on macOS, Linux, and WSL without modification.
#
# Usage:
#   make          — run check + clippy + test (default)
#   make test     — run all tests
#   make clippy   — run clippy lints
#   make fmt      — check formatting
#   make deny     — run cargo-deny supply-chain checks
#   make doc      — build documentation
#   make miri     — run tests under Miri (requires nightly)
#   make sbom     — generate software bill of materials
#   make clean    — remove build artifacts

.PHONY: all check clippy test fmt deny doc miri sbom clean

all: check clippy test

check:
	cargo check --all-features --all-targets

clippy:
	cargo clippy --all-features --all-targets

test:
	cargo test --all-features

fmt:
	cargo fmt --check

deny:
	cargo deny check

doc:
	cargo doc --no-deps --all-features

miri:
	cargo +nightly miri test

sbom:
	cargo tree --edges normal --prefix depth --format '{p} {l}' > SBOM.txt
	@echo "SBOM written to SBOM.txt"

clean:
	cargo clean
