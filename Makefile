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
#   make examples  — run all examples sequentially
#   make clean    — remove build artifacts

.PHONY: all check clippy test fmt deny doc miri sbom examples clean

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

examples:
	@for ex in basic collections enums nested value io_formats \
	           value_manipulation config serializer_config \
	           strict_parsing parser_config error_handling \
	           error_paths schema_validation anchors shared_anchors \
	           merge merge_keys multi_document spanned \
	           fmt_wrappers singleton_map custom_serialization; do \
	    printf "\033[90m%-25s\033[0m" "$$ex" ; \
	    if cargo run --example $$ex --quiet 2>/dev/null 1>/dev/null; then \
	        printf "\033[32m[ok]\033[0m\n" ; \
	    else \
	        printf "\033[31m[fail]\033[0m\n" ; \
	        exit 1; \
	    fi; \
	done
	@printf "\n\033[1;32mAll examples passed.\033[0m\n"

clean:
	cargo clean
