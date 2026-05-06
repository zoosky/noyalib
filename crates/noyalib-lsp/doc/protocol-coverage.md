# LSP protocol coverage

Which LSP methods `noyalib-lsp` implements, which it acknowledges
but treats as no-ops, and which it deliberately does not support.
Coverage is conservative: every implemented method has end-to-end
tests in `crates/noyalib-lsp/tests/` and behavioural tests
exercising the JSON-RPC envelope.

LSP version targeted: **3.17**.

## Implemented

| Method | Direction | Behaviour |
|---|---|---|
| `initialize` | client → server | Returns server capabilities (formatting, hover, didOpen / didChange / didClose, publishDiagnostics) |
| `initialized` | client → server | Acknowledged; server-side initialisation runs here |
| `shutdown` | client → server | Acknowledged; server prepares for exit |
| `exit` | notification | Server exits with code 0 (or 1 if shutdown was not requested) |
| `textDocument/didOpen` | notification | Document tracked in the open-documents map; `publishDiagnostics` runs against the new content |
| `textDocument/didChange` | notification | Document content updated; `publishDiagnostics` runs against the new content |
| `textDocument/didClose` | notification | Document removed from the open-documents map |
| `textDocument/formatting` | request | Returns a `TextEdit[]` that rewrites the document via `noyalib::cst::format`. Lossless: comments, anchors, indent style preserved |
| `textDocument/hover` | request | Returns the parsed value type at the cursor; if a JSON Schema is associated with the document, returns the schema's `description` field for the value at the cursor path |
| `textDocument/publishDiagnostics` | server → client | Emitted on every `didOpen` / `didChange`. Carries parse errors with span attached; carries schema-validation errors when a schema is associated |

## Acknowledged but no-op

These method names are recognised so the client doesn't see "method
not found" errors, but the server returns an empty result. They're
candidates for future implementation when there's a proven user
request.

| Method | Why no-op |
|---|---|
| `textDocument/didSave` | The CST formatter doesn't need a save signal — `didChange` already flushes |
| `workspace/didChangeConfiguration` | No tunable configuration exposed yet; future `noyalib.fmt.indent` etc. will land here |
| `workspace/didChangeWatchedFiles` | Diagnostics run on the open buffer; on-disk changes are picked up on the next `didOpen` |

## Deliberately unsupported

| Method | Reason |
|---|---|
| `textDocument/completion` | YAML's grammar makes meaningful completions context-heavy. The LSP intentionally defers to the YAML schema-aware completion in editor extensions like `yaml-language-server` until we have a clear value proposition |
| `textDocument/codeAction` | No quick-fix logic implemented yet. Schema validation errors carry recovery hints in their messages but no `WorkspaceEdit` |
| `textDocument/rename` | Anchor / alias renaming is a real ask but requires careful handling of cross-document references; planned post-v0.0.1 |
| `textDocument/references` | Same as rename — anchor reference graph |
| `textDocument/definition` | Same family |
| `textDocument/documentSymbol` | Outline view; planned but not yet implemented |
| `textDocument/foldingRange` | Editors typically derive folding from indentation directly |
| `workspace/symbol` | Cross-document symbols; out of scope for a YAML server |

Methods not listed above return `methodNotFound` per the LSP spec.

## Diagnostic coverage

`textDocument/publishDiagnostics` payloads are produced by:

1. **Parse errors** — every `noyalib::Error::ParseWithLocation`
   becomes a `Diagnostic` with severity `Error`, span attached.
   See [crate noyalib's errors.md](../../noyalib/doc/errors.md)
   for the full variant list.
2. **Schema validation errors** — when a JSON Schema is
   associated with the document (via the editor's schema-mapping
   config or an inline `# yaml-language-server: $schema=...`
   pragma), each violation becomes a `Diagnostic` with severity
   `Warning`. The diagnostic's `code` field carries the schema
   keyword that failed (`type`, `enum`, `required`, etc.).

Diagnostics are produced eagerly on every `didOpen` / `didChange`.
There is no debounce — the parser is fast enough that 10ms
keystroke cadence on a 50KB document is comfortably under one
event loop tick.

## Capabilities advertised at `initialize`

```json
{
  "capabilities": {
    "textDocumentSync": {
      "openClose": true,
      "change": 1
    },
    "documentFormattingProvider": true,
    "hoverProvider": true,
    "diagnosticProvider": {
      "interFileDependencies": false,
      "workspaceDiagnostics": false
    }
  },
  "serverInfo": {
    "name": "noyalib-lsp",
    "version": "0.0.1"
  }
}
```

`textDocumentSync.change` is `1` (full-document sync). Incremental
sync (`2`) would shave a small amount off the wire bandwidth on
very large files but adds protocol complexity that hasn't paid off
in benchmarks. Revisit if profiling shows it matters.

## Where the JSON-RPC envelope is handled

```text
crates/noyalib-lsp/src/
├── lib.rs                # Server type, handle_message dispatch
├── main.rs               # stdio loop, main()
├── format.rs             # textDocument/formatting handler
├── hover.rs              # textDocument/hover handler
└── diagnostics.rs        # publishDiagnostics builder
```

The dispatch in `lib.rs::handle_message` is a simple match on the
incoming method name; each handler returns a `HandleOutcome`
(`reply` / `notify` / `silent`) that tells the transport layer
what to send back.

## Testing

| Suite | What it covers |
|---|---|
| `crates/noyalib-lsp/tests/protocol.rs` | End-to-end JSON-RPC round-trips (subprocess-driven; not run under llvm-cov) |
| Per-handler unit tests in each `*.rs` | Direct handler invocation without the JSON-RPC envelope |
| `crates/noyalib-lsp/benches/lsp_handlers.rs` | Per-handler latency on small + Kubernetes-shaped inputs |

The full pyramid is documented in
[`doc/TESTING.md`](../../../doc/TESTING.md).

## Related

- [Editor setup](./editor-setup.md) — wiring per editor
- [Crate README](../README.md) — install + crate-level overview
