// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! `xtask` — internal release-pipeline tooling for the noyalib
//! workspace.
//!
//! ```text
//! cargo xtask completions   # writes complete/*.{bash,fish,zsh,ps1}
//! cargo xtask manpages      # writes doc/*.1
//! cargo xtask all           # both, in one pass
//! ```
//!
//! The same [`clap::Command`] builders the binaries parse against
//! at runtime — exposed by `noya_cli::{noyafmt_command,
//! noyavalidate_command}` — drive both the build script
//! (`crates/noya-cli/build.rs`) and this xtask. So binaries, man
//! pages, and shell completions can never drift.

use std::fs;
use std::path::{Path, PathBuf};

use clap::Parser;

#[derive(Parser)]
#[command(
    name = "xtask",
    about = "Internal release-pipeline tooling for noyalib",
    version = env!("CARGO_PKG_VERSION"),
)]
enum Cmd {
    /// Regenerate shell completions under `complete/`.
    Completions,
    /// Regenerate roff-format man pages under `doc/`.
    Manpages,
    /// Run every codegen task in one go.
    All,
}

fn main() -> std::io::Result<()> {
    match Cmd::parse() {
        Cmd::Completions => write_completions(&workspace_root()?)?,
        Cmd::Manpages => write_manpages(&workspace_root()?)?,
        Cmd::All => {
            let root = workspace_root()?;
            write_completions(&root)?;
            write_manpages(&root)?;
        }
    }
    Ok(())
}

/// Locate the workspace root by walking upward from the xtask
/// crate until we find a `Cargo.toml` that contains a `[workspace]`
/// table. Cargo doesn't expose this directly, so we resolve via
/// `CARGO_MANIFEST_DIR` (the xtask's own dir) and pop two levels.
fn workspace_root() -> std::io::Result<PathBuf> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // crates/xtask → crates/ → workspace root
    let root = manifest_dir
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "could not locate workspace root from xtask manifest dir",
            )
        })?
        .to_path_buf();
    Ok(root)
}

/// Write bash / fish / zsh / powershell completions to
/// `<root>/complete/`.
fn write_completions(root: &Path) -> std::io::Result<()> {
    use clap_complete::{generate_to, Shell};
    let out = root.join("complete");
    fs::create_dir_all(&out)?;
    for (name, mut cmd) in [
        ("noyafmt", noya_cli::noyafmt_command()),
        ("noyavalidate", noya_cli::noyavalidate_command()),
    ] {
        for shell in [Shell::Bash, Shell::Fish, Shell::Zsh, Shell::PowerShell] {
            let path = generate_to(shell, &mut cmd, name, &out).map_err(|e| {
                std::io::Error::other(format!("clap_complete::generate_to failed: {e}"))
            })?;
            eprintln!("→ {}", path.display());
        }
    }
    Ok(())
}

/// Write roff-format man pages to `<root>/doc/`.
fn write_manpages(root: &Path) -> std::io::Result<()> {
    let out = root.join("doc");
    fs::create_dir_all(&out)?;
    for (name, cmd) in [
        ("noyafmt", noya_cli::noyafmt_command()),
        ("noyavalidate", noya_cli::noyavalidate_command()),
    ] {
        let path = out.join(format!("{name}.1"));
        let mut buffer: Vec<u8> = Vec::with_capacity(4096);
        clap_mangen::Man::new(cmd)
            .render(&mut buffer)
            .map_err(|e| std::io::Error::other(format!("clap_mangen::Man::render failed: {e}")))?;
        fs::write(&path, buffer)?;
        eprintln!("→ {}", path.display());
    }
    Ok(())
}
