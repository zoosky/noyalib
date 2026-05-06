#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Noyalib
# SPDX-License-Identifier: MIT OR Apache-2.0
#
# Open a YAML buffer and request hover at a specific (line,
# column). Returns the markdown card the editor would render in
# its hover popup.

set -euo pipefail

frame() {
    local body="$1"
    local len=${#body}
    printf 'Content-Length: %d\r\n\r\n%s' "$len" "$body"
}

URI='file:///tmp/noyalib-lsp-hover.yaml'
SOURCE='server:\n  host: api.example.com\n  port: 8080\n'

{
    frame "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{\"capabilities\":{}}}"
    frame "{\"jsonrpc\":\"2.0\",\"method\":\"initialized\",\"params\":{}}"
    frame "{\"jsonrpc\":\"2.0\",\"method\":\"textDocument/didOpen\",\"params\":{\"textDocument\":{\"uri\":\"${URI}\",\"languageId\":\"yaml\",\"version\":1,\"text\":\"${SOURCE}\"}}}"
    # Hover at line 2, column 3 — the `host` key.
    frame "{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"textDocument/hover\",\"params\":{\"textDocument\":{\"uri\":\"${URI}\"},\"position\":{\"line\":1,\"character\":2}}}"
    frame '{"jsonrpc":"2.0","id":3,"method":"shutdown"}'
    frame '{"jsonrpc":"2.0","method":"exit"}'
} | noyalib-lsp
