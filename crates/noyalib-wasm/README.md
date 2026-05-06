<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->

# noyalib-wasm

`wasm-bindgen` wrapper around
[`noyalib`](https://crates.io/crates/noyalib) — pure-Rust YAML
1.2, zero `unsafe`, ~338 KB after LTO. Runs in browsers, Node,
Cloudflare Workers, Deno, and any other WASM-capable host.

Drop-in for the workloads
[`js-yaml`](https://github.com/nodeca/js-yaml) usually covers
(YAML editors, in-browser config validation, K8s-manifest tools)
with two material differences:

1. **Comments + structure are preserved** when you go through
   the lossless-CST API (`Document.parse` →
   `Document.set("server.port", "9090")` →
   `Document.toString()`). `js-yaml` discards comments by spec.
2. **Pure-Rust YAML 1.2 semantics**, not the YAML 1.1 quirks
   `js-yaml` inherits. The "Norway problem" (`country: NO` →
   `country: false`) doesn't happen here.

## Install

```sh
npm install @noyalib/noyalib-wasm
```

## Usage

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
const doc = Document.parse(yamlSource);
doc.set("server.port", "9090");
fs.writeFileSync("config.yaml", doc.toString());
```

## Bundle size

| Build | Size |
|---|---|
| Default (`wasm-pack build --release --target bundler`) | ~338 KB |
| `--features wasm-opt` (post-build pass) | ~280 KB |

Tree-shaking-friendly — the `Document` API and the plain
`parse` / `stringify` API are independent modules; bundlers drop
whichever your code doesn't import.

## Targets

`wasm-pack build` supports every target wasm-bindgen does:

```sh
wasm-pack build --target bundler   # webpack, rollup, esbuild
wasm-pack build --target web       # native ES module
wasm-pack build --target nodejs    # commonjs Node import
wasm-pack build --target deno      # Deno-native module
wasm-pack build --target no-modules # Plain global, no module loader
```

## Provenance

Every release on npm carries an
[npm provenance attestation](https://docs.npmjs.com/generating-provenance-statements)
linking the published bundle to the GitHub Actions run that
produced it. Verify via:

```sh
npm view @noyalib/noyalib-wasm provenance
```

## License

Dual-licensed under Apache 2.0 or MIT, at your option.
