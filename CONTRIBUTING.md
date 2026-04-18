# Contributing to noyalib

Contributions are welcome. This guide covers the essentials.

## Prerequisites

- Rust 1.75.0 or later
- Git with [commit signing](https://docs.github.com/en/authentication/managing-commit-signature-verification/signing-commits) configured

## Getting Started

```sh
git clone https://github.com/sebastienrousseau/noyalib.git
cd noyalib
make
```

`make` runs `cargo check`, `cargo clippy`, and `cargo test` in sequence.

## Making Changes

1. Fork the repository and create a feature branch from `main`.
2. Write code. Follow existing patterns and conventions.
3. Add or update tests for every change.
4. Run the full check suite:

```sh
make          # check + clippy + test
make fmt      # verify formatting
make deny     # supply-chain audit
```

5. Commit with a signed commit (`git commit -S`).

## Commit Guidelines

- **Sign all commits.** Unsigned commits will not be merged. [Set up GPG or SSH signing.](https://docs.github.com/en/authentication/managing-commit-signature-verification)
- Write clear, concise commit messages in imperative form:
  - `fix: resolve panic on empty mapping`
  - `feat: add block scalar threshold config`
  - `test: cover duplicate key policy edge cases`
- Keep each commit focused on a single logical change.

## Pull Requests

- Open a pull request against `main`.
- Provide a short summary of what changed and why.
- Ensure CI passes (clippy, tests, formatting, supply-chain audit).
- One approval is required before merging.

## Code Standards

- `#![forbid(unsafe_code)]` is non-negotiable. No `unsafe` blocks, ever.
- All public items require documentation (`#![warn(missing_docs)]`).
- Clippy must pass with `--all-features --all-targets` and zero warnings.
- `cargo fmt --check` must pass.

## Reporting Issues

Open an issue on GitHub. Include:

- A minimal YAML input that reproduces the problem.
- Expected behavior vs. actual behavior.
- Rust version (`rustc --version`).

## License

By contributing, you agree that contributions are licensed under the same dual license as the project: [MIT](LICENSE-MIT) or [Apache 2.0](LICENSE-APACHE).
