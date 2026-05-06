<!-- SPDX-FileCopyrightText: 2026 Noyalib -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# Migrating from `serde_yaml` to `noyalib`

`serde_yaml` 0.9 is unmaintained. The crate's last release was
2023-08; the upstream repo is archived. The `RUSTSEC-2024-0320`
advisory now flags every `cargo audit` run that depends on it.

`noyalib` is a clean-room reimplementation of YAML 1.2 with the
same `serde` data model, no `unsafe` code, and no `libyaml` C
dependency. This document is the name-for-name migration guide.

## TL;DR

For most call sites, the migration is mechanical:

```diff
-[dependencies]
-serde_yaml = "0.9"
+[dependencies]
+noyalib = "0.0"
```

```diff
-use serde_yaml::Value;
-let v: Value = serde_yaml::from_str(input)?;
-let s        = serde_yaml::to_string(&v)?;
+use noyalib::Value;
+let v: Value = noyalib::from_str(input)?;
+let s        = noyalib::to_string(&v)?;
```

If you want the change to be invisible to call sites, enable the
`compat-serde-yaml` feature:

```toml
[dependencies]
noyalib = { version = "0.0", features = ["compat-serde-yaml"] }
```

```diff
-use serde_yaml;
+use noyalib::compat::serde_yaml;
```

Every public symbol the shim exposes is backed by a noyalib-native
type — there's no transitive dependency on the archived
`serde_yaml` crate.

## Function-by-function mapping

| `serde_yaml` | `noyalib` | Notes |
|---|---|---|
| `serde_yaml::from_str::<T>(s)` | `noyalib::from_str::<T>(s)` | Identical signature. |
| `serde_yaml::from_slice::<T>(b)` | `noyalib::from_slice::<T>(b)` | Identical. |
| `serde_yaml::from_reader::<R, T>(r)` | `noyalib::from_reader::<R, T>(r)` | Identical. |
| `serde_yaml::from_value::<T>(v)` | `noyalib::from_value::<T>(&v)` | Takes a reference (no clone). |
| `serde_yaml::to_string(&v)` | `noyalib::to_string(&v)` | Identical signature. |
| `serde_yaml::to_writer(w, &v)` | `noyalib::to_writer(&mut w, &v)` | Takes `&mut`; produces no trailing `\n` change. |
| `serde_yaml::to_value(&v)` | `noyalib::to_value(&v)` | Identical. |
| `serde_yaml::Value` | `noyalib::Value` | Identical 7-variant enum (`Null`, `Bool`, `Number`, `String`, `Sequence`, `Mapping`, `Tagged`). |
| `serde_yaml::Mapping` | `noyalib::Mapping` | Wraps `IndexMap<String, Value>`; iteration order preserved. |
| `serde_yaml::Number` | `noyalib::Number` | Same `Integer` / `Float` split. |
| `serde_yaml::Error` | `noyalib::Error` | Different variant set, same `Display` shape. See "Error handling" below. |
| `serde_yaml::with::singleton_map` | `noyalib::with::singleton_map` | Identical. |
| `serde_yaml::with::singleton_map_optional` | `noyalib::with::singleton_map_optional` | Identical. |
| `serde_yaml::with::singleton_map_recursive` | `noyalib::with::singleton_map_recursive` | Identical. |
| `serde_yaml::Index` | `noyalib::value::Index` | Same trait surface (`get`, `get_mut`). |
| `serde_yaml::value::Mapping::get_mut` | `noyalib::Mapping::get_mut` | Identical. |
| `serde_yaml::Deserializer::from_str` | `noyalib::Deserializer::new` | Constructor name differs; behaviour identical. |
| `serde_yaml::Serializer::new` | `noyalib::Serializer::new` | Identical. |

## Things `noyalib` adds (no equivalent in `serde_yaml`)

These are pure additions — adopting them is optional, the
migration above doesn't require any of them.

| `noyalib` | What it does |
|---|---|
| `noyalib::from_str_strict<T>` | Like `from_str`, but errors on any key the target type `T` doesn't declare. Closes the silent-data-loss gap when a config-key typo (`replicass: 3`) deserialises into `replicas`'s `Default`. Also `from_slice_strict` and `from_reader_strict`. |
| `noyalib::Spanned<T>` | Wraps `T` and tracks the source `(line, column, byte offset)` of every deserialised value. Survives `flatten`. |
| `noyalib::cst::Document` | Lossless CST — read YAML in, mutate via `Document::set("server.port", "9090")`, write out byte-for-byte preserved (only the touched span changes). Foundation of the `noyafmt` / `noyavalidate --fix` tools. |
| `noyalib::policy::{DenyAnchors, DenyTags, MaxScalarLength}` | Pluggable parser policies. Reject documents that violate organisational constraints at parse time (e.g. ban anchors to defeat billion-laughs). |
| `noyalib::interpolate_properties` | Substitute `${VAR}` references inside string scalars from a property map; pair with `secrecy::Secret<T>` for redacted-by-default credential handling. |
| `noyalib::parallel::parse<T>` | Parse `---`-separated multi-document streams across the Rayon thread pool. Linear with cores. (Requires the `parallel` feature.) |
| `noyalib::Error::format_with_source(input)` | Renders a rustc-style snippet pointing at the offending line + column. Always available; richer output under `--features miette`. |
| `noyalib::diagnostic::*` | First-class `miette::Diagnostic` integration: error codes, help text, source spans, ANSI-coloured terminal output. Under `--features miette`. |

See the public API surface map at the top of `crates/noyalib/src/lib.rs`.

## Things `noyalib` does **not** do (yet)

- **Round-trip comments through the data-binding API.** The YAML
  data model excludes comments by spec. `noyalib::cst::Document`
  preserves them byte-for-byte for the lossless tooling path,
  but `from_str::<T>` → `to_string(&T)` does not. No Rust YAML
  library does this end-to-end.
- **Implicit `<<:` merge keys outside of explicit handling.**
  YAML 1.2 dropped merge keys from the spec; `noyalib` follows
  1.2 by default but ships an opt-in
  `MergeKeyPolicy::AutoExpand` for backwards compatibility.
- **Custom-tag dispatch via `serde(rename)`.** `noyalib`
  surfaces non-core tags as `Value::Tagged(...)` rather than
  routing them to a typed enum variant by tag-string. Adopt
  `noyalib::TagRegistry` if you need strip-through behaviour
  for known tags.

## Behavioural differences worth knowing

### 1. `Value::Tagged` is a 7th variant

`serde_yaml::Value` has six variants (`Null`, `Bool`, `Number`,
`String`, `Sequence`, `Mapping`); `noyalib::Value` adds
`Tagged(Box<TaggedValue>)` as a 7th. If you exhaustively
`match` against `Value` somewhere, the migration adds one arm:

```diff
 match value {
     Value::Null         => …,
     Value::Bool(b)      => …,
     Value::Number(n)    => …,
     Value::String(s)    => …,
     Value::Sequence(s)  => …,
     Value::Mapping(m)   => …,
+    Value::Tagged(t)    => handle_tag(&t.tag, &t.value),
 }
```

If you don't currently inspect tags, the simplest migration is
to call `value.untag()` to strip the wrapper before matching.

### 2. Default boolean recognition is YAML 1.2 strict

`serde_yaml` 0.9 followed YAML 1.1 — bare `yes` / `no` /
`on` / `off` parsed as booleans. `noyalib` follows YAML 1.2 by
default — only `true` / `false` count.

This is the **"Norway problem"** fix: `country: NO` no longer
silently rewrites to `country: false`.

If you depend on the legacy behaviour (Docker Compose, GitHub
Actions, pre-1.2 toolchains), opt back in:

```rust
use noyalib::{from_str_with_config, ParserConfig};
let cfg = ParserConfig::new().legacy_booleans(true);
let v: Value = from_str_with_config("country: NO\n", &cfg)?;
assert_eq!(v["country"].as_bool(), Some(false));
```

### 3. Multi-doc streams use `load_all_as`

`serde_yaml`'s `Deserializer::from_str` returns an iterator of
`Result<Value>`. `noyalib` exposes:

```rust
// Eager parse, returns Vec<T>.
let docs: Vec<MyType> = noyalib::load_all_as::<MyType>(stream)?;

// Or iterate Values lazily.
let docs: Vec<noyalib::Value> = noyalib::load_all(stream)?;
```

For very large streams (audit logs, Kubernetes-resource snapshots),
the parallel path is a drop-in replacement:

```rust
// Same input, parses each doc concurrently across Rayon.
let docs: Vec<MyType> = noyalib::parallel::parse(stream)?;
```

Requires the `parallel` feature.

### 4. Error handling stays type-safe

`noyalib::Error` is a `#[non_exhaustive]` enum carrying the
location and a structured message. The `Display` implementation
matches the rustc-error format closely:

```text
error: expected ',' or ']'
  --> input.yaml:2:7
   |
 1 | host: localhost
 2 | port: [broken
   |       ^^^^^^ here
```

`from_str_with_config` accepts a `ParserConfig` carrying every
limit you might want (`max_depth`, `max_alias_expansions`,
`max_scalar_length`, `duplicate_key_policy`,
`strict_booleans`, etc.). The factory `ParserConfig::strict()`
turns every dial up for untrusted input.

## Drop-in compatibility shim

If you cannot afford the call-site changes above (e.g. you're
migrating an in-flight `::serde_yaml::Value` from a module you
don't own), enable the `compat-serde-yaml` feature:

```toml
[dependencies]
noyalib = { version = "0.0", features = ["compat-serde-yaml"] }
```

```rust
// Module path matches `serde_yaml`'s, but every type is
// noyalib-native — no transitive dep on the archived crate.
use noyalib::compat::serde_yaml::{from_str, to_string, Value, Mapping, Number};
```

The shim is feature-gated so users who don't need migration help
don't see the extra surface.

## Migration checklist

For each crate that depends on `serde_yaml`:

- [ ] Replace `serde_yaml = "0.9"` → `noyalib = "0.0"` (or
      `noyalib = { version = "0.0", features =
      ["compat-serde-yaml"] }` for the shim path).
- [ ] Replace `use serde_yaml::*;` → `use noyalib::*;` (or
      `use noyalib::compat::serde_yaml::*;`).
- [ ] If you exhaustively match `Value`, add a `Tagged(_)` arm
      (or call `.untag()` first).
- [ ] Decide YAML 1.1-vs-1.2 boolean handling: stay on YAML 1.2
      strict (the safer default) or opt into
      `legacy_booleans(true)`.
- [ ] Run `cargo audit`. The `RUSTSEC-2024-0320` advisory should
      no longer match anything.
- [ ] Run your existing test suite. The `noyalib`-shaped
      `from_str` is a drop-in for the surface above; failures
      almost always trace to one of the three behavioural notes
      in this document.

If you hit a snag, file an issue at
<https://github.com/sebastienrousseau/noyalib/issues> with the
input that misbehaves — every report makes the migration story
better.
