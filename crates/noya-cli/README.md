<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->

# noya-cli

`noyafmt` and `noyavalidate` — the YAML formatter and validator
that ship as the CLI half of the
[noyalib workspace](https://github.com/sebastienrousseau/noyalib).

[![crates.io](https://img.shields.io/crates/v/noyalib.svg)](https://crates.io/crates/noyalib)
[![docs.rs](https://img.shields.io/docsrs/noyalib)](https://docs.rs/noyalib)
[![Build](https://img.shields.io/github/actions/workflow/status/sebastienrousseau/noyalib/ci.yml?branch=main)](https://github.com/sebastienrousseau/noyalib/actions)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](#license)

Both binaries are powered by
[`noyalib`](https://crates.io/crates/noyalib): pure-Rust YAML 1.2,
zero `unsafe`, byte-faithful CST formatting that preserves
comments, indentation, and document structure.

## Contents

- [Install](#install)
- [Quick Start](#quick-start)
- [`noyafmt`](#noyafmt)
- [`noyavalidate`](#noyavalidate)
- [Exit codes](#exit-codes)
- [Examples](#examples)
- [Shell completions and man pages](#shell-completions-and-man-pages)
- [Documentation](#documentation)
- [License](#license)

## Install

| Channel | Command |
|---|---|
| Cargo | `cargo install noyalib` |
| Homebrew | `brew tap sebastienrousseau/tap && brew install noyalib` |
| Arch (AUR) | `yay -S noyalib-bin` (binary) or `yay -S noyalib` (source) |
| Scoop | `scoop bucket add sebastienrousseau https://github.com/sebastienrousseau/scoop-bucket && scoop install noyalib` |
| Nix | `nix run github:sebastienrousseau/noyalib` |
| Container (GHCR) | `docker run --rm -v "$(pwd):/work" -w /work ghcr.io/sebastienrousseau/noyafmt:latest --check ci/*.yaml` |

Pre-built tarballs for 14 targets (Linux gnu + musl, macOS Intel
+ Apple Silicon + universal, Windows x86_64 + i686 + aarch64)
are attached to every GitHub Release. Each artefact is signed
with cosign keyless and carries a SLSA L3 build provenance
attestation — see
[`pkg/VERIFY.md`](https://github.com/sebastienrousseau/noyalib/blob/main/pkg/VERIFY.md)
for verification commands.

## Quick Start

```bash
# Format a file in-place; comments + indentation preserved.
noyafmt --write config.yaml

# CI gate — exits 1 if any file would change.
noyafmt --check ci/*.yaml

# Validate syntax + JSON Schema 2020-12.
noyavalidate --schema schema.yaml deploy.yaml

# Validate + auto-fix obvious type slips (port: "8080" → 8080).
noyavalidate --schema schema.yaml --fix deploy.yaml
```

## `noyafmt`

YAML formatter. Mirrors the `rustfmt` / `prettier` ergonomics:

```bash
noyafmt config.yaml                # print formatted source to stdout (default)
noyafmt --write config.yaml        # rewrite in place
noyafmt --check ci/*.yaml          # CI gate
noyafmt --indent 4 config.yaml     # override default 2-space indent
cat foo.yaml | noyafmt --stdin     # editor pipe
```

The formatter goes through noyalib's lossless CST: comments,
anchor positions, and document structure are preserved
byte-for-byte; only whitespace and quoting are normalised.

## `noyavalidate`

YAML syntax checker, optionally with JSON Schema 2020-12
enforcement and **schema-driven autofix**.

```bash
noyavalidate manifest.yaml                          # syntax only
noyavalidate --schema schema.yaml deploy.yaml       # + schema check
noyavalidate --schema schema.yaml --fix deploy.yaml # + autofix
```

The autofix engine
([`coerce_to_schema`](https://docs.rs/noyalib/latest/noyalib/fn.coerce_to_schema.html))
rewrites string-shaped scalars into the schema's expected type
when the parse succeeds. Loops until convergence; unparseable
inputs (`port: "abc"` against `type: integer`) are left in place
so a follow-up `validate_against_schema` call surfaces the
residue.

Diagnostics use [`miette`](https://crates.io/crates/miette) for
rustc-style source pointers:

```text
× schema violation: "8080" is not of type "integer"
   ╭─[deploy.yaml:3:7]
 2 │ replicas: 3
 3 │ port: "8080"
   ·       ─┬───
   ·        ╰── here
 4 │ host: api
   ╰────
   help: pass --fix to coerce string-shaped scalars to the
         schema's declared type.
```

## Exit codes

| Code | `noyafmt` | `noyavalidate` |
|---|---|---|
| 0 | success (or no changes if `--check`) | all valid |
| 1 | parse / I/O error, or `--check` found unformatted file(s) | parse error or schema violation |
| 2 | invalid usage (bad arg combination) | invalid usage |
| 3 | — | I/O error (read / write) |

## Examples

End-to-end runnable demos under
[`crates/noya-cli/examples/`](examples/):

```bash
crates/noya-cli/examples/format-precommit.sh    # rustfmt-style git pre-commit
crates/noya-cli/examples/validate-k8s.sh        # gate Kubernetes manifests in CI
crates/noya-cli/examples/fix-quoted-numbers.sh  # before/after schema autofix
```

Each script ships its sample input alongside; running it shows
the binary's stdout + the exit-code semantics.

## Shell completions and man pages

Tarball releases ship pre-built completions for bash, fish, zsh,
and PowerShell, plus roff man pages. Distro packages drop them
into the standard system locations
(`/usr/share/bash-completion/completions/`, `/usr/share/man/man1/`,
…).

If installing via `cargo install`, regenerate locally:

```bash
git clone https://github.com/sebastienrousseau/noyalib
cd noyalib
cargo xtask completions     # writes complete/{noyafmt,noyavalidate}.{bash,fish,zsh,ps1}
cargo xtask manpages        # writes doc/{noyafmt,noyavalidate}.1
```

## Documentation

- **Workspace README**:
  <https://github.com/sebastienrousseau/noyalib#readme>
- **Per-channel install / verification cookbook**:
  [`pkg/VERIFY.md`](https://github.com/sebastienrousseau/noyalib/blob/main/pkg/VERIFY.md)
- **Library API the binaries call into**:
  <https://docs.rs/noyalib>

## License

Dual-licensed under [Apache 2.0](https://www.apache.org/licenses/LICENSE-2.0)
or [MIT](https://opensource.org/licenses/MIT), at your option.
