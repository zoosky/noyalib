<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->

# `noyalib-lsp` examples

Editor-driving demos and per-editor configuration snippets.

## Protocol-level scripts

Each script pipes a sequence of Content-Length-framed
JSON-RPC 2.0 messages into `noyalib-lsp` over stdio and prints
the response stream.

| Script | What it shows |
|---|---|
| [`handshake.sh`](handshake.sh) | One-shot `initialize` / `initialized` / `shutdown` / `exit` round-trip. Smoke test for protocol compliance. |
| [`format-on-save.sh`](format-on-save.sh) | Open a poorly-formatted buffer via `didOpen`, request `textDocument/formatting`, print the resulting `TextEdit[]`. |
| [`hover-cursor.sh`](hover-cursor.sh) | Open a buffer and request `textDocument/hover` at a specific (line, column). |

```bash
chmod +x crates/noyalib-lsp/examples/*.sh
crates/noyalib-lsp/examples/handshake.sh
```

The scripts use `printf` to construct framed messages so they
work on any POSIX shell (no `jq`, no `node`). Pipe the output
through `jq -c '.[]'` or `prettier` if you want to format the
JSON responses for human reading.

## Editor configurations

Drop-in config snippets for the most common editors. Each is a
copy-paste away from a working "Format-on-save plus inline
diagnostics" experience:

| File | Editor | What it sets up |
|---|---|---|
| [`editor-vscode.md`](editor-vscode.md) | Visual Studio Code | `yaml.serverPath`, format-on-save, optional minimal extension stub |
| [`editor-neovim.lua`](editor-neovim.lua) | Neovim ≥ 0.10 (lazy.nvim / nvim-lspconfig) | Custom server registration, format-on-save, hover keymaps |
| [`editor-helix.toml`](editor-helix.toml) | Helix | `languages.toml` entry, auto-format, formatter binding |
| [`editor-emacs.el`](editor-emacs.el) | Emacs ≥ 29 (built-in eglot) | `eglot-server-programs` entry, format-on-save hook, keymaps for hover / format / rename |
| [`editor-zed.json`](editor-zed.json) | Zed | `lsp.noyalib-lsp` block, `format_on_save: on`, distinct from the MCP context-server config |
| [`editor-sublime.json`](editor-sublime.json) | Sublime Text (LSP package) | `clients.noyalib-lsp` block, syntax selectors for YAML / Jinja2 / Helm |
| [`editor-intellij.md`](editor-intellij.md) | IntelliJ IDEA / RustRover / GoLand / PyCharm | LSP4IJ plugin recipe (Community + Ultimate) |

If you wire up a different editor and the config is non-obvious,
PRs adding a new `editor-<name>.{md,lua,toml,json,el}` snippet
are welcome.

## License

Dual-licensed under Apache 2.0 or MIT, at your option.
