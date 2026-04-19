// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Value manipulation: to_value, from_value, MappingAny, get_path_mut, ValueIndex.
//!
//! Run: `cargo run --example value_manipulation`

use noyalib::{from_str, from_value, to_value, Mapping, MappingAny, Value};
use serde::{Deserialize, Serialize};

fn done(msg: &str) {
    println!("  \x1b[32m+\x1b[0m {msg}");
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Server {
    host: String,
    port: u16,
}

fn main() {
    println!("\n  \x1b[1mnoyalib value manipulation\x1b[0m\n");

    let server = Server {
        host: "localhost".to_string(),
        port: 8080,
    };
    let value = to_value(&server).unwrap();
    done("to_value: struct -> Value");

    let parsed: Server = from_value(&value).unwrap();
    assert_eq!(parsed, server);
    done("from_value: Value -> struct");

    let yaml = "servers:\n  - host: alpha\n    port: 80\n  - host: beta\n    port: 443\n";
    let doc: Value = from_str(yaml).unwrap();
    let host = doc.get_path("servers[1].host").unwrap();
    done(&format!("get_path: servers[1].host = {host}"));

    let mut doc: Value = from_str(yaml).unwrap();
    if let Some(port) = doc.get_path_mut("servers[0].port") {
        *port = Value::from(9090);
    }
    done(&format!(
        "get_path_mut: port changed to {}",
        doc.get_path("servers[0].port").unwrap()
    ));

    let val = &doc["servers"][0]["host"];
    done(&format!("ValueIndex: servers[0].host = {val}"));

    let yaml = "1: one\n2: two\n3: three\n";
    let map: MappingAny = from_str(yaml).unwrap();
    done(&format!(
        "MappingAny: {} entries with numeric keys",
        map.len()
    ));

    let mut m = Mapping::new();
    let _ = m.insert("zebra", Value::from(1));
    let _ = m.insert("alpha", Value::from(2));
    m.sort_keys();
    done(&format!(
        "Mapping.sort_keys: first key = {:?}",
        m.first().unwrap().0
    ));

    println!("\n  \x1b[90mAll value manipulations verified.\x1b[0m\n");
}
