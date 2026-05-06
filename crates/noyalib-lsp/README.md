<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->

# noyalib-lsp

Language Server Protocol implementation for
[noyalib](https://github.com/sebastienrousseau/noyalib) — YAML
formatting, validation, and hover information delivered to any
LSP-aware editor (VS Code, Zed, Neovim, Helix, JetBrains, …).

[![crates.io](https://img.shields.io/crates/v/noyalib-lsp.svg)](https://crates.io/crates/noyalib-lsp)
[![docs.rs](https://img.shields.io/docsrs/noyalib-lsp)](https://docs.rs/noyalib-lsp)
[![Build](https://img.shields.io/github/actions/workflow/status/sebastienrousseau/noyalib/ci.yml?branch=main)](https://github.com/sebastienrousseau/noyalib/actions)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](#license)

Powered by noyalib's pure-Rust, zero-`unsafe`, byte-faithful CST.
Edits preserve every comment, indentation, and sibling entry;
parse errors carry exact line / column locations.

## Contents

- [Install](#install)
- [Quick Start](#quick-start)
- [Why this approach?](#why-this-approach)
- [Surface](#surface)
- [Editor configuration](#editor-configuration)
- [Examples](#examples)
- [Documentation](#documentation)
- [License](#license)

## Install

```bash
cargo install noyalib-lsp
```

Pre-built binaries for every target the workspace ships are
attached to each GitHub Release; each is signed with cosign
keyless. See the
[install matrix in the workspace README](https://github.com/sebastienrousseau/noyalib#install)
for distro-package paths (`noyalib-lsp` is bundled into the
`noyalib` package on every channel).

## Quick Start

```bash
# Stand-alone smoke test — drives a one-shot LSP handshake
# over stdio.
noyalib-lsp --version

# As a child process spawned by your editor (the typical path).
# See the per-editor configuration below.
```

## Why this approach?

- **Byte-faithful formatting.** `textDocument/formatting`
  re-emits the document via noyalib's CST. An already-canonical
  document produces zero edits — your editor doesn't churn
  whitespace on save.
- **Real diagnostics.** Parse errors flow through
  `textDocument/publishDiagnostics` with line / column locations
  the editor's gutter can highlight directly.
- **Schema-aware hover.** When a JSON Schema is attached, hover
  surfaces the resolved field type and (in a follow-up) the
  schema description.
- **Stdio transport.** Standard `Content-Length`-framed JSON-RPC
  2.0 — works with every LSP-compliant client.

## Surface

| LSP method | What it does |
|---|---|
| `initialize` / `initialized` / `shutdown` / `exit` | Full LSP lifecycle. |
| `textDocument/didOpen` / `didChange` / `didClose` | Full-text document sync (`TextDocumentSyncKind = 1`). |
| `textDocument/publishDiagnostics` | Parse-error diagnostics on every open + change. |
| `textDocument/formatting` | Full-document `TextEdit[]` from the CST formatter. Empty array when canonical. |
| `textDocument/hover` | Markdown card with cursor position + document type. Schema-driven descriptions tracked for follow-up. |

## Editor configuration

### Visual Studio Code

The bundled experience ships through the
[noyalib VS Code extension](https://marketplace.visualstudio.com/items?itemName=sebastienrousseau.noyalib);
no manual config needed. To point at a system-installed binary
instead of the bundled one:

```json
{
  "noyalib.path": "/usr/local/bin/noyalib-lsp",
  "[yaml]": {
    "editor.defaultFormatter": "sebastienrousseau.noyalib",
    "editor.formatOnSave": true
  }
}
```

### Zed

`~/.config/zed/settings.json`:

```json
{
  "languages": {
    "YAML": {
      "language_servers": ["noyalib-lsp"],
      "format_on_save": "on"
    }
  },
  "lsp": {
    "noyalib-lsp": {
      "binary": { "path": "noyalib-lsp" }
    }
  }
}
```

### Neovim (via `nvim-lspconfig`)

```lua
require("lspconfig.configs").noyalib = {
  default_config = {
    cmd = { "noyalib-lsp" },
    filetypes = { "yaml" },
    root_dir = require("lspconfig.util").find_git_ancestor,
  },
}
require("lspconfig").noyalib.setup {
  on_attach = function(_, bufnr)
    vim.api.nvim_create_autocmd("BufWritePre", {
      buffer = bufnr,
      callback = function() vim.lsp.buf.format() end,
    })
  end,
}
```

### Helix

`~/.config/helix/languages.toml`:

```toml
[[language]]
name              = "yaml"
language-servers  = ["noyalib-lsp"]
auto-format       = true

[language-server.noyalib-lsp]
command = "noyalib-lsp"
```

## Examples

Editor-driving demos under
[`crates/noyalib-lsp/examples/`](examples/):

```bash
crates/noyalib-lsp/examples/handshake.sh        # full initialize/shutdown round-trip
crates/noyalib-lsp/examples/format-on-save.sh   # didChange → formatting
crates/noyalib-lsp/examples/hover-cursor.sh     # hover at byte offset
```

Each example pipes a sequence of JSON-RPC messages into
`noyalib-lsp` over stdio and shows the response stream.

## Documentation

- **API reference**: <https://docs.rs/noyalib-lsp>
- **Workspace README**:
  <https://github.com/sebastienrousseau/noyalib#readme>
- **LSP spec**:
  <https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/>

## License

Dual-licensed under [Apache 2.0](https://www.apache.org/licenses/LICENSE-2.0)
or [MIT](https://opensource.org/licenses/MIT), at your option.
