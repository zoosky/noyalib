# Security Policy

## Supported Versions

| Version | Supported |
|:--------|:---------:|
| 0.0.x   | Yes       |

## Reporting a Vulnerability

Report security vulnerabilities by emailing **sebastian.rousseau@gmail.com**.

Do not open a public issue for security reports.

Include:

- A description of the vulnerability.
- Steps to reproduce.
- Affected versions.
- Any suggested fix (optional).

Expect an initial response within 48 hours. A fix or mitigation plan will follow within 7 days of confirmation.

## Security Design

noyalib enforces safety at the compiler level:

- `#![forbid(unsafe_code)]` — zero unsafe blocks, guaranteed.
- No C dependencies, no FFI calls. Pure Rust only.
- No network I/O, no file system writes, no environment variable reads.

### Parser Hardening

Configurable limits protect against denial-of-service attacks:

| Limit | Default | Purpose |
|:------|:--------|:--------|
| `max_depth` | 128 | Prevents stack exhaustion from deep nesting |
| `max_document_length` | 64 MB | Rejects oversized input |
| `max_alias_expansions` | 1,024 | Prevents billion-laughs amplification |
| `max_mapping_keys` | 65,536 | Caps mapping size |
| `max_sequence_length` | 65,536 | Caps sequence size |

Use `ParserConfig::strict()` for a hardened preset suitable for untrusted input.

### Supply Chain

- 3 runtime dependencies: `serde`, `indexmap`, `thiserror`.
- `cargo-deny` enforced in CI: license validation, advisory checks, source verification.
- `Cargo.lock` committed for deterministic builds.
- All GitHub Actions SHA-pinned.

### Commit Integrity

All commits on the main branch must be signed. CI rejects unsigned pull request commits.
