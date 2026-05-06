<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->

# noyalib-mcp

**Model Context Protocol server exposing
[noyalib](https://crates.io/crates/noyalib)'s lossless YAML editing to
AI agents.**

AI agents that edit YAML configuration today regex-replace and corrupt
comments / formatting. noyalib's CST does the edits losslessly; this
server is the protocol shim that lets Claude, Cursor, Zed, and any
other MCP-aware client drive that engine safely.

## Install

```sh
cargo install noyalib-mcp
```

## Connect (Claude Desktop / Claude Code)

```sh
claude mcp add noyalib $(which noyalib-mcp)
```

For other clients (Cursor, Zed), add an MCP server entry pointing at
the binary; the transport is stdio with newline-delimited JSON-RPC.

## Tools

### `noyalib_get`

Read a value at a dotted/indexed path.

```json
{
  "name": "noyalib_get",
  "arguments": { "file": "config.yaml", "path": "server.port" }
}
```

### `noyalib_set`

Set a value at a path. Only the touched span is rewritten — every
comment, blank line, and sibling entry is preserved byte-for-byte.

```json
{
  "name": "noyalib_set",
  "arguments": {
    "file": "config.yaml",
    "path": "version",
    "value": "0.0.2"
  }
}
```

## Wire format

Stdin/stdout, one JSON-RPC 2.0 message per line. Implements MCP
`2025-06-18`. Methods supported: `initialize`, `initialized`,
`tools/list`, `tools/call`, `ping`.

## License

MIT OR Apache-2.0.
