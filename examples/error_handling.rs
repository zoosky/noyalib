//! Error handling example for noyalib.
//!
//! Demonstrates error types and formatting error messages with source context.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

#[path = "support.rs"]
mod support;

use noyalib::{from_str, Error, Location, Value};
use serde::Deserialize;

fn main() {
    support::header("noyalib -- error_handling");

    support::task_with_output("Parse error in YAML syntax", || {
        let invalid_yaml = r#"
name: test
  invalid indentation here
value: 42
"#;

        match from_str::<Value>(invalid_yaml) {
            Ok(_) => vec!["Unexpectedly succeeded".to_string()],
            Err(e) => {
                let mut lines = vec![format!("Error: {e}")];
                if let Some(loc) = e.location() {
                    lines.push(format!(
                        "Location: line {}, column {}",
                        loc.line(),
                        loc.column()
                    ));
                }
                lines
            }
        }
    });

    support::task_with_output("Type mismatch error", || {
        let yaml_with_wrong_type = "name: not_a_number\n";

        #[derive(Debug, Deserialize)]
        #[allow(dead_code)]
        struct TypedConfig {
            name: i32,
        }

        match from_str::<TypedConfig>(yaml_with_wrong_type) {
            Ok(_) => vec!["Unexpectedly succeeded".to_string()],
            Err(e) => vec![format!("Error: {e}")],
        }
    });

    support::task_with_output("Missing field error", || {
        let incomplete_yaml = "name: test\n";

        #[derive(Debug, Deserialize)]
        #[allow(dead_code)]
        struct RequiredFields {
            name: String,
            required_value: i32,
        }

        match from_str::<RequiredFields>(incomplete_yaml) {
            Ok(_) => vec!["Unexpectedly succeeded".to_string()],
            Err(e) => vec![format!("Error: {e}")],
        }
    });

    support::task_with_output("Programmatic error creation", || {
        let source = "line1\nline2\nline3\n";
        let error = Error::parse_at("Invalid syntax", source, 6);

        let mut lines = vec![format!("Created error: {error}")];
        if let Some(loc) = error.location() {
            lines.push(format!(
                "Location: line {}, column {}",
                loc.line(),
                loc.column()
            ));
        }

        let formatted = error.format_with_source(source);
        lines.push(String::new());
        lines.push("Formatted error:".to_string());
        lines.extend(formatted.lines().map(|l| l.to_string()));
        lines
    });

    support::task_with_output("Location calculations", || {
        let multiline = "first line\nsecond line\nthird line\n";

        let loc1 = Location::from_index(multiline, 0);
        let loc2 = Location::from_index(multiline, 11);
        let loc3 = Location::from_index(multiline, 23);

        vec![
            format!("Index 0: line {}, column {}", loc1.line(), loc1.column()),
            format!("Index 11: line {}, column {}", loc2.line(), loc2.column()),
            format!("Index 23: line {}, column {}", loc3.line(), loc3.column()),
        ]
    });

    support::task_with_output("Error type matching", || {
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

        errors
            .into_iter()
            .map(|error| {
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
                    _ => "Unknown",
                };
                format!("{kind}: {error}")
            })
            .collect()
    });

    support::task_with_output("Graceful error handling pattern", || {
        let yaml_sources = vec![
            ("valid", "name: test\nvalue: 42\n"),
            ("invalid", "name: test\n  bad indent\n"),
            ("empty", ""),
        ];

        yaml_sources
            .into_iter()
            .map(|(label, yaml)| match from_str::<Value>(yaml) {
                Ok(value) => format!("Parsing '{label}': Success - {value:?}"),
                Err(Error::Parse(msg)) => format!("Parsing '{label}': Parse error - {msg}"),
                Err(Error::ParseWithLocation {
                    message, location, ..
                }) => {
                    format!(
                        "Parsing '{label}': Parse error at line {}, col {}: {message}",
                        location.line(),
                        location.column(),
                    )
                }
                Err(e) => format!("Parsing '{label}': Other error - {e}"),
            })
            .collect()
    });

    support::summary(7);
}
