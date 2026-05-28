---
phase: 53-release-drain
plan: "01"
subsystem: workspace-versioning
tags: [version-bump, cargo, release-prep]
dependency_graph:
  requires: []
  provides: [workspace-at-0.57.4]
  affects: [Cargo.lock, all-5-crates]
tech_stack:
  added: []
  patterns: [semver version bump, path-dep pin bump]
key_files:
  created: []
  modified:
    - crates/nono/Cargo.toml
    - crates/nono-cli/Cargo.toml
    - crates/nono-proxy/Cargo.toml
    - crates/nono-shell-broker/Cargo.toml
    - bindings/c/Cargo.toml
    - Cargo.lock
decisions:
  - "Bump all 5 crate versions and all 6 internal path-dep pins atomically in a single commit to avoid resolver errors during intermediate states"
metrics:
  duration: ~5 minutes
  completed: "2026-05-28"
  tasks_completed: 1
  tasks_total: 1
requirements:
  - REQ-RLS-01
---

# Phase 53 Plan 01: Workspace Version Bump (0.57.3 → 0.57.4) Summary

## One-liner

Bumped all 5 crate version fields and all 6 internal path-dep pins from 0.57.3 to 0.57.4 so v2.8 CI artifacts and `nono --version` output are unambiguously distinct from the stale v0.57.3 local MSIs.

## What Was Built

A version-string-only change across the Cargo workspace:
- `crates/nono/Cargo.toml` — crate version 0.57.3 → 0.57.4
- `crates/nono-proxy/Cargo.toml` — crate version + nono path-dep pin 0.57.3 → 0.57.4
- `crates/nono-shell-broker/Cargo.toml` — crate version + nono path-dep pin 0.57.3 → 0.57.4
- `bindings/c/Cargo.toml` — crate version + nono path-dep pin 0.57.3 → 0.57.4
- `crates/nono-cli/Cargo.toml` — crate version + nono + nono-proxy + nono-shell-broker path-dep pins (4 changes total)
- `Cargo.lock` — regenerated automatically by cargo check

## Tasks

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Bump all 5 crate version fields from 0.57.3 to 0.57.4 | 75f732a7 | 5 Cargo.toml + Cargo.lock |

## Verification

- `grep -r "0.57.3" crates/*/Cargo.toml bindings/c/Cargo.toml` → zero matches
- `grep "version = \"0.57.4\"" ...` → 5 crate-level matches (line 3 of each file)
- All 6 path-dep pins read 0.57.4
- `cargo check --workspace` → `Finished dev profile` with no errors

## Deviations from Plan

None - plan executed exactly as written.

## Known Stubs

None.

## Threat Flags

None — this plan performs version-string-only edits to Cargo.toml files. No new network endpoints, auth paths, file access patterns, or schema changes introduced.

## Self-Check: PASSED

- [x] crates/nono/Cargo.toml modified — confirmed version = "0.57.4" at line 3
- [x] crates/nono-cli/Cargo.toml modified — confirmed version = "0.57.4" + 3 path-dep pins at 0.57.4
- [x] crates/nono-proxy/Cargo.toml modified — confirmed version = "0.57.4" + nono path-dep at 0.57.4
- [x] crates/nono-shell-broker/Cargo.toml modified — confirmed version = "0.57.4" + nono path-dep at 0.57.4
- [x] bindings/c/Cargo.toml modified — confirmed version = "0.57.4" + nono path-dep at 0.57.4
- [x] Commit 75f732a7 exists with DCO sign-off
- [x] cargo check --workspace exits 0
