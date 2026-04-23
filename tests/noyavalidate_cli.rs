// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Integration tests for the `noyavalidate` CLI.
//!
//! Uses the `CARGO_BIN_EXE_noyavalidate` env var Cargo injects for
//! same-crate integration tests. The whole module is gated on the
//! `noyavalidate` feature since that's what compiles the binary.

#![cfg(feature = "noyavalidate")]

use std::io::Write;
use std::process::{Command, Stdio};

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_noyavalidate"))
}

fn run_with_stdin(stdin: &str, args: &[&str]) -> (i32, String, String) {
    let mut child = bin()
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn noyavalidate");
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8(output.stdout).unwrap(),
        String::from_utf8(output.stderr).unwrap(),
    )
}

fn run_with_file(path: &std::path::Path, args: &[&str]) -> (i32, String, String) {
    let output = bin()
        .args(args)
        .arg(path)
        .output()
        .expect("spawn noyavalidate");
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8(output.stdout).unwrap(),
        String::from_utf8(output.stderr).unwrap(),
    )
}

fn tmp(name: &str, contents: &str) -> std::path::PathBuf {
    let dir =
        std::env::temp_dir().join(format!("noyavalidate_cli_{}_{}", name, std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("{name}.yaml"));
    std::fs::write(&path, contents).unwrap();
    path
}

// ── Valid YAML ───────────────────────────────────────────────────────────

#[test]
fn valid_single_doc_file() {
    let p = tmp("single", "name: ok\nvalue: 42\n");
    let (code, stdout, _) = run_with_file(&p, &[]);
    assert_eq!(code, 0);
    assert!(stdout.contains("ok: 1 document valid"));
}

#[test]
fn valid_multi_doc_file() {
    let p = tmp("multi", "---\na: 1\n---\nb: 2\n---\nc: 3\n");
    let (code, stdout, _) = run_with_file(&p, &[]);
    assert_eq!(code, 0);
    assert!(stdout.contains("ok: 3 documents valid"));
}

#[test]
fn valid_quiet_suppresses_output() {
    let p = tmp("quiet", "name: ok\n");
    let (code, stdout, _) = run_with_file(&p, &["-q"]);
    assert_eq!(code, 0);
    assert!(
        stdout.is_empty(),
        "quiet mode must not print; got: {stdout:?}"
    );
}

#[test]
fn valid_long_quiet_flag() {
    let p = tmp("long_quiet", "name: ok\n");
    let (code, stdout, _) = run_with_file(&p, &["--quiet"]);
    assert_eq!(code, 0);
    assert!(stdout.is_empty());
}

// ── Stdin ────────────────────────────────────────────────────────────────

#[test]
fn valid_stdin_implicit() {
    let (code, stdout, _) = run_with_stdin("name: ok\n", &[]);
    assert_eq!(code, 0);
    assert!(stdout.contains("<stdin>"));
    assert!(stdout.contains("ok: 1 document valid"));
}

#[test]
fn valid_stdin_explicit_dash() {
    let (code, stdout, _) = run_with_stdin("name: ok\n", &["-"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("<stdin>"));
}

// ── Parse errors ─────────────────────────────────────────────────────────

#[test]
fn parse_error_exits_1() {
    let p = tmp("bad", "key: [1, 2, 3\nnext: v\n");
    let (code, _, stderr) = run_with_file(&p, &[]);
    assert_eq!(code, 1);
    // Diagnostic code identifies the kind of error.
    assert!(stderr.contains("noyalib::parse"), "stderr was: {stderr}");
}

#[test]
fn parse_error_includes_filename_in_diagnostic() {
    let p = tmp("named", "key: [unclosed\n");
    let (code, _, stderr) = run_with_file(&p, &[]);
    assert_eq!(code, 1);
    let name = p.display().to_string();
    assert!(
        stderr.contains(&name),
        "filename {name:?} missing from stderr: {stderr}"
    );
}

// ── Help / version ───────────────────────────────────────────────────────

#[test]
fn help_flag_short() {
    let output = bin().arg("-h").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("USAGE:"));
    assert!(stdout.contains("--quiet"));
}

#[test]
fn help_flag_long() {
    let output = bin().arg("--help").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("EXIT CODES:"));
}

#[test]
fn version_flag_short() {
    let output = bin().arg("-V").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.starts_with("noyavalidate "));
}

#[test]
fn version_flag_long() {
    let output = bin().arg("--version").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.starts_with("noyavalidate "));
}

// ── Usage errors ─────────────────────────────────────────────────────────

#[test]
fn unknown_flag_exits_2() {
    let output = bin().arg("--bogus").output().unwrap();
    assert_eq!(output.status.code().unwrap(), 2);
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("unknown option"));
    assert!(stderr.contains("USAGE:"));
}

#[test]
fn too_many_files_exits_2() {
    let output = bin().args(["a.yaml", "b.yaml"]).output().unwrap();
    assert_eq!(output.status.code().unwrap(), 2);
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("too many"));
}

#[test]
fn stdin_combined_with_file_exits_2() {
    let output = bin().args(["-", "a.yaml"]).output().unwrap();
    assert_eq!(output.status.code().unwrap(), 2);
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("'-'"));
}

// ── I/O errors ───────────────────────────────────────────────────────────

#[test]
fn missing_file_exits_3() {
    let output = bin()
        .arg("/tmp/__noyavalidate_definitely_not_a_real_file__.yaml")
        .output()
        .unwrap();
    assert_eq!(output.status.code().unwrap(), 3);
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("reading input"));
}

// ── Positional `--` separator ────────────────────────────────────────────

#[test]
fn double_dash_allows_path_with_leading_dash() {
    let dir = std::env::temp_dir().join(format!("noyavalidate_dd_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("-weirdname.yaml");
    std::fs::write(&path, "ok: true\n").unwrap();
    let output = bin().args(["--"]).arg(&path).output().unwrap();
    assert_eq!(output.status.code().unwrap(), 0);
}
