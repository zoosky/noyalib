<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->

# noyalib-mcp

Model Context Protocol server exposing
[noyalib](https://github.com/sebastienrousseau/noyalib)'s
lossless YAML editing to AI agents (Claude Desktop, Claude Code,
Cursor, Zed, …).

[![crates.io](https://img.shields.io/crates/v/noyalib-mcp.svg)](https://crates.io/crates/noyalib-mcp)
[![docs.rs](https://img.shields.io/docsrs/noyalib-mcp)](https://docs.rs/noyalib-mcp)
[![Build](https://img.shields.io/github/actions/workflow/status/sebastienrousseau/noyalib/ci.yml?branch=main)](https://github.com/sebastienrousseau/noyalib/actions)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](#license)

AI agents that edit YAML config today regex-replace and corrupt
comments / formatting. noyalib's CST does the edits losslessly;
this server is the protocol shim that lets MCP-aware clients
drive that engine safely.

## Contents

- [Install](#install)
- [Quick Start](#quick-start)
- [Connect](#connect)
- [Tools exposed](#tools-exposed)
- [Examples](#examples)
- [Documentation](#documentation)
- [License](#license)

## Install

```bash
cargo install noyalib-mcp
```

Or run without a Rust toolchain:

```bash
npx noyalib-mcp        # the npm wrapper auto-downloads the binary
docker run --rm -i ghcr.io/sebastienrousseau/noyalib-mcp:latest
```

Both the npm wrapper and the GHCR image consume the same signed
binary attached to every GitHub Release. See
[`pkg/VERIFY.md`](https://github.com/sebastienrousseau/noyalib/blob/main/pkg/VERIFY.md)
for the cosign verification cookbook.

## Quick Start

The server speaks JSON-RPC 2.0 over stdio. A typical agent
launches the binary as a child process, sends `initialize`, and
then dispatches tool calls:

```json
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}
{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}
{"jsonrpc":"2.0","id":3,"method":"tools/call",
 "params":{"name":"format","arguments":{"yaml":"a:1\nb:2\n"}}}
```

## Connect

### Claude Desktop / Claude Code

```bash
claude mcp add noyalib $(which noyalib-mcp)
```

### Cursor

`~/.cursor/mcp.json`:

```json
{
  "mcpServers": {
    "noyalib": {
      "command": "noyalib-mcp"
    }
  }
}
```

### Zed

`~/.config/zed/settings.json`:

```json
{
  "context_servers": {
    "noyalib": { "command": { "path": "noyalib-mcp" } }
  }
}
```

For any other MCP-aware client, point at the binary; the
transport is stdio with newline-delimited JSON-RPC 2.0.

## Tools exposed

| Tool | What it does |
|---|---|
| `parse` | Parse a YAML document; return its AST as JSON. |
| `format` | Re-emit a YAML document via noyalib's CST formatter. Comments + indentation preserved. |
| `get` | Read a value at a dotted path (e.g. `server.port`). |
| `set` | Surgically rewrite a value at a path; returns the modified document with comments intact. |
| `validate` | Syntax check; with an optional JSON Schema 2020-12 contract for semantic checks. |

Each tool's full schema lives in the response to `tools/list`.

## Examples

Agent-driving demos under
[`crates/noyalib-mcp/examples/`](examples/):

```bash
crates/noyalib-mcp/examples/handshake.sh     # initialize → tools/list
crates/noyalib-mcp/examples/format-call.sh   # tools/call format
crates/noyalib-mcp/examples/set-then-get.sh  # CST mutation round-trip
```

Each example pipes a sequence of JSON-RPC frames into
`noyalib-mcp` over stdio and shows the response stream.

## Documentation

- **API reference**: <https://docs.rs/noyalib-mcp>
- **MCP specification**: <https://modelcontextprotocol.io>
- **Workspace README**:
  <https://github.com/sebastienrousseau/noyalib#readme>

## License

Dual-licensed under [Apache 2.0](https://www.apache.org/licenses/LICENSE-2.0)
or [MIT](https://opensource.org/licenses/MIT), at your option.
