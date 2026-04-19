// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Shared data with smart pointer anchors: RcAnchor, ArcAnchor.
//!
//! Run: `cargo run --example shared_anchors`

use std::sync::Arc;

use noyalib::{from_str, to_string, ArcAnchor, RcAnchor, Value};

fn done(msg: &str) {
    println!("  \x1b[32m+\x1b[0m {msg}");
}

fn main() {
    println!("\n  \x1b[1mnoyalib shared anchors\x1b[0m\n");

    let shared: RcAnchor<String> = RcAnchor::from("shared-config".to_string());
    let _ = to_string(&shared).unwrap();
    done("RcAnchor: serialize via Rc<T>");

    let shared: ArcAnchor<i64> = ArcAnchor::from(42i64);
    let _ = to_string(&shared).unwrap();
    done("ArcAnchor: serialize via Arc<T>");

    let yaml = r#"
defaults: &defaults
  adapter: postgres
  host: localhost

development:
  <<: *defaults
  database: dev_db

production:
  <<: *defaults
  database: prod_db
  host: db.example.com
"#;
    let config: Value = from_str(yaml).unwrap();
    assert_eq!(
        config["development"]["adapter"],
        Value::String("postgres".to_string())
    );
    done("anchor/alias merge: development inherits defaults");

    assert_eq!(
        config["production"]["host"],
        Value::String("db.example.com".to_string())
    );
    done("anchor/alias merge: production overrides host");

    let data = Arc::new("thread-safe".to_string());
    let anchor: ArcAnchor<String> = ArcAnchor::from(data.clone());
    assert_eq!(*anchor, "thread-safe");
    done(&format!(
        "ArcAnchor from Arc: strong_count={}",
        Arc::strong_count(&data)
    ));

    println!("\n  \x1b[90mAll shared anchor patterns verified.\x1b[0m\n");
}
