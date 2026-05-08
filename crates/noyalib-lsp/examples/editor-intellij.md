<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->

# `noyalib-lsp` in IntelliJ IDEA / JetBrains IDEs

JetBrains IDEs (IntelliJ IDEA, RustRover, PyCharm, GoLand, …) do
not ship LSP support in the base product. Two routes work:

## Option A — LSP4IJ plugin (recommended, free)

[LSP4IJ](https://plugins.jetbrains.com/plugin/23257-lsp4ij) is
the community-maintained generic LSP client for JetBrains IDEs.

1. Install: **Settings → Plugins → Marketplace → search "LSP4IJ"**.
2. Add the noyalib-lsp server: **Settings → Languages & Frameworks
   → Language Servers → +**.
3. Configure as follows:

| Field | Value |
|---|---|
| **Name** | `noyalib-lsp` |
| **Command** | `noyalib-lsp` (or absolute path if not on `$PATH`) |
| **Args** | (leave empty) |
| **File-name patterns** | `*.yaml`, `*.yml` |
| **Server capabilities** | (leave defaults — auto-discovered) |

4. Apply, restart the IDE.

## Option B — IntelliJ Ultimate built-in LSP API (paid IDE)

IntelliJ Ultimate, RustRover, GoLand, and other paid JetBrains
IDEs have an experimental LSP API exposed to plugin developers
since 2024.1. It's not user-facing — wiring it up requires a
small custom plugin. The community has packaged a few; see
<https://plugins.jetbrains.com/search?search=LSP%20YAML> for
current options.

For most teams, **LSP4IJ (Option A) is simpler and works on
both Community and Ultimate editions**.

## Capabilities

`noyalib-lsp` advertises:

- `textDocument/publishDiagnostics` — surfaces in the JetBrains
  *Problems* tool window with the standard squiggly underlines.
- `textDocument/formatting` — bound to the IDE's
  *Reformat Code* action (`Cmd/Ctrl+Alt+L`).
- `textDocument/hover` — bound to *Quick Documentation*
  (`Cmd/Ctrl+Q` on macOS / Linux, `Ctrl+Q` on Windows).

## Verifying the install

```bash
which noyalib-lsp
noyalib-lsp --version
```

Open a YAML file in the IDE. The LSP4IJ panel (View → Tool
Windows → LSP) should show `noyalib-lsp: Running`. Mis-format
the file and save — the *Problems* tab populates with line /
column annotations.

## Performance note

LSP4IJ keeps the language server alive across files in the
same project. The first `didOpen` after IDE start has a small
startup cost (the binary cold-starts and pre-allocates its
parser pool); subsequent opens are sub-10 ms.
