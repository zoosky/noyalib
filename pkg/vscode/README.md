<!-- SPDX-FileCopyrightText: 2026 Noyalib -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# noyalib for Visual Studio Code

YAML language support backed by `noyalib-lsp`:

- **Format on save** via `noyafmt`'s lossless CST formatter.
  Comments, anchor positions, and document structure are
  preserved byte-for-byte; only whitespace and quoting are
  normalised.
- **Inline diagnostics** for parse errors, with the rustc-style
  caret pointer rendered in the gutter.
- **JSON Schema 2020-12 validation.** Point a workspace setting
  at a schema and noyalib-lsp surfaces violations live as you
  type.
- **Schema-driven hover docs.** Hover any key whose schema
  carries a `description` — the markdown lands in the hover
  tooltip.

## Install

- **VS Code**: search for `noyalib` in the marketplace, or run
  `code --install-extension sebastienrousseau.noyalib`.
- **Open VSX** (VSCodium, Theia, Gitpod): same name on
  [open-vsx.org](https://open-vsx.org).

## How it works

The extension spawns `noyalib-lsp` as a child process and speaks
the standard LSP wire format over stdio. By default the binary
that ships with the extension is used; configure
`noyalib.path` to point at a system-installed
`noyalib-lsp` (e.g. from `cargo install --path
crates/noyalib-lsp`) if you prefer to manage your own.

The bundled binary tracks the same git tag as the extension —
the `vscode-extension` job in
`.github/workflows/release-binaries.yml` cuts a new `.vsix` per
release and publishes to both VS Code Marketplace and Open VSX
in lockstep with the GitHub Release.

## Building locally

```bash
cd pkg/vscode
npm ci
npm run compile
npm run package      # produces ../../noyalib-<version>.vsix
```

## License

Dual-licensed under MIT or Apache-2.0, at your option.
