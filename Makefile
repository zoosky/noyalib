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
#   make miri     — run focused Miri suite via scripts/miri.sh (nightly)
#   make miri-full — run full lib test suite under Miri (slow)
#   make miri-bigendian — Miri-simulate a big-endian target (mips64)
#   make sbom     — generate software bill of materials
#   make notice   — generate NOTICE via cargo-about (license attribution)
#   make vendor   — `cargo vendor` the dep tree for offline builds
#   make msrv-per-crate — verify each crate compiles on its declared MSRV
#   make coverage-gap   — list files below the workspace coverage threshold
#   make examples  — run all examples sequentially
#   make clean    — remove build artifacts

.PHONY: all check clippy test fmt deny doc miri miri-full miri-bigendian sbom notice vendor vendor-build msrv-per-crate coverage-gap examples compliance clean

all: check clippy test

check:
	cargo check --all-features --all-targets

clippy:
	cargo clippy --all-features --all-targets

test:
	cargo test --all-features

compliance:
	cargo test --test yaml_compliance_report -- --nocapture

fmt:
	cargo fmt --check

deny:
	cargo deny check

doc:
	cargo doc --no-deps --all-features

miri:
	./scripts/miri.sh

miri-full:
	cargo +nightly miri test --lib

miri-bigendian:
	MIRI_TARGET=mips64-unknown-linux-gnuabi64 ./scripts/miri.sh

sbom:
	cargo tree --edges normal --prefix depth --format '{p} {l}' > SBOM.txt
	@echo "SBOM written to SBOM.txt"

# `cargo-about generate` — produces NOTICE listing every third-
# party crate noyalib redistributes plus its license text.
# `cargo-about` is auto-installed on demand if absent.
notice:
	@cargo about --version >/dev/null 2>&1 || cargo install cargo-about --locked
	cargo about generate -c about.toml -o NOTICE about.hbs
	@echo "NOTICE written; ship it inside every release tarball."

# `cargo vendor` for offline / air-gapped / FIPS-bound builds.
# Writes vendor/ then prints the .cargo/config.toml stanza needed
# to redirect `crates-io` to the vendored copy.
vendor:
	cargo vendor --versioned-dirs vendor
	@echo "Vendored to vendor/. Configure via:"
	@echo "  [source.crates-io]"
	@echo "  replace-with = \"vendored\""
	@echo "  [source.vendored]"
	@echo "  directory = \"vendor\""
	@echo "in .cargo/config.toml, then build with \`cargo build --offline\`."

# `vendor-build` — full sanity that the vendored tree builds
# offline. CI runs this on every PR via the `vendor-build` job in
# ci.yml so the offline path can never silently regress.
vendor-build: vendor
	@mkdir -p .cargo
	@printf '[source.crates-io]\nreplace-with = "vendored"\n[source.vendored]\ndirectory = "vendor"\n' > .cargo/config.vendor.toml
	CARGO_HOME=$$PWD/.cargo cargo build --workspace --all-features --offline --locked
	@echo "Offline build clean."

# `msrv-per-crate` — verify each crate compiles cleanly against
# its declared rust-version. Catches drift in satellite crates
# (lsp / mcp / wasm / cli) independently of the workspace floor.
msrv-per-crate:
	./scripts/msrv-per-crate.sh

# `coverage-gap` — print the per-file coverage report and flag
# every file below the workspace threshold (default 98 %).
# Runs cargo +nightly llvm-cov; takes ~2 min from a cold cache.
coverage-gap:
	./scripts/coverage-gap-report.sh

examples:
	@for ex in hello std variants deep dynamic modify tags \
	           alias smart overlay inherit stream types binary \
	           strict secure schema env \
	           errors trace source style \
	           emit rename flatten bridge pipes global \
	           portable mask patch suggest schema_ext \
	           untagged borrow transcode comments \
	           diagnostic nostd preserve \
	           replay registry scientific validation anchor_shared \
	           async_io recursive bench; do \
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
