//! Basic usage example for noyalib.
//!
//! Demonstrates simple serialization and deserialization of structs.

use noyalib::{from_str, to_string};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Person {
    name: String,
    age: u32,
    city: String,
}

fn main() -> Result<(), noyalib::Error> {
    println!("noyalib basic example\n");

    // Create a person
    let person = Person {
        name: "John Doe".to_string(),
        age: 30,
        city: "New York".to_string(),
    };

    // Serialize to YAML
    let yaml = to_string(&person)?;
    println!("Serialized YAML:\n{}\n", yaml);

    // Deserialize back
    let parsed: Person = from_str(&yaml)?;
    println!("Deserialized: {:?}\n", parsed);

    // Verify round-trip
    assert_eq!(person, parsed);
    println!("Round-trip successful!");

    Ok(())
}
