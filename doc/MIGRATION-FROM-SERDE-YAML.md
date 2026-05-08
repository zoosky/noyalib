<!-- SPDX-FileCopyrightText: 2026 Noyalib -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# Migrating to `noyalib` from other Rust YAML crates

`serde_yaml` 0.9 is unmaintained — the upstream `dtolnay/serde-yaml`
repo was archived 2024-03-25 and `RUSTSEC-2024-0320` flags every
`cargo audit` run that depends on it. Several active forks now
sit downstream (`yaml_serde`, `serde-yaml-ng`, `serde-norway`,
`serde-yaml-bw`, `serde_yml` until it was archived 2025-09); the
`serde-saphyr` and `yaml-spanned` crates take different design
shapes (no DOM, span-aware parsing) but live in the same niche.

`noyalib` is a clean-room reimplementation of YAML 1.2 with the
same `serde` data model, no `unsafe` code, and no `libyaml` C
dependency. The bulk of this document is the name-for-name
migration guide for `serde_yaml` 0.9 (the most common starting
point); per-library shorthand for every other ecosystem crate
ships at the bottom under [Migrating from other YAML
libraries](#migrating-from-other-yaml-libraries).

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

### 1. `Value::Tagged` is a 7th variant — and noyalib preserves scalar tags too

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

**Where `serde_yaml` differs from noyalib**: `serde_yaml` 0.9
drops custom-tag scalars from the `Value` tree by default — a
`!Custom 'hello'` scalar deserialises into `Value::String("hello")`
and the tag is lost. `noyalib` preserves the tag:
`from_str::<Value>("!Custom 'hello'\n")` returns
`Value::Tagged(Tag("!Custom"), Value::String("hello"))`. This
matches noyalib's behaviour for tagged sequences and tagged
mappings — three Tagged shapes, one consistent rule.

If your existing code worked because the tag was silently dropped,
migrate by either:

- Calling `value.untag()` (or `value.untag_ref()` for
  borrow-friendly reads) before any `as_str` / `as_i64` /
  type-cast you used to do directly. This is the smallest diff.
- Switching to a typed deserialise. Typed targets
  (`#[derive(Deserialize)] struct Foo { ... }`) see through tags
  transparently — `from_str::<Foo>("!Foo {x: 1}")` yields
  `Foo { x: 1 }` regardless of the tag.
- Registering the tag with [`TagRegistry::with`] for inline
  strip-through on the streaming path. Useful when you want
  noyalib to dispatch into a typed handler keyed on the tag
  string.

If you don't inspect tags at all and want the old serde_yaml
behaviour wholesale, that's the `compat::serde_yaml` shim's
default: see "Drop-in compatibility shim" below.

[`TagRegistry::with`]: https://docs.rs/noyalib/latest/noyalib/struct.TagRegistry.html#method.with

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

### 5. The deserialise-target bound is `T: 'static`

`serde_yaml::from_str<T>` constrains `T: for<'de> Deserialize<'de>`.
`noyalib::from_str<T>` adds `T: 'static`. Every real-world
`DeserializeOwned` type already satisfies this — the HRTB on its
own already disallows borrowed lifetimes — and the `'static` is
what lets noyalib detect at the call site whether the caller's
target is `Value` itself (in which case the tag-preserving fast
path engages) or something else (typed deserialise stays
transparent).

Concretely: a `&'a str` target was already disallowed by the HRTB;
`String`, `Vec<...>`, `HashMap<String, V>`, and every `derive`-d
struct continue to work unchanged. If you hit a compile error
whose message mentions a missing `'static` bound on a `T` you
control, add `+ 'static` to the bound — that's the only diff.

This bound is a soft constraint. The few external trait signatures
that drop `'static` from their `DeserializeOwned` bound (notably
`figment::Format::from_str`) are accommodated by a private
internal entry point that bypasses the tag-preserving fast path.
You don't see it; your `Format` impl just works.

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

## Migrating from other YAML libraries

The Rust YAML ecosystem has a cluster of forks downstream of
`serde_yaml`. Each section below is verified against the
crates.io / docs.rs state on **2026-05-08**. If a function name
or type changes upstream, file an issue and we'll update.

### Compatibility matrix

| Crate | Version | Maintained | Drop-in for `serde_yaml`? | `Value` DOM | `Tagged` variant | Builder / options |
|---|---|---|---|---|---|---|
| `serde_yaml` | `0.9.34+deprecated` | archived 2024-03 | — | yes (7) | yes | none |
| [`serde_yml`](#1-serde_yml) | `0.0.12` | archived 2025-09 | mostly | yes (7) | yes | none |
| [`yaml_serde`](#2-yaml_serde) | `0.10.4` | active | yes (Cargo `package =` rename) | yes (7) | yes | none |
| [`serde-yaml-ng`](#3-serde-yaml-ng) | `0.10.0` | active (slow cadence) | yes | yes (7) | yes | none |
| [`serde-norway`](#4-serde-norway) | `0.9.42` | active | yes | yes (7) | yes | none |
| [`serde-yaml-bw`](#5-serde-yaml-bw) | `2.5.6` | active | **no** (breaking 2.x) | yes (8, +`Alias`) | yes | `SerializerBuilder`, `*Options` |
| [`serde-saphyr`](#6-serde-saphyr) | `0.0.26` | active | **no** (different design) | **no DOM** | n/a | `Options` / `SerializerOptions` |
| [`yaml-spanned`](#7-yaml-spanned) | `0.0.3` | active (early) | **no** (read-only, no `to_string`) | yes (+ spans) | yes | programmatic `Builder` |

> The 7-variant `Value` enums in `serde_yaml`, `serde_yml`,
> `yaml_serde`, `serde-yaml-ng`, and `serde-norway` are
> structurally identical: `Null | Bool | Number | String |
> Sequence | Mapping | Tagged(Box<TaggedValue>)`. `noyalib::Value`
> matches that shape and adds nothing — the migration is a path
> rename for those four crates.

### 1. `serde_yml`

[`serde_yml`](https://crates.io/crates/serde_yml) was a
continuation fork of `serde_yaml`. The upstream repo was
**archived 2025-09-03**; `0.0.12` (2024-08-25) is the last
release. `cargo audit` does not yet have an advisory for it,
but the maintenance state matches `serde_yaml`'s.

API surface is identical to `serde_yaml`'s, including the same
7-variant `Value` (with `Tagged(Box<TaggedValue>)`) and the
`with::singleton_map*` family.

```diff
-[dependencies]
-serde_yml = "0.0"
+[dependencies]
+noyalib = "0.0"
```

```diff
-use serde_yml::{from_str, to_string, Value};
+use noyalib::{from_str, to_string, Value};
```

The function-by-function table earlier in this document applies
verbatim — substitute `serde_yml::` for `serde_yaml::`.

### 2. `yaml_serde`

[`yaml_serde`](https://crates.io/crates/yaml_serde) (also
resolvable as `yaml-serde`) is the actively maintained fork
under the `yaml` org. `0.10.4` (2026-03-11). The README markets
it as a true drop-in: it documents a Cargo package rename so
existing `use serde_yaml::*;` imports keep compiling without
touching call sites.

```diff
-[dependencies]
-serde_yaml = "0.9"
+[dependencies]
+# either:
+yaml_serde = "0.10"
+# or stay on the serde_yaml import path via Cargo's `package`:
+# serde_yaml = { package = "yaml_serde", version = "0.10" }
```

For `noyalib`, the same approach drops the rename entirely:

```diff
-[dependencies]
-yaml_serde = "0.10"
+[dependencies]
+noyalib = "0.0"
```

```diff
-use yaml_serde::{from_str, to_string, Value};
+use noyalib::{from_str, to_string, Value};
```

The 7-variant `Value` shape matches; the function-by-function
table applies (substitute `yaml_serde::` for `serde_yaml::`).

### 3. `serde-yaml-ng`

[`serde-yaml-ng`](https://crates.io/crates/serde-yaml-ng)
(crate name `serde_yaml_ng`) is `acatton`'s independent
continuation, marketed as "as compatible as possible". Last
release `0.10.0` (2024-05-26) — actively maintained but the
release cadence is slow.

```diff
-[dependencies]
-serde_yaml_ng = "0.10"
+[dependencies]
+noyalib = "0.0"
```

```diff
-use serde_yaml_ng::{from_str, to_string, Value};
+use noyalib::{from_str, to_string, Value};
```

7-variant `Value`, identical surface, no migration tax beyond
the path rename.

### 4. `serde-norway`

[`serde-norway`](https://crates.io/crates/serde-norway) (crate
name `serde_norway`) is `cafkafk`'s hard-fork. `0.9.42`
(2024-12-21). The fork name nods at the YAML 1.1 "Norway
problem" (`country: NO` parsing as `false`); the API is
serde_yaml-shape-identical.

```diff
-[dependencies]
-serde_norway = "0.9"
+[dependencies]
+noyalib = "0.0"
```

```diff
-use serde_norway::{from_str, to_string, Value};
+use noyalib::{from_str, to_string, Value};
```

`noyalib` defaults to YAML 1.2 strict booleans, so the Norway
problem stays fixed under the migration.

### 5. `serde-yaml-bw`

[`serde-yaml-bw`](https://crates.io/crates/serde-yaml-bw) (crate
name `serde_yaml_bw`) is `bourumir-wyngs`'s hardened fork.
`2.5.6` (2026-05-02). The README explicitly states this is
**not** a drop-in replacement for `serde_yaml` — the major-version
bump signals breaking API changes (added merge keys, nested
enums, binary scalars, billion-laughs hardening). The `Value`
enum has 8 variants instead of 7 (extra `Alias` variant for
anchor references); most scalar variants carry an optional
anchor field.

Migration to `noyalib`:

```diff
-[dependencies]
-serde_yaml_bw = "2"
+[dependencies]
+noyalib = "0.0"
```

```diff
-use serde_yaml_bw::{from_str, to_string, Value};
+use noyalib::{from_str, to_string, Value};
```

API differences worth knowing about when porting:

| `serde_yaml_bw` 2.x | `noyalib` |
|---|---|
| `from_str_value` / `from_str_value_preserve` | `from_str::<Value>` (`Tagged` is preserved by default) |
| `from_str_multi` / `from_slice_multi` / `from_reader_multi` / `from_multiple` | `noyalib::load_all_as::<T>` |
| `to_string_multi` / `to_writer_multi` | `noyalib::to_string_multi` / `noyalib::to_writer_multi` |
| `SerializerBuilder::default().check_unresolved_anchors(true).build(&mut w)` | `noyalib::SerializerConfig` + `to_writer_with_config` |
| `*_with_options(input, options)` | `noyalib::from_str_with_config(input, &cfg)` |
| `Value::Alias(name)` | resolved automatically; aliases dereference to their target value during parse |

`noyalib` resolves aliases at parse time (per YAML 1.2) and does
not surface a separate `Alias` variant in `Value`. If your code
pattern-matches on `Alias`, replace those arms with the
post-resolution variant (`String`, `Mapping`, etc.).

### 6. `serde-saphyr`

[`serde-saphyr`](https://crates.io/crates/serde-saphyr) (module
name `serde_saphyr`) is `bourumir-wyngs`'s clean-room serde
adapter for the `saphyr` parser. `0.0.26` (2026-05-04). The
README states "not a fork of serde-yaml and shares no code with
it (apart from some reused tests)". Importantly, **there is no
`Value` DOM** — `serde-saphyr` streams events directly into the
typed target. If you used `serde_yaml::Value` for dynamic
inspection, you have to either (a) define a typed schema, or
(b) sink to `serde_json::Value`.

`noyalib`'s default `from_str<T>` path is also streaming-first,
so the typed-target migration is a name swap. The `Value` path
is also available when you need it.

```diff
-[dependencies]
-serde-saphyr = "0.0"
+[dependencies]
+noyalib = "0.0"
```

```diff
-use serde_saphyr::{from_str, to_string};
+use noyalib::{from_str, to_string, Value};
```

Surface mapping:

| `serde_saphyr` | `noyalib` |
|---|---|
| `from_str::<T>` | `noyalib::from_str::<T>` |
| `from_slice::<T>` | `noyalib::from_slice::<T>` |
| `from_reader::<R, T>` | `noyalib::from_reader::<R, T>` |
| `from_multiple::<T>` | `noyalib::load_all_as::<T>` |
| `to_string` | `noyalib::to_string` |
| `to_io_writer` | `noyalib::to_writer` |
| `to_fmt_writer` | `noyalib::to_fmt_writer` |
| `to_string_multiple` | `noyalib::to_string_multi` |
| `from_str_with_options(input, options)` | `noyalib::from_str_with_config(input, &cfg)` |
| `serde_saphyr::Options` (builder) | `noyalib::ParserConfig` |
| `serde_saphyr::SerializerOptions` | `noyalib::SerializerConfig` |
| (no `Value` type) | `noyalib::Value` (7 variants, including `Tagged`) |
| (no `Mapping` type) | `noyalib::Mapping` |

`serde-saphyr`'s "panic-free" guarantee corresponds to
`noyalib`'s `#![forbid(unsafe_code)]` plus the resource-limit
gates (`max_depth`, `max_alias_expansions`,
`max_document_length`); both crates target the same
"defensive parser" niche.

### 7. `yaml-spanned`

[`yaml-spanned`](https://crates.io/crates/yaml-spanned) is
`romnn`'s span-aware parser built on `libyaml-safer`. `0.0.3`
(2025-12-27). Different problem domain — it captures source
spans for diagnostics, but **does not provide
`to_string` / `to_writer`** (read-only / DOM-construction
crate). Often paired with a serializer like `serde_yaml`, not
used to replace one.

If you used `yaml-spanned` solely for its span info, `noyalib`'s
`Spanned<T>` wrapper covers the same use case while staying in
the typed-deserialise path:

```diff
-use yaml_spanned::{from_str, Value};
-let v: Value = from_str(input)?;
-let line = v.span().start.line;
+use noyalib::{from_str, Spanned};
+#[derive(serde::Deserialize)]
+struct Config { name: Spanned<String> }
+let cfg: Config = noyalib::from_str(input)?;
+let line = cfg.name.start.line();
```

Function mapping:

| `yaml_spanned` | `noyalib` |
|---|---|
| `from_str` (returns `Spanned<Value>`) | `noyalib::from_str::<Spanned<T>>` or `from_str::<Value>` then walk spans via `noyalib::span_context` |
| `from_str_all` | `noyalib::load_all_as::<Spanned<T>>` |
| `from_str_lossy*` (drop-on-error iterators) | `noyalib::load_all` (yields `Result` per doc — handle errors explicitly rather than silently dropping) |
| `from_value` / `to_value` | `noyalib::from_value` / `noyalib::to_value` |
| `Builder` (programmatic Value construction) | `noyalib::Value::Mapping(…)` and the `From`/`Into` impls; `noyalib::cst::Document` for source-faithful programmatic edits |
| (no serializer) | `noyalib::to_string` / `noyalib::to_writer` |

`yaml-spanned` is built on `libyaml-safer`, so it transitively
relies on `unsafe`. `noyalib` is `#![forbid(unsafe_code)]`
end-to-end and ships its span-tracking under the same lint.

### Cross-library `Value` matching

If you previously matched exhaustively on a 7-variant `Value`,
the noyalib match needs the same 7 arms — same names, same
shapes:

```rust
match v {
    noyalib::Value::Null        => …,
    noyalib::Value::Bool(_)     => …,
    noyalib::Value::Number(_)   => …,
    noyalib::Value::String(_)   => …,
    noyalib::Value::Sequence(_) => …,
    noyalib::Value::Mapping(_)  => …,
    noyalib::Value::Tagged(_)   => …,  // treat as inner; call .untag()
}
```

If you came from `serde-yaml-bw`'s 8-variant `Value` (with
`Alias`), drop the `Alias` arm — `noyalib` resolves aliases at
parse time. If you came from `serde-saphyr` (no DOM), this
section doesn't apply.

### Open an issue if your migration hits something not covered here

The migration paths above are verified against published
crates.io state on **2026-05-08**. Any of these crates may
diverge in a future release. If your migration runs into a
shape we don't cover, file an issue with the upstream crate
version and the input that misbehaves —
<https://github.com/sebastienrousseau/noyalib/issues>.
