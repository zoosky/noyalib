# Changelog

All notable changes to this project are documented here.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed — CST short-pointer compression

- `GreenChild::Token { len }` is now `u32` (was `usize`). YAML
  documents are bounded at 4 GiB by the parser's
  `max_document_length` cap, so a `u32` is sufficient. The
  narrower field drops `GreenChild::Token` from 24 bytes to 8
  bytes on a 64-bit target — meaningfully better L1/L2 cache
  locality on tree traversals.
- `GreenNode.text_len` is similarly narrowed from `usize` to
  `u32` (private field; public `text_len()` accessor still
  returns `usize` for ergonomic call-site arithmetic).
- Public `text_len()` accessors on `GreenChild` and `GreenNode`
  preserve their `usize` return type — the narrower storage is
  widened at the API boundary so existing callers continue to
  compile.
- 406/406 YAML 1.2 spec compliance preserved; full test suite +
  doctest sweep green.

### Added — Parallel multi-document parsing

- **`noyalib::load_all_as_parallel<T>(yaml) -> Result<Vec<T>>`**
  and **`noyalib::load_all_parallel(yaml) -> Result<Vec<Value>>`**
  — pre-scan `---` document boundaries on a single thread, then
  deserialise each document in parallel via Rayon. Targets
  multi-document streams (telemetry logs, audit exports,
  Kubernetes-resource snapshots) where single-thread parsing is
  CPU-bound.
- **`noyalib::parallel::split_documents(input) -> Vec<&str>`** —
  the standalone document-boundary pre-scanner. Useful when the
  caller wants to drive their own concurrency primitives (async
  tasks, custom thread pools).
- Gated behind the `parallel` Cargo feature (off by default —
  pulls in `rayon` only when the user asks for it).
- 10 unit tests covering boundary detection edge cases (no
  separators, empty input, implicit first document, mid-line
  `---`, dashes followed by non-whitespace) and end-to-end
  correctness against `load_all_as`.

### Added — SWAR decimal-integer parser

- **`noyalib::simd::parse_decimal_u64` / `parse_decimal_i64`** —
  branch-free 8-digits-per-cycle SWAR pipeline replacing the
  stdlib byte-by-byte loop. Three pair-wise multiply-add phases
  fold a `u64` chunk of ASCII digits into the parsed value with
  no per-byte branch.
- Plumbed into the streaming integer resolver
  (`crate::streaming::parse_integer`); base-10 plain scalars now
  flow through the SWAR path. Hex / octal / sign-prefixed paths
  retain stdlib for spec-correct overflow semantics.
- Bench results (`benches/numeric_parse.rs`):
  - 3-digit input: parity with stdlib (SWAR doesn't engage).
  - **8 digits: 2.17× faster** (8.12 ns → 3.74 ns).
  - **19 digits: 2.38× faster** (22 ns → 9.25 ns).
  - **i64::MAX / i64::MIN: 2.5× faster.**
  - **Bulk parse of 1000 integers: 47 % faster.**
- Validation: every byte checked in `b'0'..=b'9'` before
  arithmetic; `wrapping_mul` is intentional in the SWAR pipeline
  (high bits discarded by downstream shift-and-mask) and the
  validator rejects malformed input. Overflow returns `None`.
- 11 unit tests including baseline equivalence against
  `<u64 as FromStr>::from_str` across 19 representative values
  (covers `i64::MIN`, `i64::MAX`, `u64::MAX`, sign handling).
- 406/406 YAML 1.2 spec compliance preserved.

### Added — Canonical scanner needle constants

- **`simd::BLOCK_PLAIN_NEEDLES`** / **`simd::FLOW_PLAIN_NEEDLES`** /
  **`simd::LINE_BREAK_NEEDLES`** — public `&[u8]` constants
  documenting the YAML 1.2 plain-scalar boundary candidate sets
  per parser context. Future scanner refactors can reach the
  canonical set via these names without re-deriving them.

### Added — Structural bitmask discovery (`simdjson`-style)

- **`SimdScanner::structural_bitmask_32(&[u8; 32]) -> u32`** — load
  a 32-byte chunk and produce a dense bitmask where bit `i` is set
  iff `chunk[i]` is in the scanner's needle set. The building
  block of `simdjson`-style structural discovery: instead of
  walking the haystack and stopping at every delimiter, callers
  drain the mask via `mask.trailing_zeros()` + `mask & (mask - 1)`
  and advance the parser state machine directly from one delimiter
  to the next.
- **`StructuralIter`** — iterator that walks every structural-byte
  position in a haystack of arbitrary length. Handles the chunk
  loop, the partial-chunk tail, and the cached-bit drain
  internally so callers see one stream of byte offsets in order.
- Bench results (`benches/structural_bitmask.rs`, real YAML-shaped
  input):
  - **Stable Rust**: 4.2× faster than the existing memchr-loop
    structural-discovery path across 4 KiB / 64 KiB / 1 MiB.
  - **Nightly with `nightly-simd`**: 9.2× faster than the same
    baseline (single `Simd<u8, 32>` chunk + branchless
    `to_bitmask()` per 32-byte window).
- Five unit tests + cross-needle-set baseline equivalence check
  (every YAML-relevant arity 1 / 2 / 3 / 7 / 10) and four
  `StructuralIter` correctness tests covering chunk-boundary
  straddles, partial tails, and 2 KiB adversarial inputs against a
  scalar baseline.

### Removed — `thiserror` runtime dependency

- noyalib's `Error` enum no longer derives `thiserror::Error`. The
  `Display` and `std::error::Error` impls are now hand-written
  (matching the previous `#[error(...)]` format strings byte-for-
  byte, so all `Display` output is stable across the migration).
- Drops the proc-macro from every downstream crate's compile graph
  — meaningful for downstream build times in big workspaces.
- The runtime dep list is now: `serde`, `indexmap`, `rustc-hash`,
  `itoa`, `ryu`, `memchr`, `smallvec`. Every other dep is feature-
  gated and off by default.

### Changed — `serde` defaults synchronised with our `std` feature

- `serde = { default-features = false, features = ["derive",
  "alloc"] }` plus `std = ["serde/std"]` so `cargo build
  --no-default-features` actually compiles in no_std mode (serde's
  `de::Error` super-trait `StdError` resolves to a no_std-friendly
  bound when serde is itself in no_std mode).

### Removed — `serde_yaml` 0.9 upstream dependency

- The `compat-serde-yaml` shim **no longer pulls in the
  unmaintained `serde_yaml` 0.9 crate**. Every type the shim
  exposes (`Value`, `Mapping`, `Number`, `Sequence`, `Tag`,
  `TaggedValue`, `Error`, `Location`) is a noyalib-native type
  re-exported under the `serde_yaml` name; downstream
  `cargo audit` / `cargo deny` runs no longer pick up the
  archived advisory chain.
- The previous direct `From<noyalib::Value> for ::serde_yaml::Value`
  / `TryFrom<::serde_yaml::Value> for noyalib::Value` impls are
  removed. Mid-migration codebases route in-flight upstream values
  through the Serde data model instead — the universal-translator
  path the Serde ecosystem already provides for every JSON-shaped
  AST pair: `noyalib::to_value(&upstream_serde_yaml_value)?`.

### Added — Release-candidate examples and benches

- **`examples/entry_api.rs`** — surgical Kubernetes manifest
  patching via the `Document::entry` proxy API. Demonstrates
  `or_insert` / `insert_value` / `set` chained edits with every
  comment, indent, and sibling preserved byte-for-byte.
- **`examples/flattened.rs`** — `Flattened<T>` capture pattern:
  typed view + raw metadata view from one parse pass.
- **`examples/schema_validation.rs`** — library-level
  `schema_for` + `validate_against_schema` + `coerce_to_schema`
  pipeline. Mirrors what `noyavalidate --fix` does on the CLI.
- **`benches/streaming_vs_value.rs`** — head-to-head throughput
  comparison between `StreamingDeserializer` and
  `from_str::<Value>` across small / medium / large workloads,
  plus a dedicated `BTreeMap` MapAccess scenario.
- **`benches/large_doc_soak.rs`** — 1 MiB / 10 MiB / 50 MiB soak
  benchmark catching quadratic regressions and SIMD hot-path
  regressions on long-input workloads.

### Changed — MSRV-1.75 hardening

- Pinned `indexmap` to `2.10.0` and `rustc-hash` to `2.0.0` so the
  resolver does not pull manifests requiring Rust 2024 edition
  (Cargo 1.85+) and breaking the MSRV-1.75 check.
- Removed unused `yaml_lib` dev-dependency (its manifest also
  required edition 2024).
- Promoted `serde-saphyr` (optional `compare-saphyr` feature):
  the saphyr lineage adopted edition 2024 across all available
  versions; gating it lets the comparison benchmarks still run on
  newer toolchains while keeping the default 1.75 build path
  clean.
- Demoted `pub` → `pub(crate)` on internal `Span`, `Token`,
  `ScanError`, `ParsedDocument`, `SubtreeContext` fields to
  satisfy the workspace `unreachable_pub = "forbid"` lint on
  Rust 1.75 (the lint behaviour tightened between 1.75 and the
  current stable, so the existing `pub` declarations on
  `pub(crate)` parents only failed under the older toolchain).

### Added — Streaming `!!binary`

- **`StreamingDeserializer` honours `!!binary` natively** — `serde_bytes`
  byte targets (`Vec<u8>` with `#[serde(with = "serde_bytes")]`,
  `serde_bytes::ByteBuf`) now decode RFC 4648 base64 directly inside
  the streaming path without falling back to the AST. Mirrors the
  AST-path type contract: untagged plain scalars that resolve to
  int / float / bool / null produce a `TypeMismatch` rather than
  silently coercing their UTF-8 representation to bytes.

### Added — Schema-driven type coercion (surgical `--fix`)

- **`noyalib::coerce_to_schema(value, schema) -> Result<usize>`** —
  walks JSON Schema 2020-12 type-mismatch errors against an
  in-memory `Value` and coerces string-shaped values into the
  schema's expected type when the parse succeeds. Targets the most
  common hand-written-YAML failure mode: `port: "8080"` gets
  rewritten to `port: 8080` automatically when the schema says
  `port: integer`.
- Handles three coercions: `String → Integer`, `String → Number`,
  `String → Boolean`. Unparseable inputs are left in place so the
  caller can surface the residue via a follow-up
  `validate_against_schema` call.
- Iterative fix-loop (capped at 1024 passes) re-runs validation
  after each coercion so cascading errors converge cleanly.
- 8 integration tests in `tests/coerce_to_schema.rs` cover the
  three target types, nested objects, sequence items, mixed
  valid / fixable / unfixable inputs, and the no-op case.

### Added — Portable-SIMD structural scanner

- **`SimdScanner` type** in `noyalib::simd` — build-once,
  scan-many byte-set finder optimised for parser inner loops.
  Stable Rust uses the existing memchr / SWAR / bitmap path; the
  new `nightly-simd` Cargo feature widens the inner loop to a
  32-byte `Simd<u8, 32>` chunk via `core::simd` portable SIMD,
  broadcasting each needle and OR-ing equality masks for
  branch-free structural detection.
- **`build.rs` toolchain probe** — emits `cfg(noyalib_nightly)`
  when `rustc --version` reports a nightly channel, so the
  `feature(portable_simd)` attribute is gated on both the user's
  feature flag and the actual compiler — `--all-features` on
  stable continues to compile cleanly.
- Both code paths are exhaustively cross-checked against a scalar
  baseline across needle widths 2 / 4 / 8 / 10 and haystack lengths
  spanning the SIMD chunk boundary (31 / 32 / 33 / 64 / 128 / 1024).

### Added — Pluggable parser policies

- **`noyalib::policy` module** — `Policy` trait with
  `check_event(&PolicyEvent)` and `check_value(&Value)` hooks for
  enforcing organisational "Safe YAML" constraints during parsing.
- **Built-in policies**: `DenyAnchors` (rejects `&name` definitions
  and `*name` aliases — covers the billion-laughs vector and
  audit-readability concerns), `DenyTags` (rejects custom tags
  while permitting YAML 1.2 core tags), `MaxScalarLength(n)` (caps
  individual scalar size in bytes).
- **`ParserConfig::with_policy(p)`** — register one or more
  policies; they run in registration order during the AST loader's
  event walk. The streaming fast-path is bypassed automatically
  when any policy is registered, ensuring uniform enforcement.
- 11 integration tests in `tests/policy.rs` cover each built-in
  policy, custom-policy composition, short-circuit-on-first-error
  semantics, and streaming-path bypass.

## [0.0.1] - 2026-05-04

The launch release. Sections below catalogue every capability the
library ships at launch, grouped by theme. See
[`docs/design/`](docs/design/) for the architecture rationale and
the commit history on `main` for per-change context.

### Added — Property interpolation

- **`Value::interpolate_properties(&map)`** — substitute `${name}`
  references inside string scalars from a property map. Walks
  recursively into sequences, mappings, and tagged values; map
  keys are left unchanged so the schema stays stable. `${{` and
  `}}` escapes preserve literal `${` / `}`. Returns
  `Error::Custom` on unknown placeholders.
- **`Value::interpolate_properties_lossy(&map)`** — same walk,
  but unknown placeholders substitute the empty string instead of
  erroring. Suitable for env-var expansion where missing
  variables should silently degrade.
- Placeholder names match `[A-Za-z_][A-Za-z0-9_.]*` so dotted
  hierarchies like `${db.host}` work.

### Added — serde-ecosystem interop

- **`serde_path_to_error` interop** — verified by
  `tests/serde_ecosystem.rs`; the path through nested structures
  and sequences is reported correctly when wrapping noyalib's
  `Deserializer`.
- **`serde_ignored` interop** — same test file confirms unknown
  fields at the top level and at any depth are surfaced through
  the standard wrapper without noyalib-specific integration.

### Added — `figment` provider

- **`figment` Cargo feature** — pulls in `figment` 0.10 and
  exposes `noyalib::figment::Yaml`, a drop-in `Format` + `Provider`
  that plugs into `Figment::merge` / `Figment::join` chains the
  same way `figment::providers::Toml` and
  `figment::providers::Json` do.
- 8 integration tests in `tests/figment_provider.rs` cover
  string/file extraction, layered merge / join semantics, parse-
  and missing-field error propagation, nested struct round-trip,
  and YAML 1.2 anchor + alias resolution through the provider.

### Added — `ParserConfig` knobs

Four additive `ParserConfig` toggles, all defaulting to YAML 1.2
spec behaviour (zero impact on existing callers):

- **`merge_key_policy`** with [`crate::MergeKeyPolicy`] —
  `Auto` (default) preserves YAML 1.2 §10.2 merge semantics;
  `AsOrdinary` keeps `<<` as a literal key in the resulting
  mapping; `Error` rejects any document containing a `<<` key.
  When set to non-`Auto`, the deserializer routes through the
  AST loader (the streaming path hard-wires the YAML 1.2
  semantics).
- **`no_schema`** — when `true`, every plain scalar surfaces as
  a `Value::String` regardless of whether it would normally
  resolve to `null` / `bool` / int / float. The "Norway problem"
  fix: schema strictness is opt-in. Quoted scalars and explicit
  tags (`!!int`, `!!bool`) are unaffected.
- **`legacy_octal_numbers`** — when `true`, accepts YAML
  1.1-style bare `0`-prefix octal literals (`0644` → 420) in
  addition to the YAML 1.2 `0o644` form. Numerics with `8` or
  `9` digits fall through to decimal even with the toggle on.
- **`ignore_binary_tag_for_string`** — when `true`,
  deserializing `!!binary "ABCD"` into a `String` target yields
  the literal base64 source string rather than rejecting on tag
  mismatch. The canonical bytes path (`Vec<u8>`,
  `serde_bytes::ByteBuf`) is unaffected — it always decodes the
  base64 payload. Useful for migrations from Python pyyaml-style
  applications that treat the tag as advisory.

### Added — `Flattened<T>` capture wrapper

- **`noyalib::Flattened<T>`** — pairs a typed deserialization of
  `T` with the underlying [`Value`] tree captured from the
  source. Solves the "I want `#[serde(flatten)]` plus the dynamic
  view for span lookup / unknown-field detection / schema
  validation" use case that the built-in residue types
  (`HashMap<String, Value>` etc.) erase. Deserializes by
  capturing the input as a [`Value`] first, then re-running
  `T::deserialize` against the captured tree via
  [`crate::from_value`]. Both `flattened.value: T` and
  `flattened.raw: Value` are exposed; `Deref<Target = T>` makes
  the typed view ergonomic. Round-trip transparency on
  serialize: only the typed view is emitted, mirroring
  `Spanned<T>`.

### Added — `legacy_sexagesimal` ParserConfig toggle

- **`ParserConfig::legacy_sexagesimal(true)`** — accept YAML
  1.1-style colon-separated base-60 numbers (`60:00` → 3 600,
  `1:30:00` → 5 400, `-1:30:00` → -5 400) as integers.
  Fractional last-component variant (`1:30:00.5` → 5 400.5)
  resolves to a float. Off by default; YAML 1.2 dropped the
  sexagesimal schema. Robust against false positives:
  components other than the first are clamped to 0..=59 and
  ISO-8601 timestamps with embedded `:` colons are correctly
  classified as strings, not as sexagesimal.

### Added — `JsonSchema` for `noyalib::Value`

- **`impl JsonSchema for noyalib::Value`** (gated by the
  `schema` feature) — emits the JSON Schema 2020-12 idiom for
  "any JSON-expressible value": a `oneOf` union of null,
  boolean, number, string, array, and object, with the array /
  object cases referencing the same `YamlValue` definition
  recursively. Lets users derive [`schemars::JsonSchema`] on a
  struct that has a `Value` field (e.g. an envelope type whose
  `payload` is "any user-supplied YAML") without writing a
  custom impl.

### Added — Mutable-Value experience for the CST

- **`Entry::or_insert(default)`** / **`or_insert_with(f)`** /
  **`or_insert_value(default)`** — std-collections-style
  ergonomics on top of the existing path-shaped Entry handle.
  Returns `Ok(true)` when the splice ran (path was vacant),
  `Ok(false)` when the path was already occupied. Top-level
  keys and sequence-index paths get actionable errors that
  redirect to `Document::set` and `push_back`/`insert_after`
  respectively.
- **`Entry::and_modify(f)`** — closure runs only when the path
  resolves; receives a `&mut Document` for arbitrary
  cross-path mutations. Returns `self` so the standard
  `and_modify(...).or_insert(...)` pattern composes.
- **`Document::rename_anchor(old, new)`** — atomic rename of
  every `&old` declaration and every `*old` reference in one
  operation. Returns the count of touched sites. The whole
  rename is performed as a single `replace_span` over the
  document so intermediate states with mismatched anchor /
  alias names are never observed. Validates `new` against YAML
  1.2 §6.9.2 (no flow indicators or whitespace).

### Added — Style heuristics for CST inserts

- **`Document::dominant_quote_style()`** returns the file's
  preferred scalar quote style (`Plain`, `SingleQuoted`, or
  `DoubleQuoted`) by tallying every quoted scalar in the green
  tree and breaking ties in favour of the simpler form. Plain
  mapping keys are deliberately ignored — the question is
  "when the user *did* quote a value, what did they reach
  for?".
- **`Document::dominant_flow_style()`** returns the dominant
  collection layout (`FlowStyle::Block` or `FlowStyle::Auto`)
  by counting Block vs Flow mappings and sequences.
- **`Entry::insert_value`** now consumes both heuristics: a new
  `Value::String` value gets the file's dominant quote style
  applied to the spliced fragment (manual quoting since the
  serializer's `scalar_style` config does not affect top-level
  scalars); collections continue to splice in block form for
  multi-line emissions. The `dominant_flow_style()` accessor
  is exposed for callers who want to wrap typed collections in
  `fmt::FlowMap` / `fmt::FlowSeq` before serializing.

### Added — Multi-line error snippets

- **`Error::format_with_source_radius(source, radius)`** —
  rustc-style error rendering with `radius` lines of context
  above and below the offending line. Output uses a fixed-width
  gutter (line numbers right-aligned to the widest), a `|` rule,
  and a caret line under the offending column. Falls back to
  plain `Display` when the error has no location or the location
  is past EOF.
- The original [`crate::Error::format_with_source`] is preserved
  byte-for-byte; the radius variant is purely additive.

### Added — Spec compliance

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

### Added — Frictionless migration from `serde_yaml`

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

#### Added — `serde_yaml` compat shim

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

#### Added — `!!binary` first-class support

- **`!!binary` tag** with RFC 4648 base64 codec
  (`src/base64.rs`, hand-rolled, whitespace-tolerant decoder).
- `serde_bytes::Bytes` / `ByteBuf` round-trip including
  multi-line block-scalar form, inline form, quoted form, and
  the full 0..=255 byte range.
- `Value::Tagged` carries `Tag::new("!!binary")` for callers
  that walk the typed tree.

#### Added — `Spanned<Value>` flatten guard

- Bare `Value` as the target of `#[serde(flatten)]` collects
  unmatched keys into a `Value::Mapping` exactly as
  `serde_yaml` / `serde_json` users expect.
- `Spanned<Value>` in a `#[serde(flatten)]` position now errors
  with a clear, actionable message pointing at the working
  alternative (bare `Value` + `Document::span_at`) instead of
  the bare `missing_field` gibberish that resulted from serde's
  `FlatStructAccess` filtering.

### Added — Lossless editing API

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
- **Incremental repair**: localised `replace_span` re-parses the
  smallest enclosing block; Document-scope re-parse only on
  shape inversion.
- **Lazy `Value` / `SpanTree`**: typed cache invalidated rather
  than re-parsed eagerly — successive edits in a batch don't
  pay the parser cost; the deferred parse runs once on the
  first read (~6× single edit).
- **Green-tree path resolution**: walks the structural CST
  directly, skipping the typed cache for the common
  set-then-set pattern (~7.6× batch).
- **Relative-len leaves**: O(log N) splice — the green node only
  stores child lengths, not absolute byte ranges (~37× over
  baseline).

#### Added — `Entry` API

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

#### Added — automatic indent detection

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

#### Added — anchor management

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

### Added — Schema contracts

#### Added — JSON Schema codegen

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

#### Added — Schema validation and enhanced CLI

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

### Added — SIMD primitives and hot-path integration

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
| Incremental repair | baseline |
| Lazy `Value`/`SpanTree` | ~6× single edit |
| Green-tree path resolution | ~7.6× batch |
| Relative-len leaves | ~37× over baseline |

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
  Schema) with rich `miette` diagnostics; supports `--schema
  PATH` (enforces a JSON Schema 2020-12 contract) and `--fix`
  (in-place lossless reformat through the CST).
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
