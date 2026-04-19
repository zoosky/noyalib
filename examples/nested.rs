//! Nested structure example for noyalib.
//!
//! Demonstrates complex nested structures and optional fields.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use noyalib::{from_str, to_string};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Address {
    street: String,
    city: String,
    country: String,
    zip: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Contact {
    email: String,
    phone: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Employee {
    id: u32,
    name: String,
    title: String,
    address: Address,
    contact: Contact,
    skills: Vec<String>,
    active: bool,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Company {
    name: String,
    founded: u32,
    employees: Vec<Employee>,
    headquarters: Address,
}

fn main() -> Result<(), noyalib::Error> {
    println!("noyalib nested structures example\n");

    let company = Company {
        name: "TechCorp".to_string(),
        founded: 2020,
        headquarters: Address {
            street: "123 Main St".to_string(),
            city: "San Francisco".to_string(),
            country: "USA".to_string(),
            zip: Some("94102".to_string()),
        },
        employees: vec![
            Employee {
                id: 1,
                name: "Alice Smith".to_string(),
                title: "CEO".to_string(),
                address: Address {
                    street: "456 Oak Ave".to_string(),
                    city: "San Francisco".to_string(),
                    country: "USA".to_string(),
                    zip: Some("94103".to_string()),
                },
                contact: Contact {
                    email: "alice@techcorp.com".to_string(),
                    phone: Some("+1-555-0100".to_string()),
                },
                skills: vec!["leadership".to_string(), "strategy".to_string()],
                active: true,
            },
            Employee {
                id: 2,
                name: "Bob Jones".to_string(),
                title: "CTO".to_string(),
                address: Address {
                    street: "789 Pine Rd".to_string(),
                    city: "Oakland".to_string(),
                    country: "USA".to_string(),
                    zip: None,
                },
                contact: Contact {
                    email: "bob@techcorp.com".to_string(),
                    phone: None,
                },
                skills: vec![
                    "rust".to_string(),
                    "systems".to_string(),
                    "architecture".to_string(),
                ],
                active: true,
            },
        ],
    };

    let yaml = to_string(&company)?;
    println!("Company serialized:\n{}\n", yaml);

    let parsed: Company = from_str(&yaml)?;
    assert_eq!(company, parsed);

    println!("Nested structure test passed!");

    Ok(())
}
