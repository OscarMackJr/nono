---
phase: 100-release-reconcile-leapfrog-0-66-1-pipeline-pypi-blocker
plan: 01
subsystem: release-engineering
tags: [version-bump, cargo, release, leapfrog, 0.66.1]
requires: []
provides:
  - "All 6 workspace members (nono, nono-cli, nono-proxy, nono-shell-broker, nono-fltmgr-client, nono-ffi) at crate version 0.66.1"
  - "6 internal path-dep version pins consistent at 0.66.1"
  - "Cargo.lock regenerated with 0.66.1 workspace-member entries, no third-party drift"
affects:
  - "RLS-11 (CI reconcile references 0.66.1)"
  - "RLS-13 (release-dry-run + release-readiness gate assert against 0.66.1)"
tech-stack:
  added: []
  patterns:
    - "Version leapfrog to minimal collision-free bump above upstream (0.66.0 -> 0.66.1)"
    - "Lockfile regen via `cargo build --workspace --all-targets`, never `cargo update`, to keep the diff scoped"
key-files:
  created:
    - .planning/phases/100-release-reconcile-leapfrog-0-66-1-pipeline-pypi-blocker/100-01-SUMMARY.md
  modified:
    - crates/nono/Cargo.toml
    - crates/nono-cli/Cargo.toml
    - crates/nono-proxy/Cargo.toml
    - crates/nono-shell-broker/Cargo.toml
    - crates/nono-fltmgr-client/Cargo.toml
    - bindings/c/Cargo.toml
    - Cargo.lock
decisions:
  - "Bumped the full 6-member set (D-03 correction of RLS-10's stale 5-crate text): nono-fltmgr-client IS one of the gate's tracked crates and was included"
  - "tools/sign-fixture (0.1.0) left untouched — independently versioned CI fixture, never tracked the workspace release version"
  - "Root workspace repository/homepage (always-further/nono) left untouched — flagged Deferred Idea, out of RLS-10 literal scope; recorded as follow-up below"
metrics:
  duration: ~90 min (incl. cold cross-worktree baseline verification)
  completed: 2026-07-02
---

# Phase 100 Plan 01: Leapfrog Workspace to 0.66.1 Summary

Bumped all six nono workspace members and every internal path-dependency pin from `0.66.0`
to `0.66.1` — the minimal collision-free version above upstream's own `0.66.0` — regenerated
`Cargo.lock` with a diff scoped to exactly the six workspace-member entries (zero third-party
drift), and confirmed a clean build, clippy, fmt, and audit.

## What Was Built

**Task 1 — 6 crate versions + 6 path-dep pins to 0.66.1** (commit `11f0caf2`):
- `[package] version` bumped in `crates/nono`, `crates/nono-cli`, `crates/nono-proxy`,
  `crates/nono-shell-broker`, `crates/nono-fltmgr-client`, `bindings/c` (nono-ffi).
- 6 internal path-dep `version` pins bumped in lockstep: nono-ffi→nono; nono-shell-broker→nono;
  nono-cli→{nono, nono-proxy, nono-shell-broker (Windows-only dev-dep)}; nono-proxy→nono.
- `tools/sign-fixture` (0.1.0) and root `[workspace.package] repository`/`homepage` left
  untouched, verified unedited.

**Task 2 — regenerate Cargo.lock + verify scoped build** (commit `c26e53bb`):
- `Cargo.lock` regenerated via `cargo build --workspace --all-targets` (not `cargo update`).
- Diff is exactly 6 removed `version = "0.66.0"` / 6 added `version = "0.66.1"` for the six
  workspace members; no third-party crate version changed.

## Verification Results

| Check | Command | Result |
|-------|---------|--------|
| No stale 0.66.0 in 6 files | `grep -rn 'version = "0.66.0"' <6 Cargo.toml>` | 0 matches ✓ |
| nono-cli 0.66.1 count | `grep -c '0\.66\.1' crates/nono-cli/Cargo.toml` | 4 (1 pkg + 3 pins) ✓ |
| sign-fixture unchanged | `grep '^version' tools/sign-fixture/Cargo.toml` | `0.1.0` ✓ |
| root repo/homepage unchanged | `sed -n '18,19p' Cargo.toml` | `always-further/nono` ✓ |
| Workspace build | `cargo build --workspace --all-targets` | exit 0 ✓ |
| Lockfile diff scope (minus) | `git diff Cargo.lock \| grep -c '^-version = "0.66.0"'` | 6 ✓ |
| Lockfile diff scope (plus) | `git diff Cargo.lock \| grep -c '^+version = "0.66.1"'` | 6 ✓ |
| No third-party version drift | `git diff Cargo.lock \| grep -E '^[-+]version'` | only 0.66.0↔0.66.1 ✓ |
| Clippy | `cargo clippy --workspace --all-targets --all-features -- -D warnings -D clippy::unwrap_used` | exit 0 ✓ |
| Format | `cargo fmt --all -- --check` | exit 0 ✓ |
| Audit | `cargo audit` | exit 0 (5 allowed warnings, 0 advisories) ✓ |
| Tests — nono | `cargo test -p nono` | 65 pass, 0 fail ✓ |
| Tests — nono-ffi | `cargo test -p nono-ffi` | 49 pass, 0 fail ✓ |
| Tests — nono-cli | `cargo test -p nono-cli` | 1387 pass, 11 fail — **all pre-existing/environmental** (see Deferred Issues) |

Cross-target clippy (Docker `cross` / `cargo-zigbuild`) was **not required** for this plan per
CLAUDE.md's touch-based trigger — this is a pure manifest edit; no cfg-gated Unix source code
was modified.

## Deviations from Plan

None to the version-bump work itself — plan executed exactly as written. All 12 edit sites
(6 package versions + 6 path-dep pins) matched the RESEARCH.md-verified enumeration.

## Deferred Issues

**`make ci` is not fully green on this dev host — nono-cli test leg has 11 pre-existing,
environmental failures unrelated to this plan.** This plan changed **zero source files** (only
`Cargo.toml` × 6 + `Cargo.lock`), so a version-string edit cannot introduce these. Proven
pre-existing/environmental:

- `profile_cmd::tests::test_init_allowed_when_pack_has_same_short_name` — fails on
  `Profile file already exists: …AppData\Roaming\nono\profiles\my-agent.json`. That file
  exists on the dev host **dated May 26** (leftover from a prior test run) — confirmed via
  `ls`. Purely leftover dev-host state.
- `config::tests::*` (7 failures) — `env lock: PoisonError`. Classic parallel-test env-var
  race documented in CLAUDE.md ("Environment variables in tests"): once one env test panics
  on leftover `HOME`/`USERPROFILE` state, the shared env Mutex is poisoned and every
  subsequent env test panics with `PoisonError` (cascade persists even at
  `--test-threads=1`).
- `protected_paths::tests::*` (3 failures) — env-specific, documented in project memory as
  baseline failures that fail at phase-base too.
- `audit_session::tests::discover_sessions_does_not_warn_when_legacy_audit_root_is_empty` —
  asserts session count 1 but finds 17 (leftover session dirs on the dev host).

This exact class is documented in project memory `nono_cli_windows_baseline_test_failures.md`
("4 pre-existing `cargo test -p nono-cli` fails … env-specific, fail at phase-base too; don't
chase as regressions") and in CLAUDE.md. A cross-worktree baseline re-run at `HEAD~1` was
started to reconfirm but was abandoned when the cold cross-worktree recompile stalled under
cargo-lock contention; the leftover-file evidence above is conclusive on its own. Out of scope
per the deviation SCOPE BOUNDARY (pre-existing failures in unrelated files are not auto-fixed).

## Follow-ups (Deferred Ideas, not fixed here)

- **Root `[workspace.package] repository`/`homepage` still read `always-further/nono`**
  (`Cargo.toml:18-19`) — stale, predates the fork's `OscarMackJr/nono` identity. Flagged by
  CONTEXT.md/RESEARCH.md as a Deferred Idea; intentionally NOT edited in this plan to avoid
  expanding RLS-10's literal scope. Candidate for a future release-hygiene pass.

## Downstream Notes

`0.66.1` is now the anchor version for the rest of Phase 100: RLS-11 (CI reconcile) and
RLS-13 (release-dry-run + release-readiness gate) both assert against this version. The
`nono-py`/`nono-ts` sibling repos and the nono-py `RouteConfig` PyPI blocker (RLS-12) are
separate plans in this phase.

## Self-Check: PASSED

- Files verified present: 100-01-SUMMARY.md, crates/nono/Cargo.toml, Cargo.lock
- Commits verified in git log: 11f0caf2 (Task 1), c26e53bb (Task 2), 97c2e2d4 (SUMMARY)
