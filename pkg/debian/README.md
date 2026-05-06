<!-- SPDX-FileCopyrightText: 2026 Noyalib -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# Debian / Ubuntu packaging

The `.deb` packages for `noyafmt` and `noyavalidate` are built by
[`cargo-deb`](https://github.com/kornelski/cargo-deb) on the Linux
gnu legs of `release-binaries.yml`. The packaging metadata lives
inline under `crates/noyalib-cli/Cargo.toml`'s
`[package.metadata.deb]` section so the same `Cargo.toml` is the
single source of truth for the binary, the package, the man pages,
and the shell completions.

## Local build

```bash
cargo install cargo-deb
cd crates/noyalib-cli
cargo deb --target x86_64-unknown-linux-gnu
# → target/x86_64-unknown-linux-gnu/debian/noyalib_<version>_amd64.deb
```

## Reproducible builds

The release pipeline sets `SOURCE_DATE_EPOCH` from the commit
timestamp before invoking `cargo deb`, so the `.deb` produced for
a given tag is byte-reproducible. Downstream packagers can verify
the binary against the published `.deb` SHA256 from the GitHub
Release notes.

## Verification

The `.deb` is signed alongside the upstream tarball via cosign
keyless. See `pkg/VERIFY.md` for the verification command.
