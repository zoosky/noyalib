//! YAML merge keys example for noyalib.
//!
//! Demonstrates the `apply_merge()` method for processing YAML merge keys
//! (`<<`). This is useful when working with YAML files that use anchors and
//! merge keys for DRY (Don't Repeat Yourself) configuration.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

#[path = "support.rs"]
mod support;

use noyalib::{from_str, to_string, Value};

fn main() {
    support::header("noyalib -- merge_keys");

    support::task_with_output("Basic merge key", || {
        let yaml = r#"
defaults: &defaults
  timeout: 30
  retries: 3
  logging: true

development:
  <<: *defaults
  debug: true
  timeout: 60

production:
  <<: *defaults
  debug: false
  replicas: 5
"#;

        let mut lines = vec!["Original YAML:".to_string()];
        lines.extend(yaml.trim().lines().map(|l| l.to_string()));
        lines.push(String::new());

        let mut value: Value = from_str(yaml).unwrap();

        lines.push("Before apply_merge:".to_string());
        lines.extend(to_string(&value).unwrap().lines().map(|l| l.to_string()));
        lines.push(String::new());

        value.apply_merge().unwrap();

        lines.push("After apply_merge:".to_string());
        lines.extend(to_string(&value).unwrap().lines().map(|l| l.to_string()));
        lines.push(String::new());

        lines.push("Verification:".to_string());
        lines.push(format!(
            "  development.timeout: {:?} (overridden from 30 to 60)",
            value
                .get_path("development.timeout")
                .and_then(|v| v.as_i64())
        ));
        lines.push(format!(
            "  development.retries: {:?} (inherited)",
            value
                .get_path("development.retries")
                .and_then(|v| v.as_i64())
        ));
        lines.push(format!(
            "  development.debug: {:?} (local)",
            value
                .get_path("development.debug")
                .and_then(|v| v.as_bool())
        ));
        lines.push(format!(
            "  production.replicas: {:?} (local)",
            value
                .get_path("production.replicas")
                .and_then(|v| v.as_i64())
        ));
        lines
    });

    support::task_with_output("Multiple merge sources", || {
        let yaml = r#"
base: &base
  adapter: postgres

connection: &connection
  host: localhost
  port: 5432

credentials: &credentials
  user: admin
  password: secret

database:
  <<: [*base, *connection, *credentials]
  database: myapp
"#;

        let mut lines = vec!["YAML with multiple merge sources:".to_string()];
        lines.extend(yaml.trim().lines().map(|l| l.to_string()));
        lines.push(String::new());

        let mut value: Value = from_str(yaml).unwrap();
        value.apply_merge().unwrap();

        lines.push("After apply_merge:".to_string());
        lines.extend(to_string(&value).unwrap().lines().map(|l| l.to_string()));
        lines.push(String::new());

        lines.push("Database config after merge:".to_string());
        if let Some(db) = value.get("database") {
            if let Some(map) = db.as_mapping() {
                for (k, v) in map.iter() {
                    lines.push(format!("  {k}: {v}"));
                }
            }
        }
        lines
    });

    support::task_with_output("Nested merge keys", || {
        let yaml = r#"
shared: &shared
  logging:
    level: info
    format: json

service_a:
  <<: *shared
  name: service-a
  logging:
    level: debug

service_b:
  <<: *shared
  name: service-b
"#;

        let mut lines = vec!["YAML with nested structures:".to_string()];
        lines.extend(yaml.trim().lines().map(|l| l.to_string()));
        lines.push(String::new());

        let mut value: Value = from_str(yaml).unwrap();
        value.apply_merge().unwrap();

        lines.push("After apply_merge:".to_string());
        lines.extend(to_string(&value).unwrap().lines().map(|l| l.to_string()));
        lines
    });

    support::task_with_output("Merge within sequences", || {
        let yaml = r#"
defaults: &defaults
  type: worker
  replicas: 1

services:
  - name: api
    <<: *defaults
    type: web
    replicas: 3
  - name: worker-1
    <<: *defaults
  - name: worker-2
    <<: *defaults
    replicas: 2
"#;

        let mut lines = vec!["YAML with merge keys in sequences:".to_string()];
        lines.extend(yaml.trim().lines().map(|l| l.to_string()));
        lines.push(String::new());

        let mut value: Value = from_str(yaml).unwrap();
        value.apply_merge().unwrap();

        lines.push("After apply_merge:".to_string());
        lines.extend(to_string(&value).unwrap().lines().map(|l| l.to_string()));
        lines
    });

    support::task_with_output("Merge precedence", || {
        let yaml = r#"
first: &first
  a: 1
  b: 2

second: &second
  b: 20
  c: 30

# In YAML merge, later keys in the list have LOWER precedence
# So 'first' values take precedence over 'second'
result:
  <<: [*first, *second]
  c: 300
"#;

        let mut lines = vec!["YAML demonstrating merge precedence:".to_string()];
        lines.extend(yaml.trim().lines().map(|l| l.to_string()));
        lines.push(String::new());

        let mut value: Value = from_str(yaml).unwrap();
        value.apply_merge().unwrap();

        lines.push("After apply_merge:".to_string());
        lines.extend(to_string(&value).unwrap().lines().map(|l| l.to_string()));
        lines.push(String::new());

        lines.push("Result values:".to_string());
        lines.push(format!(
            "  a: {:?} (from first)",
            value.get_path("result.a").and_then(|v| v.as_i64())
        ));
        lines.push(format!(
            "  b: {:?} (from first, not second)",
            value.get_path("result.b").and_then(|v| v.as_i64())
        ));
        lines.push(format!(
            "  c: {:?} (local override)",
            value.get_path("result.c").and_then(|v| v.as_i64())
        ));
        lines
    });

    support::summary(5);
}
