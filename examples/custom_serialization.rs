//! Custom serialization example for noyalib.
//!
//! Demonstrates the `singleton_map_with` module for custom key transformations
//! when serializing enums. This allows you to control how enum variant names
//! appear in the YAML output.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use noyalib::{from_str, to_string};
use serde::{Deserialize, Serialize};

// =============================================================================
// Example 1: Snake case transformation
// =============================================================================

/// Custom serialization module for snake_case keys
mod snake_case_keys {
    use noyalib::with::singleton_map_with;
    use serde::{Deserializer, Serializer};

    pub(super) fn serialize<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        T: serde::Serialize,
        S: Serializer,
    {
        singleton_map_with::serialize_with(value, serializer, singleton_map_with::to_snake_case)
    }

    pub(super) fn deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
    where
        T: serde::de::DeserializeOwned,
        D: Deserializer<'de>,
    {
        singleton_map_with::deserialize_with(deserializer, singleton_map_with::to_pascal_case)
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
enum HttpMethod {
    GetRequest,
    PostData,
    PutResource,
    DeleteItem,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct ApiEndpoint {
    path: String,
    #[serde(with = "snake_case_keys")]
    method: HttpMethod,
}

// =============================================================================
// Example 2: Kebab case transformation
// =============================================================================

/// Custom serialization module for kebab-case keys
mod kebab_case_keys {
    use noyalib::with::singleton_map_with;
    use serde::{Deserializer, Serializer};

    pub(super) fn serialize<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        T: serde::Serialize,
        S: Serializer,
    {
        singleton_map_with::serialize_with(value, serializer, singleton_map_with::to_kebab_case)
    }

    pub(super) fn deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
    where
        T: serde::de::DeserializeOwned,
        D: Deserializer<'de>,
    {
        singleton_map_with::deserialize_with(deserializer, singleton_map_with::from_kebab_case)
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
enum LogLevel {
    TraceVerbose,
    DebugInfo,
    InfoStandard,
    WarnAlert,
    ErrorCritical,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct LogConfig {
    name: String,
    #[serde(with = "kebab_case_keys")]
    level: LogLevel,
}

// =============================================================================
// Example 3: Lowercase transformation
// =============================================================================

/// Custom serialization module for lowercase keys
mod lowercase_keys {
    use noyalib::with::singleton_map_with;
    use serde::{Deserializer, Serializer};

    pub(super) fn serialize<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        T: serde::Serialize,
        S: Serializer,
    {
        singleton_map_with::serialize_with(value, serializer, singleton_map_with::to_lowercase)
    }

    pub(super) fn deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
    where
        T: serde::de::DeserializeOwned,
        D: Deserializer<'de>,
    {
        singleton_map_with::deserialize_with(deserializer, singleton_map_with::to_uppercase)
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[allow(clippy::upper_case_acronyms)]
enum Environment {
    DEVELOPMENT,
    STAGING,
    PRODUCTION,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct DeployConfig {
    app_name: String,
    #[serde(with = "lowercase_keys")]
    environment: Environment,
}

// =============================================================================
// Example 4: Custom prefix transformation
// =============================================================================

/// Custom serialization module that adds a prefix
mod prefixed_keys {
    use serde::{Deserializer, Serializer};

    fn add_prefix(s: &str) -> String {
        // Convert PascalCase to snake_case with prefix
        let mut result = String::from("action");
        for c in s.chars() {
            if c.is_uppercase() {
                result.push('_');
                result.push(c.to_lowercase().next().unwrap_or(c));
            } else {
                result.push(c);
            }
        }
        result
    }

    fn remove_prefix(s: &str) -> String {
        // Remove "action_" prefix and convert snake_case to PascalCase
        let without_prefix = s.strip_prefix("action_").unwrap_or(s);
        without_prefix
            .split('_')
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    Some(first) => {
                        let upper: String = first.to_uppercase().collect();
                        let rest: String = chars.collect();
                        upper + &rest
                    }
                    None => String::new(),
                }
            })
            .collect()
    }

    pub(super) fn serialize<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        T: serde::Serialize,
        S: Serializer,
    {
        noyalib::with::singleton_map_with::serialize_with(value, serializer, add_prefix)
    }

    pub(super) fn deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
    where
        T: serde::de::DeserializeOwned,
        D: Deserializer<'de>,
    {
        noyalib::with::singleton_map_with::deserialize_with(deserializer, remove_prefix)
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
enum TaskAction {
    StartProcess,
    StopProcess,
    RestartService,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Task {
    name: String,
    #[serde(with = "prefixed_keys")]
    action: TaskAction,
}

fn main() -> Result<(), noyalib::Error> {
    println!("noyalib custom serialization example\n");

    // =========================================================================
    // Example 1: Snake case
    // =========================================================================
    println!("=== Example 1: Snake case transformation ===\n");

    let endpoint = ApiEndpoint {
        path: "/api/users".to_string(),
        method: HttpMethod::GetRequest,
    };

    let yaml = to_string(&endpoint)?;
    println!("Serialized (snake_case):");
    println!("{}", yaml);

    // Deserialize back
    let parsed: ApiEndpoint = from_str(&yaml)?;
    println!("Deserialized: {:?}", parsed);
    assert_eq!(endpoint, parsed);

    // =========================================================================
    // Example 2: Kebab case
    // =========================================================================
    println!("\n=== Example 2: Kebab case transformation ===\n");

    let log_config = LogConfig {
        name: "my-app".to_string(),
        level: LogLevel::DebugInfo,
    };

    let yaml = to_string(&log_config)?;
    println!("Serialized (kebab-case):");
    println!("{}", yaml);

    let parsed: LogConfig = from_str(&yaml)?;
    println!("Deserialized: {:?}", parsed);
    assert_eq!(log_config, parsed);

    // =========================================================================
    // Example 3: Lowercase
    // =========================================================================
    println!("\n=== Example 3: Lowercase transformation ===\n");

    let deploy = DeployConfig {
        app_name: "my-service".to_string(),
        environment: Environment::PRODUCTION,
    };

    let yaml = to_string(&deploy)?;
    println!("Serialized (lowercase):");
    println!("{}", yaml);

    let parsed: DeployConfig = from_str(&yaml)?;
    println!("Deserialized: {:?}", parsed);
    assert_eq!(deploy, parsed);

    // =========================================================================
    // Example 4: Custom prefix
    // =========================================================================
    println!("\n=== Example 4: Custom prefix transformation ===\n");

    let task = Task {
        name: "deploy-task".to_string(),
        action: TaskAction::RestartService,
    };

    let yaml = to_string(&task)?;
    println!("Serialized (with action_ prefix):");
    println!("{}", yaml);

    let parsed: Task = from_str(&yaml)?;
    println!("Deserialized: {:?}", parsed);
    assert_eq!(task, parsed);

    // =========================================================================
    // Example 5: Multiple enum values
    // =========================================================================
    println!("\n=== Example 5: Multiple endpoints ===\n");

    let endpoints = vec![
        ApiEndpoint {
            path: "/api/users".to_string(),
            method: HttpMethod::GetRequest,
        },
        ApiEndpoint {
            path: "/api/users".to_string(),
            method: HttpMethod::PostData,
        },
        ApiEndpoint {
            path: "/api/users/1".to_string(),
            method: HttpMethod::PutResource,
        },
        ApiEndpoint {
            path: "/api/users/1".to_string(),
            method: HttpMethod::DeleteItem,
        },
    ];

    let yaml = to_string(&endpoints)?;
    println!("Multiple endpoints:");
    println!("{}", yaml);

    println!("\nCustom serialization example completed successfully!");

    Ok(())
}
