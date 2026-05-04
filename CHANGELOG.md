# Changelog

All notable changes to this project are documented here.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.0.1] - 2026-05-04

The launch release. Every section below catalogues a deliberate
phase of work that was bundled into v0.0.1 — see
[`docs/design/`](docs/design/) for the architecture rationale and
the per-phase commit messages on `main` for full context.

### Added — Spec & compliance (Phase 0)

- **Native YAML 1.2 scanner and parser**, written entirely in safe
  Rust — `#![forbid(unsafe_code)]` at the crate root.
- **100% YAML Test Suite compliance — literal**: 406/406 cases
  pass with **zero skips**. The skip list — used during
  development to bound the work — is empty.
- Full serde `Serialize` and `Deserialize` support including
  `#[serde(flatten)]`, `#[serde(default)]`, `#[serde(rename)]`,
  enum representations (externally-tagged, internally-tagged,
  adjacently-tagged, untagged).
- **Multi-document streams**: `load_all`, `load_all_as`,
  `to_string_multi`, `to_writer_multi`, `from_str_multi` (under
  the `compat-serde-yaml` feature) — `---` / `...` separators
  honoured, byte-faithful concatenation when paired with the
  CST.
- **YAML 1.1 compatibility** via `ParserConfig::legacy_booleans`:
  resolves `yes`/`no`/`on`/`off`/`y`/`n` as booleans (the
  "Norway problem" — opt-in, never silent).
- **Strict-mode hardening**: `ParserConfig::strict_booleans`,
  depth limits, document-length cap, alias-expansion cap,
  duplicate-key policy, recursion-depth probe.

### Added — Frictionless migration (Phase 1)

- **Comment-aware reads** (`load_comments`, `Comment`,
  `CommentKind`) — extract leading / trailing / standalone
  comments without touching the typed `Value` path.
- **`noyafmt` CLI**: lossless YAML formatter that round-trips
  through the CST, normalising whitespace and quoting without
  changing semantics.
- **`noyalib-mcp`**: Model Context Protocol server exposing
  `parse`, `format`, `get`, `set`, `validate` tools — drop-in
  for any LLM agent that needs YAML manipulation.
- **WASM playground** (`noyalib-wasm`): 201 KB
  `wasm32-unknown-unknown` build with browser demo.

#### Added — `serde_yaml` compat shim (Phase 1.1)

- **`compat-serde-yaml` feature**: drop-in surface for the
  unmaintained `serde_yaml` 0.9 crate.
- Type-level parity with `serde_yaml::Value`,
  `serde_yaml::Mapping`, `serde_yaml::Number` via `From` /
  `TryFrom` conversions both directions, with
  `SerdeYamlConversionError { NonStringKey, UnrepresentableNumber }`
  for the lossy edges.
- `noyalib::compat::serde_yaml::Error` re-export wrapping
  `noyalib::Error` with location parity.
- **`Document::validate`**: non-panicking sibling of `ensure_cache`
  for callers that want to surface invalid-source errors as
  `Result` rather than via lazy panic.

#### Added — `!!binary` first-class support (Phase 1.2)

- **`!!binary` tag** with RFC 4648 base64 codec
  (`src/base64.rs`, hand-rolled, whitespace-tolerant decoder).
- `serde_bytes::Bytes` / `ByteBuf` round-trip including
  multi-line block-scalar form, inline form, quoted form, and
  the full 0..=255 byte range.
- `Value::Tagged` carries `Tag::new("!!binary")` for callers
  that walk the typed tree.

#### Added — `Spanned<Value>` flatten guard (Phase 1.3)

- Bare `Value` as the target of `#[serde(flatten)]` collects
  unmatched keys into a `Value::Mapping` exactly as
  `serde_yaml` / `serde_json` users expect.
- `Spanned<Value>` in a `#[serde(flatten)]` position now errors
  with a clear, actionable message pointing at the working
  alternative (bare `Value` + `Document::span_at`) instead of
  the bare `missing_field` gibberish that resulted from serde's
  `FlatStructAccess` filtering.

### Added — Lossless editing API (Phase 2)

- **Side-table CST** (`noyalib::cst`) for byte-faithful
  round-tripping: `parse_document(s)?.to_string() == s` for any
  input the parser accepts.
- `Document::source`, `Document::span_at`, `Document::get`,
  `Document::comments_at`, `Document::syntax`,
  `Document::as_value` for read access by path.
- `Document::set`, `Document::set_value`, `Document::remove`,
  `Document::push_back`, `Document::insert_after`,
  `Document::replace_span` for mutation — every edit is
  byte-faithful outside the spliced region; comments, blank
  lines, and sibling formatting survive verbatim.
- **Phase A.1 incremental repair**: localised `replace_span`
  re-parses the smallest enclosing block; Document-scope
  re-parse only on shape inversion.
- **Phase A.2 lazy `Value` / `SpanTree`**: typed cache
  invalidated rather than re-parsed eagerly — successive edits
  in a batch don't pay the parser cost; the deferred parse
  runs once on the first read (~6× single edit).
- **Phase A.3 green-tree path resolution**: walks the structural
  CST directly, skipping the typed cache for the common
  set-then-set pattern (~7.6× batch).
- **Phase B relative-len leaves**: O(log N) splice — the green
  node only stores child lengths, not absolute byte ranges
  (~37× over baseline).

#### Added — `Entry` API (Phase 2.1)

- **`Document::entry(path) -> Entry<'_>`** path-shaped mutable
  handle, complementing the functional `set` / `remove` /
  `push_back` / `insert_after` methods (both stay first-class).
- 12 methods on `Entry`: `path`, `exists`, `get`, `span_at`,
  `comments`, `set`, `set_value`, `remove`, `insert`,
  `insert_value`, `push_back`, `insert_after`, plus chained
  drill-down via `Entry::entry(child)` with smart path
  composition (`items[0]` not `items.[0]`).
- New primitive `Document::insert_entry` — mapping-side
  analogue of `push_back` for sequences.

#### Added — automatic indent detection (Phase 2.2)

- **`Document::indent_unit()`**: detects 2- / 3- / 4-space block
  indents from non-empty/non-comment line deltas; defaults to 2
  when undetectable. Tab-indented lines short-circuit.
- `Entry::insert_value` and `Document::insert_entry` plumb the
  detected unit into the serializer so inserts conform to the
  surrounding file's convention.
- Bug fix bundled: `column_of_key_at` now walks back to the
  actual key line (not the value's first byte), so a sibling
  insert under a parent whose last value is a nested block
  lands at the correct column.

#### Added — anchor management ("Smart Aliases") (Phase 2.3)

- **`Document::anchors()`**, **`aliases()`**, **`aliases_of(name)`**:
  every `&name` / `*name` lexeme in source order with byte spans.
- **`Document::materialise_alias_at(byte_pos)`**: replace `*name`
  with the source bytes of `&name`'s scalar value, leaving the
  alias's site independent of any future edits to the anchor.
- **`Document::materialise_aliases_of(name)`**: bulk; reverse
  source-order so each splice's offsets stay valid.
- Propagation contract documented: edits to anchored values are
  visible at every alias site after the next load (because
  aliases are pointers in YAML's data model).
- Multi-line block-valued anchors return a clear "follow-up"
  error pointing at `Document::anchors()` + `replace_span()`
  for manual splicing — out of scope for v0.0.1.

### Added — Contracts & governance (Phase 3)

#### Added — JSON Schema codegen (Phase 3.1)

- **`schema` Cargo feature** (off by default).
- **`pub use schemars::JsonSchema`** — derive imported via
  `noyalib`, no second crate dep for users.
- **`schema_for::<T>() -> Result<Value>`**: schema as a
  `noyalib::Value` tree.
- **`schema_for_yaml::<T>() -> Result<String>`**: schema as YAML
  text for sharing / version control.
- Honours `#[doc]` (→ `description`), `#[serde(default)]` (drops
  from `required`), `#[serde(rename)]` (renames property), and
  emits `minimum`/`maximum` for fixed-width integers.

#### Added — Schema validation + enhanced CLI (Phase 3.2)

- **`validate-schema` Cargo feature** (implies `schema`).
- **`validate_against_schema(value, schema) -> Result<()>`**:
  enforce a JSON Schema 2020-12 contract against parsed YAML.
  Multiple violations aggregated with RFC 6901 JSON-pointer
  paths.
- **`validate_against_schema_str(yaml, schema_yaml)`**:
  convenience for raw text.
- **`noyavalidate -s/--schema PATH`**: validate each parsed
  document against the schema (YAML or JSON; both parse).
  Multi-doc streams prefix each failing document with
  `[document N]`.
- **`noyavalidate --fix`**: in-place lossless reformat via the
  CST formatter. Stdin + `--fix` keeps stdout clean for piping.
- **Critical guard**: `--fix` does NOT run when `--schema`
  rejects the input — otherwise a buggy file would be silently
  rewritten with the violation in place.

### Added — SIMD primitives + hot-path integration (Phase 4)

- **`noyalib::simd` module**: pure-safe Rust multi-byte search
  primitives.
- `find_any_of(haystack, needles) -> Option<usize>` — dispatches
  to `memchr` for arity 1/2/3, SWAR (8-byte-stride packed
  membership lookup) for arity 4+.
- `clean_prefix_len(haystack, needles)` — length of the leading
  no-needle run; the "skip-clean-prefix" call shape.
- `ByteBitmap` + `bitmap_for(needles)` + `find_byte_in_bitmap` —
  256-bit bitmap surface for callers amortising bitmap
  construction across many calls with the same needle set.
- **Hot-path integration**: the plain-scalar inner loop in
  `fetch_plain_scalar` skips ahead via `clean_prefix_len`
  before applying the state-dependent boundary rules.
  Equivalence-tested against the byte-by-byte baseline; YAML
  1.2 official suite stays at 100% with the integration on.
- **Throughput** (Apple M1, criterion --quick, 64 KiB sparse
  haystack): arity-3 memchr 29 GiB/s vs scalar 509 MiB/s
  (~58×); arity-8 SWAR 1.45 GiB/s vs scalar 270 MiB/s (~5.4×).
- **`unsafe_code = "forbid"` invariant preserved** — no
  `core::arch::*` intrinsics, no platform-specific deps.

### Performance

Benchmarked on Apple M4, Rust 1.94 stable:

| Benchmark | noyalib | serde\_yaml\_ng | Improvement |
|---|---|---|---|
| Serialize (simple) | 358 ns | 1.41 us | **75% faster** |
| Serialize (nested) | 2.80 us | 8.32 us | **66% faster** |
| Deserialize (simple) | 1.39 us | 2.79 us | **50% faster** |
| Deserialize (nested) | 9.16 us | 17.3 us | **47% faster** |
| Deserialize (large) | 0.83 ms | 1.49 ms | **44% faster** |

CST-only metrics (Apple M1, criterion --quick, batch of 500
single-key edits):

| Optimisation | Speedup |
|---|---|
| Phase A.1 — incremental repair | baseline |
| Phase A.2 — lazy `Value`/`SpanTree` | ~6× single edit |
| Phase A.3 — green-tree path resolution | ~7.6× batch |
| Phase B — relative-len leaves | ~37× over baseline |

### Added — API surface (foundation)

- `Value`, `Mapping`, `MappingAny`, `Sequence`, `Number`, `Tag`,
  `TaggedValue` types.
- `from_str`, `from_slice`, `from_reader`, `from_value`
  deserialization functions.
- `to_string`, `to_writer`, `to_fmt_writer`, `to_value`
  serialization functions.
- All functions available with `_with_config` variants for
  custom security / formatting limits.
- `SerializerConfig` with indent, flow style, scalar style,
  block scalars, document markers, `quote_all`,
  `compact_list_indent`, `folded_wrap_chars`, `min_fold_chars`.
- `ParserConfig` with depth limits, document-length limits,
  alias-expansion caps, duplicate-key policy,
  `strict_booleans`, `legacy_booleans`.
- **`Streaming` deserializer** (`StreamingDeserializer`):
  bypasses the `Value` AST for typed deserialization (50%
  faster than the Value-based path).
- **`BorrowedValue<'a>`**: zero-copy AST that borrows strings
  from input — 18% faster than the owned `Value`.
- **Path queries**: `value.query("items[*].name")` with
  wildcards (`*`) and recursive descent (`..`).
- **`Spanned<T>`** for tracking source line, column, and byte
  offset of deserialized values.
- **`apply_merge()`** for YAML merge key (`<<`) expansion.
- **`Path` type** for structured error location tracking.
- **Anchor & alias support**: `RcAnchor`, `ArcAnchor`,
  `RcWeakAnchor`, `ArcWeakAnchor`, `AnchorRegistry`,
  `ArcAnchorRegistry`.
- **`fmt` module**: `FlowSeq`, `FlowMap`, `LitStr`, `FoldStr`,
  `Commented`, `SpaceAfter`.
- **`with` module**: `singleton_map`, `singleton_map_optional`,
  `singleton_map_recursive`, `singleton_map_with`.
- **YAML 1.2 spec-schemas**: `validate_yaml_core_schema`,
  `validate_yaml_json_schema`, `validate_yaml_failsafe_schema`,
  `is_yaml_failsafe_compatible`, `is_yaml_json_compatible`.
- **`miette` diagnostic integration** (`miette` feature): rich
  terminal diagnostics with error codes, help text, source
  spans.
- **`garde` / `validator` integration** (`garde` / `validator`
  features): declarative post-deserialise validation via
  `Validated<T>` / `ValidatedValidator<T>`.
- **`#[non_exhaustive]`** on `ParserConfig`, `SerializerConfig`,
  `FlowStyle`, `ScalarStyle`.
- **`#[must_use]`** on 83 query methods.

### Added — Tooling & CLIs

- **`noyavalidate`**: validate YAML syntax (and optional JSON
  Schema) with rich `miette` diagnostics. `--schema` and
  `--fix` flags shipped in Phase 3.2.
- **`noyafmt`**: lossless CST-driven formatter.
- **`noyalib-mcp`**: Model Context Protocol server (separate
  workspace member).
- **`noyalib-wasm`**: WASM bindings + browser playground
  (separate workspace member).

### Added — Examples

- **45 branded examples** under `examples/`, each with the
  animated spinner UI from `examples/support.rs`.
- Categorised into Core, Spec, Logic & Security, DX, Advanced,
  Future-Proof, Deep Rust, Final, Platform, and Competitive
  Features.

### Added — Testing

- **2,200+ tests** including YAML spec compliance,
  property-based tests (`proptest`), competitor parity tests
  (`yaml-rust2`, `serde-saphyr`, `yaml_lib`, `rust-yaml`,
  `serde_yaml_ng`), and edge cases.
- **5 fuzz targets** (`cargo fuzz`) with seed corpus committed
  under `fuzz/corpus/seed/`.
- **Differential fuzz smoke** in CI (10 s per push).
- **Soak fuzz** (weekly, 1 hour per target) under
  `.github/workflows/security.yml`.
- **YAML 1.2 official suite vendored** under
  `tests/yaml-test-suite/` (MIT, upstream).
- **Cross-platform CI**: Linux, macOS, Windows × stable,
  1.75.0 (MSRV), nightly. Nightly is `continue-on-error`.

### Added — Supply chain & governance

- **`#![forbid(unsafe_code)]`** at the crate root.
- **`unreachable_pub = "forbid"`**, `non_ascii_idents = "forbid"`,
  full `clippy::all + pedantic + cargo + nursery` policy.
- **MSRV pinned at 1.75.0** with a dedicated CI job.
- **`cargo-deny`** licenses + advisories + bans + sources.
- **`cargo-vet`** with the Mozilla, Google, Bytecode Alliance,
  Embark, ISRG audit imports plus a bootstrap exemption list.
- **`cargo-semver-checks`** on every PR (gated against
  pre-publication state until the first crates.io release).
- **OpenSSF Scorecard** badge.
- **CodeQL** static analysis.
- **REUSE.software 3.3 compliance** — every file has SPDX
  copyright + license headers, blanket `REUSE.toml`
  annotations cover meta / CI / docs / fixtures.
- **SLSA L3 provenance** + **sigstore** signing in the
  release workflow.
- **SHA256 / SHA512 checksums** + **SBOM** generated per
  release.
- **`Assisted-by:` trailer** auto-injected on every commit per
  the Linux kernel coding-assistants standard.
- **Signed commits** (SSH ed25519) verified by CI.

### Added — `no_std` posture

- Full `#![no_std]` support: `default-features = false` keeps
  the `alloc`-only build working. Core parsing / serialization
  (`from_str`, `to_string`, `Value`, schemas) and the streaming
  deserializer all run without `std`.
- I/O functions (`from_reader`, `to_writer`),
  `Spanned<T>` deserialization (thread-local storage), the
  `cst` module, and the `noyavalidate` / `noyafmt` CLIs require
  the `std` feature.
- **CI enforces `cargo check --no-default-features` on every
  push.**

### Added — Cargo feature matrix

| Feature | Default | Pulls in |
|---|---|---|
| `std` | yes | (none — gates std-only items) |
| `miette` | no | `miette` rich diagnostics |
| `garde` | no | `garde` 0.22 derive-based validation |
| `validator` | no | `validator` 0.19 derive-based validation |
| `compat-serde-yaml` | no | `serde_yaml` 0.9 (drop-in shim) |
| `schema` | no | `schemars` 1.2 + `serde_json` (codegen) |
| `validate-schema` | no | implies `schema` + `jsonschema` 0.33 |
| `noyavalidate` | no | binary feature: `std` + `miette` + `validate-schema` |
| `simd` | no | currently a no-op (forward-reserved) |
| `robotics` | no | numeric helpers for robotics workloads |
| `wasm-opt` | no | size-tuned WASM build profile |

[Unreleased]: https://github.com/sebastienrousseau/noyalib/compare/v0.0.1...HEAD
[0.0.1]: https://github.com/sebastienrousseau/noyalib/releases/tag/v0.0.1
