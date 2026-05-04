// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Tool registry for the MCP server.
//!
//! Each entry in [`descriptors`] is the JSON Schema that a client
//! sees from `tools/list`; [`call`] is the dispatch entry point for
//! `tools/call`. Tools delegate the actual YAML work to noyalib's
//! `cst::Document` so edits round-trip with comments, indentation,
//! and sibling entries preserved byte-for-byte.

use noyalib::cst::parse_document;
use serde_json::{json, Value as JsonValue};
use std::fs;

/// Descriptors returned to MCP clients via `tools/list`.
pub(crate) fn descriptors() -> Vec<JsonValue> {
    vec![
        json!({
            "name": "noyalib_get",
            "description": "Read the YAML value at a dotted/indexed path \
                in the given file. Returns the source slice exactly — no \
                re-quoting, no canonicalisation. Preserves comments and \
                formatting for any later `noyalib_set`.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "file": {
                        "type": "string",
                        "description": "Path to the YAML file on disk."
                    },
                    "path": {
                        "type": "string",
                        "description": "Dotted/indexed path into the YAML, \
                            e.g. `server.host` or `items[0].name`."
                    }
                },
                "required": ["file", "path"]
            }
        }),
        json!({
            "name": "noyalib_set",
            "description": "Set the YAML value at a dotted/indexed path \
                in the given file. Only the touched span is rewritten — \
                every comment, blank line, and sibling entry is preserved \
                byte-for-byte. Useful for Renovate-style version bumps and \
                config patches by AI agents.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "file": {
                        "type": "string",
                        "description": "Path to the YAML file on disk."
                    },
                    "path": {
                        "type": "string",
                        "description": "Dotted/indexed path into the YAML."
                    },
                    "value": {
                        "type": "string",
                        "description": "Replacement value as a YAML \
                            fragment (e.g. `0.0.2`, `\\\"hello\\\"`, \
                            `[1, 2, 3]`). Must parse in the target \
                            position; the document is left unchanged on \
                            parse error."
                    }
                },
                "required": ["file", "path", "value"]
            }
        }),
    ]
}

/// `tools/call` dispatcher. Returns the JSON-RPC `result` payload on
/// success, or `(code, message)` for an error envelope.
pub(crate) fn call(params: JsonValue) -> Result<JsonValue, (i32, String)> {
    let name = params
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| (-32602, "missing field: name".to_string()))?;
    let args = params.get("arguments").cloned().unwrap_or(JsonValue::Null);

    match name {
        "noyalib_get" => tool_get(&args),
        "noyalib_set" => tool_set(&args),
        _ => Err((-32601, format!("unknown tool: {name}"))),
    }
}

/// Wrap a tool result string into the MCP `tools/call` reply shape.
fn ok_text(text: String) -> JsonValue {
    json!({
        "content": [
            { "type": "text", "text": text }
        ]
    })
}

fn tool_get(args: &JsonValue) -> Result<JsonValue, (i32, String)> {
    let file = arg_str(args, "file")?;
    let path = arg_str(args, "path")?;
    let src = fs::read_to_string(file).map_err(|e| (-32000, format!("read {file}: {e}")))?;
    let doc = parse_document(&src).map_err(|e| (-32001, format!("parse {file}: {e}")))?;
    match doc.get(path) {
        Some(value) => Ok(ok_text(value.to_string())),
        None => Err((-32002, format!("path not found in {file}: {path}"))),
    }
}

fn tool_set(args: &JsonValue) -> Result<JsonValue, (i32, String)> {
    let file = arg_str(args, "file")?;
    let path = arg_str(args, "path")?;
    let value = arg_str(args, "value")?;
    let src = fs::read_to_string(file).map_err(|e| (-32000, format!("read {file}: {e}")))?;
    let mut doc = parse_document(&src).map_err(|e| (-32001, format!("parse {file}: {e}")))?;
    doc.set(path, value)
        .map_err(|e| (-32003, format!("set {path} = {value}: {e}")))?;
    fs::write(file, doc.to_string().as_bytes())
        .map_err(|e| (-32000, format!("write {file}: {e}")))?;
    Ok(ok_text(format!(
        "set {path} = {value} in {file} (lossless: comments and formatting preserved)"
    )))
}

fn arg_str<'a>(args: &'a JsonValue, key: &str) -> Result<&'a str, (i32, String)> {
    args.get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| (-32602, format!("missing string argument: {key}")))
}
