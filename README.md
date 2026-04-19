# noyalib

A YAML 1.2 library for Rust. Pure safe code. Full serde integration.

[![CI](https://github.com/sebastienrousseau/noyalib/actions/workflows/ci.yml/badge.svg)](https://github.com/sebastienrousseau/noyalib/actions/workflows/ci.yml)
[![docs.rs](https://docs.rs/noyalib/badge.svg)](https://docs.rs/noyalib)
[![crates.io](https://img.shields.io/crates/v/noyalib.svg)](https://crates.io/crates/noyalib)
[![License](https://img.shields.io/crates/l/noyalib.svg)](LICENSE-MIT)

## Why noyalib

- **Pure Rust** -- native YAML 1.2 scanner and parser. No C bindings. No FFI.
- **Zero `unsafe`** -- `#![forbid(unsafe_code)]` enforced at compile time.
- **Fast** -- 74% faster serialization than serde\_yaml\_ng. Sub-microsecond on simple docs.
- **Serde-native** -- serialize and deserialize any `Serialize` / `Deserialize` type.
- **Ordered mappings** -- `IndexMap`-backed. Insertion order preserved.
- **Source spans** -- `Spanned<T>` tracks exact line, column, and byte offset.
- **Hardened parser** -- configurable depth, size, and alias limits. Billion-laughs safe.
- **Five dependencies** -- `serde`, `indexmap`, `thiserror`, `itoa`, `ryu`. That's it.

## Quick Start

```sh
cargo add noyalib
```

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

    // Deserialize
    let config: Config = from_str(yaml)?;
    assert_eq!(config.name, "myapp");
    assert_eq!(config.port, 8080);

    // Serialize
    let output = to_string(&config)?;
    assert!(output.contains("name: myapp"));

    // Roundtrip
    let roundtrip: Config = from_str(&output)?;
    assert_eq!(config, roundtrip);

    Ok(())
}
```

## Performance

Benchmarked on Apple M4, Rust 1.94 stable (lower is better):

| Operation | noyalib | serde\_yaml\_ng | vs |
|:---|---:|---:|---:|
| **Deserialize (simple)** | 2.25 us | 2.84 us | 21% faster |
| **Deserialize (nested)** | 14.6 us | 17.2 us | 15% faster |
| **Deserialize (large)** | 1.32 ms | 1.48 ms | 11% faster |
| **Serialize (simple)** | 366 ns | 1.43 us | 74% faster |
| **Serialize (nested)** | 2.93 us | 8.50 us | 66% faster |

Reproduce: `cargo bench --bench comparison`.

## Deserialization

```rust
use noyalib::{from_str, from_slice, from_reader, from_value, ParserConfig};

// From string
let config: Config = from_str(yaml)?;

// From byte slice
let config: Config = from_slice(bytes)?;

// From reader (file, network, etc.)
let config: Config = from_reader(file)?;

// From a Value
let config: Config = from_value(&value)?;

// With security limits
let parser = ParserConfig::strict();
let config: Config = noyalib::from_str_with_config(yaml, &parser)?;
let config: Config = noyalib::from_slice_with_config(bytes, &parser)?;
let config: Config = noyalib::from_reader_with_config(reader, &parser)?;
```

## Serialization

```rust
use noyalib::{to_string, to_writer, to_fmt_writer, to_value, SerializerConfig};

// To string
let yaml: String = to_string(&config)?;

// To io::Write (file, Vec<u8>, etc.)
to_writer(&mut file, &config)?;

// To fmt::Write (String buffer, etc.)
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

## Dynamic Values

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

## Source Spans

Track exact source locations for every deserialized field:

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

## Parser Configuration

Set safety limits for untrusted input:

```rust
use noyalib::{from_str_with_config, ParserConfig, DuplicateKeyPolicy};

let config = ParserConfig::new()
    .max_depth(64)
    .max_document_length(1_000_000)
    .max_alias_expansions(1000)
    .max_mapping_keys(10_000)
    .max_sequence_length(10_000)
    .duplicate_key_policy(DuplicateKeyPolicy::Error)
    .strict_booleans(true);  // Only "true"/"false", not "True"/"FALSE"

let value: noyalib::Value = from_str_with_config(input, &config)?;
```

For maximum strictness, use `ParserConfig::strict()`.

## Serializer Configuration

```rust
use noyalib::{to_string_with_config, SerializerConfig, FlowStyle, ScalarStyle};

let config = SerializerConfig::new()
    .indent(4)                   // 4 spaces per level
    .flow_style(FlowStyle::Auto) // Inline small collections
    .scalar_style(ScalarStyle::DoubleQuoted)
    .quote_all(true)             // Force-quote all strings
    .document_start(true)        // Emit ---
    .document_end(true)          // Emit ...
    .block_scalars(true)         // Use | for multiline
    .block_scalar_threshold(3);  // Trigger at 3+ newlines

let yaml = to_string_with_config(&value, &config)?;
```

## Merge Keys

Expand YAML `<<` merge keys:

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

## Multi-Document Streams

```rust
use noyalib::{load_all, to_string_multi};

// Parse
let docs = load_all("---\na: 1\n---\nb: 2\n")?;
for doc in &docs {
    println!("{doc:?}");
}

// Typed
let items: Vec<Config> = noyalib::load_all_as::<Config>(yaml)?;

// Serialize
let yaml = to_string_multi(&[config1, config2])?;
```

## Enum Serialization

| Module | Purpose |
|:---|:---|
| `singleton_map` | Serialize enums as `{Variant: data}` |
| `singleton_map_optional` | Same, for `Option<Enum>` |
| `singleton_map_recursive` | Apply recursively to nested enums |
| `singleton_map_with` | Custom key transforms (snake\_case, kebab-case) |

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

## Formatting Wrappers

Per-value output control via the `fmt` module:

| Wrapper | Effect |
|:---|:---|
| `FlowSeq<T>` | Inline sequence: `[a, b, c]` |
| `FlowMap<T>` | Inline mapping: `{a: 1, b: 2}` |
| `LitStr` / `LitString` | Literal block scalar (`\|`) |
| `FoldStr` / `FoldString` | Folded block scalar (`>`) |
| `Commented<T>` | Attach a YAML comment |
| `SpaceAfter<T>` | Insert a blank line after the value |

## Schema Validation

Validate values against YAML schema levels:

```rust
use noyalib::{from_str, validate_json_schema, validate_core_schema, Value};

let value: Value = from_str("port: 8080")?;
validate_json_schema(&value)?;   // No NaN, no tags
validate_core_schema(&value)?;   // Permissive
```

## Error Handling

Errors include source locations and render annotated context:

```rust
match noyalib::from_str::<noyalib::Value>(yaml) {
    Ok(value) => { /* ... */ }
    Err(e) => {
        eprintln!("Error: {e}");
        if let Some(loc) = e.location() {
            eprintln!("  at line {}, column {}", loc.line(), loc.column());
        }
        eprintln!("{}", e.format_with_source(yaml));
    }
}
```

## Fuzzing

Five `cargo-fuzz` targets exercise the parser under adversarial input:

```sh
cargo +nightly fuzz run fuzz_parse       # Arbitrary YAML parsing
cargo +nightly fuzz run fuzz_roundtrip   # Parse -> serialize -> re-parse
cargo +nightly fuzz run fuzz_from_value  # Value -> typed deserialization
cargo +nightly fuzz run fuzz_multi_doc   # Multi-document streams
cargo +nightly fuzz run fuzz_strict      # Tight security limits
```

## Examples

Run all 24 examples:

```sh
cargo run --example run_all
```

Or individually:

```sh
cargo run --example basic               # Struct roundtrip
cargo run --example collections         # Vec, HashMap
cargo run --example enums               # Enum strategies
cargo run --example nested              # Complex structures
cargo run --example value               # Dynamic Value type
cargo run --example io_formats          # from_slice, from_reader, to_writer, to_fmt_writer
cargo run --example value_manipulation  # to_value, from_value, get_path, MappingAny
cargo run --example config              # SerializerConfig options
cargo run --example serializer_config   # quote_all, flow styles, multi-doc
cargo run --example strict_parsing      # strict_booleans, DuplicateKeyPolicy
cargo run --example parser_config       # Security limits
cargo run --example error_handling      # Errors and source context
cargo run --example error_paths         # Path tracking
cargo run --example schema_validation   # YAML schema validation
cargo run --example anchors             # Anchors, aliases, merge keys
cargo run --example shared_anchors      # RcAnchor, ArcAnchor
cargo run --example merge               # Value merging
cargo run --example merge_keys          # apply_merge() for << keys
cargo run --example multi_document      # Multi-document parsing
cargo run --example spanned             # Source locations
cargo run --example fmt_wrappers        # Formatting wrappers
cargo run --example singleton_map       # Enum singleton maps
cargo run --example custom_serialization # Key transformations
cargo run --example bench-comparison    # Performance overview
```

## Development

```sh
make              # check + clippy + test
make test         # all tests
make clippy       # lint
make fmt          # check formatting
make examples     # run all examples
make doc          # build documentation
make deny         # supply-chain audit
make miri         # Miri (requires nightly)
```

## Safety

noyalib is written entirely in safe Rust:

```rust
#![forbid(unsafe_code)]
```

No C dependencies. No FFI. No `unsafe` blocks.

Runtime dependencies: [`serde`](https://crates.io/crates/serde), [`indexmap`](https://crates.io/crates/indexmap), [`thiserror`](https://crates.io/crates/thiserror), [`itoa`](https://crates.io/crates/itoa), [`ryu`](https://crates.io/crates/ryu).

## Minimum Supported Rust Version

Rust **1.75.0** or later. Tested on Linux, macOS, and Windows.

## License

Dual-licensed under [MIT](LICENSE-MIT) and [Apache 2.0](LICENSE-APACHE).
