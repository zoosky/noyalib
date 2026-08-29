# noyalib -- Accent's fork (zoosky/noyalib)

This checkout is the `zoosky/noyalib` fork of `sebastienrousseau/noyalib`,
vendored beside `accentcms/` as `../dependencies/noyalib/`. Accent CMS pins a
commit of this fork in its `Cargo.toml`; the tracking epic on the Accent side
is E042 (`accentcms/specs/epics/e042-rust-native-spine-retrofits.md`), and
the fork's own work is spec f334 there.

## What this fork is for

- **A stable pin.** Accent depends on `noyalib` at an exact commit on the
  `accent` branch, so an upstream release cannot move under it.
- **Carrying PR branches ahead of upstream.** Fixes Accent needs are
  developed here on `feat/*` branches, proposed to upstream as PRs, and
  merged into `accent` by merge commit so Accent can pin them before
  upstream merges. When upstream merges, the next sync replaces the fork
  commit with the upstream one.

The fork is **not** where work is tracked. Issues go to upstream
(`sebastienrousseau/noyalib`; Zoosky is a contributor there). The fork's
issue tracker is disabled on purpose.

## Branches

- `main` -- mirrors `upstream/main`; never commit here directly.
- `accent` -- what Accent pins. Starts at the upstream release tag Accent
  adopted (`v0.0.28`, commit `0a0c75f`) and advances only by merge commits:
  upstream syncs, and the fork's own `feat/*` branches once their upstream
  PR is open.
- `feat/<topic>` -- one per upstream PR, named per upstream's
  `CONTRIBUTING.md` (`feat/`, `fix/`, `docs/`, ... prefixes).

## Upstream sync workflow

Monthly, or whenever upstream lands something Accent wants:

```bash
git fetch upstream --tags
git checkout accent
git checkout -b chore/merge-upstream-<yyyy-mm-dd>-<short-sha>
git merge upstream/main              # NOT --squash, NOT --rebase
# resolve conflicts; cargo +1.97.1 test -p noyalib --features validate-schema
git push -u origin HEAD
gh pr create --repo zoosky/noyalib --base accent \
  --title "chore(sync): merge upstream <short-sha> (<N> commits, <yyyy-mm-dd>)"
```

Merge the PR with **"Create a merge commit"**. Squash-merge and
rebase-merge destroy the upstream parent pointer and orphan the upstream
commits: the patches stay in the tree, but `git log accent..upstream/main`
reports the commits as missing forever (see
`../accent-mmdr/CLAUDE.md`, "Historical note: PR #11"). The fork's
repository settings still allow squash and rebase merging; until that is
turned off at the GitHub level, the rule is honoured by hand.

After a sync, Accent's pin moves in an Accent PR whose gate is the f183
characterization harness (`accentcms/tests/serde_yaml_swap_characterization.rs`)
plus the model corpus.

## Outbound PRs to upstream

Upstream's `CONTRIBUTING.md` is the contract; the parts that bite:

- **Every commit signed** (`git commit -S`). Unsigned commits are not
  merged. The signing key lives only on the MacBook Pro; the Mac Studio has
  no `commit.gpgsign`, so author the PR commits there, or re-commit them
  with `-S` there before opening the PR.
- Conventional Commits; the branch prefix matches the change kind.
- Tests in the same PR as the behaviour change, never as a follow-up.
- `make` (check, clippy, test), `make fmt`, `make deny` green.
- An ADR under `doc/adr/` (template `doc/adr/TEMPLATE.md`) for anything
  that touches the public API surface, the parse output shape, the
  dependency floor, or the unsafe policy.
- Open the PR against upstream `main` and link the upstream issue.

```bash
git checkout accent
git checkout -b feat/<topic>
# ... signed commits ...
git push -u origin feat/<topic>
gh pr create --repo sebastienrousseau/noyalib --base main \
  --head zoosky:feat/<topic> --title "feat: ..." --body "Closes #NNN ..."
# then, so Accent can pin it before upstream merges:
gh pr create --repo zoosky/noyalib --base accent --head feat/<topic> \
  --title "chore(accent): carry feat/<topic> ahead of upstream"
```

Open upstream issues Accent filed: #327 (parent-creating `set_path`),
#328 (typed collection `set_value`), #329 (`CompiledSchema`).

## Toolchain

The repo's `rust-toolchain.toml` selects `stable`; upstream's MSRV is
1.86.0. Accent builds and tests the fork with its own pinned toolchain:

```bash
cargo +1.97.1 build -p noyalib --features validate-schema
cargo +1.97.1 test  -p noyalib --features validate-schema
```

## Conventions

- No Accent-specific patches beyond the `feat/*` branches described above.
  Anything else is proposed upstream first and cherry-picked.
- Never commit from the Accent session's working directory; `cd` here first
  so the commit lands in this repository (the two repos have separate
  `origin` remotes).
- The tag comparison Accent ran at adoption: `cargo package --list` in
  `crates/noyalib` at `v0.0.28` matches the published crate's 715 files
  exactly.
