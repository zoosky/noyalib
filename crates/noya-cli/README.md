<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->

# noya-cli

`noyafmt` — a YAML formatter.
`noyavalidate` — a YAML syntax + JSON Schema validator with
schema-driven autofix.

Both binaries are powered by
[`noyalib`](https://crates.io/crates/noyalib): pure-Rust YAML 1.2,
zero `unsafe`, byte-faithful CST formatting that preserves
comments, indentation, and document structure.

## Install

```sh
# Cargo
cargo install noyalib

# Homebrew (personal tap)
brew tap sebastienrousseau/tap
brew install noyalib

# Arch (AUR)
yay -S noyalib-bin            # pre-built binary
yay -S noyalib                # build from source

# Scoop (Windows)
scoop bucket add sebastienrousseau https://github.com/sebastienrousseau/scoop-bucket
scoop install noyalib

# Nix
nix run github:sebastienrousseau/noyalib

# Container (GHCR)
docker run --rm -v "$(pwd):/work" -w /work \
  ghcr.io/sebastienrousseau/noyafmt:latest --check ci/*.yaml
```

Pre-built tarballs for 14 targets (Linux gnu + musl, macOS
Intel + Apple Silicon + universal, Windows x86_64 + i686 +
aarch64) are attached to every GitHub Release. Each artefact is
signed with cosign keyless and carries a SLSA L3 build provenance
attestation — see
[`pkg/VERIFY.md`](https://github.com/sebastienrousseau/noyalib/blob/main/pkg/VERIFY.md)
for verification commands.

## `noyafmt`

YAML formatter. Mirrors the `rustfmt` / `prettier` ergonomics:

```sh
# Print formatted source to stdout (default).
noyafmt config.yaml

# Rewrite in place.
noyafmt --write config.yaml

# CI gate — exits 1 if any file would change.
noyafmt --check ci/*.yaml

# Editor pipe.
cat foo.yaml | noyafmt --stdin
```

The formatter goes through noyalib's lossless CST: comments,
anchor positions, and document structure are preserved
byte-for-byte; only whitespace and quoting are normalised.

`--indent N` overrides the default 2-space indent (noyalib
detects the dominant indent automatically when reading; this
flag forces a specific style on output).

## `noyavalidate`

YAML syntax checker, optionally with JSON Schema 2020-12
enforcement and **schema-driven autofix**.

```sh
# Pure syntax check — exits 0 if valid, 1 if not.
noyavalidate manifest.yaml

# JSON Schema validation. Schema may itself be YAML or JSON.
noyavalidate --schema schema.yaml deploy.yaml

# Surgical fix: rewrite in place to satisfy the schema where
# possible. e.g. `port: "8080"` (string) → `port: 8080`
# (integer) when the schema declares port as an integer.
noyavalidate --schema schema.yaml --fix deploy.yaml
```

The autofix engine
([`coerce_to_schema`](https://docs.rs/noyalib/latest/noyalib/fn.coerce_to_schema.html))
rewrites string-shaped scalars into the schema's expected type
when the parse succeeds. Loops until convergence; unparseable
inputs (e.g. `port: "abc"` against `type: integer`) are left in
place so the caller sees the residue via a follow-up
`validate_against_schema` call.

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

## Shell completions and man pages

Tarball releases ship pre-built completions for bash, fish, zsh,
and PowerShell, plus roff man pages. Distro packages drop them
into the standard system locations
(`/usr/share/bash-completion/completions/`, `/usr/share/man/man1/`,
…).

If installing via `cargo install`, regenerate locally:

```sh
git clone https://github.com/sebastienrousseau/noyalib
cd noyalib
cargo xtask completions    # writes complete/{noyafmt,noyavalidate}.{bash,fish,zsh,ps1}
cargo xtask manpages       # writes doc/{noyafmt,noyavalidate}.1
```

## License

Dual-licensed under Apache 2.0 or MIT, at your option.
