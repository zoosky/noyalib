#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Noyalib
# SPDX-License-Identifier: MIT OR Apache-2.0
#
# Open a poorly-formatted YAML buffer, request formatting, and
# print the resulting TextEdit[]. Mirrors the call sequence an
# editor issues on save.

set -euo pipefail

frame() {
    local body="$1"
    local len=${#body}
    printf 'Content-Length: %d\r\n\r\n%s' "$len" "$body"
}

URI='file:///tmp/noyalib-lsp-demo.yaml'
SOURCE='a  :   1\nb: 2\n'

{
    frame "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{\"capabilities\":{}}}"
    frame "{\"jsonrpc\":\"2.0\",\"method\":\"initialized\",\"params\":{}}"
    frame "{\"jsonrpc\":\"2.0\",\"method\":\"textDocument/didOpen\",\"params\":{\"textDocument\":{\"uri\":\"${URI}\",\"languageId\":\"yaml\",\"version\":1,\"text\":\"${SOURCE}\"}}}"
    frame "{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"textDocument/formatting\",\"params\":{\"textDocument\":{\"uri\":\"${URI}\"},\"options\":{\"tabSize\":2,\"insertSpaces\":true}}}"
    frame '{"jsonrpc":"2.0","id":3,"method":"shutdown"}'
    frame '{"jsonrpc":"2.0","method":"exit"}'
} | noyalib-lsp
