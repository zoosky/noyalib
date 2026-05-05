// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! `noyafmt` — format YAML files via the CST formatter.
//!
//! Mirrors the `rustfmt` / `prettier` ergonomics so it slots into
//! existing developer workflows: `--check` for CI gates, `--write`
//! for in-place rewrites, stdin/stdout for editor integration.

use noyalib::cst::{format_with_config, FormatConfig};
use std::env;
use std::fs;
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "\
noyafmt — auto-format YAML via the noyalib CST.

USAGE:
    noyafmt [OPTIONS] [FILE...]
    cat FILE | noyafmt --stdin

OPTIONS:
    --check         Verify each FILE is formatted; print the list of
                    files that need formatting and exit 1 if any do.
                    Non-destructive. Suitable as a pre-commit / CI gate.
    --write         Rewrite each FILE in place. Default is to print
                    the formatted source to stdout.
    --stdin         Read from stdin, write to stdout. Mutually
                    exclusive with FILE arguments.
    --indent <N>    Indentation width in spaces (default: 2).
    -h, --help      Print this help and exit.
    -V, --version   Print noyafmt version and exit.

EXIT CODES:
      0  success — formatting performed (or no changes needed)
      1  --check found unformatted file(s), or a parse / I/O error
      2  invalid usage

EXAMPLES:
    noyafmt config.yaml               # print formatted source to stdout
    noyafmt --write config.yaml       # rewrite in place
    noyafmt --check ci/*.yaml         # CI gate
    git ls-files '*.yaml' | xargs noyafmt --check
    cat foo.yaml | noyafmt --stdin
";

#[derive(Debug, Default)]
struct Args {
    check: bool,
    write: bool,
    stdin: bool,
    indent: Option<usize>,
    files: Vec<PathBuf>,
}

#[derive(Debug)]
enum ArgError {
    Help,
    Version,
    Bad(String),
}

fn parse_args(argv: impl IntoIterator<Item = String>) -> Result<Args, ArgError> {
    let mut a = Args::default();
    let mut it = argv.into_iter();
    let _ = it.next(); // argv[0]
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "-h" | "--help" => return Err(ArgError::Help),
            "-V" | "--version" => return Err(ArgError::Version),
            "--check" => a.check = true,
            "--write" => a.write = true,
            "--stdin" => a.stdin = true,
            "--indent" => {
                let v = it
                    .next()
                    .ok_or_else(|| ArgError::Bad("--indent requires a value".to_string()))?;
                let n: usize = v.parse().map_err(|_| {
                    ArgError::Bad(format!("--indent: not a non-negative integer: {v}"))
                })?;
                a.indent = Some(n);
            }
            "--" => {
                for rest in it.by_ref() {
                    a.files.push(PathBuf::from(rest));
                }
                break;
            }
            s if s.starts_with("--indent=") => {
                let v = &s["--indent=".len()..];
                let n: usize = v.parse().map_err(|_| {
                    ArgError::Bad(format!("--indent: not a non-negative integer: {v}"))
                })?;
                a.indent = Some(n);
            }
            s if s.starts_with('-') => {
                return Err(ArgError::Bad(format!("unknown option: {s}")));
            }
            _ => a.files.push(PathBuf::from(arg)),
        }
    }
    if a.check && a.write {
        return Err(ArgError::Bad(
            "--check and --write are mutually exclusive".to_string(),
        ));
    }
    if a.stdin && !a.files.is_empty() {
        return Err(ArgError::Bad(
            "--stdin cannot be combined with FILE arguments".to_string(),
        ));
    }
    if !a.stdin && a.files.is_empty() {
        return Err(ArgError::Bad(
            "no FILE arguments and --stdin not given (use --help)".to_string(),
        ));
    }
    Ok(a)
}

fn main() -> ExitCode {
    let argv: Vec<String> = env::args().collect();
    let args = match parse_args(argv) {
        Ok(a) => a,
        Err(ArgError::Help) => {
            print!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        Err(ArgError::Version) => {
            println!("noyafmt {}", env!("CARGO_PKG_VERSION"));
            return ExitCode::SUCCESS;
        }
        Err(ArgError::Bad(msg)) => {
            eprintln!("error: {msg}");
            eprintln!();
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
    };

    let cfg = FormatConfig {
        indent_size: args.indent.unwrap_or(2),
    };

    if args.stdin {
        return run_stdin(&cfg);
    }

    let mut any_changed = false;
    let mut had_error = false;
    for file in &args.files {
        match run_file(file, &cfg, args.check, args.write) {
            Ok(changed) => any_changed |= changed,
            Err(e) => {
                eprintln!("{}: {}", file.display(), e);
                had_error = true;
            }
        }
    }

    if had_error {
        return ExitCode::from(1);
    }
    if args.check && any_changed {
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}

fn run_stdin(cfg: &FormatConfig) -> ExitCode {
    let mut input = String::new();
    if let Err(e) = io::stdin().read_to_string(&mut input) {
        eprintln!("error: reading stdin: {e}");
        return ExitCode::from(1);
    }
    match format_with_config(&input, cfg) {
        Ok(formatted) => {
            if let Err(e) = io::stdout().write_all(formatted.as_bytes()) {
                eprintln!("error: writing stdout: {e}");
                return ExitCode::from(1);
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}

/// Returns `Ok(true)` when the file would change under formatting,
/// `Ok(false)` when it is already canonical.
fn run_file(
    file: &std::path::Path,
    cfg: &FormatConfig,
    check: bool,
    write: bool,
) -> io::Result<bool> {
    let input = fs::read_to_string(file)?;
    let formatted = match format_with_config(&input, cfg) {
        Ok(s) => s,
        Err(e) => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("parse: {e}"),
            ));
        }
    };
    let changed = formatted != input;
    if check {
        if changed {
            // rustfmt convention: print the path of each unformatted file
            // to stdout so `xargs` / shell pipelines can act on them.
            println!("{}", file.display());
        }
        return Ok(changed);
    }
    if write {
        if changed {
            fs::write(file, formatted.as_bytes())?;
        }
        return Ok(changed);
    }
    // Default: print formatted source to stdout.
    io::stdout().write_all(formatted.as_bytes())?;
    Ok(changed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(parts: &[&str]) -> Vec<String> {
        std::iter::once("noyafmt".to_string())
            .chain(parts.iter().map(|s| s.to_string()))
            .collect()
    }

    #[test]
    fn parse_help_flag() {
        let r = parse_args(args(&["--help"]));
        assert!(matches!(r, Err(ArgError::Help)));
    }

    #[test]
    fn parse_help_short_flag() {
        let r = parse_args(args(&["-h"]));
        assert!(matches!(r, Err(ArgError::Help)));
    }

    #[test]
    fn parse_version_flag() {
        let r = parse_args(args(&["--version"]));
        assert!(matches!(r, Err(ArgError::Version)));
    }

    #[test]
    fn parse_version_short_flag() {
        let r = parse_args(args(&["-V"]));
        assert!(matches!(r, Err(ArgError::Version)));
    }

    #[test]
    fn parse_check_with_files() {
        let r = parse_args(args(&["--check", "a.yaml", "b.yaml"])).unwrap();
        assert!(r.check);
        assert_eq!(r.files.len(), 2);
    }

    #[test]
    fn parse_write_with_file() {
        let r = parse_args(args(&["--write", "x.yaml"])).unwrap();
        assert!(r.write);
        assert_eq!(r.files.len(), 1);
    }

    #[test]
    fn parse_stdin_alone() {
        let r = parse_args(args(&["--stdin"])).unwrap();
        assert!(r.stdin);
        assert!(r.files.is_empty());
    }

    #[test]
    fn parse_indent_separate_value() {
        let r = parse_args(args(&["--indent", "4", "--stdin"])).unwrap();
        assert_eq!(r.indent, Some(4));
    }

    #[test]
    fn parse_indent_eq_value() {
        let r = parse_args(args(&["--indent=8", "--stdin"])).unwrap();
        assert_eq!(r.indent, Some(8));
    }

    #[test]
    fn parse_indent_missing_value_errors() {
        let r = parse_args(args(&["--indent"]));
        assert!(matches!(r, Err(ArgError::Bad(_))));
    }

    #[test]
    fn parse_indent_non_numeric_errors() {
        let r = parse_args(args(&["--indent", "abc", "--stdin"]));
        assert!(matches!(r, Err(ArgError::Bad(_))));
    }

    #[test]
    fn parse_indent_eq_non_numeric_errors() {
        let r = parse_args(args(&["--indent=abc", "--stdin"]));
        assert!(matches!(r, Err(ArgError::Bad(_))));
    }

    #[test]
    fn parse_unknown_option_errors() {
        let r = parse_args(args(&["--frobnicate"]));
        assert!(matches!(r, Err(ArgError::Bad(_))));
    }

    #[test]
    fn parse_check_and_write_rejected() {
        let r = parse_args(args(&["--check", "--write", "f.yaml"]));
        assert!(matches!(r, Err(ArgError::Bad(_))));
    }

    #[test]
    fn parse_stdin_with_files_rejected() {
        let r = parse_args(args(&["--stdin", "f.yaml"]));
        assert!(matches!(r, Err(ArgError::Bad(_))));
    }

    #[test]
    fn parse_no_args_errors() {
        let r = parse_args(args(&[]));
        assert!(matches!(r, Err(ArgError::Bad(_))));
    }

    #[test]
    fn parse_double_dash_treats_remainder_as_files() {
        let r = parse_args(args(&["--", "--check", "literal.yaml"])).unwrap();
        assert_eq!(r.files.len(), 2);
        assert!(!r.check);
    }
}
