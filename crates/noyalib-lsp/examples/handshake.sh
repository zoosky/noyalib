#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Noyalib
# SPDX-License-Identifier: MIT OR Apache-2.0
#
# Drive a one-shot LSP initialize / shutdown / exit handshake
# against `noyalib-lsp`. Useful as a smoke test that the server
# binary is on $PATH and speaks the protocol.

set -euo pipefail

# Frame helper — emits a Content-Length-framed JSON-RPC message.
frame() {
    local body="$1"
    local len=${#body}
    printf 'Content-Length: %d\r\n\r\n%s' "$len" "$body"
}

{
    frame '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}'
    frame '{"jsonrpc":"2.0","method":"initialized","params":{}}'
    frame '{"jsonrpc":"2.0","id":2,"method":"shutdown"}'
    frame '{"jsonrpc":"2.0","method":"exit"}'
} | noyalib-lsp
