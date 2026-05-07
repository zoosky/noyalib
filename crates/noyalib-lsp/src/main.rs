// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! `noyalib-lsp` — Language Server Protocol implementation for
//! noyalib. Stdio transport with the standard LSP framing
//! (`Content-Length` headers).
//!
//! This binary is the transport shim around [`noyalib_lsp::Server`].
//! All handler logic lives in the library so `cargo test` covers
//! it directly without standing up a real LSP client.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use noyalib_lsp::Server;
use std::io::{self, Read, Write};
use std::process::ExitCode;

const HELP: &str = "\
noyalib-lsp — Language Server Protocol implementation for noyalib.

USAGE:
  noyalib-lsp                   Start the LSP stdio server (the
                                normal mode an editor invokes).
  noyalib-lsp --version | -V    Print version and exit.
  noyalib-lsp --help | -h       Print this help and exit.

NOTES:
  This binary speaks the standard LSP wire format with
  `Content-Length` framing over stdio. It is not designed for
  interactive use — configure your editor to spawn it. Example for
  Neovim with `lspconfig`:

    require('lspconfig.configs').noyalib = {
      default_config = {
        cmd = { 'noyalib-lsp' },
        filetypes = { 'yaml' },
        root_dir = require('lspconfig.util').find_git_ancestor,
      },
    }
    require('lspconfig').noyalib.setup {}

REPORTING BUGS:
  https://github.com/sebastienrousseau/noyalib/issues
";

fn main() -> ExitCode {
    // Honour the conventional `--version` / `--help` flags before
    // falling into the LSP stdio loop. Without these, a user
    // running `noyalib-lsp` to verify the install just sees a hung
    // process; printing version / help is the standard CLI hygiene.
    if let Some(arg) = std::env::args().nth(1) {
        match arg.as_str() {
            "--version" | "-V" => {
                println!("noyalib-lsp {}", env!("CARGO_PKG_VERSION"));
                return ExitCode::SUCCESS;
            }
            "--help" | "-h" => {
                print!("{HELP}");
                return ExitCode::SUCCESS;
            }
            other => {
                eprintln!("noyalib-lsp: unknown argument `{other}`");
                eprintln!("Run `noyalib-lsp --help` for usage.");
                return ExitCode::from(2);
            }
        }
    }
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("noyalib-lsp: {e}");
            ExitCode::from(3)
        }
    }
}

fn run() -> io::Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout().lock();
    let mut server = Server::new();
    let mut buf = [0u8; 4096];
    let mut pending: Vec<u8> = Vec::new();
    let mut handle = stdin.lock();

    loop {
        let n = handle.read(&mut buf)?;
        if n == 0 {
            return Ok(());
        }
        pending.extend_from_slice(&buf[..n]);

        while let Some((header_end, content_length)) = parse_header(&pending) {
            if pending.len() < header_end + content_length {
                break;
            }
            let body_start = header_end;
            let body_end = body_start + content_length;
            let body = std::str::from_utf8(&pending[body_start..body_end])
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?
                .to_owned();
            let outcome = server.handle_message(&body);
            if let Some(reply) = outcome.reply {
                write_message(&mut stdout, &reply)?;
            }
            for note in outcome.notifications {
                write_message(&mut stdout, &note)?;
            }
            stdout.flush()?;
            pending.drain(..body_end);
        }
    }
}

/// Parse an LSP message header from the front of `bytes`. Returns
/// the header end index (`\r\n\r\n` past the last header) and the
/// declared `Content-Length`. Returns `None` if the header is
/// incomplete; the caller buffers more input and retries.
fn parse_header(bytes: &[u8]) -> Option<(usize, usize)> {
    let header_end = find_header_end(bytes)?;
    let header = std::str::from_utf8(&bytes[..header_end]).ok()?;
    let content_length = header.lines().find_map(|line| {
        line.strip_prefix("Content-Length:")
            .map(str::trim)
            .and_then(|n| n.parse::<usize>().ok())
    })?;
    Some((header_end, content_length))
}

fn find_header_end(bytes: &[u8]) -> Option<usize> {
    let mut i = 0;
    while i + 3 < bytes.len() {
        if &bytes[i..i + 4] == b"\r\n\r\n" {
            return Some(i + 4);
        }
        i += 1;
    }
    None
}

fn write_message(out: &mut impl Write, body: &str) -> io::Result<()> {
    write!(out, "Content-Length: {}\r\n\r\n{}", body.len(), body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_header_extracts_content_length() {
        let bytes = b"Content-Length: 42\r\n\r\nbody";
        let (end, len) = parse_header(bytes).unwrap();
        assert_eq!(end, 22);
        assert_eq!(len, 42);
    }

    #[test]
    fn parse_header_returns_none_when_header_incomplete() {
        assert!(parse_header(b"Content-Length: 42\r\n").is_none());
    }

    #[test]
    fn parse_header_returns_none_when_content_length_missing() {
        assert!(parse_header(b"Other-Header: x\r\n\r\n").is_none());
    }

    #[test]
    fn parse_header_handles_extra_headers() {
        let bytes = b"Content-Type: application/json\r\nContent-Length: 7\r\n\r\nabc";
        let (end, len) = parse_header(bytes).unwrap();
        assert!(end > 0);
        assert_eq!(len, 7);
    }

    #[test]
    fn find_header_end_locates_terminator() {
        assert_eq!(find_header_end(b"a\r\n\r\nb"), Some(5));
        assert_eq!(find_header_end(b"abc"), None);
    }

    #[test]
    fn write_message_uses_content_length_prefix() {
        let mut out = Vec::new();
        write_message(&mut out, r#"{"hello":"world"}"#).unwrap();
        let s = std::str::from_utf8(&out).unwrap();
        assert!(s.starts_with("Content-Length: 17\r\n\r\n"));
        assert!(s.ends_with(r#"{"hello":"world"}"#));
    }
}
