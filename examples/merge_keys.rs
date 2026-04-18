//! YAML merge keys example for noyalib.
//!
//! Demonstrates the `apply_merge()` method for processing YAML merge keys
//! (`<<`). This is useful when working with YAML files that use anchors and
//! merge keys for DRY (Don't Repeat Yourself) configuration.

use noyalib::{from_str, to_string, Value};

fn main() -> Result<(), noyalib::Error> {
    println!("noyalib merge keys example\n");

    // =========================================================================
    // Example 1: Basic merge key usage
    // =========================================================================
    println!("=== Example 1: Basic merge key ===\n");

    // YAML with anchors and merge keys
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

    println!("Original YAML:");
    println!("{}", yaml);

    let mut value: Value = from_str(yaml)?;

    // Before apply_merge, the merge key is present as a literal
    println!("Before apply_merge:");
    println!("{}\n", to_string(&value)?);

    // Apply merge to expand all merge keys
    value.apply_merge()?;

    println!("After apply_merge:");
    println!("{}", to_string(&value)?);

    // Verify values are merged correctly
    println!("Verification:");
    println!(
        "  development.timeout: {:?} (overridden from 30 to 60)",
        value
            .get_path("development.timeout")
            .and_then(|v| v.as_i64())
    );
    println!(
        "  development.retries: {:?} (inherited)",
        value
            .get_path("development.retries")
            .and_then(|v| v.as_i64())
    );
    println!(
        "  development.debug: {:?} (local)",
        value
            .get_path("development.debug")
            .and_then(|v| v.as_bool())
    );
    println!(
        "  production.replicas: {:?} (local)",
        value
            .get_path("production.replicas")
            .and_then(|v| v.as_i64())
    );

    // =========================================================================
    // Example 2: Multiple merge sources
    // =========================================================================
    println!("\n=== Example 2: Multiple merge sources ===\n");

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

    println!("YAML with multiple merge sources:");
    println!("{}", yaml);

    let mut value: Value = from_str(yaml)?;
    value.apply_merge()?;

    println!("After apply_merge:");
    println!("{}", to_string(&value)?);

    // Show all merged values
    println!("Database config after merge:");
    if let Some(db) = value.get("database") {
        if let Some(map) = db.as_mapping() {
            for (k, v) in map.iter() {
                println!("  {}: {}", k, v);
            }
        }
    }

    // =========================================================================
    // Example 3: Nested merge keys
    // =========================================================================
    println!("\n=== Example 3: Nested merge keys ===\n");

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

    println!("YAML with nested structures:");
    println!("{}", yaml);

    let mut value: Value = from_str(yaml)?;
    value.apply_merge()?;

    println!("After apply_merge:");
    println!("{}", to_string(&value)?);

    // =========================================================================
    // Example 4: Merge in sequences
    // =========================================================================
    println!("\n=== Example 4: Merge within sequences ===\n");

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

    println!("YAML with merge keys in sequences:");
    println!("{}", yaml);

    let mut value: Value = from_str(yaml)?;
    value.apply_merge()?;

    println!("After apply_merge:");
    println!("{}", to_string(&value)?);

    // =========================================================================
    // Example 5: Merge precedence
    // =========================================================================
    println!("\n=== Example 5: Merge precedence ===\n");

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

    println!("YAML demonstrating merge precedence:");
    println!("{}", yaml);

    let mut value: Value = from_str(yaml)?;
    value.apply_merge()?;

    println!("After apply_merge:");
    println!("{}", to_string(&value)?);

    println!("Result values:");
    println!(
        "  a: {:?} (from first)",
        value.get_path("result.a").and_then(|v| v.as_i64())
    );
    println!(
        "  b: {:?} (from first, not second)",
        value.get_path("result.b").and_then(|v| v.as_i64())
    );
    println!(
        "  c: {:?} (local override)",
        value.get_path("result.c").and_then(|v| v.as_i64())
    );

    println!("\nMerge keys example completed successfully!");

    Ok(())
}
