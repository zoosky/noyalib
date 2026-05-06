<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->

# noyalib

A YAML 1.2 parser and serialiser for Rust, with full
[`serde`](https://crates.io/crates/serde) integration and
**zero `unsafe` code**.

[![crates.io](https://img.shields.io/crates/v/noyalib.svg)](https://crates.io/crates/noyalib)
[![docs.rs](https://img.shields.io/docsrs/noyalib)](https://docs.rs/noyalib)
[![Build](https://img.shields.io/github/actions/workflow/status/sebastienrousseau/noyalib/ci.yml?branch=main)](https://github.com/sebastienrousseau/noyalib/actions)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](#license)

The library half of the
[noyalib workspace](https://github.com/sebastienrousseau/noyalib).
Drop-in source-level replacement for the unmaintained
[`serde_yaml`](https://crates.io/crates/serde_yaml) 0.9 — the
[`compat-serde-yaml`](#cargo-features) feature exposes a
name-for-name shim backed entirely by noyalib-native types so
downstream `cargo audit` runs no longer flag the archived
upstream advisory chain.

## Contents

- [Install](#install)
- [Quick Start](#quick-start)
- [Why this approach?](#why-this-approach)
- [Surface](#surface)
- [YAML 1.2 conformance](#yaml-12-conformance)
- [Cargo features](#cargo-features)
- [Examples](#examples)
- [Benchmarks](#benchmarks)
- [Documentation](#documentation)
- [Migrating from `serde_yaml`](#migrating-from-serde_yaml)
- [License](#license)

## Install

```toml
[dependencies]
noyalib = "0.0.1"
```

## Quick Start

```rust
use noyalib::{from_str, to_string};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Config {
    host: String,
    port: u16,
    features: Vec<String>,
}

let yaml = "host: api.example.com\nport: 8080\nfeatures: [auth, api]\n";
let cfg: Config = from_str(yaml).unwrap();
let out: String = to_string(&cfg).unwrap();
let round: Config = from_str(&out).unwrap();
assert_eq!(cfg, round);
```

## Why this approach?

Two architectural choices motivate the rewrite (full rationale
in the
[workspace README](https://github.com/sebastienrousseau/noyalib#why-this-approach)):

1. **Streaming-first deserialise.** The default `from_str::<T>`
   path walks parser events directly into the typed target.
   `serde_yaml`-shaped libraries materialise an intermediate
   `Value` AST per call — noyalib bypasses that AST when the
   caller asked for a typed `T`, saving one allocation per
   scalar plus the rebuild work.
2. **`#![forbid(unsafe_code)]`** at the workspace root — no
   FFI to a C `libyaml`, no raw-pointer dereferences. Verified
   by CI on every push.

## Surface

| Module | What it exposes |
|---|---|
| [`from_str`](https://docs.rs/noyalib/latest/noyalib/fn.from_str.html), [`from_slice`](https://docs.rs/noyalib/latest/noyalib/fn.from_slice.html), [`from_reader`](https://docs.rs/noyalib/latest/noyalib/fn.from_reader.html), [`from_value`](https://docs.rs/noyalib/latest/noyalib/fn.from_value.html) | Read YAML into `T: Deserialize`. Each has a `_with_config` variant. |
| [`to_string`](https://docs.rs/noyalib/latest/noyalib/fn.to_string.html), [`to_writer`](https://docs.rs/noyalib/latest/noyalib/fn.to_writer.html), [`to_value`](https://docs.rs/noyalib/latest/noyalib/fn.to_value.html) | Write `T: Serialize` back as YAML. |
| [`from_str_strict`](https://docs.rs/noyalib/latest/noyalib/fn.from_str_strict.html) (+ `_slice`, `_reader`) | Strict deserialise — error on any key the target type does not declare. |
| [`Spanned<T>`](https://docs.rs/noyalib/latest/noyalib/struct.Spanned.html) | Wraps any `T` with `(line, column, byte offset)`. Survives `#[serde(flatten)]`. |
| [`cst::Document`](https://docs.rs/noyalib/latest/noyalib/cst/struct.Document.html) | Lossless CST. `doc.set("server.port", "9090")` rewrites only the touched span; comments + indentation preserved. |
| [`policy::{DenyAnchors, DenyTags, MaxScalarLength}`](https://docs.rs/noyalib/latest/noyalib/policy/index.html) | Pluggable parser policies. Reject documents at parse time. |
| [`schema_for`](https://docs.rs/noyalib/latest/noyalib/fn.schema_for.html), [`validate_against_schema`](https://docs.rs/noyalib/latest/noyalib/fn.validate_against_schema.html), [`coerce_to_schema`](https://docs.rs/noyalib/latest/noyalib/fn.coerce_to_schema.html) | JSON Schema 2020-12 codegen, validation, schema-driven autofix. |
| [`parallel::parse`](https://docs.rs/noyalib/latest/noyalib/parallel/fn.parse.html) | Multi-doc parse across the Rayon thread pool. Linear with cores. |
| [`borrowed::from_str_borrowed`](https://docs.rs/noyalib/latest/noyalib/borrowed/fn.from_str_borrowed.html) | Zero-copy AST. Scalars borrow from input bytes. |
| [`compat::serde_yaml`](https://docs.rs/noyalib/latest/noyalib/compat/serde_yaml/index.html) | Drop-in shim — `use noyalib::compat::serde_yaml as serde_yaml`. |

## YAML 1.2 conformance

Validated against
[406/406 cases of the official YAML test suite](https://github.com/yaml/yaml-test-suite),
zero skips. The conformance report rebuilds on every CI run via
`cargo test --test yaml_compliance_report`.

## Cargo features

| Feature | Pulls in | Adds |
|---|---|---|
| `std` *(default)* | — | I/O, `Spanned<T>`, CST module |
| `miette` | `miette` 7 | Rich terminal diagnostics with source spans |
| `schema` | `schemars`, `serde_json` | `JsonSchema` derive + `schema_for::<T>()` |
| `validate-schema` | `schema` + `jsonschema` | `validate_against_schema`, `coerce_to_schema` |
| `figment` | `figment` 0.10 | `noyalib::figment::Yaml` provider |
| `garde` | `garde` 0.22 | `Validated<T>` wrapper |
| `validator` | `validator` 0.19 | `ValidatedValidator<T>` wrapper |
| `robotics` | — | `Degrees` / `Radians` / `StrictFloat` newtypes |
| `parallel` | `rayon` 1.10 | `noyalib::parallel::parse<T>` |
| `simd` | — | `noyalib::simd::*` primitives + parser hot path |
| `nightly-simd` | `simd` (nightly) | `core::simd`-backed 32-byte structural-bitmask scanner |
| `compat-serde-yaml` | — | `noyalib::compat::serde_yaml` shim for migration |

## Examples

60+ runnable examples under
[`crates/noyalib/examples/`](examples/):

```bash
cargo run --example all                       # runs every default-feature example
cargo run --example hello                     # struct round-trip
cargo run --example lossless_edit             # CST edits, comments preserved
cargo run --example diagnostic --features miette   # rich error reports
cargo run --example schema_validation --features validate-schema
cargo run --example figment       --features figment
cargo run --example validation_garde --features garde
```

Highlights: `hello`, `dynamic`, `lossless_edit`, `entry_api`,
`comments_at`, `flatten`, `flattened`, `bridge`,
`include`, `figment`, `validation_garde`, `validation_validator`,
`diagnostic_path`, `robotics_polymorphism`, `replay`,
`registry`, `scientific`. See
[`examples/README.md`](examples/README.md) for the full
catalogue.

## Benchmarks

```bash
cargo bench --bench benchmarks      # core throughput
cargo bench --bench comparison      # vs serde_yaml_ng, yaml-rust2, saphyr
cargo bench --bench architecture    # streaming vs AST, zero-copy, span-free
cargo bench --bench simd            --features simd
cargo bench --bench numeric_parse   # SWAR decimal pipeline
cargo bench --bench structural_bitmask
cargo bench --bench streaming_vs_value
cargo bench --bench incremental_repair
```

Reproducible on Apple M-series + ubuntu-latest. See the
[Benchmarks section of the workspace README](https://github.com/sebastienrousseau/noyalib#benchmarks)
for the published throughput tables.

## Documentation

- **API reference**: <https://docs.rs/noyalib>
- **User guide**:
  [`doc/USER-GUIDE.md`](https://github.com/sebastienrousseau/noyalib/blob/main/doc/USER-GUIDE.md)
- **Architecture overview**:
  [`doc/ARCHITECTURE.md`](https://github.com/sebastienrousseau/noyalib/blob/main/doc/ARCHITECTURE.md)

## Migrating from `serde_yaml`

The headline diff plus a 13-row name-for-name mapping table sits
in the
[workspace README](https://github.com/sebastienrousseau/noyalib#one-minute-migration-from-serde_yaml);
the deeper guide (behavioural-difference notes, drop-in shim,
checklist) is
[`doc/MIGRATION-FROM-SERDE-YAML.md`](https://github.com/sebastienrousseau/noyalib/blob/main/doc/MIGRATION-FROM-SERDE-YAML.md).

## License

Dual-licensed under [Apache 2.0](https://www.apache.org/licenses/LICENSE-2.0)
or [MIT](https://opensource.org/licenses/MIT), at your option.
