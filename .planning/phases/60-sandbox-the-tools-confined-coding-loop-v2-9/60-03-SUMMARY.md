---
phase: "60"
plan: "03"
subsystem: nono-cli/claude_code_hook
tags: [security, windows, hook-guard, fail-secure, path-canonicalization]
gap_closure: true
dependency_graph:
  requires: [60-01]
  provides: [path_covers-fail-secure, cwd-guard-symmetric-check, home-dir-fail-closed]
  affects: [crates/nono-cli/src/claude_code_hook.rs]
tech_stack:
  added: []
  patterns: [canonicalize-existing-prefix, fail-closed-on-none, symmetric-starts_with-check]
key_files:
  modified:
    - crates/nono-cli/src/claude_code_hook.rs
decisions:
  - "canonicalize_with_existing_prefix() walks the ancestor chain via path.ancestors() and re-appends tail components using Path component operations — no string manipulation, no .unwrap()/.expect()"
  - "if let Ok(canonical_base) = ancestor.canonicalize() used instead of .ok() to avoid clippy::match_result_ok lint"
  - "home_dir() None path returns Ok(Some(deny-reason)) not Err — the guard fires and denies rather than propagating an error up; Err propagation is reserved for canonical_existing_path() failure on a path that does exist"
  - "Pre-existing fmt issues in nono-proxy, nono::sandbox::windows, exec_strategy.rs are out of scope — make ci fmt-check fails pre-plan-03 and post-plan-03 identically on those files; claude_code_hook.rs is fmt-clean"
  - "4 pre-existing test failures (profile_cmd + protected_paths) are out of scope — same failures on main without plan-03 changes"
metrics:
  duration_minutes: 35
  completed_date: "2026-05-29"
  tasks_completed: 3
  tasks_total: 3
  files_changed: 1
---

# Phase 60 Plan 03: CR-01 path_covers() Fail-Open Fix — Summary

## One-Liner

Fix path_covers() Windows fail-open by walking the ancestor chain to canonicalize the deepest existing prefix and re-appending missing tail components, so both sides of starts_with() use the \\?\-extended-length representation; also add symmetric inside-~/.claude check (WR-01) and home-dir fail-closed path (WR-02).

## What Was Built

### Task 1: Production code fixes in claude_code_hook.rs

**canonicalize_with_existing_prefix() — new Windows-gated helper**

Walks `path.ancestors()` from the path itself up to the root. For each ancestor, if it exists on disk, calls `ancestor.canonicalize()`. On success, collects the stripped tail components (number of iterations = tail depth) and folds them back onto the canonical base using `Path::join()` (component operations, not string concatenation). Returns `path.to_path_buf()` only in the degenerate case where no ancestor can be canonicalized.

**path_covers() — CR-01 fix**

Replaced `canonical_existing_path(child).unwrap_or_else(|_| child.to_path_buf())` with `canonicalize_with_existing_prefix(child)`. The old fallback returned a raw `C:\...` path when the child didn't exist on disk; the parent was `\\?\`-canonical from `canonical_existing_path(cwd)`, so `Path::starts_with()` always returned false — guard silently failed open. The new helper ensures both sides share the same extended-length prefix representation.

**cwd_covers_home_claude_state() — WR-01 fix**

Added `if cwd.starts_with(&home_claude) { return true; }` after the existing `path_covers(cwd, &home_claude)` check. The original code only caught the case where the CWD was an ancestor-or-equal of `~/.claude`; a CWD nested inside `~/.claude` (e.g., `~/.claude/projects/foo`) was not caught. Both directions are now checked.

**cwd_self_disable_risk_reason_for() — WR-02 fix**

Replaced `if let Some(home) = home_dir().and_then(|path| canonical_existing_path(&path).ok()) { ... }` with a two-step match. When `home_dir()` returns `None`, the function now returns `Ok(Some(self_disable_reason_no_home()))` — an explicit deny — instead of silently skipping the home-state check. The `canonical_existing_path(&path)?` propagates Err upward (fail closed) if home exists but cannot be canonicalized.

**self_disable_reason_no_home() — new Windows-gated helper**

Returns the denial message: "refusing to wrap Bash: home directory cannot be resolved; cannot verify CWD does not cover Claude Code hook state".

### Task 2: Regression tests

**windows_cwd_guard_denies_home_claude_ancestor_absent_file**

Creates only the home directory (no `.claude.json`, no `.claude/`), canonicalizes it as `cwd`, and asserts `cwd_covers_home_claude_state(&cwd, &cwd)` returns true. This is the exact CR-01 scenario: file absent, guard must fire via `canonicalize_with_existing_prefix` re-constructing the `\\?\`-canonical child path from the existing home ancestor.

**windows_cwd_guard_denies_inside_home_claude**

Creates `home/.claude/projects/myrepo`, canonicalizes it as `cwd`, and asserts the guard fires. Exercises the WR-01 symmetric `cwd.starts_with(&home_claude)` check.

### Task 3: CI verification + cross-target clippy disposition

- `cargo clippy --bin nono -- -D warnings -D clippy::unwrap_used`: PASS (caught and fixed `if let Some(...)` / `.ok()` pattern → `if let Ok(...)`)
- `cargo test --bin nono claude_code_hook`: 16 tests PASS (14 prior + 2 new)
- `cargo fmt` on `claude_code_hook.rs`: PASS (applied rustfmt; committed as separate style commit)
- Pre-existing failures (nono-proxy clippy, 4 workspace tests, workspace fmt): out of scope — identical before and after these changes
- Cross-target clippy (x86_64-unknown-linux-gnu): **PARTIAL** — Linux C toolchain (`x86_64-linux-gnu-gcc`) not installed on Windows dev host; deferred to CI per CLAUDE.md cross-target checklist

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] clippy::match_result_ok lint in canonicalize_with_existing_prefix**

- **Found during:** Task 3 (cargo clippy --bin nono)
- **Issue:** `if let Some(canonical_base) = ancestor.canonicalize().ok()` triggers `clippy::match_result_ok` — lint says to use `if let Ok(canonical_base) = ancestor.canonicalize()` instead
- **Fix:** Changed to `if let Ok(canonical_base) = ancestor.canonicalize()` — equivalent semantics, no `.ok()` conversion needed
- **Files modified:** `crates/nono-cli/src/claude_code_hook.rs`
- **Commit:** ddb711dc → re-verified after fix

**2. [Rule 3 - Fmt] rustfmt reformatting of claude_code_hook.rs**

- **Found during:** Task 3 (rustfmt --check)
- **Issue:** Pre-existing fmt drift in the file (from Plan 01 baseline) plus minor formatting differences in the two new test function signatures
- **Fix:** Applied `rustfmt --edition 2021` to the file; committed as `style(60-03)` commit
- **Files modified:** `crates/nono-cli/src/claude_code_hook.rs`
- **Commit:** 309c94a4

## Cross-Target Clippy

**Disposition: PARTIAL**

Attempted: `cargo clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used`

Result: Failed with `ToolNotFound: failed to find tool "x86_64-linux-gnu-gcc"` — Linux cross-compilation toolchain not installed on Windows dev host. This is expected per CLAUDE.md cross-target checklist.

All new and modified functions in `claude_code_hook.rs` are gated under `#[cfg(target_os = "windows")]`. The Linux and macOS targets do not compile these functions at all; the cfg gate is the structural guard. Deferred to CI for live cross-target verification.

## Known Residual (T-60-03-04 — Accepted)

`home_dir()` reads `USERPROFILE`/`HOME` without ownership or identity validation. An adversarial launcher controlling these env vars could redirect the guard away from the real `~/.claude`. Per the threat model disposition `accept`: an attacker who controls the process environment already operates at Medium-IL or above — outside the Low-IL confinement boundary. Validated home resolution is out of scope for this phase.

## Self-Check

### Files

- `crates/nono-cli/src/claude_code_hook.rs` — exists and contains all changes

### Commits

- `ddb711dc` — fix(60-03): close CR-01 path_covers fail-open + WR-01/WR-02 guard gaps
- `fe832dfc` — test(60-03): add regression tests for absent-file path and inside-~/.claude CWD
- `309c94a4` — style(60-03): apply rustfmt to claude_code_hook.rs (make ci gate)

### Source Assertions

| Assertion | Result |
|-----------|--------|
| `grep -c "unwrap_or_else" path_covers` | 0 — PASS |
| `grep -c "canonicalize_with_existing_prefix"` | 2 — PASS (definition + call site) |
| `grep -c "cwd.starts_with"` | 1 — PASS |
| `grep -c "self_disable_reason_no_home"` | 2 — PASS (definition + call site) |
| `grep -c "absent_file"` | 1 — PASS |
| `grep -c "inside_home_claude"` | 1 — PASS |
| `cargo test --bin nono claude_code_hook` | 16 PASSED — PASS |
| `cargo clippy --bin nono -- -D warnings -D clippy::unwrap_used` | PASS |

## Self-Check: PASSED
