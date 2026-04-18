# Changelog

All notable changes to this project are documented here.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.0.1] - 2026-03-20

### Added

- Native YAML 1.2 scanner and parser, written entirely in safe Rust.
- Full serde `Serialize` and `Deserialize` support.
- `Value`, `Mapping`, `MappingAny`, `Sequence`, `Number`, `Tag`, `TaggedValue` types.
- `from_str`, `from_slice`, `from_reader`, `from_value` deserialization functions.
- `to_string`, `to_writer`, `to_value` serialization functions.
- `SerializerConfig` with indent, flow style, scalar style, block scalars, and document markers.
- `ParserConfig` with depth limits, document length limits, alias expansion caps, and duplicate key policy.
- `Spanned<T>` for tracking source line, column, and byte offset of deserialized values.
- `apply_merge()` for YAML merge key (`<<`) expansion.
- Multi-document support: `load_all`, `load_all_as`, `to_string_multi`, `to_writer_multi`.
- `Path` type for structured error location tracking.
- `fmt` module: `FlowSeq`, `FlowMap`, `LitStr`, `FoldStr`, `Commented`, `SpaceAfter`.
- `with` module: `singleton_map`, `singleton_map_optional`, `singleton_map_recursive`, `singleton_map_with`.
- Anchor and alias support with `RcAnchor`, `ArcAnchor`, and weak variants.
- Schema validation: `validate_core_schema`, `validate_json_schema`, `validate_failsafe_schema`.
- Error types with source location and annotated context formatting.
- 1,300+ tests including YAML spec compliance, property-based tests, and edge cases.
- Cross-platform CI (Linux, macOS, Windows) with Miri, clippy, supply-chain audit.

[Unreleased]: https://github.com/sebastienrousseau/noyalib/compare/v0.0.1...HEAD
[0.0.1]: https://github.com/sebastienrousseau/noyalib/releases/tag/v0.0.1
