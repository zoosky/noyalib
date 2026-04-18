//! Demonstrates formatting wrappers for per-value output control.
//!
//! Run with: `cargo run --example fmt_wrappers`

use std::collections::BTreeMap;

use noyalib::fmt::{Commented, FlowMap, FlowSeq, LitString, SpaceAfter};
use noyalib::to_string;
use serde::Serialize;

#[derive(Serialize)]
struct Config {
    tags: FlowSeq<Vec<String>>,
    metadata: FlowMap<BTreeMap<String, String>>,
    script: LitString,
    name: Commented<String>,
    section: SpaceAfter<String>,
}

fn main() -> Result<(), noyalib::Error> {
    let mut metadata = BTreeMap::new();
    let _ = metadata.insert("version".into(), "1.0".into());
    let _ = metadata.insert("author".into(), "team".into());

    let config = Config {
        tags: FlowSeq(vec!["yaml".into(), "serde".into(), "rust".into()]),
        metadata: FlowMap(metadata),
        script: LitString("#!/bin/sh\necho hello\nexit 0\n".into()),
        name: Commented::new("noyalib".into(), "the project name"),
        section: SpaceAfter("end of header".into()),
    };

    let yaml = to_string(&config)?;
    println!("{yaml}");

    Ok(())
}
