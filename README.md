# noyalib

A YAML 1.2 library for Rust. Pure safe code. Full serde integration.

[![CI](https://github.com/sebastienrousseau/noyalib/actions/workflows/ci.yml/badge.svg)](https://github.com/sebastienrousseau/noyalib/actions/workflows/ci.yml)
[![docs.rs](https://docs.rs/noyalib/badge.svg)](https://docs.rs/noyalib)
[![crates.io](https://img.shields.io/crates/v/noyalib.svg)](https://crates.io/crates/noyalib)
[![License](https://img.shields.io/crates/l/noyalib.svg)](LICENSE-MIT)

## Why noyalib

- **Pure Rust** — native YAML 1.2 scanner and parser. No C bindings. No FFI.
- **Zero `unsafe`** — `#![forbid(unsafe_code)]` enforced at compile time.
- **Fast** — sub-microsecond serialization. Optimized for throughput.
- **Serde-native** — serialize and deserialize any `Serialize` / `Deserialize` type.
- **Ordered mappings** — `IndexMap`-backed. Insertion order preserved.
- **Source spans** — `Spanned<T>` tracks exact line, column, and byte offset.
- **Hardened parser** — configurable depth, size, and alias limits. Billion-laughs safe.
- **Three dependencies** — `serde`, `indexmap`, `thiserror`. That's it.

## Quick Start

```sh
cargo add noyalib
```

```rust
use noyalib::{from_str, to_string};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
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

    let config: Config = from_str(yaml)?;
    let output = to_string(&config)?;
    println!("{output}");
    Ok(())
}
```

## Performance

Benchmarked on Apple M4, Rust stable (lower is better):

| Operation | Simple | Nested | Large (500 items) |
|:---|---:|---:|---:|
| **Deserialize** | 2.15 µs | 13.9 µs | 1.28 ms |
| **Typed deserialize** | 1.90 µs | 12.1 µs | — |
| **Serialize** | 0.35 µs | 2.69 µs | — |
| **Roundtrip** | — | 17.0 µs | — |

Reproduce: `cargo bench --bench comparison`.

## API

### Deserialize

```rust
let config: Config = noyalib::from_str(yaml)?;
let config: Config = noyalib::from_slice(bytes)?;
let config: Config = noyalib::from_reader(file)?;
let config: Config = noyalib::from_value(&value)?;
let config: Config = noyalib::from_str_with_config(yaml, &parser_config)?;
```

### Serialize

```rust
let yaml: String = noyalib::to_string(&config)?;
noyalib::to_writer(&mut file, &config)?;
let value: noyalib::Value = noyalib::to_value(&config)?;
let yaml = noyalib::to_string_with_config(&config, &serializer_config)?;
```

### Dynamic Values

```rust
use noyalib::Value;

let value: Value = noyalib::from_str("name: test\nitems:\n  - one\n  - two\n")?;

let name = value.get("name").and_then(|v| v.as_str());
let first = value.get("items").and_then(|v| v.get(0));
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
assert_eq!(*config.port, 8080);
assert_eq!(config.port.start.line(), 1);
assert_eq!(config.port.start.column(), 7);
```

`Spanned<T>` serializes transparently as `T`.

## Serializer Configuration

```rust
use noyalib::{to_string_with_config, SerializerConfig, FlowStyle, ScalarStyle};

let config = SerializerConfig::new()
    .indent(4)
    .flow_style(FlowStyle::Flow)
    .scalar_style(ScalarStyle::DoubleQuoted)
    .document_start(true)
    .document_end(true)
    .block_scalars(true)
    .block_scalar_threshold(3);

let yaml = to_string_with_config(&value, &config)?;
```

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
    .duplicate_key_policy(DuplicateKeyPolicy::Error);

let value: noyalib::Value = from_str_with_config(input, &config)?;
```

For maximum strictness, use `ParserConfig::strict()`.

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
// production → {timeout: 60, retries: 3}
```

## Multi-Document Streams

```rust
use noyalib::{load_all, to_string_multi};

let docs = load_all("---\na: 1\n---\nb: 2\n")?;
for doc in &docs {
    println!("{doc:?}");
}

let items: Vec<Config> = noyalib::load_all_as::<Config>(yaml)?;
let yaml = to_string_multi(&[config1, config2])?;
```

## Enum Serialization

| Module | Purpose |
|:---|:---|
| `singleton_map` | Serialize enums as `{Variant: data}` |
| `singleton_map_optional` | Same, for `Option<Enum>` |
| `singleton_map_recursive` | Apply recursively to nested enums |
| `singleton_map_with` | Custom key transforms (snake_case, kebab-case) |

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
cargo +nightly fuzz run fuzz_roundtrip   # Parse → serialize → re-parse
cargo +nightly fuzz run fuzz_from_value  # Value → typed deserialization
cargo +nightly fuzz run fuzz_multi_doc   # Multi-document streams
cargo +nightly fuzz run fuzz_strict      # Tight security limits
```

Seed corpus included in `fuzz/corpus/seed/`.

## Examples

```sh
cargo run --example basic               # Struct roundtrip
cargo run --example collections          # Vec, HashMap
cargo run --example enums                # Enum strategies
cargo run --example nested               # Complex structures
cargo run --example value                # Dynamic Value type
cargo run --example multi_document       # Multi-document parsing
cargo run --example config               # SerializerConfig
cargo run --example merge                # Value merging
cargo run --example merge_keys           # apply_merge() for << keys
cargo run --example anchors              # Anchors and aliases
cargo run --example error_handling       # Errors and source context
cargo run --example error_paths          # Path tracking
cargo run --example singleton_map        # Enum singleton maps
cargo run --example custom_serialization # Key transformations
cargo run --example spanned              # Source locations
cargo run --example schema_validation    # YAML schema validation
cargo run --example parser_config        # Security limits
cargo run --example fmt_wrappers         # Formatting wrappers
```

## Development

```sh
make              # check + clippy + test
make test         # all tests
make clippy       # lint
make fmt          # check formatting
make doc          # build documentation
make deny         # supply-chain audit
make miri         # Miri (requires nightly)
```

## Safety

noyalib is written entirely in safe Rust:

```rust
#![forbid(unsafe_code)]
```

No C dependencies. No FFI. No `unsafe` blocks. Runtime dependencies: [`serde`](https://crates.io/crates/serde), [`indexmap`](https://crates.io/crates/indexmap), [`thiserror`](https://crates.io/crates/thiserror).

## Minimum Supported Rust Version

Rust **1.75.0** or later. Tested on Linux, macOS, and Windows.

## License

Dual-licensed under [MIT](LICENSE-MIT) and [Apache 2.0](LICENSE-APACHE).
