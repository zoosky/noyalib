// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! `noyavalidate` — validate YAML syntax with rich miette diagnostics.
//!
//! Reads one or more YAML documents from a file (or stdin) and reports any
//! syntax errors via the `miette` fancy renderer — coloured terminal output
//! with source-span highlighting.
//!
//! # Usage
//!
//! ```text
//! noyavalidate [OPTIONS] [FILE]
//!
//! Options:
//!   -q, --quiet    Suppress success output.
//!   -h, --help     Show this message.
//!   -V, --version  Print version.
//!
//! If FILE is omitted or is `-`, input is read from standard input.
//! ```
//!
//! # Exit codes
//!
//! | Code | Meaning                     |
//! |------|-----------------------------|
//! | 0    | All documents are valid.    |
//! | 1    | Parse error.                |
//! | 2    | Usage error (bad args).     |
//! | 3    | I/O error (reading input).  |

use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use miette::{NamedSource, Report};

const USAGE: &str = "\
noyavalidate — validate YAML syntax

USAGE:
    noyavalidate [OPTIONS] [FILE]

ARGS:
    <FILE>    YAML file to validate. Use '-' or omit for stdin.

OPTIONS:
    -q, --quiet      Suppress success output.
    -h, --help       Print this help.
    -V, --version    Print version.

EXIT CODES:
    0    All documents are valid
    1    Parse error
    2    Usage error
    3    I/O error";

const VERSION: &str = env!("CARGO_PKG_VERSION");

enum Action {
    Validate { path: Option<PathBuf>, quiet: bool },
    Help,
    Version,
    Error(String),
}

fn parse_args<I: IntoIterator<Item = String>>(argv: I) -> Action {
    let mut path: Option<PathBuf> = None;
    let mut quiet = false;
    let mut stdin_explicit = false;

    let mut iter = argv.into_iter().skip(1);
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "-h" | "--help" => return Action::Help,
            "-V" | "--version" => return Action::Version,
            "-q" | "--quiet" => quiet = true,
            "-" => stdin_explicit = true,
            "--" => {
                // Everything after `--` is positional.
                if let Some(rest) = iter.next() {
                    if path.is_some() {
                        return Action::Error("too many positional arguments".into());
                    }
                    path = Some(PathBuf::from(rest));
                }
                if iter.next().is_some() {
                    return Action::Error("too many positional arguments".into());
                }
            }
            a if a.starts_with('-') => {
                return Action::Error(format!("unknown option: {a}"));
            }
            a => {
                if path.is_some() {
                    return Action::Error("too many positional arguments".into());
                }
                path = Some(PathBuf::from(a));
            }
        }
    }

    if stdin_explicit && path.is_some() {
        return Action::Error("cannot combine '-' with a FILE argument".into());
    }

    Action::Validate { path, quiet }
}

fn read_input(path: Option<&Path>) -> io::Result<(String, String)> {
    match path {
        None => {
            let mut buf = String::new();
            let _ = io::stdin().read_to_string(&mut buf)?;
            Ok(("<stdin>".to_string(), buf))
        }
        Some(p) => {
            let source = fs::read_to_string(p)?;
            Ok((p.display().to_string(), source))
        }
    }
}

fn run() -> ExitCode {
    let argv: Vec<String> = std::env::args().collect();
    match parse_args(argv) {
        Action::Help => {
            println!("{USAGE}");
            ExitCode::from(0)
        }
        Action::Version => {
            println!("noyavalidate {VERSION}");
            ExitCode::from(0)
        }
        Action::Error(msg) => {
            eprintln!("error: {msg}");
            eprintln!();
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
        Action::Validate { path, quiet } => {
            let (name, source) = match read_input(path.as_deref()) {
                Ok(pair) => pair,
                Err(e) => {
                    eprintln!("error: reading input: {e}");
                    return ExitCode::from(3);
                }
            };
            match noyalib::load_all_as::<noyalib::Value>(&source) {
                Ok(docs) => {
                    if !quiet {
                        let n = docs.len();
                        let plural = if n == 1 { "document" } else { "documents" };
                        println!("ok: {n} {plural} valid ({name})");
                    }
                    ExitCode::from(0)
                }
                Err(e) => {
                    let report =
                        Report::new(e).with_source_code(NamedSource::new(name, source.clone()));
                    eprintln!("{report:?}");
                    ExitCode::from(1)
                }
            }
        }
    }
}

fn main() -> ExitCode {
    run()
}
