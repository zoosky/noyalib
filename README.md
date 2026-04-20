<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->

<p align="center">
  <img src="https://cloudcdn.pro/noyalib/v1/logos/noyalib.svg" alt="Noyalib logo" width="128" />
</p>

<h1 align="center">noyalib</h1>

<p align="center">
  <strong>Pure Rust YAML 1.2 library. Zero unsafe code. Full serde integration.</strong>
</p>

<p align="center">
  <a href="https://github.com/sebastienrousseau/noyalib/actions"><img src="https://img.shields.io/github/actions/workflow/status/sebastienrousseau/noyalib/ci.yml?style=for-the-badge&logo=github" alt="Build" /></a>
  <a href="https://crates.io/crates/noyalib"><img src="https://img.shields.io/crates/v/noyalib.svg?style=for-the-badge&color=fc8d62&logo=rust" alt="Crates.io" /></a>
  <a href="https://docs.rs/noyalib"><img src="https://img.shields.io/badge/docs.rs-noyalib-66c2a5?style=for-the-badge&labelColor=555555&logo=docs.rs" alt="Docs.rs" /></a>
  <a href="https://lib.rs/crates/noyalib"><img src="https://img.shields.io/badge/lib.rs-v0.0.1-orange.svg?style=for-the-badge" alt="lib.rs" /></a>
</p>

---

## Contents

- [Install](#install) -- Cargo, source
- [Quick Start](#quick-start) -- parse and serialize in 10 lines
- [Overview](#overview) -- what noyalib does
- [Benchmarks](#benchmarks) -- performance vs competitors
- [Features](#features) -- capability matrix
- [Library Usage](#library-usage) -- deserialization, serialization, values, spans
- [Configuration](#configuration) -- parser and serializer options
- [Examples](#examples) -- 45 branded examples
- [Development](#development) -- make targets, fuzzing, CI
- [Security](#security) -- safety guarantees and compliance
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

Requires `alloc`. Only `from_reader` and `to_writer` require the `std` feature.

### Build from source

```bash
git clone https://github.com/sebastienrousseau/noyalib.git
cd noyalib
make          # check + clippy + test
```

Requires **Rust 1.75.0+** (pinned in `rust-toolchain.toml`). Tested on Linux, macOS, and Windows.

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

## Overview

noyalib is designed to be the **no-compromise** YAML library for Rust: fast, safe, and hardened — simultaneously. Most libraries trade one for the other (fast but unsafe, or safe but slow). noyalib achieves all three.

- **Pure Rust** -- no C bindings, no FFI, no `unsafe` blocks
- **Streaming deserializer** -- bypasses the Value AST for typed targets
- **Zero-copy scanner** -- `Cow<'a, str>` scalars borrow directly from input
- **DoS hardened** -- 7 configurable limits, billion-laughs safe
- **`#![no_std]`** -- works with `alloc` only for embedded and WASM
- **`miette` diagnostics** -- rich terminal errors with source spans
- **201 KB WASM binary** -- runs in browsers via wasm-bindgen
- **5 runtime dependencies** -- serde, indexmap, thiserror, itoa, ryu
- **2,206 tests** -- unit, integration, doc-tests, property-based
- **45 branded examples** with animated spinner UI

---

## Ecosystem Comparison

noyalib competes across four categories of Rust YAML libraries:

| | noyalib | serde\_yml | serde\_yaml\_ng | saphyr | yaml-rust2 |
| :--- | :---: | :---: | :---: | :---: | :---: |
| **Pure Rust** | Yes | No (C-FFI) | No (C-FFI) | Yes | Yes |
| **Zero `unsafe`** | Yes | No | No | Yes | Yes |
| **Serde integration** | Yes | Yes | Yes | Yes | No |
| **Streaming deser** | Yes | No | No | No | No |
| **`#![no_std]`** | Yes | No | No | No | No |
| **Zero-copy scalars** | Yes | No | No | No | No |
| **DoS hardened** | 7 limits | Basic | Basic | Yes | No |
| **`miette` diagnostics** | Yes | No | No | No | No |
| **WASM** | 201 KB | No | No | No | No |
| **Source spans** | Yes | No | No | Yes | No |
| **YAML 1.1 compat** | Yes | Yes | Yes | No | Yes |

---

## Benchmarks

Benchmarked on Apple M4, Rust 1.94 stable. All libraries compiled with `--release`.

### Deserialization throughput

| Library | K8s payload (60 lines) | Plain scalars (16 fields) | Large (500 items) |
| :--- | ---: | ---: | ---: |
| **noyalib** | **18.0 us** | **6.65 us** | **0.83 ms** |
| yaml-rust2 | 27.7 us (1.5x) | — | 1.24 ms (1.5x) |
| serde\_yaml\_ng | 32.6 us (1.8x) | 12.3 us (1.9x) | 1.49 ms (1.8x) |
| serde-saphyr | 39.0 us (2.2x) | 14.7 us (2.2x) | — |

### Serialization throughput

| Library | Simple (3 fields) | Nested (20 fields) |
| :--- | ---: | ---: |
| **noyalib** | **358 ns** | **2.80 us** |
| serde\_yaml\_ng | 1.41 us (3.9x) | 8.32 us (3.0x) |

### Architecture validation

| Capability | Measured Impact |
| :--- | :--- |
| Streaming deserializer (typed targets) | 36% faster than Value AST path |
| Zero-copy scanner (`Cow::Borrowed`) | 26% fewer heap allocations |
| Span-free path (`from_str` default) | 77% less overhead vs span tracking |
| DoS rejection (billion laughs) | <3 us to reject with `ParserConfig::strict()` |
| DoS rejection (50-level nesting) | <3 us to reject |

Reproduce: `cargo bench --bench comparison` and `cargo bench --bench architecture`.

### Project metrics

| Metric | Value |
| :--- | :--- |
| **Source** | 23,866 lines across 22 modules |
| **Test suite** | 2,206 tests + 69 doc-tests |
| **Examples** | 45 branded examples + WASM demo |
| **Coverage** | 95.7% line coverage |
| **Dependencies** | 5 runtime + 1 optional (miette) |
| **WASM binary** | 201 KB (release, LTO) |
| **MSRV** | Rust 1.75.0 |

---

## Features

| | |
| :--- | :--- |
| **Serde** | `from_str`, `from_slice`, `from_reader`, `to_string`, `to_writer`, `to_fmt_writer` -- all with `_with_config` variants. `to_value`, `from_value` for Value conversion. Multi-document: `load_all`, `load_all_as`, `to_string_multi`. Streaming deserializer bypasses Value AST for typed targets. |
| **Values** | 7-variant `Value` enum: Null, Bool, Number, String, Sequence, Mapping, Tagged. Path traversal via `get_path("server.host")`. Deep merge via `merge()` and `merge_concat()`. `MappingAny` for non-string keys. |
| **Spans** | `Spanned<T>` tracks line, column, and byte offset for every deserialized field. Serializes transparently as `T`. |
| **Formatting** | Per-value output control: `FlowSeq<T>`, `FlowMap<T>`, `LitStr`, `FoldStr`, `Commented<T>`, `SpaceAfter<T>`. |
| **Enums** | `singleton_map`, `singleton_map_optional`, `singleton_map_recursive`, `singleton_map_with` -- custom key transforms (snake\_case, kebab-case, lowercase). |
| **Schemas** | Validate against YAML schema levels: `validate_failsafe_schema`, `validate_json_schema`, `validate_core_schema`. |
| **Anchors** | Anchors (`&`), aliases (`*`), and merge keys (`<<`). Smart pointer wrappers: `RcAnchor`, `ArcAnchor`, `RcWeakAnchor`, `ArcWeakAnchor`. |
| **Security** | 7 configurable limits in `ParserConfig`: depth, document size, alias expansions, mapping keys, sequence length, duplicate key policy, strict booleans. `ParserConfig::strict()` for untrusted input. Billion-laughs safe via `max_alias_expansions` with `saturating_add` overflow protection. |
| **Compat** | YAML 1.1 legacy boolean mode (`legacy_booleans`): resolves `yes`/`no`/`on`/`off`/`y`/`n` as booleans for Docker Compose, GitHub Actions, and other YAML 1.1 tooling. Solves the "Norway problem". |
| **WASM** | Compiles to `wasm32-unknown-unknown`. wasm-bindgen bindings: `parse()`, `stringify()`, `get_path()`, `validate_json()`, `merge()`. Browser demo included. |
| **Errors** | Source locations on all parse errors. `format_with_source()` renders rustc-style diagnostics with `-->` pointer. `#[track_caller]` on all Index panics. `miette::Diagnostic` integration included (`--features miette`) for rich terminal reports with error codes, actionable help text, and source spans. |
| **no\_std** | Full `#![no_std]` support with `alloc`. Use `default-features = false`. Core parsing (`from_str`, `to_string`, `Value`, schemas) works without std. I/O functions (`from_reader`, `to_writer`) require `std` feature. |

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
```

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

Run all 45 examples:

```bash
cargo run --example all
```

<details>
<summary><b>All 45 examples</b></summary>

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
| **Runtime** | `async_io` | Async integration (spawn\_blocking pattern) |
| | `recursive` | Self-referential types (trees, org charts) |
| **Bench** | `bench` | Performance overview |

</details>

---

## Development

```bash
make              # check + clippy + test
make test         # run all tests
make clippy       # lint with Clippy
make fmt          # check formatting
make examples     # run all 45 examples
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

<details>
<summary><b>Safety guarantees and compliance</b></summary>

- `#![forbid(unsafe_code)]` across the entire codebase
- `#[non_exhaustive]` on `ParserConfig`, `SerializerConfig`, `FlowStyle`, `ScalarStyle`
- `#[must_use]` on 83 query methods
- `#[track_caller]` on 13 Index/IndexMut panic paths
- All internal invariant panics documented with `expect("internal: ...")`
- 7 configurable DoS limits with `ParserConfig::strict()`
- **Billion-laughs protection**: `max_alias_expansions` with `saturating_add` overflow guard on alias byte tracking
- `cargo audit` with zero advisories
- `cargo deny` -- license, advisory, ban, and source checks
- SPDX license headers on all source files
- Signed commits enforced via CI
- Comment round-tripping: not supported (YAML spec excludes comments from data model). `Commented<T>` provides write-only comment injection. CST-based preservation tracked as future work

</details>

---

## License

Dual-licensed under [Apache 2.0](https://www.apache.org/licenses/LICENSE-2.0) or [MIT](https://opensource.org/licenses/MIT), at your option.

See [CHANGELOG.md](CHANGELOG.md) for release history.

<p align="right"><a href="#contents">Back to Top</a></p>
