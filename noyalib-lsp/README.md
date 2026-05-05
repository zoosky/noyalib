# noyalib-lsp

Language Server Protocol implementation for [noyalib](https://github.com/sebastienrousseau/noyalib) — YAML formatting,
validation, and hover-driven JSON Schema descriptions delivered to any
LSP-aware editor (VS Code, Zed, Neovim, Helix, JetBrains, …).

## Capabilities

- `textDocument/didOpen` / `didChange` / `didClose` — incremental
  document tracking.
- `textDocument/publishDiagnostics` — YAML parse errors and JSON
  Schema violations published as the document changes.
- `textDocument/formatting` — re-emits the document via noyalib's
  byte-faithful CST formatter.
- `textDocument/hover` — returns the JSON Schema description at the
  cursor position when a schema is attached.

## Install + connect

```sh
cargo install noyalib-lsp
```

VS Code (`settings.json`):

```json
{
  "yaml.customTags": [],
  "[yaml]": {
    "editor.defaultFormatter": "noyalib.lsp"
  }
}
```

Zed (`~/.config/zed/settings.json`):

```json
{
  "languages": {
    "YAML": {
      "language_servers": ["noyalib-lsp"]
    }
  }
}
```

Neovim (`init.lua`):

```lua
vim.lsp.config('noyalib_lsp', {
  cmd = { 'noyalib-lsp' },
  filetypes = { 'yaml' },
  root_markers = { '.git' },
})
```

## Wire format

JSON-RPC 2.0 over stdio per the LSP spec, with `Content-Length` headers.

## License

MIT OR Apache-2.0.
