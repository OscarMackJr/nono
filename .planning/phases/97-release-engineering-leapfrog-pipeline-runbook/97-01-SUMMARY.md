---
phase: 97-release-engineering-leapfrog-pipeline-runbook
plan: "01"
subsystem: release-engineering
tags: [version-bump, leapfrog, bindings, cargo, release]
dependency_graph:
  requires: []
  provides: [workspace-at-0.66.0, lockfile-regenerated, bindings-unified-0.66.0]
  affects: [crates/nono, crates/nono-cli, crates/nono-proxy, crates/nono-shell-broker, crates/nono-fltmgr-client, bindings/c, ../nono-py, ../nono-ts]
tech_stack:
  added: []
  patterns: [semver-leapfrog, cross-repo-version-unification]
key_files:
  created: []
  modified:
    - crates/nono/Cargo.toml
    - crates/nono-cli/Cargo.toml
    - crates/nono-proxy/Cargo.toml
    - crates/nono-shell-broker/Cargo.toml
    - crates/nono-fltmgr-client/Cargo.toml
    - bindings/c/Cargo.toml
    - Cargo.lock
    - ../nono-py/Cargo.toml (sibling repo — operator commits separately)
    - ../nono-py/pyproject.toml (sibling repo — operator commits separately)
    - ../nono-ts/package.json (sibling repo — operator commits separately)
    - ../nono-ts/Cargo.toml (sibling repo — operator commits separately)
decisions:
  - "Target version 0.66.0 chosen as first SemVer > upstream nolabs-ai/nono v0.65.1 (fork must tag past upstream's highest)"
  - "nono-ts path-dep pin uses loose minor 0.66 (not 0.66.0) to stay compatible across future patch releases"
  - "tools/sign-fixture left at 0.1.0 — independently versioned CI tool, not a release artifact"
  - "cargo update --workspace only — rewrote exactly 6 lock entries, no external crate versions changed"
metrics:
  duration: "~6 minutes"
  completed: "2026-06-26T17:05:41Z"
  tasks_completed: 2
  files_modified: 7
---

# Phase 97 Plan 01: Version Leapfrog to 0.66.0 Summary

**One-liner:** Leapfrog the entire nono version family from 0.62.2 to 0.66.0 — clearing upstream nolabs-ai/nono v0.65.1 — across all six workspace crates, seven path-dep pins, and four sibling binding repo manifests.

## What Was Built

- **Task 1:** All six version-family workspace crates (`nono`, `nono-cli`, `nono-proxy`, `nono-shell-broker`, `nono-fltmgr-client`, `nono-ffi`) bumped from 0.62.2 to 0.66.0. All six internal path-dep version pins moved in lockstep. `cargo update --workspace` regenerated Cargo.lock with exactly 6 changed entries (no external crate version drift). `cargo build --workspace --all-targets` exits 0.

- **Task 2:** Both sibling binding repos unified onto 0.66.0:
  - `../nono-py/Cargo.toml` and `../nono-py/pyproject.toml`: 0.9.0 → 0.66.0 (maturin requires both to agree)
  - `../nono-ts/package.json`: top-level version 0.4.0 → 0.66.0 plus all four `optionalDependencies` platform sub-packages (nono-ts-darwin-x64, nono-ts-darwin-arm64, nono-ts-linux-x64-gnu, nono-ts-linux-arm64-gnu) moved from 0.4.0 → 0.66.0
  - `../nono-ts/Cargo.toml`: package version 0.4.0 → 0.66.0; `nono` path-dep pin `version = "0.62"` → `version = "0.66"` (loose minor)

## Verification Results

- `grep -rn '0.62.2' crates/ bindings/c/Cargo.toml` → empty (CLEAN)
- `cargo metadata --no-deps --format-version 1` → nono, nono-cli, nono-proxy, nono-shell-broker, nono-fltmgr-client, nono-ffi each at 0.66.0; sign-fixture at 0.1.0 (unchanged)
- `grep -c '0.66.0' Cargo.lock` → 7 (6 package entries + 1 checksum reference)
- `cargo build --workspace --all-targets` → exits 0 (Finished dev profile in ~83s)
- Binding acceptance checks: nono-py 0.9.0 absent; nono-ts 0.4.0 absent from package.json; 0.62 pin absent from nono-ts Cargo.toml

## Deviations from Plan

None — plan executed exactly as written.

## Sibling Repo Commits (Operator Action Required)

The four binding repo file edits are on disk at their sibling paths but are NOT committed — they live in separate git repositories:
- `C:\Users\OMack\nono-py\Cargo.toml` — version 0.9.0 → 0.66.0
- `C:\Users\OMack\nono-py\pyproject.toml` — version 0.9.0 → 0.66.0
- `C:\Users\OMack\nono-ts\package.json` — version 0.4.0 → 0.66.0 (incl. 4 optionalDependencies)
- `C:\Users\OMack\nono-ts\Cargo.toml` — version 0.4.0 → 0.66.0; nono dep pin 0.62 → 0.66

Operator must `git add` and commit these in their respective repos before plans 02-04 proceed.

## Commits

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Leapfrog workspace crates + regenerate Cargo.lock | ebef3587 | crates/nono/Cargo.toml, crates/nono-cli/Cargo.toml, crates/nono-proxy/Cargo.toml, crates/nono-shell-broker/Cargo.toml, crates/nono-fltmgr-client/Cargo.toml, bindings/c/Cargo.toml, Cargo.lock |
| 2 | Unify sibling binding repos onto 0.66.0 | (operator commits in nono-py + nono-ts repos) | ../nono-py/Cargo.toml, ../nono-py/pyproject.toml, ../nono-ts/package.json, ../nono-ts/Cargo.toml |

## Known Stubs

None.

## Threat Flags

None — this plan makes only version-string changes to manifests. No new network endpoints, auth paths, file access patterns, or schema changes introduced.

## Self-Check: PASSED

- [x] `crates/nono/Cargo.toml` contains `version = "0.66.0"` ✓
- [x] `Cargo.lock` contains 7 occurrences of `0.66.0` ✓
- [x] `cargo build --workspace --all-targets` exits 0 ✓
- [x] `../nono-py/Cargo.toml` contains `version = "0.66.0"` ✓
- [x] `../nono-py/pyproject.toml` contains `version = "0.66.0"` ✓
- [x] `../nono-ts/package.json` contains no `"0.4.0"` ✓
- [x] `../nono-ts/Cargo.toml` contains no `"0.62"` pin ✓
- [x] Commit ebef3587 exists in git log ✓
