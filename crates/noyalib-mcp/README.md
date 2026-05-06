<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->

<p align="center">
  <img src="https://cloudcdn.pro/noyalib/v1/logos/noyalib.svg" alt="Noyalib logo" width="128" />
</p>

<h1 align="center">noyalib-mcp</h1>

<p align="center">
  <strong>Model Context Protocol server exposing noyalib's
  lossless YAML editing to AI agents (Claude Desktop, Claude
  Code, Cursor, Zed, Continue.dev, …).</strong>
</p>

<p align="center">
  <a href="https://github.com/sebastienrousseau/noyalib/actions"><img src="https://img.shields.io/github/actions/workflow/status/sebastienrousseau/noyalib/ci.yml?style=for-the-badge&logo=github" alt="Build" /></a>
  <a href="https://crates.io/crates/noyalib-mcp"><img src="https://img.shields.io/crates/v/noyalib-mcp.svg?style=for-the-badge&color=fc8d62&logo=rust" alt="Crates.io" /></a>
  <a href="https://docs.rs/noyalib-mcp"><img src="https://img.shields.io/badge/docs.rs-noyalib--mcp-66c2a5?style=for-the-badge&labelColor=555555&logo=docs.rs" alt="Docs.rs" /></a>
  <a href="https://lib.rs/crates/noyalib-mcp"><img src="https://img.shields.io/badge/lib.rs-v0.0.1-orange.svg?style=for-the-badge" alt="lib.rs" /></a>
  <a href="https://api.securityscorecards.dev/projects/github.com/sebastienrousseau/noyalib"><img src="https://api.securityscorecards.dev/projects/github.com/sebastienrousseau/noyalib/badge" alt="OpenSSF Scorecard" /></a>
</p>

---

## Contents

- [Install](#install) — Cargo, npx, Docker
- [Quick Start](#quick-start) — JSON-RPC handshake
- [Why this approach?](#why-this-approach) — design rationale
- [Connect](#connect) — per-client configuration
- [Tools exposed](#tools-exposed) — MCP tool reference
- [Examples](#examples) — runnable scripts
- [Verification](#verification) — cosign + npm provenance
- [When not to use noyalib-mcp](#when-not-to-use-noyalib-mcp)
- [Documentation](#documentation)
- [License](#license)

---

## Install

```bash
cargo install noyalib-mcp
```

For environments without a Rust toolchain (the typical AI-agent
deployment shape):

```bash
# npm wrapper — auto-downloads the matching binary on first run,
# caches under ~/.cache/noyalib-mcp/<version>/.
npx noyalib-mcp

# Container — multi-arch (linux/amd64, linux/arm64).
docker run --rm -i ghcr.io/sebastienrousseau/noyalib-mcp:latest
```

Both consume the same signed binary attached to every GitHub
Release. See [Verification](#verification) for the verify
commands.

---

## Quick Start

The server speaks JSON-RPC 2.0 over stdio with newline-delimited
frames, per the
[MCP specification](https://modelcontextprotocol.io). A typical
agent launches the binary as a child process, sends
`initialize`, then dispatches tool calls:

```json
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"agent","version":"0.0.1"}}}
{"jsonrpc":"2.0","method":"notifications/initialized"}
{"jsonrpc":"2.0","id":2,"method":"tools/list"}
{"jsonrpc":"2.0","id":3,"method":"tools/call",
 "params":{"name":"format","arguments":{"yaml":"a:1\nb:2\n"}}}
```

---

## Why this approach?

AI agents that edit YAML configuration today regex-replace and
corrupt comments, indentation, and document structure. The same
agent fixing a port number in a Kubernetes manifest can shift
every comment by a line, reorder sibling keys, or strip
trailing whitespace that a downstream linter cared about.

noyalib's CST does the edits losslessly — a `set("server.port",
"9090")` rewrites only the byte span of the `8080` scalar; the
surrounding comments and indentation pass through untouched.
This server is the protocol shim that lets MCP-aware clients
drive that engine safely:

- **Lossless mutation.** `tools/call set` returns a document
  byte-identical to the input outside the touched span.
- **Surgical reads.** `tools/call get` walks the dotted path
  and returns just the value, not the whole tree.
- **Schema validation.** `tools/call validate --schema` runs
  the same JSON Schema 2020-12 engine `noyavalidate` ships.
- **Stdio transport.** Standard MCP. Works with every
  spec-compliant client.

---

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
    "noyalib": {
      "command": { "path": "noyalib-mcp" }
    }
  }
}
```

### Continue.dev

`~/.continue/config.json`:

```json
{
  "experimental": {
    "modelContextProtocolServers": [
      { "transport": { "type": "stdio", "command": "noyalib-mcp" } }
    ]
  }
}
```

### Any other MCP-aware client

Point at the binary; the transport is stdio with newline-
delimited JSON-RPC 2.0.

---

## Tools exposed

| Tool | Arguments | Returns |
|---|---|---|
| `parse` | `{ yaml: string }` | The parsed value as JSON. Surfaces parse errors with line / column. |
| `format` | `{ yaml: string }` | The same document re-emitted via the CST formatter. Comments + indentation preserved. |
| `get` | `{ yaml: string, path: string }` | The scalar at the dotted path (e.g. `server.port`). |
| `set` | `{ yaml: string, path: string, value: string }` | The document with the path's value rewritten. Surrounding bytes untouched. |
| `validate` | `{ yaml: string, schema?: string }` | `{ ok: true }` or a structured violation list. Optional JSON Schema 2020-12 contract. |

Every tool's full input + output schema lives in the response
to `tools/list`. The server also supports the
`notifications/cancelled` and `notifications/initialized`
notifications.

---

## Examples

Agent-driving demos under
[`crates/noyalib-mcp/examples/`](examples/):

| Script | What it shows |
|---|---|
| [`handshake.sh`](examples/handshake.sh) | `initialize` → `tools/list` smoke test. Confirms the binary speaks the protocol and announces the expected tools. |
| [`format-call.sh`](examples/format-call.sh) | `tools/call format` on a poorly-spaced document. Demonstrates that comments + indentation pass through the CST formatter unchanged. |
| [`set-then-get.sh`](examples/set-then-get.sh) | Round-trip the mutation surface: `set` rewrites `server.port`, `get` reads it back. Surgical edit; surrounding bytes untouched. |

```bash
chmod +x crates/noyalib-mcp/examples/*.sh
crates/noyalib-mcp/examples/handshake.sh | jq -c .
```

POSIX-shell only — no `jq`, no `node` dependencies. Pipe
through `jq -c .` if you want pretty-printed JSON responses.

---

## Verification

The npm wrapper and the GHCR image both consume the signed
binary attached to every GitHub Release. To verify the
underlying binary before trusting it:

```bash
COSIGN_EXPERIMENTAL=1 cosign verify-blob \
  --certificate-identity-regexp 'https://github.com/sebastienrousseau/noyalib/' \
  --certificate-oidc-issuer 'https://token.actions.githubusercontent.com' \
  --certificate <artefact>.pem \
  --signature   <artefact>.sig \
  <artefact>
```

The npm wrapper additionally carries an
[npm provenance attestation](https://docs.npmjs.com/generating-provenance-statements):

```bash
npm view noyalib-mcp provenance
```

Full cookbook: [`pkg/VERIFY.md`](https://github.com/sebastienrousseau/noyalib/blob/main/pkg/VERIFY.md).

---

## When not to use noyalib-mcp

- **You don't trust your AI agent with filesystem access at
  all.** noyalib-mcp doesn't read or write files itself —
  every operation takes the YAML document as a string argument
  and returns the result as a string. The agent decides what
  to do with the result. If the agent has filesystem access,
  it can persist the response wherever it wants.
- **You need a sandboxed schema registry.** noyalib-mcp accepts
  schemas as inline strings in `tools/call validate`; it does
  not fetch schemas from URLs. If your workflow needs
  network-resolved schemas, the agent is responsible for
  fetching the schema first and passing the bytes.

---

## Documentation

- **API reference**: <https://docs.rs/noyalib-mcp>
- **Tools reference (input schemas + error codes)**:
  [`doc/tools-reference.md`](https://github.com/sebastienrousseau/noyalib/blob/main/crates/noyalib-mcp/doc/tools-reference.md)
- **Agent integration (Claude Desktop, Cursor, Continue.dev)**:
  [`doc/agent-integration.md`](https://github.com/sebastienrousseau/noyalib/blob/main/crates/noyalib-mcp/doc/agent-integration.md)
- **MCP specification**: <https://modelcontextprotocol.io>
- **Workspace README**:
  <https://github.com/sebastienrousseau/noyalib#readme>

---

## License

Dual-licensed under [Apache 2.0](https://www.apache.org/licenses/LICENSE-2.0)
or [MIT](https://opensource.org/licenses/MIT), at your option.
