# Quick task 260522-di8 — Deferred Items

## Pre-existing rustfmt drift in 34 locations (out of scope for mechanical version bump)

**Discovered during:** Gate 6 (`cargo fmt --all -- --check`)
**Scope rule applied:** Pre-existing warnings/formatting in files NOT modified by this task.

Verified at HEAD `775bd2c8` (before any 260522-di8 edits) via `git stash` → re-run
`cargo fmt --all -- --check` → 34 diff hunks reported (identical count after `git stash pop`).
This task's edits touched only Cargo.toml / .wxs / CHANGELOG.md, none of which rustfmt
processes — so this task introduced ZERO new fmt drift.

### Affected files (34 hunks across 13 files)

- `crates/nono/src/error.rs` (1 hunk)
- `crates/nono-cli/src/exec_strategy.rs` (1 hunk)
- `crates/nono-cli/src/learn.rs` (1 hunk)
- `crates/nono-cli/src/package_cmd.rs` (1 hunk)
- `crates/nono-cli/src/platform.rs` (1 hunk)
- `crates/nono-cli/src/profile/mod.rs` (5 hunks)
- `crates/nono-cli/src/trust_cmd.rs` (5 hunks)
- `crates/nono-cli/src/trust_refresh.rs` (12 hunks)
- `crates/nono-cli/tests/auto_pull_e2e_linux.rs` (5 hunks)
- `crates/nono-cli/tests/deny_overlap_run.rs` (1 hunk)
- `crates/nono-cli/tests/resl_nix_linux.rs` (1 hunk)

### Recommended disposition

Run `cargo fmt --all` as a separate `style:` commit (or fold into the next `chore:` rollup).
This is unrelated to the v0.53.1 mechanical bump and should not pollute the version-bump commit.
