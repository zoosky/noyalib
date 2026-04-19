// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Shared data with smart pointer anchors: RcAnchor, ArcAnchor.
//!
//! Run: `cargo run --example shared_anchors`

#[path = "support.rs"]
mod support;

use std::sync::Arc;

use noyalib::{from_str, to_string, ArcAnchor, RcAnchor, Value};

fn main() {
    support::header("noyalib -- shared_anchors");

    support::task("RcAnchor: serialize via Rc<T>", || {
        let shared: RcAnchor<String> = RcAnchor::from("shared-config".to_string());
        let _ = to_string(&shared).unwrap();
    });

    support::task("ArcAnchor: serialize via Arc<T>", || {
        let shared: ArcAnchor<i64> = ArcAnchor::from(42i64);
        let _ = to_string(&shared).unwrap();
    });

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

    support::task("anchor/alias merge: development inherits defaults", || {
        assert_eq!(
            config["development"]["adapter"],
            Value::String("postgres".to_string())
        );
    });

    support::task("anchor/alias merge: production overrides host", || {
        assert_eq!(
            config["production"]["host"],
            Value::String("db.example.com".to_string())
        );
    });

    support::task("ArcAnchor from Arc", || {
        let data = Arc::new("thread-safe".to_string());
        let anchor: ArcAnchor<String> = ArcAnchor::from(data.clone());
        assert_eq!(*anchor, "thread-safe");
        assert_eq!(Arc::strong_count(&data), 2);
    });

    support::summary(5);
}
