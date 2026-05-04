// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! `noyalib-mcp` — Model Context Protocol server exposing
//! noyalib's lossless YAML editing to AI agents.
//!
//! Communicates via newline-delimited JSON-RPC 2.0 over stdio per
//! the MCP 2025-06 spec. Two tools:
//!
//! - `noyalib_get`: read the value at a path in a YAML file.
//! - `noyalib_set`: set the value at a path, preserving every
//!   untouched byte (comments, indentation, sibling entries).
//!
//! # Why this exists
//!
//! AI agents that edit YAML configuration today regex-replace and
//! corrupt comments / formatting. noyalib's CST does the edits
//! losslessly; this server is the protocol shim that lets Claude,
//! Cursor, Zed, and any other MCP-aware client drive that engine
//! safely.
//!
//! # Install + connect
//!
//! ```text
//! cargo install noyalib-mcp
//! claude mcp add noyalib /usr/local/bin/noyalib-mcp
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};
use std::io::{self, BufRead, Write};

mod tools;

/// JSON-RPC 2.0 request envelope. Method-specific parameters live
/// in [`JsonValue`] to keep parsing flexible across the few methods
/// the MCP spec asks of a server.
#[derive(Debug, Deserialize)]
struct Request {
    /// JSON-RPC version. MCP requires `"2.0"`.
    jsonrpc: String,
    /// Method name, e.g. `tools/call`. Notifications have no `id`.
    method: String,
    /// Method parameters. Shape depends on `method`.
    #[serde(default)]
    params: JsonValue,
    /// Request id; absent on notifications.
    id: Option<JsonValue>,
}

/// JSON-RPC 2.0 success response envelope.
#[derive(Debug, Serialize)]
struct Response {
    /// Always `"2.0"`.
    jsonrpc: &'static str,
    /// The result payload.
    result: JsonValue,
    /// Echo of the corresponding request's id.
    id: JsonValue,
}

/// JSON-RPC 2.0 error envelope.
#[derive(Debug, Serialize)]
struct ErrorResponse {
    /// Always `"2.0"`.
    jsonrpc: &'static str,
    /// Error payload.
    error: ErrorObject,
    /// Echo of the corresponding request's id.
    id: JsonValue,
}

/// JSON-RPC 2.0 error object.
#[derive(Debug, Serialize)]
struct ErrorObject {
    /// Numeric error code per JSON-RPC convention.
    code: i32,
    /// Human-readable message.
    message: String,
}

fn main() -> io::Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout().lock();
    let mut line = String::new();
    let mut handle = stdin.lock();

    loop {
        line.clear();
        let n = handle.read_line(&mut line)?;
        if n == 0 {
            return Ok(());
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        match handle_message(trimmed) {
            HandleOutcome::Reply(payload) => {
                writeln!(stdout, "{payload}")?;
                stdout.flush()?;
            }
            HandleOutcome::Silent => {}
        }
    }
}

enum HandleOutcome {
    Reply(String),
    Silent,
}

fn handle_message(raw: &str) -> HandleOutcome {
    let req: Request = match serde_json::from_str(raw) {
        Ok(r) => r,
        Err(e) => {
            return HandleOutcome::Reply(error_str(
                JsonValue::Null,
                -32700,
                format!("parse error: {e}"),
            ));
        }
    };
    if req.jsonrpc != "2.0" {
        return HandleOutcome::Reply(error_str(
            req.id.unwrap_or(JsonValue::Null),
            -32600,
            "invalid request: jsonrpc must be \"2.0\"".to_string(),
        ));
    }
    // Notifications (no id) get processed but never replied to.
    let id = req.id.clone();
    let result = dispatch(&req.method, req.params);
    match (id, result) {
        (None, _) => HandleOutcome::Silent,
        (Some(id), Ok(value)) => HandleOutcome::Reply(
            serde_json::to_string(&Response {
                jsonrpc: "2.0",
                result: value,
                id,
            })
            .expect("infallible serialise"),
        ),
        (Some(id), Err((code, msg))) => HandleOutcome::Reply(error_str(id, code, msg)),
    }
}

fn dispatch(method: &str, params: JsonValue) -> Result<JsonValue, (i32, String)> {
    match method {
        "initialize" => Ok(json!({
            "protocolVersion": "2025-06-18",
            "serverInfo": {
                "name": "noyalib-mcp",
                "version": env!("CARGO_PKG_VERSION"),
            },
            "capabilities": {
                "tools": {}
            }
        })),
        "initialized" | "notifications/initialized" => Ok(JsonValue::Null),
        "tools/list" => Ok(json!({
            "tools": tools::descriptors()
        })),
        "tools/call" => tools::call(params),
        "ping" => Ok(JsonValue::Object(serde_json::Map::new())),
        other => Err((
            -32601,
            format!("method not found: {other}"),
        )),
    }
}

fn error_str(id: JsonValue, code: i32, message: String) -> String {
    serde_json::to_string(&ErrorResponse {
        jsonrpc: "2.0",
        error: ErrorObject { code, message },
        id,
    })
    .expect("infallible serialise")
}
