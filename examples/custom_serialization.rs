// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Custom key transformations with singleton_map_with.
//!
//! Run: `cargo run --example custom_serialization`

#[path = "support.rs"]
mod support;

use noyalib::{from_str, to_string};
use serde::{Deserialize, Serialize};

// ── Snake case ──────────────────────────────────────────────────────────

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

// ── Kebab case ──────────────────────────────────────────────────────────

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

// ── Lowercase ───────────────────────────────────────────────────────────

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

fn main() {
    support::header("noyalib -- custom_serialization");

    // Snake case roundtrip
    support::task_with_output("Snake case: GetRequest -> get_request", || {
        let endpoint = ApiEndpoint {
            path: "/api/users".to_string(),
            method: HttpMethod::GetRequest,
        };
        let yaml = to_string(&endpoint).unwrap();
        let parsed: ApiEndpoint = from_str(&yaml).unwrap();
        assert_eq!(endpoint, parsed);
        yaml.lines().map(|l| l.to_string()).collect()
    });

    // Kebab case roundtrip
    support::task_with_output("Kebab case: DebugInfo -> debug-info", || {
        let log = LogConfig {
            name: "my-app".to_string(),
            level: LogLevel::DebugInfo,
        };
        let yaml = to_string(&log).unwrap();
        let parsed: LogConfig = from_str(&yaml).unwrap();
        assert_eq!(log, parsed);
        yaml.lines().map(|l| l.to_string()).collect()
    });

    // Lowercase roundtrip
    support::task_with_output("Lowercase: PRODUCTION -> production", || {
        let deploy = DeployConfig {
            app_name: "my-service".to_string(),
            environment: Environment::PRODUCTION,
        };
        let yaml = to_string(&deploy).unwrap();
        let parsed: DeployConfig = from_str(&yaml).unwrap();
        assert_eq!(deploy, parsed);
        yaml.lines().map(|l| l.to_string()).collect()
    });

    support::summary(3);
}
