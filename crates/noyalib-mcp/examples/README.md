<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->

# `noyalib-mcp` examples

Agent-driving demos. Each script pipes a sequence of
newline-delimited JSON-RPC 2.0 frames into `noyalib-mcp` over
stdio and prints the response stream.

| Script | What it shows |
|---|---|
| [`handshake.sh`](handshake.sh) | `initialize` → `tools/list` smoke test. Confirms the binary speaks the protocol and announces the expected tools. |
| [`format-call.sh`](format-call.sh) | Call the `format` tool on a poorly-spaced document. Demonstrates that comments + indentation pass through the CST formatter unchanged. |
| [`set-then-get.sh`](set-then-get.sh) | Round-trip the mutation surface: `set` rewrites `server.port`, `get` reads it back. Surgical edit; surrounding bytes untouched. |

```bash
chmod +x crates/noyalib-mcp/examples/*.sh
crates/noyalib-mcp/examples/handshake.sh
```

The scripts use `printf` / `echo` only (no `jq`, no `node`), so
they work on any POSIX shell. Pipe through `jq -c .` if you
want pretty-printed JSON responses for human reading.

## License

Dual-licensed under Apache 2.0 or MIT, at your option.
