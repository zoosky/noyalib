<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->

# `noyalib-lsp` in Visual Studio Code

`noyalib-lsp` speaks the standard
[Language Server Protocol](https://microsoft.github.io/language-server-protocol/).
VS Code reaches it through any LSP-client extension; the most
common path is the lightweight
[`generic-language-server`](https://marketplace.visualstudio.com/items?itemName=jeff-hykin.better-yaml-syntax)
extension or a tiny custom client.

## Option A — built-in `languageServer` configuration

Add to your workspace `.vscode/settings.json`:

```json
{
  "yaml.serverPath": "noyalib-lsp",
  "yaml.serverArgs": [],
  "[yaml]": {
    "editor.defaultFormatter": "noyalib-lsp",
    "editor.formatOnSave": true
  }
}
```

The `yaml.serverPath` key is read by extensions that follow
[VS Code's LSP convention](https://code.visualstudio.com/api/language-extensions/language-server-extension-guide).

## Option B — minimal extension stub

If your team needs a turnkey extension wrapping `noyalib-lsp`,
[the LSP sample](https://github.com/microsoft/vscode-extension-samples/tree/main/lsp-sample)
is the canonical scaffold. Replace the demo server with:

```typescript
// client/src/extension.ts
import * as vscode from "vscode";
import { LanguageClient, ServerOptions } from "vscode-languageclient/node";

export function activate(ctx: vscode.ExtensionContext) {
  const serverOptions: ServerOptions = {
    command: "noyalib-lsp",  // assumes binary on $PATH
    args: [],
  };
  const client = new LanguageClient(
    "noyalib-lsp",
    "noyalib YAML Language Server",
    serverOptions,
    { documentSelector: [{ scheme: "file", language: "yaml" }] },
  );
  ctx.subscriptions.push(client.start());
}
```

## Capabilities exposed

`noyalib-lsp` advertises the following LSP capabilities; VS Code
binds the matching commands automatically:

| Capability | What you get |
|---|---|
| `textDocument/publishDiagnostics` | Real-time YAML 1.2 syntax errors with span highlighting |
| `textDocument/formatting` | `Format Document` (Shift+Alt+F / ⇧⌥F) runs `noyafmt` |
| `textDocument/hover` | Hover-over key shows the resolved value, byte span, and any inline comment |

## Verifying the install

```bash
# 1. Confirm the binary is on $PATH
which noyalib-lsp

# 2. Smoke-test the LSP handshake (returns one JSON-RPC reply)
crates/noyalib-lsp/examples/handshake.sh

# 3. Open a YAML file in VS Code; diagnostics should appear inline
```

## Performance note

`noyalib-lsp` parses each `didOpen` / `didChange` event in a
single pass (`O(n)` in document bytes). A 1 MB YAML file
typically reports diagnostics in under 5 ms on commodity
hardware — the LSP is not the bottleneck for editor
responsiveness.
