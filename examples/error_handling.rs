//! Error handling example for noyalib.
//!
//! Demonstrates error types and formatting error messages with source context.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use noyalib::{from_str, Error, Location, Value};
use serde::Deserialize;

fn main() {
    println!("noyalib error handling example\n");

    // Example 1: Parse error in YAML syntax
    println!("=== Parse error example ===");
    let invalid_yaml = r#"
name: test
  invalid indentation here
value: 42
"#;

    match from_str::<Value>(invalid_yaml) {
        Ok(_) => println!("Unexpectedly succeeded"),
        Err(e) => {
            println!("Error: {}", e);
            if let Some(loc) = e.location() {
                println!("Location: line {}, column {}", loc.line(), loc.column());
            }
        }
    }

    // Example 2: Type mismatch error
    println!("\n=== Type mismatch error ===");
    let yaml_with_wrong_type = "name: not_a_number\n";

    #[derive(Debug, Deserialize)]
    #[allow(dead_code)]
    struct TypedConfig {
        name: i32, // This expects an integer but we provide a string
    }

    match from_str::<TypedConfig>(yaml_with_wrong_type) {
        Ok(_) => println!("Unexpectedly succeeded"),
        Err(e) => {
            println!("Error: {}", e);
        }
    }

    // Example 3: Missing field error
    println!("\n=== Missing field error ===");
    let incomplete_yaml = "name: test\n";

    #[derive(Debug, Deserialize)]
    #[allow(dead_code)]
    struct RequiredFields {
        name: String,
        required_value: i32, // This field is missing in the YAML
    }

    match from_str::<RequiredFields>(incomplete_yaml) {
        Ok(_) => println!("Unexpectedly succeeded"),
        Err(e) => {
            println!("Error: {}", e);
        }
    }

    // Example 4: Creating errors programmatically
    println!("\n=== Programmatic error creation ===");

    // Create a parse error with location
    let source = "line1\nline2\nline3\n";
    let error = Error::parse_at("Invalid syntax", source, 6); // Position in "line2"

    println!("Created error: {}", error);
    if let Some(loc) = error.location() {
        println!("Location: line {}, column {}", loc.line(), loc.column());
    }

    // Format error with source context
    let formatted = error.format_with_source(source);
    println!("\nFormatted error:\n{}", formatted);

    // Example 5: Location calculations
    println!("\n=== Location calculations ===");
    let multiline = "first line\nsecond line\nthird line\n";

    let loc1 = Location::from_index(multiline, 0);
    println!("Index 0: line {}, column {}", loc1.line(), loc1.column());

    let loc2 = Location::from_index(multiline, 11); // Start of "second"
    println!("Index 11: line {}, column {}", loc2.line(), loc2.column());

    let loc3 = Location::from_index(multiline, 23); // Start of "third"
    println!("Index 23: line {}, column {}", loc3.line(), loc3.column());

    // Example 6: Error types
    println!("\n=== Error type matching ===");
    let errors = vec![
        Error::Parse("parse error".to_string()),
        Error::Serialize("serialize error".to_string()),
        Error::Deserialize("deserialize error".to_string()),
        Error::Invalid("invalid error".to_string()),
        Error::TypeMismatch {
            expected: "string",
            found: "integer".to_string(),
        },
        Error::MissingField("name".to_string()),
        Error::Custom("custom error".to_string()),
    ];

    for error in errors {
        let kind = match &error {
            Error::Parse(_) => "Parse",
            Error::ParseWithLocation { .. } => "ParseWithLocation",
            Error::Serialize(_) => "Serialize",
            Error::Deserialize(_) => "Deserialize",
            Error::DeserializeWithLocation { .. } => "DeserializeWithLocation",
            Error::Invalid(_) => "Invalid",
            Error::TypeMismatch { .. } => "TypeMismatch",
            Error::MissingField(_) => "MissingField",
            Error::UnknownField(_) => "UnknownField",
            Error::RecursionLimitExceeded { .. } => "RecursionLimitExceeded",
            Error::Custom(_) => "Custom",
            Error::Io(_) => "Io",
            // New error types added in Phase 2
            Error::RepetitionLimitExceeded => "RepetitionLimitExceeded",
            Error::UnknownAnchor(_) => "UnknownAnchor",
            Error::ScalarInMerge => "ScalarInMerge",
            Error::TaggedInMerge => "TaggedInMerge",
            Error::ScalarInMergeElement => "ScalarInMergeElement",
            Error::SequenceInMergeElement => "SequenceInMergeElement",
            Error::EmptyTag => "EmptyTag",
            Error::FailedToParseNumber(_) => "FailedToParseNumber",
            Error::EndOfStream => "EndOfStream",
            Error::MoreThanOneDocument => "MoreThanOneDocument",
            // Wildcard for future error types (enum is non_exhaustive)
            _ => "Unknown",
        };
        println!("{}: {}", kind, error);
    }

    // Example 7: Graceful error handling pattern
    println!("\n=== Graceful error handling pattern ===");
    let yaml_sources = vec![
        ("valid", "name: test\nvalue: 42\n"),
        ("invalid", "name: test\n  bad indent\n"),
        ("empty", ""),
    ];

    for (label, yaml) in yaml_sources {
        print!("Parsing '{}': ", label);
        match from_str::<Value>(yaml) {
            Ok(value) => println!("Success - {:?}", value),
            Err(Error::Parse(msg)) => println!("Parse error - {}", msg),
            Err(Error::ParseWithLocation { message, location }) => {
                println!(
                    "Parse error at line {}, col {}: {}",
                    location.line(),
                    location.column(),
                    message
                );
            }
            Err(e) => println!("Other error - {}", e),
        }
    }

    println!("\nError handling example completed!");
}
