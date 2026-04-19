//! Singleton map serialization example for noyalib.
//!
//! Demonstrates using the singleton_map module for enum serialization.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

#[path = "support.rs"]
mod support;

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

fn main() {
    support::header("noyalib -- singleton_map");

    support::task_with_output("Basic singleton_map usage", || {
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

        let mut lines = vec!["Pending task:".to_string()];
        lines.extend(
            to_string(&task_pending)
                .unwrap()
                .lines()
                .map(|l| l.to_string()),
        );
        lines.push("Active task:".to_string());
        lines.extend(
            to_string(&task_active)
                .unwrap()
                .lines()
                .map(|l| l.to_string()),
        );
        lines.push("Completed task:".to_string());
        lines.extend(
            to_string(&task_completed)
                .unwrap()
                .lines()
                .map(|l| l.to_string()),
        );
        lines.push("Error task:".to_string());
        lines.extend(
            to_string(&task_error)
                .unwrap()
                .lines()
                .map(|l| l.to_string()),
        );
        lines
    });

    support::task_with_output("Roundtrip verification", || {
        let task_completed = Task {
            name: "Task 3".to_string(),
            status: Status::Completed {
                at: "2024-01-15".to_string(),
            },
        };

        let yaml = to_string(&task_completed).unwrap();
        let parsed: Task = from_str(&yaml).unwrap();
        assert_eq!(task_completed, parsed);

        vec![
            format!("Original: {task_completed:?}"),
            format!("Parsed:   {parsed:?}"),
            "Roundtrip successful!".to_string(),
        ]
    });

    support::task_with_output("Optional singleton_map", || {
        let with_status = OptionalTask {
            name: "Has status".to_string(),
            status: Some(Status::Active),
        };

        let without_status = OptionalTask {
            name: "No status".to_string(),
            status: None,
        };

        let mut lines = vec!["With status:".to_string()];
        lines.extend(
            to_string(&with_status)
                .unwrap()
                .lines()
                .map(|l| l.to_string()),
        );
        lines.push("Without status:".to_string());
        lines.extend(
            to_string(&without_status)
                .unwrap()
                .lines()
                .map(|l| l.to_string()),
        );
        lines
    });

    support::task_with_output("Recursive singleton_map", || {
        let workflow = Workflow {
            name: "My Workflow".to_string(),
            actions: vec![
                Action::Simple,
                Action::WithData { value: 42 },
                Action::Simple,
            ],
        };

        let mut lines = vec!["Workflow:".to_string()];
        lines.extend(to_string(&workflow).unwrap().lines().map(|l| l.to_string()));

        let yaml = to_string(&workflow).unwrap();
        let parsed: Workflow = from_str(&yaml).unwrap();
        assert_eq!(workflow, parsed);
        lines.push("Recursive roundtrip successful!".to_string());
        lines
    });

    support::task_with_output("Parsing singleton map format", || {
        let yaml_input = r#"
name: Parsed Task
status:
  Completed:
    at: "2024-12-01"
"#;

        let parsed: Task = from_str(yaml_input).unwrap();
        vec![format!("Parsed task: {parsed:?}")]
    });

    support::summary(5);
}
