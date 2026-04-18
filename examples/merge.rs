//! Value merge example for noyalib.
//!
//! Demonstrates merging YAML values together, useful for configuration
//! layering.

use noyalib::{from_str, to_string, Value};

fn main() -> Result<(), noyalib::Error> {
    println!("noyalib merge example\n");

    // Base configuration
    let base_yaml = r#"
server:
  host: localhost
  port: 8080
  timeout: 30
database:
  host: localhost
  port: 5432
logging:
  level: info
  format: text
"#;

    // Override configuration (e.g., production settings)
    let override_yaml = r#"
server:
  host: prod.example.com
  ssl: true
database:
  host: db.example.com
  pool_size: 20
logging:
  level: warn
"#;

    let mut base: Value = from_str(base_yaml)?;
    let overrides: Value = from_str(override_yaml)?;

    println!("=== Base configuration ===");
    println!("{}", to_string(&base)?);

    println!("\n=== Override configuration ===");
    println!("{}", to_string(&overrides)?);

    // Merge overrides into base
    base.merge(overrides);

    println!("\n=== Merged configuration ===");
    println!("{}", to_string(&base)?);

    // Verify merged values
    println!("\n=== Verification ===");
    println!(
        "server.host: {:?}",
        base.get_path("server.host").and_then(|v| v.as_str())
    );
    println!(
        "server.port: {:?}",
        base.get_path("server.port").and_then(|v| v.as_i64())
    );
    println!(
        "server.timeout: {:?}",
        base.get_path("server.timeout").and_then(|v| v.as_i64())
    );
    println!(
        "server.ssl: {:?}",
        base.get_path("server.ssl").and_then(|v| v.as_bool())
    );
    println!(
        "database.pool_size: {:?}",
        base.get_path("database.pool_size").and_then(|v| v.as_i64())
    );
    println!(
        "logging.level: {:?}",
        base.get_path("logging.level").and_then(|v| v.as_str())
    );

    // Demonstrate merge_concat for sequences
    println!("\n=== Sequence merge (concat) ===");
    let mut list1: Value = from_str("items:\n  - a\n  - b\n")?;
    let list2: Value = from_str("items:\n  - c\n  - d\n")?;

    println!("List 1: {:?}", list1);
    println!("List 2: {:?}", list2);

    list1.merge_concat(list2);
    println!("After merge_concat: {:?}", list1);

    // Demonstrate merge vs replace for sequences
    println!("\n=== Sequence merge (replace) ===");
    let mut base_seq: Value = from_str("tags:\n  - old1\n  - old2\n")?;
    let new_seq: Value = from_str("tags:\n  - new1\n  - new2\n  - new3\n")?;

    println!("Before: {:?}", base_seq.get("tags"));
    base_seq.merge(new_seq);
    println!("After merge: {:?}", base_seq.get("tags"));

    // Demonstrate programmatic value modification
    println!("\n=== Programmatic value modification ===");
    let mut config: Value = from_str("settings:\n  timeout: 30\n")?;

    // Insert new values
    if let Some(settings) = config.get_mut("settings") {
        if let Some(map) = settings.as_mapping_mut() {
            let _ = map.insert("retries".to_string(), Value::from(3));
            let _ = map.insert("debug".to_string(), Value::from(false));
        }
    }

    // Insert at root level
    let _ = config.insert("version", Value::from("1.0.0"));

    println!("Modified config:\n{}", to_string(&config)?);

    println!("\nMerge example completed successfully!");

    Ok(())
}
