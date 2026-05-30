<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->

<p align="center">
  <img src="https://cloudcdn.pro/noyalib/v1/logos/noyalib.svg" alt="Noyalib logo" width="128" />
</p>

<h1 align="center">noyalib-lsp</h1>

<p align="center">
  <strong>Language Server Protocol implementation for noyalib —
  YAML formatting, validation, and hover information delivered
  to any LSP-aware editor over stdio JSON-RPC.</strong>
</p>

<p align="center">
  <a href="https://github.com/sebastienrousseau/noyalib/actions"><img src="https://img.shields.io/github/actions/workflow/status/sebastienrousseau/noyalib/ci.yml?style=for-the-badge&logo=github" alt="Build" /></a>
  <a href="https://crates.io/crates/noyalib-lsp"><img src="https://img.shields.io/crates/v/noyalib-lsp.svg?style=for-the-badge&color=fc8d62&logo=rust" alt="Crates.io" /></a>
  <a href="https://docs.rs/noyalib-lsp"><img src="https://img.shields.io/badge/docs.rs-noyalib--lsp-66c2a5?style=for-the-badge&labelColor=555555&logo=docs.rs" alt="Docs.rs" /></a>
  <a href="https://lib.rs/crates/noyalib-lsp"><img src="https://img.shields.io/badge/lib.rs-noyalib-orange.svg?style=for-the-badge" alt="lib.rs" /></a>
  <a href="https://scorecard.dev/viewer/?uri=github.com/sebastienrousseau/noyalib"><img src="https://img.shields.io/ossf-scorecard/github.com/sebastienrousseau/noyalib?style=for-the-badge&label=OpenSSF%20Scorecard&logo=openssf" alt="OpenSSF Scorecard" /></a>
</p>

---

## Contents

- [Install](#install) — Cargo, distro packages
- [Quick Start](#quick-start) — smoke test
- [Why this approach?](#why-this-approach) — design rationale
- [Surface](#surface) — LSP methods supported
- [Editor configuration](#editor-configuration) — VS Code, Zed, Neovim, Helix, Sublime
- [Examples](#examples) — JSON-RPC scripts
- [Performance](#performance) — formatter cost
- [When not to use noyalib-lsp](#when-not-to-use-noyalib-lsp)
- [Documentation](#documentation)
- [License](#license)

---

## Install

```bash
cargo install noyalib-lsp
```

Pre-built binaries for every target the workspace ships are
attached to each GitHub Release; each is signed with cosign
keyless. See the
[install matrix in the workspace README](https://github.com/sebastienrousseau/noyalib#install)
for distro-package paths — `noyalib-lsp` is bundled into the
top-level `noyalib` package on every channel (Homebrew, AUR,
Scoop, Nix, GHCR).

**MSRV: Rust 1.85.0.** The transitive LSP transport stack
(`litemap`, `uuid`) requires recent stables; the noyalib core
library itself stays at 1.75.

---

## Quick Start

```bash
# Smoke test — drives a one-shot LSP handshake over stdio.
noyalib-lsp --version

# As a child process spawned by your editor (the typical path).
# See "Editor configuration" below for VS Code / Zed / Neovim /
# Helix / Sublime examples.
```

---

## Why this approach?

The market has two YAML language servers (`yaml-language-server`,
`taplo`-style hybrids) and both make tradeoffs noyalib-lsp avoids:

- **Byte-faithful formatting.** `textDocument/formatting` runs
  through noyalib's lossless CST. An already-canonical document
  produces an empty `TextEdit[]` — your editor doesn't churn
  whitespace on save. Comments stay where they were; indent
  width follows the file's dominant style; only quoting and
  inter-key whitespace normalise.
- **Real diagnostics.** Parse errors flow through
  `textDocument/publishDiagnostics` with line / column
  locations the editor's gutter can highlight directly. No
  best-effort parsers, no recovery hand-waving.
- **Schema-aware hover.** When a JSON Schema is attached, hover
  surfaces the resolved field type. Schema descriptions land
  in the hover card in a follow-up.
- **Stdio transport.** Standard `Content-Length`-framed
  JSON-RPC 2.0. Works with every LSP-compliant client; no
  client-specific protocol extensions.
- **Pure-Rust, zero `unsafe`.** Same `#![forbid(unsafe_code)]`
  guarantee as the noyalib core library.

The whole thing is ~5 KLOC of Rust; the heavy lifting is in the
`noyalib` library, the LSP wrapper just bridges JSON-RPC to the
library's CST + parser surface.

---

## Surface

| LSP method | What it does |
|---|---|
| `initialize` / `initialized` / `shutdown` / `exit` | Full LSP lifecycle handshake. |
| `textDocument/didOpen` / `didChange` / `didClose` | Full-text document sync (`TextDocumentSyncKind = 1`). |
| `textDocument/publishDiagnostics` | Parse-error diagnostics emitted on every open + change. Line / column locations. |
| `textDocument/formatting` | Full-document `TextEdit[]` from the CST formatter. Empty array when the document is already canonical. |
| `textDocument/hover` | Markdown card with cursor position + document type. Schema-driven descriptions tracked for follow-up. |

Server capabilities response includes `textDocumentSync = 1`,
`documentFormattingProvider = true`, `hoverProvider = true`.
Future capabilities (`rangeFormatting`, `documentSymbols`,
`codeActions`) are gated behind the same lossless-CST surface
in the library.

---

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

### Sublime Text (via `LSP` package)

`~/.config/sublime-text/Packages/User/LSP.sublime-settings`:

```json
{
  "clients": {
    "noyalib-lsp": {
      "enabled": true,
      "command": ["noyalib-lsp"],
      "selector": "source.yaml"
    }
  }
}
```

---

## Examples

Editor-driving demos under
[`crates/noyalib-lsp/examples/`](examples/):

| Script | What it shows |
|---|---|
| [`handshake.sh`](examples/handshake.sh) | One-shot `initialize` / `initialized` / `shutdown` / `exit` round-trip. Smoke test for protocol compliance. |
| [`format-on-save.sh`](examples/format-on-save.sh) | `didOpen` → `textDocument/formatting`. Returns the `TextEdit[]` an editor would apply on save. |
| [`hover-cursor.sh`](examples/hover-cursor.sh) | `didOpen` → `textDocument/hover` at a specific `(line, column)`. |

Each script pipes a sequence of `Content-Length`-framed JSON-RPC
messages into `noyalib-lsp` over stdio and prints the response
stream. POSIX-shell only — no `jq`, no `node` dependencies.

```bash
chmod +x crates/noyalib-lsp/examples/*.sh
crates/noyalib-lsp/examples/handshake.sh
```

---

## Performance

`textDocument/formatting` for a 1 MiB YAML document on Apple
M-series ≈ 12 ms (the same wall-clock as `noyafmt --write`
since both share the CST). For per-keystroke formatting under
human-perceivable latency, the server returns an empty
`TextEdit[]` when the document is already canonical, so the
editor avoids a round-trip on every `didChange` after a save.

`textDocument/publishDiagnostics` runs on every `didChange`;
parse cost on a freshly-edited buffer is dominated by the byte
range that changed, not the buffer size.

---

## When not to use noyalib-lsp

- **You want OpenAPI / AsyncAPI / Kubernetes-CRD hover docs out
  of the box.** That comes from the JSON Schema attached to
  the buffer. noyalib-lsp doesn't ship a schema registry; you
  point it at a schema explicitly via the
  `yaml.schemas` (style) settings exposed by your editor.
- **You need WebDAV-style multi-document operations
  (`workspace/applyEdit` for cross-file refactor).** Not
  implemented yet; tracked as a v0.1.x capability extension.

---

## Documentation

- **Engineering policies** (MSRV, SemVer, security, performance, concurrency, platform support, feature flags):
  [`doc/POLICIES.md`](https://github.com/sebastienrousseau/noyalib/blob/main/doc/POLICIES.md)
- **Security policy**:
  [`SECURITY.md`](https://github.com/sebastienrousseau/noyalib/blob/main/SECURITY.md)
- **API reference**: <https://docs.rs/noyalib-lsp>
- **Editor setup (VS Code, Neovim, Emacs, Helix, Zed, Sublime)**:
  [`doc/editor-setup.md`](https://github.com/sebastienrousseau/noyalib/blob/main/crates/noyalib-lsp/doc/editor-setup.md)
- **Protocol coverage (which LSP methods are implemented)**:
  [`doc/protocol-coverage.md`](https://github.com/sebastienrousseau/noyalib/blob/main/crates/noyalib-lsp/doc/protocol-coverage.md)
- **Workspace README**:
  <https://github.com/sebastienrousseau/noyalib#readme>
- **LSP specification**:
  <https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/>

---

## License

Dual-licensed under [Apache 2.0](https://www.apache.org/licenses/LICENSE-2.0)
or [MIT](https://opensource.org/licenses/MIT), at your option.
