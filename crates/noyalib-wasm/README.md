<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->

# noyalib-wasm

`wasm-bindgen` wrapper around
[noyalib](https://github.com/sebastienrousseau/noyalib) — pure-
Rust YAML 1.2, zero `unsafe`, ~338 KB after LTO. Runs in
browsers, Node, Cloudflare Workers, Deno, and any other
WASM-capable host.

[![npm](https://img.shields.io/npm/v/@noyalib/noyalib-wasm.svg)](https://www.npmjs.com/package/@noyalib/noyalib-wasm)
[![Build](https://img.shields.io/github/actions/workflow/status/sebastienrousseau/noyalib/ci.yml?branch=main)](https://github.com/sebastienrousseau/noyalib/actions)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](#license)

Drop-in for the workloads
[`js-yaml`](https://github.com/nodeca/js-yaml) usually covers
(YAML editors, in-browser config validation, Kubernetes-manifest
tools) with two material differences:

1. **Comments + structure are preserved** when you go through
   the lossless-CST API (`Document.parse` →
   `Document.set("server.port", "9090")` →
   `Document.toString()`). `js-yaml` discards comments by spec.
2. **Pure-Rust YAML 1.2 semantics**, not the YAML 1.1 quirks
   `js-yaml` inherits. The "Norway problem" (`country: NO`
   parsed as `false`) doesn't happen here.

## Contents

- [Install](#install)
- [Quick Start](#quick-start)
- [Surface](#surface)
- [Bundle size](#bundle-size)
- [Targets](#targets)
- [Provenance](#provenance)
- [Examples](#examples)
- [Documentation](#documentation)
- [License](#license)

## Install

```sh
npm install @noyalib/noyalib-wasm
```

Or build from source:

```sh
git clone https://github.com/sebastienrousseau/noyalib
cd noyalib/crates/noyalib-wasm
wasm-pack build --release --target bundler
```

## Quick Start

```js
import init, {
  parse,
  stringify,
  validate_json,
  Document,
} from "@noyalib/noyalib-wasm";

await init();          // load the WASM blob

// Plain parse / stringify — like js-yaml.
const obj = parse("host: api.example.com\nport: 8080\n");
const yaml = stringify(obj);

// Schema validation.
const valid = validate_json(obj, schema);

// Lossless CST edit — comments + indentation preserved.
const doc = Document.parse(source);
doc.set("server.port", "9090");
fs.writeFileSync("config.yaml", doc.toString());
```

## Surface

| Export | What it does |
|---|---|
| `parse(yaml)` | Parse a YAML document into a JS value (mirrors `js-yaml`'s `load`). |
| `stringify(value)` | Serialise a JS value back to YAML. |
| `validate_json(value, schema)` | Validate a value against a JSON Schema 2020-12 contract. |
| `Document.parse(yaml)` | Open a lossless CST. |
| `Document.set(path, fragment)` | Surgically rewrite a value at a dotted path. |
| `Document.get(path)` | Read a value at a dotted path. |
| `Document.toString()` | Serialise the CST back to bytes. |
| `merge(a, b)` | Deep-merge YAML documents (delegates to `noyalib::Value::merge`). |

## Bundle size

| Build | Size |
|---|---|
| Default (`wasm-pack build --release --target bundler`) | ~338 KB |
| `--features wasm-opt` (post-build pass) | ~280 KB |

Tree-shaking-friendly — the `Document` API and the plain
`parse` / `stringify` API are independent modules; bundlers drop
whichever your code does not import.

## Targets

`wasm-pack build` supports every target wasm-bindgen does:

```sh
wasm-pack build --target bundler    # webpack, rollup, esbuild
wasm-pack build --target web        # native ES module
wasm-pack build --target nodejs     # commonjs Node import
wasm-pack build --target deno       # Deno-native module
wasm-pack build --target no-modules # plain global, no module loader
```

## Provenance

Every release on npm carries an
[npm provenance attestation](https://docs.npmjs.com/generating-provenance-statements)
linking the published bundle to the GitHub Actions run that
produced it. Verify via:

```sh
npm view @noyalib/noyalib-wasm provenance
```

## Examples

Browser + Node demos under
[`crates/noyalib-wasm/examples/`](examples/):

```text
crates/noyalib-wasm/examples/browser/index.html   # in-page YAML editor demo
crates/noyalib-wasm/examples/node-stringify.js    # parse + stringify round-trip
crates/noyalib-wasm/examples/cst-edit.js          # lossless edit preserving comments
crates/noyalib-wasm/examples/schema-validate.js   # validate against JSON Schema
```

## Documentation

- **npm package**:
  <https://www.npmjs.com/package/@noyalib/noyalib-wasm>
- **API reference (rustdoc)**: <https://docs.rs/noyalib-wasm>
- **Workspace README**:
  <https://github.com/sebastienrousseau/noyalib#readme>

## License

Dual-licensed under [Apache 2.0](https://www.apache.org/licenses/LICENSE-2.0)
or [MIT](https://opensource.org/licenses/MIT), at your option.
