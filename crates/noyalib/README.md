<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->

# noyalib

A YAML 1.2 parser and serialiser for Rust, with full
[`serde`](https://crates.io/crates/serde) integration and
**zero `unsafe` code**.

Drop-in source-level replacement for the unmaintained
[`serde_yaml`](https://crates.io/crates/serde_yaml) 0.9 — the
[`compat-serde-yaml`](#feature-flags) feature exposes a
name-for-name shim backed entirely by noyalib-native types so
downstream `cargo audit` runs no longer flag the archived
upstream advisory chain. See
[doc/MIGRATION-FROM-SERDE-YAML.md](https://github.com/sebastienrousseau/noyalib/blob/main/doc/MIGRATION-FROM-SERDE-YAML.md)
for the function-by-function migration guide.

## Quick start

```toml
[dependencies]
noyalib = "0.0.1"
```

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
let cfg: Config = from_str(yaml)?;
let out: String   = to_string(&cfg)?;
# Ok::<_, noyalib::Error>(())
```

## Why this approach?

Two architectural choices motivate the rewrite (full rationale in
[`README.md`](https://github.com/sebastienrousseau/noyalib/blob/main/README.md)
at the workspace root):

1. **Streaming-first deserialise.** The default `from_str::<T>`
   path walks parser events directly into the typed target.
   `serde_yaml`-shaped libraries always materialise an
   intermediate `Value` AST — noyalib bypasses that AST when the
   caller asked for a typed `T`, saving one allocation per scalar
   plus the rebuild work.
2. **`#![forbid(unsafe_code)]`** at the workspace root — no FFI
   to a C `libyaml`, no raw-pointer dereferences. Verified by CI
   on every push.

## Surface

- **Reading**: `from_str`, `from_slice`, `from_reader`,
  `from_value`. Each has a `_with_config` variant accepting a
  `ParserConfig` (DoS limits, duplicate-key policy, strict
  booleans, …).
- **Writing**: `to_string`, `to_writer`, `to_fmt_writer`,
  `to_value`. Same `_with_config` shape.
- **Strict deserialise**: `from_str_strict<T>`, `from_slice_strict`,
  `from_reader_strict` — error if the input carries a key the
  target type does not declare. Closes the silent-data-loss gap
  on config-key typos.
- **Source spans**: wrap any deserialise target in
  [`Spanned<T>`](https://docs.rs/noyalib/latest/noyalib/struct.Spanned.html);
  every value carries `(line, column, byte offset)`. Survives
  `flatten`. Pair with `--features miette` for rustc-style
  diagnostics out of the box.
- **Lossless CST**:
  [`noyalib::cst::Document`](https://docs.rs/noyalib/latest/noyalib/cst/struct.Document.html)
  — `doc.set("server.port", "9090")` rewrites only the touched
  span; comments, indentation, and sibling entries survive.
  Foundation of the `noyafmt` / `noyavalidate --fix` tools.
- **Schema codegen + validation** (`schema` / `validate-schema`
  features): derive `JsonSchema`, emit YAML, validate parsed
  documents against a JSON Schema 2020-12 contract.
- **Parser policies** (`policy::Policy` trait):
  `DenyAnchors`, `DenyTags`, `MaxScalarLength` — reject
  documents that violate organisational constraints at parse
  time.
- **Path queries**: `value.query("items[*].name")` with
  wildcards and recursive descent.
- **Zero-copy AST**:
  [`BorrowedValue<'a>`](https://docs.rs/noyalib/latest/noyalib/borrowed/index.html)
  borrows scalar bytes from input (~18% faster than `Value`).
- **Parallel parse** (`parallel` feature):
  `noyalib::parallel::parse<T>` for `---`-separated streams,
  linear with cores via Rayon.

## YAML 1.2 conformance

Validated against 406/406 cases of the
[official YAML test suite](https://github.com/yaml/yaml-test-suite),
zero skips. The conformance report rebuilds on every CI run via
`cargo test --test yaml_compliance_report`.

## Feature flags

| Flag | Pulls in | Adds |
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

## Documentation

- **API reference**: <https://docs.rs/noyalib>
- **User guide**: [`doc/USER-GUIDE.md`](https://github.com/sebastienrousseau/noyalib/blob/main/doc/USER-GUIDE.md)
- **Architecture overview**: [`doc/ARCHITECTURE.md`](https://github.com/sebastienrousseau/noyalib/blob/main/doc/ARCHITECTURE.md)
- **`serde_yaml` migration**: [`doc/MIGRATION-FROM-SERDE-YAML.md`](https://github.com/sebastienrousseau/noyalib/blob/main/doc/MIGRATION-FROM-SERDE-YAML.md)

## License

Dual-licensed under Apache 2.0 or MIT, at your option.
