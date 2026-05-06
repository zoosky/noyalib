<!-- SPDX-FileCopyrightText: 2026 Noyalib -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# `noyalib` user guide

A long-form companion to the README. The README is the elevator
pitch and reference; this guide walks through every public-facing
surface in the order you typically encounter it.

If you've used `serde_yaml` 0.9 before, also see
[`MIGRATION-FROM-SERDE-YAML.md`](MIGRATION-FROM-SERDE-YAML.md) —
most call sites are mechanical to update.

## Contents

1. [Reading YAML](#1-reading-yaml)
2. [Writing YAML](#2-writing-yaml)
3. [The dynamic `Value` tree](#3-the-dynamic-value-tree)
4. [Source spans (`Spanned<T>`)](#4-source-spans-spannedt)
5. [Strict deserialise (typo detection)](#5-strict-deserialise-typo-detection)
6. [Parser policies (defence in depth)](#6-parser-policies-defence-in-depth)
7. [Diagnostics (`miette`-friendly errors)](#7-diagnostics-miette-friendly-errors)
8. [Lossless edits (`cst::Document`)](#8-lossless-edits-cstdocument)
9. [Schema validation and autofix](#9-schema-validation-and-autofix)
10. [Multi-document streams + parallel parse](#10-multi-document-streams--parallel-parse)
11. [The CLI tools (`noyafmt`, `noyavalidate`)](#11-the-cli-tools-noyafmt-noyavalidate)
12. [WASM, MCP, and LSP](#12-wasm-mcp-and-lsp)

## 1. Reading YAML

The simplest case — read into a typed struct via `serde`:

```rust
use noyalib::from_str;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Server {
    host: String,
    port: u16,
}

let yaml = r#"
host: api.example.com
port: 8080
"#;
let s: Server = from_str(yaml)?;
assert_eq!(s.port, 8080);
```

Every input shape `serde_yaml` supports has a noyalib equivalent:

| Source | API | Returns |
|---|---|---|
| `&str` | `from_str::<T>(s)` | `Result<T>` |
| `&[u8]` | `from_slice::<T>(b)` | `Result<T>` |
| `impl io::Read` | `from_reader::<R, T>(r)` | `Result<T>` |
| `&noyalib::Value` | `from_value::<T>(&v)` | `Result<T>` |

Each has a `_with_config(input, &ParserConfig)` variant for when
you need to tighten parser limits or opt into legacy behaviours
(see [section 6](#6-parser-policies-defence-in-depth)).

## 2. Writing YAML

Mirror surface for the write side:

```rust
use noyalib::to_string;
use serde::Serialize;

#[derive(Serialize)]
struct Server { host: String, port: u16 }

let yaml = to_string(&Server {
    host: "api.example.com".into(),
    port: 8080,
})?;
println!("{yaml}");
// host: api.example.com
// port: 8080
```

| Sink | API |
|---|---|
| `String` | `to_string(&v)` |
| `impl io::Write` | `to_writer(&mut w, &v)` |
| `impl fmt::Write` | `to_fmt_writer(&mut w, &v)` |
| `noyalib::Value` | `to_value(&v)` |

`to_string_with_config(&v, &cfg)` accepts a `SerializerConfig`
that tunes indent, flow style, scalar style, document markers,
and block-scalar thresholds. See
[`crates/noyalib/examples/emit.rs`](../crates/noyalib/examples/emit.rs)
for every option in action.

## 3. The dynamic `Value` tree

When the schema isn't fixed at compile time, work with `Value`:

```rust
use noyalib::{from_str, Value};

let v: Value = from_str(yaml)?;

// Dot-path traversal.
let port = v.get_path("server.port").and_then(|n| n.as_u16());

// Sequence indexing.
let first = v.get("items").and_then(|s| s.get(0));

// Path queries — wildcards and recursive descent.
let names = v.query("items[*].name");          // every name in the list
let any   = v.query("..debug");                // every `debug` at any depth

// Mutation.
let mut v = v;
v["server"]["port"] = Value::from(9090u16);
```

Missing keys return `None`; never panic on lookup.

For lower-overhead reads, the borrowed AST shares scalar bytes
with the input:

```rust
use noyalib::borrowed::from_str_borrowed;

let yaml  = "host: localhost\nport: 8080\n";
let v     = from_str_borrowed(yaml)?;
let host  = v.as_mapping().unwrap().get("host").unwrap().as_str();
// host points into `yaml` — no String allocation for the scalar.
```

`BorrowedValue<'a>` doesn't follow YAML aliases (`*name`); use
the owned path for documents that anchor.

## 4. Source spans (`Spanned<T>`)

Wrap any deserialise target in `Spanned<T>` to record where in
the input it came from:

```rust
use noyalib::{from_str, Spanned};
use serde::Deserialize;

#[derive(Deserialize)]
struct Cfg {
    port: Spanned<u16>,
    host: Spanned<String>,
}

let cfg: Cfg = from_str("port: 8080\nhost: api\n")?;
assert_eq!(cfg.port.value, 8080);
assert_eq!(cfg.port.start.line(), 1);
assert_eq!(cfg.port.start.column(), 6);
```

`Spanned<T>` round-trips through `serde::Serialize` as `T` —
the span info is read-side only. Combine with `miette` to render
the exact offending region:

```rust
let report = noyalib::diagnostic::spanned_error(
    yaml,
    &cfg.port,
    "port must be >= 1024 (privileged ports not allowed)",
);
eprintln!("{report:?}");
```

See [`crates/noyalib/examples/validation.rs`](../crates/noyalib/examples/validation.rs).

## 5. Strict deserialise (typo detection)

The single biggest config-file footgun: a typo in a key name
silently deserialises into the field's `Default`. `from_str` is
lenient by design (extras are ignored), `from_str_strict` errors
out:

```rust
#[derive(serde::Deserialize)]
struct Cfg { port: u16, host: String }

let yaml = "port: 8080\nhost: api\nporrt: 9090\n";

// Lenient: silently drops the typo.
let c: Cfg = noyalib::from_str(yaml)?;
assert_eq!(c.port, 8080);

// Strict: typed error pointing at `porrt`.
let err = noyalib::from_str_strict::<Cfg>(yaml).unwrap_err();
assert!(err.to_string().contains("porrt"));
```

The strict path walks nested structs — a typo at
`server.unknown` is reported with its parent path. Available on
every input shape:

| Input | Strict variant |
|---|---|
| `&str` | `from_str_strict::<T>(s)` |
| `&[u8]` | `from_slice_strict::<T>(b)` |
| `impl io::Read` | `from_reader_strict::<R, T>(r)` |

## 6. Parser policies (defence in depth)

YAML's anchor / merge-key / custom-tag features have historically
been amplification vectors (billion-laughs) and remote-code-
execution vectors (`!!python/object`). `noyalib` doesn't
instantiate arbitrary objects via tags, but you may still want
to reject documents that abuse the spec at parse time:

```rust
use noyalib::{from_str_with_config, ParserConfig};
use noyalib::policy::{DenyAnchors, DenyTags, MaxScalarLength};

let cfg = ParserConfig::new()
    .with_policy(DenyAnchors)             // no `&name` / `*name`
    .with_policy(DenyTags)                // reject custom !tag scalars
    .with_policy(MaxScalarLength(64_000)) // cap individual scalar size
    .max_alias_expansions(100)            // billion-laughs guard
    .max_depth(64);

let res: Result<noyalib::Value, _> =
    from_str_with_config(input, &cfg);
```

`ParserConfig::strict()` enables a sane "untrusted-input"
preset; tweak from there if you need to relax specific dials.

| Dial | Default | `strict()` | Protects against |
|---|---|---|---|
| `max_depth` | 128 | 64 | Stack-blowing nested structures |
| `max_document_length` | 64 MiB | 1 MiB | Oversized payloads |
| `max_alias_expansions` | 1024 | 100 | Billion-laughs amplification |
| `max_mapping_keys` | 64 K | 1024 | Hash-collision DoS |
| `max_sequence_length` | 64 K | 1024 | Memory-spike DoS |
| `duplicate_key_policy` | `Last` | `Error` | Silent data loss |
| `strict_booleans` | off | on | Norway problem |

## 7. Diagnostics (`miette`-friendly errors)

Every parse error carries `(line, column, byte_offset)` plus a
machine-readable error code. The minimum, no-feature path
renders a rustc-style snippet:

```rust
let err = noyalib::from_str::<Value>("port: [unclosed").unwrap_err();
println!("{}", err.format_with_source(input));
// error: expected ',' or ']'
//   --> input.yaml:1:7
//    |
//  1 | port: [unclosed
//    |       ^^^^^^^^^ here
```

Enable `--features miette` to surface this through the
`miette::Diagnostic` interface — `cargo` / `rustc`-style ANSI
output, error codes, help text, source-span underlining:

```rust
fn main() -> miette::Result<()> {
    let cfg: Config = noyalib::from_str(yaml)
        .map_err(|e| miette::Report::new(e)
            .with_source_code(yaml.to_owned()))?;
    Ok(())
}
```

See [`crates/noyalib/examples/diagnostic.rs`](../crates/noyalib/examples/diagnostic.rs).

## 8. Lossless edits (`cst::Document`)

`from_str` → `to_string` round-tripping discards comments and
exact whitespace by design — the YAML data model excludes
both. When you need byte-faithful editing (Renovate-style
version bumps, manifest patchers, formatters), use the lossless
CST instead:

```rust
use noyalib::cst::parse_document;

let mut doc = parse_document(r#"
# Production config
server:
  host: api.example.com
  port: 8080      # bind to public IP
"#)?;

// Surgical edit — only the touched span is rewritten.
doc.set("server.port", "9090")?;
println!("{doc}");
//
// # Production config
// server:
//   host: api.example.com
//   port: 9090      # bind to public IP
//
// (Comment and trailing whitespace preserved byte-for-byte.)
```

The CST exposes:

| API | Use |
|---|---|
| `parse_document(s)` / `parse_stream(s)` | Read |
| `doc.set(path, fragment)` | Write a literal scalar |
| `doc.set_value(path, &Value)` | Write any `Value` |
| `doc.entry(path)` | Chainable mutable handle (12 methods, smart `items[0]` paths) |
| `doc.remove(path)` | Delete a key or sequence item |
| `doc.push_back(path, fragment)` | Append to a sequence |
| `doc.materialise_aliases_of(name)` | Inline every `*name` reference |
| `doc.indent_unit()` | Detect 2- / 3- / 4-space conventions |
| `doc.dominant_quote_style()` | `"`, `'`, or plain |

See [`crates/noyalib/examples/lossless_edit.rs`](../crates/noyalib/examples/lossless_edit.rs)
and [`crates/noyalib/examples/entry_api.rs`](../crates/noyalib/examples/entry_api.rs).

## 9. Schema validation and autofix

Under `--features validate-schema`:

```rust
use noyalib::{from_str, schema_for, validate_against_schema, JsonSchema};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, JsonSchema)]
struct ServerConfig {
    /// Port the server binds on.
    port: u16,
    /// Hostname or IP literal.
    host: String,
}

let schema = schema_for::<ServerConfig>()?;
let data: noyalib::Value = from_str("port: 8080\nhost: api")?;
validate_against_schema(&data, &schema)?;
```

The matching `coerce_to_schema(&mut data, &schema)` engine
rewrites obvious type slips — e.g. `port: "8080"` (quoted string
where the schema declares integer) — back to the schema's
expected type, then re-validates. This is the library engine
behind `noyavalidate --fix`. See
[`crates/noyalib/examples/schema_validation.rs`](../crates/noyalib/examples/schema_validation.rs).

## 10. Multi-document streams + parallel parse

YAML's `---` document separator lets one file carry many
independent documents (Kubernetes manifests, audit-event
streams). The eager API:

```rust
use noyalib::{load_all, load_all_as};

let docs: Vec<Value>      = load_all(stream)?;
let typed: Vec<MyConfig>  = load_all_as::<MyConfig>(stream)?;
```

For large streams, the `parallel` feature unlocks linear-with-
cores throughput:

```rust
// Drop-in for `load_all_as`. Pre-scans `---` boundaries on a
// single thread, then deserialises each document concurrently.
let docs: Vec<MyConfig> = noyalib::parallel::parse(stream)?;
```

The pre-scan is `O(input_len)`; the per-document work
parallelises across the Rayon thread pool.

## 11. The CLI tools (`noyafmt`, `noyavalidate`)

Two command-line companions ship under `crates/noya-cli/`:

```bash
# Format YAML in-place via the lossless CST.
noyafmt --write config.yaml

# Verify formatting without writing — for CI gates.
noyafmt --check ci/*.yaml

# Validate syntax + (optionally) JSON Schema 2020-12.
noyavalidate --schema schema.yaml in.yaml

# Validate + auto-fix obvious type slips.
noyavalidate --schema schema.yaml --fix in.yaml
```

Both use the same `noyalib::cst::format` + `coerce_to_schema`
engines exposed to library callers. Distro-installable via
crates.io (`cargo install noyalib`), Homebrew, AUR, and the
other channels in [`pkg/PUBLISH.md`](../pkg/PUBLISH.md).

## 12. WASM, MCP, and LSP

Three satellite crates in the workspace target specific
deployment shapes:

- **`noyalib-wasm`** (`crates/noyalib-wasm/`). `wasm-pack`
  output published to npm as `@noyalib/noyalib-wasm`. Browser
  IDEs use it for live YAML formatting / validation; the
  bundle is ~338 KB after LTO.

- **`noyalib-mcp`** (`crates/noyalib-mcp/`). Model Context
  Protocol server speaking JSON-RPC over stdio. AI agents
  (Claude Code, GitHub Copilot, …) call `parse`, `format`,
  `get`, `set`, `validate` tools without needing a Rust
  toolchain. Available as `npx noyalib-mcp` (the wrapper at
  `pkg/npm-mcp-wrapper/` bootstraps the binary) and as a
  GHCR container image.

- **`noyalib-lsp`** (`crates/noyalib-lsp/`). Language Server
  Protocol implementation. Editors get format-on-save,
  inline diagnostics, schema-driven hover docs. Bundled into
  the noyalib VS Code / Open VSX extension (`pkg/vscode/`).

## Where next

- **Source spans + miette diagnostics**:
  [`crates/noyalib/examples/diagnostic.rs`](../crates/noyalib/examples/diagnostic.rs)
- **Anchor manipulation**:
  [`crates/noyalib/examples/anchor_shared.rs`](../crates/noyalib/examples/anchor_shared.rs)
- **Schema codegen**:
  [`crates/noyalib/examples/schema_validation.rs`](../crates/noyalib/examples/schema_validation.rs)
- **Tagged-enum dispatch**:
  [`crates/noyalib/examples/robotics_polymorphism.rs`](../crates/noyalib/examples/robotics_polymorphism.rs)
- **Figment / garde / validator integration**:
  [`crates/noyalib/examples/figment.rs`](../crates/noyalib/examples/figment.rs),
  [`crates/noyalib/examples/validation_garde.rs`](../crates/noyalib/examples/validation_garde.rs),
  [`crates/noyalib/examples/validation_validator.rs`](../crates/noyalib/examples/validation_validator.rs)
- **CST architecture**:
  [`ARCHITECTURE.md`](ARCHITECTURE.md)
- **Migrating from `serde_yaml`**:
  [`MIGRATION-FROM-SERDE-YAML.md`](MIGRATION-FROM-SERDE-YAML.md)

## Reporting issues / sending PRs

Filed at <https://github.com/sebastienrousseau/noyalib>.
Reproducer YAML + the parser config you used together is the
fastest path to a fix.
