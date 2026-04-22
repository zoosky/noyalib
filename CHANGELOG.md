# Changelog

All notable changes to this project are documented here.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.0.1] - 2026-04-20

### Added

- Native YAML 1.2 scanner and parser, written entirely in safe Rust.
- **100% YAML Test Suite compliance**: 392/392 official test cases pass (14 skipped for tag directives and edge cases tracked for v0.0.2).
- Full serde `Serialize` and `Deserialize` support.
- **Streaming deserializer**: Bypasses Value AST for typed deserialization (50% faster than Value-based path).
- **Zero-copy scanner**: `Cow<'a, str>` scalars borrow from input without heap allocation.
- **Zero-copy AST**: `BorrowedValue<'a>` borrows strings from input — 18% faster than owned `Value`.
- **Path queries**: `value.query("items[*].name")` with wildcards (`*`) and recursive descent (`..`).
- **SIMD-accelerated scanning**: `memchr` for delimiter search on large inputs.
- **Span-free fast path**: `NoSpanLoader` for deserialization without span tracking overhead.
- **Unicode escape sequences**: Full `\xNN`, `\uNNNN`, `\UNNNNNNNN` support in double-quoted scalars.
- **Adjacent value detection**: `:` after JSON-like keys in flow context without trailing whitespace.
- **Empty document handling**: Empty, whitespace-only, and comment-only documents resolve to `Value::Null`.
- **Compact block notation**: Block sequences at same indent as mapping keys (`key:\n- item`).
- `Value`, `Mapping`, `MappingAny`, `Sequence`, `Number`, `Tag`, `TaggedValue` types.
- `from_str`, `from_slice`, `from_reader`, `from_value` deserialization functions.
- `to_string`, `to_writer`, `to_fmt_writer`, `to_value` serialization functions.
- All functions available with `_with_config` variants for custom security/formatting limits.
- `SerializerConfig` with indent, flow style, scalar style, block scalars, document markers, `quote_all`, `compact_list_indent`, `folded_wrap_chars`, `min_fold_chars`.
- `ParserConfig` with depth limits, document length limits, alias expansion caps, duplicate key policy, `strict_booleans`, and `legacy_booleans`.
- **YAML 1.1 compatibility**: `legacy_booleans` mode resolves `yes`/`no`/`on`/`off`/`y`/`n` as booleans (solves the "Norway problem").
- `Spanned<T>` for tracking source line, column, and byte offset of deserialized values.
- `apply_merge()` for YAML merge key (`<<`) expansion.
- Multi-document support: `load_all`, `load_all_as`, `to_string_multi`, `to_writer_multi`.
- `Path` type for structured error location tracking.
- `fmt` module: `FlowSeq`, `FlowMap`, `LitStr`, `FoldStr`, `Commented`, `SpaceAfter`.
- `with` module: `singleton_map`, `singleton_map_optional`, `singleton_map_recursive`, `singleton_map_with`.
- Anchor and alias support with `RcAnchor`, `ArcAnchor`, and weak variants.
- Schema validation: `validate_core_schema`, `validate_json_schema`, `validate_failsafe_schema`.
- Error types with source location, annotated context formatting (`format_with_source`), and `#[track_caller]` on all Index panics.
- **Optional `miette::Diagnostic` integration** (`--features miette`): Rich terminal diagnostics with error codes, help text, and source spans.
- **Full `#![no_std]` support**: Works with `alloc` only (`default-features = false`). Core parsing/serialization available without `std`. I/O functions gated behind `std` feature.
- **WASM support**: Compiles to `wasm32-unknown-unknown` (201 KB). wasm-bindgen bindings with browser demo.
- `#[non_exhaustive]` on `ParserConfig`, `SerializerConfig`, `FlowStyle`, `ScalarStyle`.
- `#[must_use]` on 83 query methods.
- 2,206 tests including YAML spec compliance, property-based tests, and edge cases.
- 45 branded examples with animated spinner UI.
- 5 fuzz targets with seed corpus.
- Cross-platform CI (Linux, macOS, Windows) with Miri, cargo-deny, CodeQL, commit signature verification.
- SPDX license headers on all source files.
- Release workflow with checksums (SHA256/SHA512), SBOM, and crates.io publish gate.

### Performance

Benchmarked on Apple M4, Rust 1.94 stable:

| Benchmark | noyalib | serde\_yaml\_ng | Improvement |
|---|---|---|---|
| Serialize (simple) | 358 ns | 1.41 us | **75% faster** |
| Serialize (nested) | 2.80 us | 8.32 us | **66% faster** |
| Deserialize (simple) | 1.39 us | 2.79 us | **50% faster** |
| Deserialize (nested) | 9.16 us | 17.3 us | **47% faster** |
| Deserialize (large) | 0.83 ms | 1.49 ms | **44% faster** |

[Unreleased]: https://github.com/sebastienrousseau/noyalib/compare/v0.0.1...HEAD
[0.0.1]: https://github.com/sebastienrousseau/noyalib/releases/tag/v0.0.1
