//! Singleton map serialization example for noyalib.
//!
//! Demonstrates using the singleton_map module for enum serialization.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use noyalib::{from_str, to_string};
use serde::{Deserialize, Serialize};

// Enum with various variant types
#[derive(Debug, Serialize, Deserialize, PartialEq)]
enum Status {
    Pending,
    Active,
    Completed { at: String },
    Error(String),
}

// Container using singleton_map for cleaner YAML
#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Task {
    name: String,
    #[serde(with = "noyalib::with::singleton_map")]
    status: Status,
}

// Container with optional status using singleton_map_optional
#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct OptionalTask {
    name: String,
    #[serde(
        with = "noyalib::with::singleton_map_optional",
        skip_serializing_if = "Option::is_none",
        default
    )]
    status: Option<Status>,
}

// Nested enum structure using singleton_map_recursive
#[derive(Debug, Serialize, Deserialize, PartialEq)]
enum Action {
    Simple,
    WithData { value: i32 },
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Workflow {
    name: String,
    #[serde(with = "noyalib::with::singleton_map_recursive")]
    actions: Vec<Action>,
}

fn main() -> Result<(), noyalib::Error> {
    println!("noyalib singleton_map example\n");

    // Example 1: Basic singleton_map usage
    println!("=== Basic singleton_map ===");

    let task_pending = Task {
        name: "Task 1".to_string(),
        status: Status::Pending,
    };

    let task_active = Task {
        name: "Task 2".to_string(),
        status: Status::Active,
    };

    let task_completed = Task {
        name: "Task 3".to_string(),
        status: Status::Completed {
            at: "2024-01-15".to_string(),
        },
    };

    let task_error = Task {
        name: "Task 4".to_string(),
        status: Status::Error("Connection failed".to_string()),
    };

    println!("Pending task:");
    println!("{}", to_string(&task_pending)?);

    println!("Active task:");
    println!("{}", to_string(&task_active)?);

    println!("Completed task:");
    println!("{}", to_string(&task_completed)?);

    println!("Error task:");
    println!("{}", to_string(&task_error)?);

    // Example 2: Roundtrip verification
    println!("\n=== Roundtrip verification ===");
    let yaml = to_string(&task_completed)?;
    let parsed: Task = from_str(&yaml)?;
    println!("Original: {:?}", task_completed);
    println!("Parsed:   {:?}", parsed);
    assert_eq!(task_completed, parsed);
    println!("Roundtrip successful!");

    // Example 3: Optional singleton_map
    println!("\n=== Optional singleton_map ===");

    let with_status = OptionalTask {
        name: "Has status".to_string(),
        status: Some(Status::Active),
    };

    let without_status = OptionalTask {
        name: "No status".to_string(),
        status: None,
    };

    println!("With status:");
    println!("{}", to_string(&with_status)?);

    println!("Without status:");
    println!("{}", to_string(&without_status)?);

    // Example 4: Recursive singleton_map
    println!("\n=== Recursive singleton_map ===");

    let workflow = Workflow {
        name: "My Workflow".to_string(),
        actions: vec![
            Action::Simple,
            Action::WithData { value: 42 },
            Action::Simple,
        ],
    };

    println!("Workflow:");
    println!("{}", to_string(&workflow)?);

    // Verify roundtrip
    let yaml = to_string(&workflow)?;
    let parsed: Workflow = from_str(&yaml)?;
    assert_eq!(workflow, parsed);
    println!("Recursive roundtrip successful!");

    // Example 5: Parsing YAML with singleton map format
    println!("\n=== Parsing singleton map format ===");
    let yaml_input = r#"
name: Parsed Task
status:
  Completed:
    at: "2024-12-01"
"#;

    let parsed: Task = from_str(yaml_input)?;
    println!("Parsed task: {:?}", parsed);

    println!("\nSingleton map example completed successfully!");

    Ok(())
}
