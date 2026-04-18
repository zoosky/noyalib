//! Collection serialization example for noyalib.
//!
//! Demonstrates serializing Vec, HashMap, and other collections.

use std::collections::HashMap;

use noyalib::{from_str, to_string};
use serde::{Deserialize, Serialize};

fn main() -> Result<(), noyalib::Error> {
    println!("noyalib collections example\n");

    // Vec serialization
    let numbers = vec![1, 2, 3, 4, 5];
    let yaml = to_string(&numbers)?;
    println!("Vec serialized:\n{}\n", yaml);

    let parsed: Vec<i32> = from_str(&yaml)?;
    assert_eq!(numbers, parsed);

    // HashMap serialization
    let mut map = HashMap::new();
    let _ = map.insert("key1".to_string(), "value1".to_string());
    let _ = map.insert("key2".to_string(), "value2".to_string());
    let _ = map.insert("key3".to_string(), "value3".to_string());

    let yaml = to_string(&map)?;
    println!("HashMap serialized:\n{}\n", yaml);

    // Nested collections
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Config {
        name: String,
        tags: Vec<String>,
        settings: HashMap<String, i32>,
    }

    let config = Config {
        name: "my-app".to_string(),
        tags: vec!["rust".to_string(), "yaml".to_string(), "safe".to_string()],
        settings: {
            let mut s = HashMap::new();
            let _ = s.insert("timeout".to_string(), 30);
            let _ = s.insert("retries".to_string(), 3);
            s
        },
    };

    let yaml = to_string(&config)?;
    println!("Nested config serialized:\n{}\n", yaml);

    let parsed: Config = from_str(&yaml)?;
    assert_eq!(config, parsed);
    println!("All collection tests passed!");

    Ok(())
}
