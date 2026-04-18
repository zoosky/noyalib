//! Enum serialization example for noyalib.
//!
//! Demonstrates serializing various enum types.

use noyalib::{from_str, to_string};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
enum Shape {
    Rectangle { width: u32, height: u32 },
    Circle { radius: f64 },
    Triangle { base: u32, height: u32 },
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
enum Status {
    Active,
    Inactive,
    Pending,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
enum Message {
    Text(String),
    Number(i32),
    Data { id: u32, payload: String },
}

fn main() -> Result<(), noyalib::Error> {
    println!("noyalib enums example\n");

    // Struct variants
    let shapes = vec![
        Shape::Rectangle {
            width: 10,
            height: 20,
        },
        Shape::Circle { radius: 5.0 },
        Shape::Triangle {
            base: 8,
            height: 12,
        },
    ];

    let yaml = to_string(&shapes)?;
    println!("Shapes serialized:\n{}\n", yaml);

    let parsed: Vec<Shape> = from_str(&yaml)?;
    assert_eq!(shapes, parsed);

    // Unit variants
    let status = Status::Active;
    let yaml = to_string(&status)?;
    println!("Status serialized: {}", yaml);

    let parsed: Status = from_str(&yaml)?;
    assert_eq!(status, parsed);

    // Mixed variants
    let messages = vec![
        Message::Text("Hello".to_string()),
        Message::Number(42),
        Message::Data {
            id: 1,
            payload: "test".to_string(),
        },
    ];

    let yaml = to_string(&messages)?;
    println!("\nMessages serialized:\n{}\n", yaml);

    let parsed: Vec<Message> = from_str(&yaml)?;
    assert_eq!(messages, parsed);

    println!("All enum tests passed!");

    Ok(())
}
