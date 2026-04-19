// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Value manipulation: to_value, from_value, MappingAny, get_path_mut, ValueIndex.
//!
//! Run: `cargo run --example value_manipulation`

#[path = "support.rs"]
mod support;

use noyalib::{from_str, from_value, to_value, Mapping, MappingAny, Value};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Server {
    host: String,
    port: u16,
}

fn main() {
    support::header("noyalib -- value_manipulation");

    let server = Server {
        host: "localhost".to_string(),
        port: 8080,
    };

    support::task("to_value: struct -> Value", || {
        let _value = to_value(&server).unwrap();
    });

    let value = to_value(&server).unwrap();

    support::task("from_value: Value -> struct", || {
        let parsed: Server = from_value(&value).unwrap();
        assert_eq!(parsed, server);
    });

    let yaml = "servers:\n  - host: alpha\n    port: 80\n  - host: beta\n    port: 443\n";

    support::task("get_path: servers[1].host", || {
        let doc: Value = from_str(yaml).unwrap();
        let _host = doc.get_path("servers[1].host").unwrap();
    });

    support::task("get_path_mut: change port to 9090", || {
        let mut doc: Value = from_str(yaml).unwrap();
        if let Some(port) = doc.get_path_mut("servers[0].port") {
            *port = Value::from(9090);
        }
        assert_eq!(doc.get_path("servers[0].port").unwrap(), &Value::from(9090));
    });

    support::task("ValueIndex: servers[0].host", || {
        let doc: Value = from_str(yaml).unwrap();
        let _val = &doc["servers"][0]["host"];
    });

    support::task("MappingAny: numeric keys", || {
        let yaml = "1: one\n2: two\n3: three\n";
        let map: MappingAny = from_str(yaml).unwrap();
        assert_eq!(map.len(), 3);
    });

    support::task("Mapping.sort_keys", || {
        let mut m = Mapping::new();
        let _ = m.insert("zebra", Value::from(1));
        let _ = m.insert("alpha", Value::from(2));
        m.sort_keys();
        assert_eq!(m.first().unwrap().0, "alpha");
    });

    support::summary(7);
}
