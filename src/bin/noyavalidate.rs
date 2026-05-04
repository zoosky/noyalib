// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! `noyavalidate` — validate YAML syntax and (optionally) schema.
//!
//! Reads one or more YAML documents from a file (or stdin), reports
//! syntax errors via the `miette` fancy renderer, and — when
//! `--schema PATH` is given — validates each parsed document against
//! a JSON Schema 2020-12 contract (the schema may itself be written
//! in YAML or JSON; either parses).
//!
//! `--fix` rewrites the input in-place through the lossless CST
//! formatter (`noyalib::cst::format`), normalising whitespace and
//! quoting without changing semantics. When the input is stdin,
//! the formatted output is written to stdout instead.
//!
//! # Usage
//!
//! ```text
//! noyavalidate [OPTIONS] [FILE]
//!
//! Options:
//!   -s, --schema <PATH>   Validate against JSON Schema at PATH (YAML or JSON).
//!       --fix             Rewrite FILE in place via the CST formatter (or
//!                         emit to stdout if reading from stdin).
//!   -q, --quiet           Suppress success output.
//!   -h, --help            Show this message.
//!   -V, --version         Print version.
//! ```
//!
//! # Exit codes
//!
//! | Code | Meaning                                       |
//! |------|-----------------------------------------------|
//! | 0    | All documents valid (and fixed if --fix).     |
//! | 1    | Parse error or schema violation.              |
//! | 2    | Usage error (bad args).                       |
//! | 3    | I/O error (reading or writing).               |

use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use miette::{NamedSource, Report};

const USAGE: &str = "\
noyavalidate -- validate YAML syntax (and optional JSON Schema)

USAGE:
    noyavalidate [OPTIONS] [FILE]

ARGS:
    <FILE>    YAML file to validate. Use '-' or omit for stdin.

OPTIONS:
    -s, --schema <PATH>   Validate each document against JSON Schema 2020-12
                          at PATH (the schema may itself be YAML or JSON).
        --fix             Rewrite FILE in place via the CST formatter
                          (lossless: byte-faithful for everything except
                          normalised whitespace / line endings). With
                          stdin input, the formatted bytes go to stdout.
    -q, --quiet           Suppress success output.
    -h, --help            Print this help.
    -V, --version         Print version.

EXIT CODES:
    0    All documents valid (and fixed if --fix)
    1    Parse error or schema violation
    2    Usage error
    3    I/O error";

const VERSION: &str = env!("CARGO_PKG_VERSION");

enum Action {
    Validate {
        path: Option<PathBuf>,
        schema: Option<PathBuf>,
        fix: bool,
        quiet: bool,
    },
    Help,
    Version,
    Error(String),
}

fn parse_args<I: IntoIterator<Item = String>>(argv: I) -> Action {
    let mut path: Option<PathBuf> = None;
    let mut schema: Option<PathBuf> = None;
    let mut fix = false;
    let mut quiet = false;
    let mut stdin_explicit = false;

    let mut iter = argv.into_iter().skip(1);
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "-h" | "--help" => return Action::Help,
            "-V" | "--version" => return Action::Version,
            "-q" | "--quiet" => quiet = true,
            "--fix" => fix = true,
            "-s" | "--schema" => match iter.next() {
                Some(v) => schema = Some(PathBuf::from(v)),
                None => return Action::Error("--schema requires a PATH argument".into()),
            },
            s if s.starts_with("--schema=") => {
                schema = Some(PathBuf::from(&s["--schema=".len()..]));
            }
            "-" => stdin_explicit = true,
            "--" => {
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

    Action::Validate {
        path,
        schema,
        fix,
        quiet,
    }
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

fn read_schema(path: &Path) -> io::Result<String> {
    fs::read_to_string(path)
}

/// Run schema validation across every parsed document. Returns the
/// number of violations found (0 = success). Emits one miette report
/// per failing document so the user sees all issues in one pass.
fn run_schema_validation(
    docs: &[noyalib::Value],
    schema_text: &str,
    schema_path_label: &str,
    source_label: &str,
    full_source: &str,
) -> usize {
    // Parse the schema once.
    let schema: noyalib::Value = match noyalib::from_str(schema_text) {
        Ok(v) => v,
        Err(e) => {
            let report = Report::new(e)
                .with_source_code(NamedSource::new(schema_path_label, schema_text.to_owned()));
            eprintln!("error: parsing schema:");
            eprintln!("{report:?}");
            return 1;
        }
    };

    let mut violations = 0;
    for (i, doc) in docs.iter().enumerate() {
        if let Err(e) = noyalib::validate_against_schema(doc, &schema) {
            violations += 1;
            // For multi-document streams, prefix every diagnostic
            // with the doc number so the user knows which document
            // failed. miette's source-pointer label is empty for
            // span-less errors, so we surface this explicitly.
            if docs.len() > 1 {
                eprintln!("[document {}]", i + 1);
            }
            let report = Report::new(e)
                .with_source_code(NamedSource::new(source_label, full_source.to_owned()));
            eprintln!("{report:?}");
        }
    }
    violations
}

/// Run the lossless CST formatter and write the result back to
/// `path` (or to stdout if `path` is `None`). Returns Ok if the
/// write succeeded.
fn run_fix(path: Option<&Path>, source: &str) -> io::Result<()> {
    let formatted = match noyalib::cst::format(source) {
        Ok(s) => s,
        Err(e) => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("--fix: formatter rejected the input: {e}"),
            ));
        }
    };
    match path {
        None => {
            let mut stdout = io::stdout().lock();
            stdout.write_all(formatted.as_bytes())?;
        }
        Some(p) => fs::write(p, formatted.as_bytes())?,
    }
    Ok(())
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
        Action::Validate {
            path,
            schema,
            fix,
            quiet,
        } => {
            let (name, source) = match read_input(path.as_deref()) {
                Ok(pair) => pair,
                Err(e) => {
                    eprintln!("error: reading input: {e}");
                    return ExitCode::from(3);
                }
            };

            // Phase 1: syntax check.
            let docs = match noyalib::load_all_as::<noyalib::Value>(&source) {
                Ok(d) => d,
                Err(e) => {
                    let report =
                        Report::new(e).with_source_code(NamedSource::new(name, source.clone()));
                    eprintln!("{report:?}");
                    return ExitCode::from(1);
                }
            };

            // Phase 2: optional schema check.
            if let Some(schema_path) = schema.as_deref() {
                let schema_text = match read_schema(schema_path) {
                    Ok(t) => t,
                    Err(e) => {
                        eprintln!("error: reading schema {}: {e}", schema_path.display());
                        return ExitCode::from(3);
                    }
                };
                let label = schema_path.display().to_string();
                let violations = run_schema_validation(&docs, &schema_text, &label, &name, &source);
                if violations > 0 {
                    return ExitCode::from(1);
                }
            }

            // Phase 3: optional fix.
            if fix {
                if let Err(e) = run_fix(path.as_deref(), &source) {
                    eprintln!("error: applying --fix: {e}");
                    let code = if e.kind() == io::ErrorKind::InvalidData {
                        1
                    } else {
                        3
                    };
                    return ExitCode::from(code);
                }
            }

            // Suppress the chatter when --fix is reading from stdin
            // — stdout is reserved for the formatted bytes and any
            // trailing message would corrupt downstream consumers.
            let stdin_fix = fix && path.is_none();
            if !quiet && !stdin_fix {
                let n = docs.len();
                let plural = if n == 1 { "document" } else { "documents" };
                let suffix = match (schema.is_some(), fix) {
                    (true, true) => " (schema-checked, fixed)",
                    (true, false) => " (schema-checked)",
                    (false, true) => " (fixed)",
                    (false, false) => "",
                };
                println!("ok: {n} {plural} valid ({name}){suffix}");
            }
            ExitCode::from(0)
        }
    }
}

fn main() -> ExitCode {
    run()
}
