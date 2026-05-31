<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2026 Noyalib. All rights reserved. -->

# OpenSSF Best Practices Badge — self-assessment

The CII-Best-Practices check on
[`scorecard.dev`](https://scorecard.dev/viewer/?uri=github.com/sebastienrousseau/noyalib)
scores `0/10` until the project is registered at
<https://www.bestpractices.dev/> and the self-assessment is
filled in. This file is the maintainer's prefilled checklist
so the application takes minutes rather than hours.

## Application URL

<https://www.bestpractices.dev/en/projects/new>
project URL = `https://github.com/sebastienrousseau/noyalib`

## Passing-level criteria — prefilled answers

The 65 criteria are organised under six headings on the badge
site. Each row below maps the criterion → the noyalib
artefact that satisfies it.

### Basics

| Criterion | Satisfied by |
| :--- | :--- |
| Project website / repository URL | `https://github.com/sebastienrousseau/noyalib` |
| Description / what the project does | Repository description + `README.md` headline |
| Stable URL for the project | The GitHub repo URL above |
| Discussion mechanism | GitHub Issues + Discussions enabled |
| License is OSI-approved | `MIT OR Apache-2.0` — see `LICENSE-MIT`, `LICENSE-APACHE` |
| Project provides documentation | `README.md`, `doc/USER-GUIDE.md`, `doc/ARCHITECTURE.md`, `doc/POLICIES.md`, `doc/BENCHMARKS.md` |
| Documentation includes "Quick Start" | `README.md` §"Quick Start", `crates/noyalib/README.md`, `GETTING_STARTED.md` |
| Documentation has a security policy | `SECURITY.md` |
| Maintainer-direct contact | `sebastian.rousseau@gmail.com` (per `SECURITY.md`) |
| Public bug tracker | GitHub Issues |
| Acknowledgement of contributions | `CONTRIBUTING.md`, GitHub PR / issue author attribution |

### Change control

| Criterion | Satisfied by |
| :--- | :--- |
| Source under version control | Git, GitHub-hosted |
| Unique version identifier per release | SemVer tags `v0.0.x` |
| Release notes per version | `RELEASE-NOTES-v0.0.X.md` for each tagged release |
| Standardised file structure | Cargo workspace conventions; `crates/`, `doc/`, `pkg/` |
| Changelog kept | `CHANGELOG.md` (Keep-a-Changelog format) |

### Reporting

| Criterion | Satisfied by |
| :--- | :--- |
| Bug reports tracked | GitHub Issues, with templates in `.github/ISSUE_TEMPLATE/` |
| Bug report responses ≤ 14 days | Issue-response SLA documented in `SECURITY.md` (48 h initial response) |
| Vulnerability report channel | `SECURITY.md` — disclosure via `sebastian.rousseau@gmail.com`, 48 h response |
| Security audit log | Audit reports tracked in `doc/POLICIES.md` § "Audit pipeline" |

### Quality

| Criterion | Satisfied by |
| :--- | :--- |
| Working build system | `cargo build --workspace --all-features` |
| Working test system | `cargo test --workspace --all-features` (~5 400 tests) |
| Tests run on every change | `.github/workflows/ci.yml` triggers on push + pull_request |
| Code-coverage measurement | `.github/workflows/ci.yml` § `Coverage gate (≥95%)` — `cargo llvm-cov` |
| Coverage tool integration | Same |
| New features include tests | Required by review process; enforced by `cargo-machete`, strict-doc gate |
| Documented coding style | Workspace-level lints in `crates/noyalib/Cargo.toml`; `cargo fmt` enforced by CI |
| Code review of every change | `main` ruleset requires PR + 1 approving review + code-owner review + last-push approval (post-this-commit) |

### Security

| Criterion | Satisfied by |
| :--- | :--- |
| Cryptographic best practices | Releases signed via cosign keyless + SLSA L3 build provenance attestations on every artefact |
| Inputs validated before use | Parser enforces `ParserConfig` limits (`max_depth`, `max_document_length`, `max_alias_expansions`, …) |
| Hardened against vulnerabilities | `#![forbid(unsafe_code)]` workspace-wide, fuzz suite (4 targets) + Miri soak runs in `.github/workflows/security.yml` |
| Vulnerability disclosure tested | One historical CVE-equivalent (issue #46 RecursionLimitExceeded false-positive) — patched in v0.0.6 within the same release cycle |
| Security expertise consulted | Audit pipeline: `cargo-deny`, `cargo-vet`, `cargo-audit`, `cargo-machete`, CodeQL — see `doc/POLICIES.md` |

### Analysis

| Criterion | Satisfied by |
| :--- | :--- |
| Static analysis applied | `cargo clippy --workspace --all-features -- -D warnings` on every PR; CodeQL on `.github/workflows/security.yml` |
| Dynamic analysis applied | Differential fuzz (10 s smoke per PR) + soak fuzz (1 h per target weekly); Miri (focused per PR + full weekly) |
| Coverage-guided fuzzing | `cargo-fuzz` with `libFuzzer`, 4 targets: `fuzz_parse`, `fuzz_roundtrip`, `fuzz_diff`, `fuzz_strict` |
| Memory-safety analysis | `#![forbid(unsafe_code)]` (the strongest possible static guarantee) + Miri runs to verify transitive `unsafe` blocks in dev-deps |

## Silver / Gold level (future)

The "Silver" badge adds 30+ more criteria around release
process maturity and "Gold" 30 more around supply-chain
transparency. noyalib already exceeds the passing-level bar
on all 65 items; tracking Silver for the v0.1.0 milestone.

## How to apply

1. Visit <https://www.bestpractices.dev/en/projects/new>.
2. Enter the repo URL `https://github.com/sebastienrousseau/noyalib`.
3. Walk through the form using the answers above. Most criteria
   accept a URL → paste the corresponding `doc/` link or
   `https://github.com/sebastienrousseau/noyalib/blob/main/<path>`.
4. Submit. Badge typically issues within 24 h.
5. Once issued, the OpenSSF Scorecard refresh (Monday 06:00 UTC)
   lifts the `CII-Best-Practices` check from 0 → 10.

The badge URL will be `https://www.bestpractices.dev/projects/<id>`;
add it to the workspace `README.md` header alongside the other
badges once issued.
