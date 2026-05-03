// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! A CLI tool for auto-formatting YAML files using noyalib.

use std::env;
use std::fs;
use std::io::{self, Read};
use noyalib::cst::format;

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    
    let input = if args.len() > 1 {
        // Read from file
        fs::read_to_string(&args[1])?
    } else {
        // Read from stdin
        let mut buffer = String::new();
        let _ = io::stdin().read_to_string(&mut buffer)?;
        buffer
    };

    match format(&input) {
        Ok(formatted) => {
            print!("{}", formatted);
            Ok(())
        }
        Err(e) => {
            eprintln!("Error formatting YAML: {}", e);
            std::process::exit(1);
        }
    }
}
