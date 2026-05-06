# noyalib-lsp

Language Server Protocol implementation for
[noyalib](https://github.com/sebastienrousseau/noyalib) — YAML
formatting, validation, and hover information delivered to any
LSP-aware editor (VS Code, Zed, Neovim, Helix, JetBrains, …).

Powered by noyalib's pure-Rust, zero-`unsafe`, byte-faithful CST.
Edits preserve every comment, indentation, and sibling entry; parse
errors carry exact line / column locations.

## Why noyalib-lsp

- **Byte-faithful formatting.** `textDocument/formatting` re-emits
  the document via noyalib's CST. An already-canonical document
  produces zero edits — your editor doesn't churn whitespace on
  save.
- **Real diagnostics.** Parse errors flow through
  `textDocument/publishDiagnostics` with line / column locations
  the editor's gutter can highlight directly.
- **Schema-aware hover.** When a JSON Schema is attached, hover
  surfaces the resolved field type and (in a follow-up) the schema
  description.
- **Stdio transport.** Standard `Content-Length`-framed JSON-RPC
  2.0 — works with every LSP-compliant client.

## Capabilities

- `initialize` / `initialized` / `shutdown` / `exit` — full LSP
  lifecycle handshake.
- `textDocument/didOpen` / `didChange` / `didClose` — full-text
  document sync (`TextDocumentSyncKind = 1`).
- `textDocument/publishDiagnostics` — parse-error diagnostics
  emitted on every open and change.
- `textDocument/formatting` — full-document `TextEdit[]` from the
  CST formatter; empty edit array when the document is already
  canonical.
- `textDocument/hover` — markdown card with cursor position +
  document type. Schema-driven descriptions tracked for follow-up.

## Install

From crates.io:

```sh
cargo install noyalib-lsp
```

Or from source:

```sh
git clone https://github.com/sebastienrousseau/noyalib
cd noyalib
cargo install --path noyalib-lsp
```

The binary is named `noyalib-lsp` and reads / writes LSP messages
on stdio.

## Editor integration

### VS Code

`noyalib-lsp` does not yet ship a packaged VS Code extension, so
the simplest hookup is via the generic
[`vscode-languageclient`](https://code.visualstudio.com/api/language-extensions/language-server-extension-guide)
template — point a custom client extension at the `noyalib-lsp`
binary on stdio for `yaml` filetypes.

For users who only want noyalib's formatter, configure VS Code's
`editor.formatOnSave` against an external command:

```jsonc
{
  "[yaml]": {
    "editor.formatOnSave": true,
    "editor.defaultFormatter": "noyalib.lsp"
  }
}
```

### Zed

`~/.config/zed/settings.json`:

```json
{
  "lsp": {
    "noyalib-lsp": {
      "binary": {
        "path": "noyalib-lsp"
      }
    }
  },
  "languages": {
    "YAML": {
      "language_servers": ["noyalib-lsp"],
      "format_on_save": "on"
    }
  }
}
```

### Neovim

With Neovim 0.11+ (`vim.lsp.config`):

```lua
vim.lsp.config('noyalib_lsp', {
  cmd = { 'noyalib-lsp' },
  filetypes = { 'yaml' },
  root_markers = { '.git' },
})
vim.lsp.enable('noyalib_lsp')
```

With `nvim-lspconfig` on older versions:

```lua
require('lspconfig.configs').noyalib_lsp = {
  default_config = {
    cmd = { 'noyalib-lsp' },
    filetypes = { 'yaml' },
    root_dir = require('lspconfig.util').find_git_ancestor,
  },
}
require('lspconfig').noyalib_lsp.setup({})
```

### Helix

`~/.config/helix/languages.toml`:

```toml
[language-server.noyalib-lsp]
command = "noyalib-lsp"

[[language]]
name = "yaml"
language-servers = ["noyalib-lsp"]
formatter = { command = "noyalib-lsp" }
auto-format = true
```

### Emacs (`lsp-mode`)

```elisp
(with-eval-after-load 'lsp-mode
  (lsp-register-client
   (make-lsp-client
    :new-connection (lsp-stdio-connection "noyalib-lsp")
    :major-modes '(yaml-mode)
    :server-id 'noyalib-lsp)))
```

## Wire format

JSON-RPC 2.0 over stdio per the LSP 3.17 spec, with
`Content-Length` headers. The server logs are silent on stdout
(reserved for the protocol stream); diagnostic logging goes to
stderr.

## Architecture

`noyalib-lsp` is a library + thin binary:

- `noyalib_lsp::Server` — pure-Rust dispatch surface. The
  document store, JSON-RPC envelope handling, and every LSP
  method handler (`initialize`, `did_open`, `formatting`,
  `hover`, …) live here. Reachable from `cargo test` so unit
  tests don't need a real LSP client.
- `noyalib-lsp` (binary) — stdio loop that frames JSON-RPC
  messages (`Content-Length: N\r\n\r\n<body>`) and drives
  `Server::handle_message` per message.

## License

Dual-licensed under [MIT](../LICENSE-MIT) or
[Apache-2.0](../LICENSE-APACHE) at your option.
