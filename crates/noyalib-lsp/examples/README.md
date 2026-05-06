<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->

# `noyalib-lsp` examples

Editor-driving demos. Each script pipes a sequence of
Content-Length-framed JSON-RPC 2.0 messages into `noyalib-lsp`
over stdio and prints the response stream.

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

## License

Dual-licensed under Apache 2.0 or MIT, at your option.
