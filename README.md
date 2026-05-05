<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->

<p align="center">
  <img src="https://cloudcdn.pro/noyalib/v1/logos/noyalib.svg" alt="Noyalib logo" width="128" />
</p>

<h1 align="center">noyalib</h1>

<p align="center">
  A YAML 1.2 parser and serialiser for Rust, with full
  <code>serde</code> integration and zero <code>unsafe</code> code.
</p>

<p align="center">
  <a href="https://github.com/sebastienrousseau/noyalib/actions"><img src="https://img.shields.io/github/actions/workflow/status/sebastienrousseau/noyalib/ci.yml?style=for-the-badge&logo=github" alt="Build" /></a>
  <a href="https://crates.io/crates/noyalib"><img src="https://img.shields.io/crates/v/noyalib.svg?style=for-the-badge&color=fc8d62&logo=rust" alt="Crates.io" /></a>
  <a href="https://docs.rs/noyalib"><img src="https://img.shields.io/badge/docs.rs-noyalib-66c2a5?style=for-the-badge&labelColor=555555&logo=docs.rs" alt="Docs.rs" /></a>
  <a href="https://lib.rs/crates/noyalib"><img src="https://img.shields.io/badge/lib.rs-v0.0.1-orange.svg?style=for-the-badge" alt="lib.rs" /></a>
  <a href="https://api.securityscorecards.dev/projects/github.com/sebastienrousseau/noyalib"><img src="https://api.securityscorecards.dev/projects/github.com/sebastienrousseau/noyalib/badge" alt="OpenSSF Scorecard" /></a>
</p>

---

## Contents

- [Install](#install) — Cargo, source
- [Quick Start](#quick-start) — parse and serialise in ten lines
- [Why this approach?](#why-this-approach) — design rationale
- [Capabilities in 0.0.1](#capabilities-in-001) — release inventory
- [Two APIs, one parser](#two-apis-one-parser) — data binding vs. tooling
- [Tooling](#tooling) — `noyafmt`, `noyavalidate`, MCP, WASM
- [Ecosystem comparison](#ecosystem-comparison) — feature matrix
- [Benchmarks](#benchmarks) — measurements vs. other libraries
- [Features](#features) — module-level capability list
- [Library Usage](#library-usage) — deserialise, serialise, values, spans
- [Configuration](#configuration) — parser and serialiser options
- [Examples](#examples) — runnable example index
- [When not to use noyalib](#when-not-to-use-noyalib) — limitations
- [Development](#development) — make targets, fuzzing, CI
- [Security](#security) — guarantees and compliance
- [License](#license)

---

## Install

```toml
[dependencies]
noyalib = "0.0.1"
```

### `no_std` support

```toml
[dependencies]
noyalib = { version = "0.0.1", default-features = false }
```

Requires `alloc`. Core data binding (`from_str`, `to_string`, `Value`,
schemas) and the streaming deserializer all compile and run without
the standard library. `from_reader`, `to_writer`, the `Spanned<T>`
deserialization helper (which uses thread-local storage), and the CST
module require the `std` feature, which is enabled by default.

### Build from source

```bash
git clone https://github.com/sebastienrousseau/noyalib.git
cd noyalib
make          # check + clippy + test
```

Requires **Rust 1.75.0+** for the core feature set (`default-features
= false` and the standard `std` default). Optional features pull in
ergonomics deps that have themselves bumped past 1.75 — `miette`
→ backtrace 1.82+, `garde` → 1.84+, `validate-schema` /
`figment` → ICU chain 1.86+, `parallel` → rayon-core 1.80+. Use
those with a current stable toolchain; the core lib stays
buildable on the Ubuntu 24.04 LTS rustc-1.75 floor.

`rust-toolchain.toml` itself selects `stable` for local
development; the 1.75.0 floor on the core surface is enforced by
the dedicated `msrv-1-75-core` CI job (Ubuntu, no-default-features
+ default-features build paths).

### Cargo features

All optional integrations are off by default. Enable only what
the application needs.

| Feature | Pulls in | Adds | Documented in |
| :--- | :--- | :--- | :--- |
| `std` *(default)* | — | `from_reader`, `to_writer`, `Spanned<T>`, CST module | [Install](#install) |
| `miette` | `miette` 7 | Rich terminal diagnostics with source spans | [Library Usage](#library-usage), `examples/diagnostic.rs` |
| `schema` | `schemars`, `serde_json` | `JsonSchema` derive + `schema_for::<T>()` | [Capabilities in 0.0.1](#capabilities-in-001) |
| `validate-schema` | `schema` + `jsonschema` | `validate_against_schema`, `coerce_to_schema` | [Governance: schema-driven autofix](#governance-schema-driven-autofix) |
| `figment` | `figment` 0.10 | `noyalib::figment::Yaml` provider | `examples/figment.rs` |
| `garde` | `garde` 0.22 | `Validated<T>` wrapper | `examples/validation_garde.rs` |
| `validator` | `validator` 0.19 | `ValidatedValidator<T>` wrapper | `examples/validation_validator.rs` |
| `robotics` | — | `Degrees`, `Radians`, `StrictFloat` newtypes | `examples/robotics_polymorphism.rs` |
| `parallel` | `rayon` 1.10 | `noyalib::parallel::parse<T>` for `---`-separated streams | [Benchmarks](#benchmarks) |
| `simd` | — | `noyalib::simd::*` primitives + parser hot path | [Benchmarks](#benchmarks) |
| `nightly-simd` | `simd` (nightly toolchain) | `core::simd`-backed `StructuralIter` (32-byte chunks) | [Benchmarks](#benchmarks) |
| `compat-serde-yaml` | — | `noyalib::compat::serde_yaml` shim for migration | [When not to use noyalib](#when-not-to-use-noyalib) |
| `compare-saphyr` | `serde-saphyr` *(dev only)* | Cross-library bench comparison arms | `benches/comparison.rs` |
| `noyavalidate` | `std` + `miette` + `validate-schema` | The `noyavalidate` CLI binary | [Tooling](#tooling) |

```toml
# Example: rich diagnostics + schema validation
[dependencies]
noyalib = { version = "0.0.1", features = ["miette", "validate-schema"] }
```

---

## Quick Start

```rust
use noyalib::{from_str, to_string};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Config {
    name: String,
    port: u16,
    features: Vec<String>,
}

fn main() -> Result<(), noyalib::Error> {
    let yaml = "
name: myapp
port: 8080
features:
  - auth
  - api
";

    let config: Config = from_str(yaml)?;
    let output = to_string(&config)?;
    let roundtrip: Config = from_str(&output)?;
    assert_eq!(config, roundtrip);

    Ok(())
}
```

---

## Why this approach?

noyalib targets the niche `serde_yaml` / `serde_yml` / `libyml`
occupy — read YAML into typed Rust structs, write Rust structs back
as YAML — and is written from scratch against the YAML 1.2 spec.
The implementation passes the official YAML test suite at 406/406
with zero skips. It is not a fork of `serde_yaml`; the parser,
scanner, serialiser, and CST are independent code.

Two architectural choices motivate the rewrite:

1. **Streaming-first deserialise.** The default `from_str<T>` path
   walks parser events directly into the typed target. The
   `serde_yaml`-shaped pattern is `parse → Value →
   T::deserialize(&Value)`, which allocates every key, scalar, and
   nested mapping into an intermediate AST that the typed target
   then throws away. noyalib bypasses that AST when the caller
   asked for a typed `T`. The dynamic `Value` tree is still
   available when callers want it, but it is no longer in the hot
   path.

2. **`#![forbid(unsafe_code)]` at the workspace root.** There is no
   FFI to a C library, no raw-pointer dereferences, and no
   `unsafe` blocks in the parser, scanner, formatter, or CST. CI
   enforces the attribute on every push. Most popular Rust YAML
   crates wrap `libyaml` via C-FFI; the `unsafe` blocks involved
   are usually well-vetted, but their existence makes a
   security-conscious downstream audit meaningfully harder.

A few features built on top of those choices:

- **SIMD-accelerated structural discovery.** Stable Rust dispatches
  to `memchr` (SSE2 / NEON) for arity-1/2/3 needles and SWAR for
  arity-4+. With `nightly-simd` on, the structural-bitmask scanner
  widens to a 32-byte `Simd<u8, 32>` chunk and walks delimiters via
  `mask.trailing_zeros()` — the same shape that powers `simdjson`.
- **SWAR decimal parsing.** The plain-scalar integer resolver folds
  8 ASCII digits per `u64` cycle via three pair-wise multiply-add
  phases. ~2× faster than `<i64 as FromStr>::from_str` on big
  numbers.
- **Lossless CST.** A side-table green tree (`noyalib::cst::Document`)
  reproduces the source byte-for-byte and supports surgical edits.
  `doc.set("version", "0.0.2")` rewrites only the touched span;
  comments, indentation, and sibling entries are left alone.

The dependency tree is eight required crates: `serde`, `indexmap`,
`rustc-hash`, `itoa`, `ryu`, `memchr`, `smallvec`, `serde_ignored`.
**No archived or unmaintained crates appear in the graph** —
`serde_yaml` 0.9 (archived), `libyaml` (C-FFI), and `thiserror` are
all absent. `cargo audit`, `cargo deny`, and `cargo vet` are CI
gates on every push.

---

## Capabilities in 0.0.1

The 0.0.1 release covers a complete YAML 1.2 stack. See
[`CHANGELOG.md`](CHANGELOG.md) for the detailed inventory; the
table below groups the inventory by capability theme.

| Theme | Headline deliverables |
| :--- | :--- |
| Spec compliance | YAML 1.2 official test suite at 100% literal (406/406 pass, zero skips); YAML 1.1 opt-in compatibility for the "Norway problem"; multi-document streams |
| Migration from `serde_yaml` | `compat-serde-yaml` feature with name-for-name re-exports; `From`/`TryFrom` parity for `Value`/`Mapping`/`Number`; `Document::validate`; comment-aware reads via `load_comments` |
| Binary scalars | First-class `!!binary` tag; RFC 4648 base64 round-trip with `serde_bytes::ByteBuf`/`Bytes`; non-UTF-8 payloads supported |
| Flatten guard | `Spanned<Value>` in `#[serde(flatten)]` returns an actionable error pointing at the working alternative |
| Lossless editing | Side-table CST with byte-faithful round-trip; `Document::entry(path)` chainable mutable handle (12 methods); automatic indent detection (2/3/4-space) |
| Anchor management | `Document::anchors()` / `aliases()` / `aliases_of(name)`; `materialise_alias_at(byte_pos)` and `materialise_aliases_of(name)` for breaking aliases |
| Schema codegen | `schema` feature: `JsonSchema` derive re-export; `schema_for::<T>()` and `schema_for_yaml::<T>()`; honours `#[doc]`, `#[serde(default)]`, `#[serde(rename)]` |
| Schema validation | `validate-schema` feature: `validate_against_schema(value, schema)`; aggregated violations with RFC 6901 paths |
| Tooling | `noyavalidate` (with `--schema` and `--fix`), `noyafmt`, `noyalib-mcp`, `noyalib-wasm` |
| Performance | `noyalib::simd` primitives — `find_any_of`, `clean_prefix_len`, `ByteBitmap`; parser hot path integrated; ~58× and ~5.4× over byte-by-byte at arities 3 and 8 |
| Supply chain | SLSA L3 provenance, sigstore signing, OpenSSF Scorecard, REUSE.software 3.3 compliance, signed commits, `cargo-deny` / `cargo-vet` / `cargo-semver-checks` gates, differential and soak fuzz |

---

## Two APIs, one parser

noyalib exposes two complementary surfaces over the same scanner and strictness rules:

- **Data binding** — `from_str`, `to_string`, `Value`, `StreamingDeserializer`, `BorrowedValue`. Read YAML into typed Rust data, write Rust data back out. The round-trip travels through a `Value`/struct, so comments and exact whitespace are not preserved. Use this for config loaders, RPC payloads, and the 95% of YAML workloads that just want data.
- **Tooling / automation** — `noyalib::cst::parse_document`, `parse_stream`, and the `Document` handle. Read YAML into a side-table CST that reproduces the source byte-for-byte, then run targeted edits like `doc.set("version", "0.0.2")` — only the touched span is rewritten, every comment and the original indentation is left alone. Use this for Renovate-style version bumps, manifest patchers, formatters, and schema-driven linters. See `examples/lossless_edit.rs`.

The CST also lets a tool break aliases by inlining the anchored
content at every reference site — useful when a manifest needs to
become self-contained before being shipped to a system that does
not resolve YAML aliases:

```rust
use noyalib::cst::parse_document;

let yaml = "a: &shared 7\nb: *shared\nc: *shared\n";
let mut doc = parse_document(yaml).unwrap();

// Replace every `*shared` reference with the bytes of the
// anchored value. The `&shared` declaration stays in place.
let n = doc.materialise_aliases_of("shared").unwrap();
assert_eq!(n, 2);
assert!(!doc.to_string().contains('*'));
```

`Document::materialise_alias_at(byte_pos)` is the single-site
variant for callers that already know the alias's source position.

---

## Tooling

Built on top of the lossless CST:

- **`noyafmt`** — CLI formatter (and `noyalib::cst::format` API)
  that rewrites YAML into a canonical style while preserving
  comments and directives. Run `cargo run --bin noyafmt -- <file>`.
- **`noyavalidate`** — validates YAML syntax and, optionally, a
  JSON Schema 2020-12 contract. `--schema PATH` enforces the
  contract; `--fix` rewrites the input through the lossless CST
  formatter; both flags compose. Build with `cargo build
  --features noyavalidate`.
- **`noyalib-mcp`** — Model Context Protocol server (separate
  workspace member) exposing `parse`, `format`, `get`, `set`, and
  `validate` tools so an MCP-aware agent can manipulate YAML
  through a typed interface.
- **`noyalib-wasm`** — `wasm-bindgen` wrapper exposing the
  `Document` API to JavaScript / TypeScript. Lets browser-based
  YAML editors run the lossless edit path without leaving the
  browser.
- **`noyalib-lsp`** — Language Server Protocol implementation
  built on the lossless CST. Speaks the standard LSP wire format
  over stdio so any conforming editor can use it directly:

  ```bash
  # Build the server binary.
  cargo build -p noyalib-lsp --release
  # → target/release/noyalib-lsp
  ```

  - **Neovim** (`lspconfig`):

    ```lua
    require("lspconfig.configs").noyalib = {
      default_config = {
        cmd = { "noyalib-lsp" },
        filetypes = { "yaml" },
        root_dir = require("lspconfig.util").find_git_ancestor,
      },
    }
    require("lspconfig").noyalib.setup {}
    ```

  - **Zed** — add `"noyalib"` to the YAML `language_servers` list
    in `~/.config/zed/settings.json` and point the binary path at
    the build above.
  - **VS Code** — a published extension is on the roadmap; in the
    interim, any `vscode-languageclient`-shaped extension can spawn
    `noyalib-lsp` over stdio.

---

## Ecosystem comparison

How noyalib lines up against the other Rust YAML libraries it is
likely to be evaluated alongside. Cells reflect the published
state at the time of writing; corrections welcome via PR.

| | noyalib | serde\_yml | serde\_yaml\_ng | saphyr | yaml-rust2 | rust-yaml |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: |
| **YAML Test Suite** | 100% (406/406) | — | — | — | — | — |
| **Pure Rust** | Yes | No (C-FFI) | No (C-FFI) | Yes | Yes | Yes |
| **Zero `unsafe`** | Yes | No | No | Yes | Yes | Yes |
| **Serde integration** | Yes | Yes | Yes | Yes | No | Yes |
| **Streaming deser** | Yes | No | No | No | No | No |
| **`#![no_std]`** | Yes | No | No | No | No | No |
| **Zero-copy scalars** | Yes | No | No | No | No | Yes |
| **SIMD scanning** | Yes (memchr + bitmask) | No | No | No | No | No |
| **SWAR numeric parse** | Yes | No | No | No | No | No |
| **Parallel multi-doc** | Yes (`parallel::parse`) | No | No | No | No | No |
| **DoS hardened** | 7 budgets | Basic | Basic | Yes | No | Yes |
| **Pluggable policies** | Yes (`policy::Policy`) | No | No | No | No | No |
| **Secret interpolation** | Yes (`${VAR}`) | No | No | Yes | No | No |
| **CST manipulation** | Yes (`cst::Document`) | No | No | No | No | No |
| **Native LSP** | Yes (`noyalib-lsp`) | No | No | No | No | No |
| **MCP server** | Yes (`noyalib-mcp`) | No | No | No | No | No |
| **JSON Schema codegen** | Yes (`schema_for`) | No | No | No | No | No |
| **JSON Schema validate** | Yes (`validate_against_schema`) | No | No | No | No | No |
| **Schema-driven autofix** | Yes (`coerce_to_schema`) | No | No | No | No | No |
| **`miette` diagnostics** | Yes | No | No | No | No | No |
| **WASM** | 338 KB | No | No | No | No | No |
| **Source spans** | Yes | No | No | Yes | No | No |
| **YAML 1.1 compat** | Yes | Yes | Yes | No | Yes | No |
| **Serialization** | Yes | Yes | Yes | Yes | No | No |
| **Path queries** | `query("..name")` | No | No | No | No | No |
| **Zero-copy AST** | `BorrowedValue<'a>` | No | No | No | No | Partial |

---

## Benchmarks

Benchmarked on Apple M4, Rust 1.94 stable. All libraries compiled with `--release`.

### Deserialization throughput

| Library | Simple (3 fields) | Nested (20 fields) | Large (500 items) |
| :--- | ---: | ---: | ---: |
| **noyalib** | **1.51 us** | **9.93 us** | **0.89 ms** |
| yaml-rust2 | 2.08 us (1.4x) | 13.5 us (1.4x) | 1.23 ms (1.4x) |
| serde\_yaml\_ng | 2.82 us (1.9x) | 16.9 us (1.7x) | 1.48 ms (1.7x) |
| serde-saphyr | 3.29 us (2.2x) | 20.5 us (2.1x) | 1.84 ms (2.1x) |

### Typed deserialization (streaming, no Value AST)

| Library | Simple struct | Nested struct |
| :--- | ---: | ---: |
| **noyalib** | **1.34 us** | **7.67 us** |
| serde\_yaml\_ng | 2.39 us (1.8x) | 12.6 us (1.6x) |

### Serialization throughput

| Library | Simple (3 fields) | Nested (20 fields) |
| :--- | ---: | ---: |
| **noyalib** | **330 ns** | **2.54 us** |
| serde\_yaml\_ng | 1.43 us (4.3x) | 8.04 us (3.2x) |

### Roundtrip (deserialize + serialize)

| Library | Nested (20 fields) |
| :--- | ---: |
| **noyalib** | **12.7 us** |
| serde\_yaml\_ng | 25.5 us (2.0x) |

### SIMD structural-discovery throughput

How fast each library can find every YAML delimiter in a 1 MiB
real-shaped document. The structural-bitmask path replaces the
classical "find one delimiter at a time" pattern with a 32-byte
chunk that drains every delimiter via `mask.trailing_zeros()`
before reloading. (`benches/structural_bitmask.rs`)

| Path | 4 KiB | 64 KiB | 1 MiB | vs memchr loop |
| :--- | ---: | ---: | ---: | ---: |
| scalar (byte-by-byte baseline) | 13.0 us | 206 us | 3.33 ms | 0.86x |
| memchr + `find_any_of` loop | 11.3 us | 179 us | 2.89 ms | 1.0x |
| **`StructuralIter` (stable)** | **2.7 us** | **42.3 us** | **681 us** | **4.2x** |
| **`StructuralIter` (nightly-simd)** | **1.20 us** | **19.7 us** | **311 us** | **9.2x** |

`serde_yaml_ng` and `serde-saphyr` use byte-by-byte structural
discovery — they sit alongside the `scalar baseline` row and lose
to the 32-byte-bitmask path by an order of magnitude on the
1 MiB workload.

### SWAR decimal-integer parsing

Plain-scalar integer resolution via the SIMD-Within-A-Register
pipeline that folds 8 ASCII digits per `u64` cycle.
(`benches/numeric_parse.rs`)

| Width | stdlib `from_str` | **SWAR** | speedup |
| :--- | ---: | ---: | ---: |
| 8 digits | 8.12 ns | **3.74 ns** | **2.17x** |
| 19 digits | 22.0 ns | **9.25 ns** | **2.38x** |
| `i64::MAX` | 24.6 ns | **9.75 ns** | **2.52x** |
| Bulk parse 1000 ints | 7.93 us | **5.38 us** | **1.47x** |

### Parallel multi-document throughput

Linear scaling across CPU cores for `---`-separated streams
(telemetry logs, audit exports, Kubernetes-resource snapshots).
Pre-scan runs in `O(input_len)` on the main thread; the per-
document parse work distributes across the Rayon thread pool.
(`benches/streaming_vs_value.rs`, `benches/large_doc_soak.rs`)

```rust
// Single-threaded baseline:
let docs: Vec<MyType> = noyalib::load_all_as(yaml)?;

// Parallel (off by default — pulls Rayon under `parallel`
// feature). Drop-in replacement, scales near-linearly with cores
// on multi-document inputs:
let docs: Vec<MyType> = noyalib::parallel::parse(yaml)?;
```

Other Rust YAML libraries the comparison table below covers run
single-threaded.

### Architecture validation

| Capability | Measured Impact |
| :--- | :--- |
| Streaming deserializer (bypasses Value AST) | **30% faster** (14.0 vs 19.4 us) |
| `BorrowedValue<'a>` (zero-copy AST) | **18% faster** (16.0 vs 19.4 us) |
| Zero-copy scanner (`Cow::Borrowed`) | **12% fewer** allocations (6.3 vs 7.1 us) |
| Span-free path (`from_str` default) | **34% less** overhead (5.6 vs 8.5 us) |
| FxHasher for Mapping keys | Faster key insertion and lookup |
| SIMD scanning (`memchr`) | Faster delimiter search on large inputs |
| Path queries | `value.query("items[*].name")` with `*` and `..` |
| DoS rejection (billion laughs) | **<3 us** with `ParserConfig::strict()` |
| DoS rejection (deep nesting) | **<4 us** |

Reproduce: `cargo bench --bench comparison` and `cargo bench --bench architecture`.

### Project metrics

| Metric | Value |
| :--- | :--- |
| **Source** | 26,000+ lines across 25 modules |
| **Test suite** | 3,600+ tests + doc-tests + CLI smoke |
| **YAML Test Suite** | 100% literal compliance: 406/406 cases pass with zero skips |
| **Examples** | 50+ runnable examples + WASM demo |
| **Coverage** | 95%+ line coverage |
| **Dependencies** | 8 runtime + 7 optional (miette, garde, validator, schemars, serde_json, jsonschema, figment) |
| **WASM binary** | 338 KB (release, LTO) |
| **MSRV** | Rust 1.75.0 (core); newer for optional features |

---

## Features

| | |
| :--- | :--- |
| **Serde** | `from_str`, `from_slice`, `from_reader`, `to_string`, `to_writer`, `to_fmt_writer` -- all with `_with_config` variants. `to_value`, `from_value` for Value conversion. Multi-document: `load_all`, `load_all_as`, `to_string_multi`. Streaming deserializer bypasses Value AST for typed targets. |
| **Values** | 7-variant `Value` enum: Null, Bool, Number, String, Sequence, Mapping, Tagged. Path traversal via `get_path("server.host")`. Path queries via `query("items[*].name")` with wildcards (`*`) and recursive descent (`..`). Deep merge via `merge()` and `merge_concat()`. `MappingAny` for non-string keys. Zero-copy `BorrowedValue<'a>` borrows strings from input (18% faster). |
| **Spans** | `Spanned<T>` tracks line, column, and byte offset for every deserialized field. Serializes transparently as `T`. Span tracking is opt-in — disabled by default in `from_str` for zero overhead. For large documents, consider the memory impact of the span HashMap. |
| **Formatting** | Per-value output control: `FlowSeq<T>`, `FlowMap<T>`, `LitStr`, `FoldStr`, `Commented<T>`, `SpaceAfter<T>`. |
| **Enums** | `singleton_map`, `singleton_map_optional`, `singleton_map_recursive`, `singleton_map_with` -- custom key transforms (snake\_case, kebab-case, lowercase). |
| **Schemas** | Validate against YAML schema levels: `validate_yaml_failsafe_schema`, `validate_yaml_json_schema`, `validate_yaml_core_schema`. |
| **Anchors** | Anchors (`&`), aliases (`*`), and merge keys (`<<`). Smart pointer wrappers: `RcAnchor`, `ArcAnchor`, `RcWeakAnchor`, `ArcWeakAnchor`. |
| **Security** | 7 configurable limits in `ParserConfig`: depth, document size, alias expansions, mapping keys, sequence length, duplicate key policy, strict booleans. `ParserConfig::strict()` for untrusted input. Billion-laughs safe via `max_alias_expansions` with `saturating_add` overflow protection. |
| **Compat** | YAML 1.1 legacy boolean mode (`legacy_booleans`): resolves `yes`/`no`/`on`/`off`/`y`/`n` as booleans for Docker Compose, GitHub Actions, and other YAML 1.1 tooling. Solves the "Norway problem". |
| **WASM** | Compiles to `wasm32-unknown-unknown`. wasm-bindgen bindings: `parse()`, `stringify()`, `get_path()`, `validate_json()`, `merge()`. Browser demo included. |
| **Errors** | Source locations on all parse errors. `format_with_source()` renders rustc-style diagnostics with `-->` pointer. `#[track_caller]` on all Index panics. `miette::Diagnostic` integration included (`--features miette`) for rich terminal reports with error codes, actionable help text, and source spans. |
| **no\_std** | Full `#![no_std]` support with `alloc`. Use `default-features = false`. Core parsing (`from_str`, `to_string`, `Value`, schemas) works without `std`. I/O functions (`from_reader`, `to_writer`), `Spanned<T>` deserialization (TLS), and the CST module require the `std` feature. CI enforces `cargo check --no-default-features` on every push. |
| **CST editing** | Side-table CST (`noyalib::cst`) for byte-faithful round-tripping. `Document::set("server.port", "9090")` rewrites only the touched bytes; comments, blank lines, and sibling formatting survive. `Document::entry(path)` is the chainable mutable handle (12 methods, smart `items[0]` path composition). `Document::indent_unit()` detects 2-/3-/4-space conventions so inserts conform to the file's existing style. |
| **Anchors v2** | `Document::anchors()` / `aliases()` / `aliases_of(name)` enumerate every `&name` / `*name` lexeme in source order. `Document::materialise_alias_at(byte_pos)` and `materialise_aliases_of(name)` "break" an alias by inlining the anchored scalar's source bytes — leaves the alias site independent of future anchor edits. |
| **Schema codegen** | `schema` feature: derive `JsonSchema` (re-exported from `schemars`), then `schema_for::<T>() -> Result<Value>` or `schema_for_yaml::<T>() -> Result<String>` to emit the JSON Schema 2020-12 document. Honours `#[doc]`, `#[serde(default)]`, `#[serde(rename)]`, integer bounds, nested types via `$defs`. |
| **Schema validation** | `validate-schema` feature (implies `schema`): `validate_against_schema(value, schema) -> Result<()>` enforces a JSON Schema 2020-12 contract on parsed YAML. Multiple violations aggregated with RFC 6901 JSON-pointer paths. `validate_against_schema_str` is the raw-text convenience. |
| **Binary scalars** | First-class `!!binary` tag with RFC 4648 base64 round-trip. `serde_bytes::ByteBuf` / `Bytes` work end-to-end including non-UTF-8 payloads. |
| **`serde_yaml` shim** | `compat-serde-yaml` feature: name-for-name re-exports backed by noyalib-native types — **the unmaintained `serde_yaml` 0.9 crate is intentionally not a dependency**. Migrating in-flight `::serde_yaml::Value` from un-migrated modules flows through the Serde bridge: `noyalib::to_value(&upstream)?`. |
| **SIMD primitives** | `noyalib::simd::find_any_of` / `clean_prefix_len` / `SimdScanner` / `StructuralIter` / `ByteBitmap` / `parse_decimal_{u64,i64}`. Parser hot path routes through them for free; public for downstream scanner authors. **`StructuralIter`** delivers 4.2× stable / 9.2× nightly-simd vs the memchr loop on 1 MiB workloads. |
| **Parallel parsing** | `parallel` feature: `noyalib::parallel::parse<T>` and `noyalib::parallel::values` deserialise multi-document streams across the Rayon thread pool. Pre-scan in `O(input_len)`; per-document work parallelises naturally. Linear-with-cores on `---`-separated logs / audit dumps / Kubernetes snapshots. |
| **Pluggable policies** | `noyalib::policy::Policy` trait + `ParserConfig::with_policy(p)`. Built-ins: `DenyAnchors` (rejects `&name` / `*name` — billion-laughs guard), `DenyTags` (rejects custom tags), `MaxScalarLength(n)` (caps individual scalar size). Custom policies implement the trait. |
| **Schema autofix** | `validate-schema` feature: `coerce_to_schema(value, schema) -> Result<usize>` walks JSON Schema type-mismatch errors and rewrites string-shaped scalars into the schema's expected type when the parse succeeds. Solves the `port: "8080"` quoting slip-up automatically. Library engine behind `noyavalidate --fix`. |
| **Key interner** | `noyalib::interner::KeyInterner` — `&str → Arc<str>` deduplication for repeated-key workloads. Kubernetes-shaped streams with 20-byte keys × 10 000 records: footprint drops from ~200 KB of fresh allocations to ~20 B + Arc pointers. |

---

## Governance: schema-driven autofix

YAML written by hand often has type slips that strict validation
catches but humans don't notice — `port: "8080"` (string) where
the schema declares `port: integer`. noyalib ships a one-call
fix-up engine that rewrites these to match the schema:

```rust
use noyalib::{coerce_to_schema, from_str, schema_for, validate_against_schema, JsonSchema};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, JsonSchema)]
struct ServerConfig {
    /// Port the server binds on.
    port: u16,
    /// Hostname or IP literal.
    host: String,
}

let schema = schema_for::<ServerConfig>().unwrap();

// Hand-written YAML with the port quoted by mistake.
let mut data = from_str("port: \"8080\"\nhost: api.example.com\n").unwrap();

// Strict pass — fails because `port` is a string.
assert!(validate_against_schema(&data, &schema).is_err());

// Apply schema-driven coercions.
let n = coerce_to_schema(&mut data, &schema).unwrap();
assert_eq!(n, 1, "one fix expected");

// Re-validation passes — port is now an integer.
validate_against_schema(&data, &schema).unwrap();
```

Library engine behind `noyavalidate --fix`. Handles `String →
Integer`, `String → Number`, `String → Boolean` coercions; the
fix-loop iterates until convergence. Unparseable inputs (e.g.
`port: "abc"` against `type: integer`) are left in place so the
caller can surface the residue via a follow-up
`validate_against_schema` call.

---

## Policy enforcement (Safe YAML)

The pluggable [`policy::Policy`] trait + `ParserConfig::with_policy(p)`
lets you reject documents that violate organisational constraints
*at parse time*, before any data flows to downstream code. Built-in
policies cover the most common asks; custom policies implement the
trait directly.

```rust
use noyalib::policy::DenyAnchors;
use noyalib::{from_str_with_config, ParserConfig, Value};

let cfg = ParserConfig::new().with_policy(DenyAnchors);

// Anchors / aliases are the classical billion-laughs vector and a
// readability hazard in audited configs. With `DenyAnchors`
// registered, any document that defines `&name` or dereferences
// `*name` is rejected before deserialise begins.
let res: Result<Value, _> =
    from_str_with_config("key: &x 1\nval: *x\n", &cfg);
assert!(res.is_err());

// Anchor-free input passes through unchanged.
let v: Value = from_str_with_config("key: 1\nval: 1\n", &cfg).unwrap();
assert!(matches!(v, Value::Mapping(_)));
```

Built-ins: `DenyAnchors`, `DenyTags` (rejects custom tags while
preserving YAML 1.2 core tags), `MaxScalarLength(n)`. Compose
freely — `cfg.with_policy(DenyAnchors).with_policy(DenyTags)`
runs both in registration order.

[`policy::Policy`]: https://docs.rs/noyalib/latest/noyalib/policy/trait.Policy.html

---

## Strict deserialise (typo detection)

`noyalib::from_str_strict<T>` errors if the YAML carries any keys
the target type `T` doesn't declare — closing the silent-data-loss
gap when a config-key typo (e.g. `replicass: 3`) deserialises into
a struct whose `replicas` field stays at its `Default`:

```rust
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Config {
    port: u16,
    host: String,
}

let yaml = "port: 8080\nhost: api.example.com\nporrt: 9090\n";

// Lenient path silently drops `porrt`.
let cfg: Config = noyalib::from_str(yaml).unwrap();
assert_eq!(cfg.port, 8080);

// Strict path surfaces the typo as a typed error.
let err = noyalib::from_str_strict::<Config>(yaml).unwrap_err();
assert!(err.to_string().contains("porrt"));
```

The strict path walks nested structs — a typo at `server.unknown`
is reported with its parent path so the user sees exactly where
the bad key lives.

The same check is available on every input shape:

| Input shape | Lenient | Strict |
| :--- | :--- | :--- |
| `&str` | `from_str` | `from_str_strict` |
| `&[u8]` | `from_slice` | `from_slice_strict` |
| `impl io::Read` | `from_reader` | `from_reader_strict` |

The slice and reader variants share `from_str_strict`'s semantics —
they exist so callers that already hold bytes (a buffer, a
network frame, a `bytes::Bytes`) don't have to round-trip through
`String` to opt in.

---

## Compact list indentation

Two camps disagree on whether YAML lists under a mapping key
should indent (the default) or align with the key column. noyalib
ships both styles; opt into the compact form via
`SerializerConfig::compact_list_indent(true)`:

```rust
use noyalib::{to_string_with_config, SerializerConfig, Value};

let yaml = "items:\n  - a\n  - b\n";
let v: Value = noyalib::from_str(yaml).unwrap();

// Default — list items indented one level past the key.
let std = noyalib::to_string(&v).unwrap();
// Output:
//   items:
//     - a
//     - b

// Compact — list items align with the key column. Style preferred
// by Kubernetes manifests and GitHub Actions workflows.
let cfg = SerializerConfig::new().compact_list_indent(true);
let compact = to_string_with_config(&v, &cfg).unwrap();
// Output:
//   items:
//   - a
//   - b
```

---

## Key interning (memory footprint)

For Kubernetes-shaped streams that repeat the same keys
(`metadata`, `labels`, `name`, `apiVersion`, `selector`) thousands
of times, `noyalib::interner::KeyInterner` deduplicates the heap
allocations:

```rust
use noyalib::interner::KeyInterner;
use std::sync::Arc;

let mut interner = KeyInterner::new();
let a = interner.intern("metadata");
let b = interner.intern("metadata");
// Same `Arc` — second call returned a clone of the cached entry.
assert!(Arc::ptr_eq(&a, &b));
```

| Workload | Without interning | With `KeyInterner` |
| :--- | ---: | ---: |
| 20-byte key × 100 records | 2 KB heap | 20 B + 100 × 16 B (Arc clones) ≈ 1.6 KB |
| 20-byte key × 10 000 records | 200 KB | 20 B + 160 KB ≈ **20 B for the strings** |
| Distinct K8s key set × 1 000 manifests | ~520 KB | 13 distinct allocations + 13 000 × 16 B Arc clones |

The Mapping public API stays `String`-keyed for v0.0.1 — the
interner is the explicit opt-in primitive. A future major version
may switch the internal storage to `Arc<str>` and use the interner
transparently during parse.

---

## The "Norway problem"

YAML 1.1 resolved the bare scalar `no` as the boolean `false`. The
country code for **Norway** is `NO`, so a YAML 1.1 parser
silently rewrites `country: NO` to `country: false`. Real-world
data corruption.

**YAML 1.2's official fix** is to drop those bare-word booleans —
only `true` and `false` count. noyalib defaults to YAML 1.2
strict semantics, so `country: NO` round-trips as the string
`"NO"`.

For migrating from Docker Compose / GitHub Actions / pre-1.2 YAML
toolchains that *expect* the legacy behaviour, opt back in via
`legacy_booleans`:

```rust
use noyalib::{from_str_with_config, ParserConfig, Value};

// Default (YAML 1.2 strict): "NO" → string.
let v: Value = noyalib::from_str("country: NO\n").unwrap();
assert_eq!(v["country"].as_str(), Some("NO"));

// Opt-in legacy mode: "NO" → false. Use only when migrating
// from a toolchain that depended on the YAML 1.1 behaviour.
let cfg = ParserConfig::new().legacy_booleans(true);
let v: Value = from_str_with_config("country: NO\n", &cfg).unwrap();
assert_eq!(v["country"].as_bool(), Some(false));
```

Conversely, `strict_booleans = true` *tightens* the YAML 1.2
contract — only the lowercase forms (`true` / `false`) are
recognised; `True`, `TRUE`, `Yes`, etc. all stay strings. Use
this for schema-strict pipelines where boolean-shape ambiguity
is a contract violation.

---

## Library Usage

<details>
<summary><b>Deserialization</b></summary>

```rust
use noyalib::{from_str, from_slice, from_reader, from_value, ParserConfig};

// From string, byte slice, reader, or Value
let config: Config = from_str(yaml)?;
let config: Config = from_slice(bytes)?;
let config: Config = from_reader(file)?;
let config: Config = from_value(&value)?;

// With security limits
let parser = ParserConfig::strict();
let config: Config = noyalib::from_str_with_config(yaml, &parser)?;
let config: Config = noyalib::from_slice_with_config(bytes, &parser)?;
let config: Config = noyalib::from_reader_with_config(reader, &parser)?;
```

</details>

<details>
<summary><b>Serialization</b></summary>

```rust
use noyalib::{to_string, to_writer, to_fmt_writer, to_value, SerializerConfig};

// To string, io::Write, or fmt::Write
let yaml: String = to_string(&config)?;
to_writer(&mut file, &config)?;
let mut buf = String::new();
to_fmt_writer(&mut buf, &config)?;

// To Value
let value: noyalib::Value = to_value(&config)?;

// With custom config
let ser_config = SerializerConfig::new()
    .indent(4)
    .quote_all(true)
    .document_start(true);
let yaml = noyalib::to_string_with_config(&config, &ser_config)?;
```

</details>

<details>
<summary><b>Dynamic values</b></summary>

```rust
use noyalib::{from_str, Value};

let value: Value = from_str("
name: test
items:
  - one
  - two
settings:
  debug: true
")?;

// Field access
let name = value.get("name").and_then(|v| v.as_str());

// Path-based traversal
let debug = value.get_path("settings.debug");

// Sequence indexing
let first = value.get("items").and_then(|v| v.get(0));

// Missing keys return None (never panic)
assert!(value.get("nonexistent").is_none());
assert!(value.get_path("a.b.c").is_none());

// Path queries: wildcards and recursive descent
let all_names = value.query("items[*]");       // all items
let deep = value.query("..debug");              // find at any depth
```

</details>

<details>
<summary><b>Zero-copy borrowed values</b></summary>

```rust
use noyalib::borrowed::from_str_borrowed;

let yaml = "host: localhost\nport: 8080\n";
let value = from_str_borrowed(yaml).unwrap();

// Strings borrow directly from input — zero heap allocation
assert_eq!(value.as_mapping().unwrap().get("host").unwrap().as_str(), Some("localhost"));

// Convert to owned Value when needed
let owned = value.into_owned();
```

18% faster than `from_str::<Value>` on typical payloads. Aliases not supported in borrowed mode.

</details>

<details>
<summary><b>Source spans</b></summary>

```rust
use noyalib::{from_str, Spanned};
use serde::Deserialize;

#[derive(Deserialize)]
struct Config {
    port: Spanned<u16>,
}

let config: Config = from_str("port: 8080")?;
assert_eq!(config.port.value, 8080);
assert_eq!(config.port.start.line(), 1);
assert_eq!(config.port.start.column(), 6);
```

`Spanned<T>` serializes transparently as `T`.

</details>

<details>
<summary><b>Merge keys</b></summary>

```rust
use noyalib::{from_str, Value};

let yaml = "
defaults: &defaults
  timeout: 30
  retries: 3

production:
  <<: *defaults
  timeout: 60
";

let mut value: Value = from_str(yaml)?;
value.apply_merge()?;
// production -> {timeout: 60, retries: 3}
```

</details>

<details>
<summary><b>Multi-document streams</b></summary>

```rust
use noyalib::{load_all, to_string_multi};

let docs = load_all("---\na: 1\n---\nb: 2\n")?;
for doc in &docs {
    println!("{doc:?}");
}

let items: Vec<Config> = noyalib::load_all_as::<Config>(yaml)?;
let yaml = to_string_multi(&[config1, config2])?;
```

</details>

<details>
<summary><b>Enum serialization</b></summary>

```rust
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
enum Action { StartServer, StopServer }

#[derive(Serialize, Deserialize)]
struct Task {
    #[serde(with = "noyalib::with::singleton_map")]
    action: Action,
}
```

| Module | Purpose |
| :--- | :--- |
| `singleton_map` | Serialize enums as `{Variant: data}` |
| `singleton_map_optional` | Same, for `Option<Enum>` |
| `singleton_map_recursive` | Apply recursively to nested enums |
| `singleton_map_with` | Custom key transforms (snake\_case, kebab-case) |

</details>

<details>
<summary><b>Formatting wrappers</b></summary>

| Wrapper | Effect |
| :--- | :--- |
| `FlowSeq<T>` | Inline sequence: `[a, b, c]` |
| `FlowMap<T>` | Inline mapping: `{a: 1, b: 2}` |
| `LitStr` / `LitString` | Literal block scalar (`\|`) |
| `FoldStr` / `FoldString` | Folded block scalar (`>`) |
| `Commented<T>` | Attach a YAML comment |
| `SpaceAfter<T>` | Insert a blank line after the value |

</details>

---

## Configuration

<details>
<summary><b>Parser configuration</b></summary>

```rust
use noyalib::{from_str_with_config, ParserConfig, DuplicateKeyPolicy};

let config = ParserConfig::new()
    .max_depth(64)
    .max_document_length(1_000_000)
    .max_alias_expansions(1000)
    .max_mapping_keys(10_000)
    .max_sequence_length(10_000)
    .duplicate_key_policy(DuplicateKeyPolicy::Error)
    .strict_booleans(true);

let value: noyalib::Value = from_str_with_config(input, &config)?;
```

For maximum strictness, use `ParserConfig::strict()`.

</details>

<details>
<summary><b>Serializer configuration</b></summary>

```rust
use noyalib::{to_string_with_config, SerializerConfig, FlowStyle, ScalarStyle};

let config = SerializerConfig::new()
    .indent(4)
    .flow_style(FlowStyle::Auto)
    .scalar_style(ScalarStyle::DoubleQuoted)
    .quote_all(true)
    .document_start(true)
    .document_end(true)
    .block_scalars(true)
    .block_scalar_threshold(3);

let yaml = to_string_with_config(&value, &config)?;
```

</details>

---

## Examples

Run all examples:

```bash
cargo run --example all
```

<details>
<summary><b>All examples</b></summary>

| Category | Example | Purpose |
| :--- | :--- | :--- |
| **Core** | `hello` | Struct roundtrip |
| | `std` | Vec, HashMap |
| | `variants` | Enum strategies |
| | `deep` | Nested structures |
| | `dynamic` | Dynamic Value type |
| | `modify` | to\_value, from\_value, get\_path, MappingAny |
| | `tags` | Singleton map enums |
| **Spec** | `alias` | Anchors, aliases, merge keys |
| | `smart` | RcAnchor, ArcAnchor |
| | `overlay` | Value merging |
| | `inherit` | Merge key precedence |
| | `stream` | Multi-document streams |
| | `types` | Custom YAML tags |
| | `binary` | Large ints, .inf, .nan, hex/octal |
| **Security** | `strict` | strict\_booleans, DuplicateKeyPolicy |
| | `secure` | ParserConfig limits |
| | `schema` | Schema validation |
| | `env` | ${VAR} expansion |
| **DX** | `errors` | Error types, diagnostics |
| | `trace` | Path tracking |
| | `source` | Spanned\<T\> locations |
| | `style` | FlowSeq, FlowMap, Commented |
| **Advanced** | `emit` | SerializerConfig options |
| | `rename` | Custom key transforms |
| | `flatten` | serde flatten, untagged |
| | `bridge` | JSON <-> YAML interop |
| | `pipes` | from\_slice, from\_reader, to\_writer |
| | `global` | Configuration layering |
| **Future** | `portable` | WASM portability proof |
| | `mask` | Secret\<T\> redaction |
| | `patch` | Surgical YAML patching |
| | `suggest` | "Did you mean?" typo detection |
| | `schema_ext` | Self-documenting config schemas |
| **Deep Rust** | `untagged` | Polymorphic deserialization |
| | `borrow` | Zero-copy patterns |
| | `transcode` | Value-to-Value transcoding |
| | `comments` | Comment handling |
| | `async_io` | Async integration |
| | `recursive` | Self-referential types |
| **Platform** | `diagnostic` | miette::Diagnostic integration |
| | `nostd` | #![no\_std] compatibility guide |
| | `preserve` | CST preservation foundations |
| | `lossless_edit` | Renovate-style version bump (CST `Document::set`) |
| **Runtime** | `async_io` | Async integration (spawn\_blocking pattern) |
| | `recursive` | Self-referential types (trees, org charts) |
| **Bench** | `bench` | Performance overview |
| **Ecosystem** | `include` | `$include`-key modular config (Argo CD / JSON-Schema-style refs) |
| | `figment` | Layered defaults / YAML / env via the `figment` Provider |
| | `validation_garde` | Declarative validation through `garde` + `Validated<T>` |
| | `validation_validator` | Declarative validation through `validator` + `ValidatedValidator<T>` |
| | `diagnostic_path` | `serde_path_to_error` — pinpoint the offending nested key |
| | `robotics_polymorphism` | Tagged-enum dispatch with `Degrees` / `Radians` / `StrictFloat` |

</details>

---

## When not to use noyalib

A few cases where another tool fits better, listed because the
short answer is "we don't do that yet" rather than because of a
disagreement on priorities.

- **You need to round-trip comments through the data-binding API.**
  The YAML data model excludes comments by spec. The lossless CST
  (`noyalib::cst::Document`) preserves them byte-for-byte for the
  tooling path, but `from_str::<T>` → `to_string(&T)` does not.
  No Rust YAML library currently round-trips comments through a
  typed deserialise / serialise pair.
- **You need YAML 1.1-only behaviour, top to bottom.** noyalib
  defaults to YAML 1.2 strict semantics. The `legacy_booleans`
  opt-in covers the most common 1.1 idiom (`yes` / `no` / `on` /
  `off`), but a document that depends on the 1.1 type-resolution
  rules in deeper ways may not parse identically to a 1.1
  implementation. Use a 1.1 parser if that matches your contract.
- **You need a dependency-free YAML parser.** noyalib has eight
  required dependencies. `yaml-rust2` is a smaller surface if you
  do not need serde integration.
- **You need flow-style aliases on the borrowed path.**
  `BorrowedValue<'a>` borrows scalar bytes from the input but does
  not resolve YAML aliases (`*name`). Use the owned `Value` path
  when the document uses anchors.
- **You're paste-replacing `serde_yaml` 0.9 today and cannot edit
  any types.** The `compat-serde-yaml` shim covers the common
  surface, but a few rarely-used items (e.g. full coverage parity
  on `with::*`) may still need a small migration. See
  `examples/bridge.rs`.

If you hit a case that should be on this list, please open an
issue — that's how it gets fixed or moved into the supported set.

---

## Development

```bash
make              # check + clippy + test
make test         # run all tests
make clippy       # lint with Clippy
make fmt          # check formatting
make examples     # run all examples
make doc          # build API documentation
make deny         # supply-chain audit
make miri         # Miri memory checking (requires nightly)
make sbom         # generate software bill of materials
make clean        # remove build artifacts
```

### Fuzzing

```bash
cargo +nightly fuzz run fuzz_parse       # Arbitrary YAML parsing
cargo +nightly fuzz run fuzz_roundtrip   # Parse -> serialize -> re-parse
cargo +nightly fuzz run fuzz_from_value  # Value -> typed deserialization
cargo +nightly fuzz run fuzz_multi_doc   # Multi-document streams
cargo +nightly fuzz run fuzz_strict      # Tight security limits
```

Seed corpus included in `fuzz/corpus/seed/`.

### Miri (UB / aliasing / leak verification)

noyalib is `#![forbid(unsafe_code)]` so Miri does not police
noyalib's own code — every byte is checked at compile time. The
reason a Miri job exists is to verify the *interaction* with the
runtime dependencies (`indexmap`, `rustc-hash`, `ryu`, `itoa`,
`memchr`, `smallvec` — all of which use `unsafe` internally) is
sound, plus to validate that the SWAR decimal parser and the
structural-bitmask iterator behave correctly under simulated
big-endian targets.

```bash
make miri              # focused suite — parser, scanner, value, interner, simd
make miri-full         # full lib test suite under Miri (slow)
make miri-bigendian    # focused suite simulated on mips64 big-endian

# Or invoke the script directly:
./scripts/miri.sh                    # full focused suite
./scripts/miri.sh simd               # subset
MIRI_TARGET=mips64-unknown-linux-gnuabi64 ./scripts/miri.sh
```

The CI matrix runs the focused suite on every PR (`miri-focused`)
and the full + big-endian sweep on a weekly schedule
(`miri-full`).

### CI

| Workflow | Trigger | Purpose |
| :--- | :--- | :--- |
| `ci.yml` | push, PR | Clippy, fmt, test (3 OS x 3 toolchains), coverage, audit, cargo-deny |
| `docs.yml` | push to main | Build and deploy API docs to GitHub Pages |
| `release.yml` | tag `v*` | Validate, cross-verify, checksums, SBOM, GitHub Release, crates.io |
| `security.yml` | push, PR, weekly | Dependency review, CodeQL analysis |

See [CONTRIBUTING.md](CONTRIBUTING.md) for signed commits and PR guidelines.

---

## Security

YAML parsers are notorious attack surface — `libyaml`-based wrappers
have shipped multiple critical CVEs over the years (deserialisation
RCE in PyYAML, billion-laughs amplification in dozens of language
ecosystems, code-execution-via-tag in legacy Ruby YAML). noyalib's
posture is built around closing each of those vectors at the
*architectural* level, not via opt-in flags.

### RCE prevention (no arbitrary object instantiation)

**noyalib does not instantiate arbitrary objects via tags.** YAML's
custom-tag mechanism (`!Foo`, `!!python/object`) is the historical
RCE vector — a malicious tag can load arbitrary code in legacy
parsers. noyalib only deserialises into Rust types you've defined
at compile time, with `#[derive(Deserialize)]`. Custom tags are
either:

- Surfaced as `Value::Tagged(tag, inner)` — pure data, no code path
  invoked.
- Routed through an explicit `TagRegistry` you opt into — every
  recognised tag is one you've named.

There is no path from a parsed YAML document to running attacker-
chosen code. Period.

### Configurable resource budgets

The `ParserConfig::strict()` preset enforces seven **resource
budgets** that cap every dimension of input size — designed to
make billion-laughs and similar memory-amplification attacks
mathematically impossible:

| Budget | Default | `strict()` | Protects against |
| :--- | ---: | ---: | :--- |
| `max_depth` | 128 | 64 | Stack-blowing nested structures |
| `max_document_length` | 64 MiB | 1 MiB | Oversized payloads |
| `max_alias_expansions` | 1024 | 100 | **Billion-laughs amplification** |
| `max_mapping_keys` | 64 K | 1024 | Hash-collision DoS |
| `max_sequence_length` | 64 K | 1024 | Memory-spike DoS |
| `duplicate_key_policy` | `Last` | `Error` | Silent data loss |
| `strict_booleans` | off | on | The "Norway problem" |

Alias-byte accumulation uses `saturating_add` so a crafted input
that overflows a `usize` counter still triggers the limit cleanly
— no integer-wrap escape hatch.

### Secret redaction (`${VAR}` interpolation)

`Value::interpolate_properties` (see `examples/env.rs`) substitutes
`${name}` references inside string scalars from a property map.
The interpolator is deliberately **lossy by default for unknown
keys** — pair it with a `secrecy::Secret<T>` field to keep the
substituted value out of `Debug` / log output.

```rust
use noyalib::{from_str, Value};
use std::collections::HashMap;

let yaml = "db_url: ${DATABASE_URL}\napi_key: ${API_KEY}\n";
let mut map = HashMap::new();
map.insert("DATABASE_URL".into(), "postgres://...".into());
map.insert("API_KEY".into(), "redacted-token".into());

let mut value: Value = from_str(yaml).unwrap();
value.interpolate_properties(&map).unwrap();
// `api_key` now holds the substituted secret — wrap downstream
// in `secrecy::Secret<String>` to keep it out of Debug formatting.
```

### Compile- and runtime-safety guarantees

- `#![forbid(unsafe_code)]` across the **entire workspace** — no
  FFI, no raw-pointer dereferences, no `unsafe` blocks anywhere.
- `#[non_exhaustive]` on `ParserConfig`, `SerializerConfig`,
  `FlowStyle`, `ScalarStyle`, `Error` — adding a variant is not a
  semver break.
- `#[must_use]` on 83 query methods — silent result-discarding
  bugs caught at compile time.
- `#[track_caller]` on 13 Index/IndexMut panic paths — panic
  diagnostics point at *your* call site, not noyalib internals.
- Pluggable parser policies (`DenyAnchors`, `DenyTags`,
  `MaxScalarLength`) — opt into "Safe YAML" enforcement at parse
  time. See `noyalib::policy`.

### Supply chain

- `cargo audit` clean — zero advisories.
- `cargo deny` clean — license / advisory / ban / source checks.
- `cargo vet` clean — every dependency in the graph has a local
  audit or imported coverage from Mozilla / Google / Bytecode
  Alliance / Embark / ISRG audit sets.
- `OpenSSF Scorecard` tracked, badge in the header.
- `SLSA L3` build provenance + sigstore signing on every release.
- `REUSE.software` 3.3 compliant — every source file carries
  SPDX headers.
- Signed commits (SSH ed25519) enforced via CI.

### Notes

- Comment round-tripping for the data-binding API is not supported
  (YAML spec excludes comments from the data model). `Commented<T>`
  provides write-only comment injection. The CST API
  (`noyalib::cst::Document`) preserves comments byte-for-faithfully
  for the lossless-tooling path.

---

## License

Dual-licensed under [Apache 2.0](https://www.apache.org/licenses/LICENSE-2.0) or [MIT](https://opensource.org/licenses/MIT), at your option.

See [CHANGELOG.md](CHANGELOG.md) for release history.

<p align="right"><a href="#contents">Back to Top</a></p>
