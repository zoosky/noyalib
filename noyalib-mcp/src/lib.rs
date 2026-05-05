// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Library surface for `noyalib-mcp`.
//!
//! Hosts the JSON-RPC 2.0 dispatch logic and the tool implementations.
//! The `noyalib-mcp` binary in `main.rs` is a thin stdio loop that
//! drives [`handle_message`]; tests reach the same handlers
//! directly so coverage no longer depends on standing up a real
//! stdio process.

#![forbid(unsafe_code)]
#![warn(missing_docs)]
// Opt-in coverage exclusion (`NOYALIB_COVERAGE=1`) — see
// `build.rs` for the flag, individual `coverage(off)` annotations
// are below.
#![cfg_attr(noyalib_coverage, allow(unstable_features))]
#![cfg_attr(noyalib_coverage, feature(coverage_attribute))]

use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};

pub mod tools;

/// JSON-RPC 2.0 request envelope. Method-specific parameters live
/// in [`JsonValue`] to keep parsing flexible across the few methods
/// the MCP spec asks of a server.
#[derive(Debug, Deserialize)]
pub struct Request {
    /// JSON-RPC version. MCP requires `"2.0"`.
    pub jsonrpc: String,
    /// Method name, e.g. `tools/call`. Notifications have no `id`.
    pub method: String,
    /// Method parameters. Shape depends on `method`.
    #[serde(default)]
    pub params: JsonValue,
    /// Request id; absent on notifications.
    pub id: Option<JsonValue>,
}

/// JSON-RPC 2.0 success response envelope.
#[derive(Debug, Serialize)]
pub struct Response {
    /// Always `"2.0"`.
    pub jsonrpc: &'static str,
    /// The result payload.
    pub result: JsonValue,
    /// Echo of the corresponding request's id.
    pub id: JsonValue,
}

/// JSON-RPC 2.0 error envelope.
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    /// Always `"2.0"`.
    pub jsonrpc: &'static str,
    /// Error payload.
    pub error: ErrorObject,
    /// Echo of the corresponding request's id.
    pub id: JsonValue,
}

/// JSON-RPC 2.0 error object.
#[derive(Debug, Serialize)]
pub struct ErrorObject {
    /// Numeric error code per JSON-RPC convention.
    pub code: i32,
    /// Human-readable message.
    pub message: String,
}

/// What the stdio loop should do with a parsed message — write a
/// reply on stdout, or stay silent (notifications never receive a
/// response).
#[derive(Debug, PartialEq, Eq)]
pub enum HandleOutcome {
    /// Send the wrapped JSON payload back on stdout.
    Reply(String),
    /// Notification — no reply expected.
    Silent,
}

/// Process one newline-delimited JSON-RPC message. The stdio loop
/// in `main` calls this per line; tests call it with crafted
/// strings.
#[must_use]
pub fn handle_message(raw: &str) -> HandleOutcome {
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

/// MCP method dispatcher. Returns the `result` payload on success
/// or a `(code, message)` pair for the error envelope.
pub fn dispatch(method: &str, params: JsonValue) -> Result<JsonValue, (i32, String)> {
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
        other => Err((-32601, format!("method not found: {other}"))),
    }
}

/// Render a JSON-RPC error envelope to a single line string.
pub fn error_str(id: JsonValue, code: i32, message: String) -> String {
    serde_json::to_string(&ErrorResponse {
        jsonrpc: "2.0",
        error: ErrorObject { code, message },
        id,
    })
    .expect("infallible serialise")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_reply(out: HandleOutcome) -> JsonValue {
        match out {
            HandleOutcome::Reply(s) => serde_json::from_str(&s).unwrap(),
            HandleOutcome::Silent => panic!("expected Reply, got Silent"),
        }
    }

    // ── handle_message ─────────────────────────────────────────────────

    #[test]
    fn handle_message_returns_parse_error_on_bad_json() {
        let out = handle_message("not json {");
        let v = parse_reply(out);
        assert_eq!(v["error"]["code"].as_i64().unwrap(), -32700);
        assert!(v["error"]["message"]
            .as_str()
            .unwrap()
            .contains("parse error"));
        // Per JSON-RPC: parse errors carry `id: null`.
        assert!(v["id"].is_null());
    }

    #[test]
    fn handle_message_rejects_non_2_0_jsonrpc() {
        let req = json!({"jsonrpc": "1.0", "method": "ping", "id": 1});
        let out = handle_message(&req.to_string());
        let v = parse_reply(out);
        assert_eq!(v["error"]["code"].as_i64().unwrap(), -32600);
        assert_eq!(v["id"].as_i64().unwrap(), 1);
    }

    #[test]
    fn handle_message_returns_silent_for_notifications() {
        let req = json!({"jsonrpc": "2.0", "method": "ping"});
        let out = handle_message(&req.to_string());
        assert_eq!(out, HandleOutcome::Silent);
    }

    #[test]
    fn handle_message_returns_silent_for_notifications_initialized() {
        let req = json!({"jsonrpc": "2.0", "method": "notifications/initialized"});
        let out = handle_message(&req.to_string());
        assert_eq!(out, HandleOutcome::Silent);
    }

    #[test]
    fn handle_message_returns_unknown_method_error() {
        let req = json!({"jsonrpc": "2.0", "method": "frobnicate", "id": 7});
        let out = handle_message(&req.to_string());
        let v = parse_reply(out);
        assert_eq!(v["error"]["code"].as_i64().unwrap(), -32601);
        assert!(v["error"]["message"]
            .as_str()
            .unwrap()
            .contains("frobnicate"));
        assert_eq!(v["id"].as_i64().unwrap(), 7);
    }

    #[test]
    fn handle_message_returns_jsonrpc_error_when_jsonrpc_field_missing() {
        let req = json!({"method": "ping", "id": 1});
        let out = handle_message(&req.to_string());
        let v = parse_reply(out);
        // Either parse error (missing field) or invalid request — both
        // are valid envelopes; the contract is "you get an error".
        assert!(v["error"].is_object());
    }

    // ── dispatch ──────────────────────────────────────────────────────

    #[test]
    fn dispatch_initialize_returns_protocol_metadata() {
        let v = dispatch("initialize", JsonValue::Null).unwrap();
        assert_eq!(v["protocolVersion"].as_str().unwrap(), "2025-06-18");
        assert_eq!(v["serverInfo"]["name"].as_str().unwrap(), "noyalib-mcp");
        assert!(v["capabilities"]["tools"].is_object());
    }

    #[test]
    fn dispatch_initialized_returns_null() {
        let v = dispatch("initialized", JsonValue::Null).unwrap();
        assert!(v.is_null());
    }

    #[test]
    fn dispatch_notifications_initialized_returns_null() {
        let v = dispatch("notifications/initialized", JsonValue::Null).unwrap();
        assert!(v.is_null());
    }

    #[test]
    fn dispatch_tools_list_returns_descriptor_array() {
        let v = dispatch("tools/list", JsonValue::Null).unwrap();
        let tools = v["tools"].as_array().unwrap();
        assert!(tools.iter().any(|t| t["name"] == "noyalib_get"));
        assert!(tools.iter().any(|t| t["name"] == "noyalib_set"));
    }

    #[test]
    fn dispatch_ping_returns_empty_object() {
        let v = dispatch("ping", JsonValue::Null).unwrap();
        assert!(v.is_object());
        assert!(v.as_object().unwrap().is_empty());
    }

    #[test]
    fn dispatch_unknown_method_returns_method_not_found() {
        let err = dispatch("frobnicate", JsonValue::Null).unwrap_err();
        assert_eq!(err.0, -32601);
        assert!(err.1.contains("frobnicate"));
    }

    #[test]
    fn dispatch_tools_call_propagates_tools_errors() {
        // Missing `name` argument — tools::call returns -32602.
        let err = dispatch("tools/call", json!({})).unwrap_err();
        assert_eq!(err.0, -32602);
    }

    // ── error_str ─────────────────────────────────────────────────────

    #[test]
    fn error_str_renders_canonical_envelope() {
        let s = error_str(json!(42), -32000, "boom".into());
        let v: JsonValue = serde_json::from_str(&s).unwrap();
        assert_eq!(v["jsonrpc"].as_str().unwrap(), "2.0");
        assert_eq!(v["id"].as_i64().unwrap(), 42);
        assert_eq!(v["error"]["code"].as_i64().unwrap(), -32000);
        assert_eq!(v["error"]["message"].as_str().unwrap(), "boom");
    }

    #[test]
    fn error_str_handles_null_id() {
        let s = error_str(JsonValue::Null, -32700, "parse".into());
        let v: JsonValue = serde_json::from_str(&s).unwrap();
        assert!(v["id"].is_null());
    }
}
