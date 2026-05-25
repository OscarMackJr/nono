---
plan_id: 48-04
phase: 48
plan: 4
subsystem: linux-policy
cluster: C5
cluster_disposition: will-sync
upstream_sha_range: 1122c315..e6215f8b
upstream_commit_count: 3
baseline_sha: 3f638dc6
branch: worktree-agent-a824c9c849b7c7d63
status: COMPLETE
generated: 2026-05-25
tags: [upstream-sync, linux, landlock, deny-overlap, diagnostic, code-review-polish]
dependency_graph:
  requires: [48-01]
  provides: [C5-cherry-picks]
  affects: [crates/nono-cli/src/policy.rs, crates/nono/src/sandbox/linux.rs, crates/nono-cli/tests/deny_overlap_run.rs]
tech_stack:
  added: []
  patterns: [D-19-trailer, upstream-chronological-cherry-pick, deny-overlap-aggregation]
key_files:
  created:
    - .planning/phases/48-upst6-sync-execution/48-04-CLOSE-GATE.md
    - .planning/phases/48-upst6-sync-execution/48-04-PR-SECTION.md
    - .planning/phases/48-upst6-sync-execution/48-04-SUMMARY.md
  modified:
    - crates/nono/src/sandbox/linux.rs
    - crates/nono-cli/src/policy.rs
    - crates/nono-cli/tests/deny_overlap_run.rs
decisions:
  - Upstream-chronological apply order: 1122c315 (May 14) -> 4fa9f6a6 (May 16) -> e6215f8b (May 16)
  - linux.rs conflict (empty HEAD vs upstream rename): accepted upstream new test body (Plan 48-01 deleted old test; upstream renames it; additive resolution)
  - validate_deny_overlaps per-deny warn! removed (C5-02): quieting preserves fatal Err path; Phase 44 D-44-C1 OR-assertion composes with new per-deny format check
  - PREVIEW_LIMIT=5 constant (C5-03): replaces inline count/first/more arithmetic with named constant plus full preview list format
lane_transitions: []
skipped_gates_environmental:
  - Gate 3: cross-target Linux clippy (x86_64-unknown-linux-gnu cross-toolchain not installed on macOS dev host; deferred to CI)
skipped_gates_preexisting_debt:
  - Gate 4: macOS clippy (-D warnings) blocked by 8 pre-existing Class-B errors in session_commands.rs/format_util.rs; zero new errors from C5
phase_41_class_d_test_status: green
pr_section: .planning/phases/48-upst6-sync-execution/48-04-PR-SECTION.md
metrics:
  duration_minutes: ~90
  completed: 2026-05-25
  tasks_completed: 4
  files_modified: 3
  files_created: 3
  commits: 4
---

# Phase 48 Plan 04: Cluster C5 — Linux Policy + Landlock Deny-Overlap Diagnostic Polish Summary

Landed 3 upstream v0.55.0 commits (Cluster C5) onto the Phase 48 Wave 2 worktree: moved `open_port 0` rejection to unconditional position in `apply_with_abi`; quieted per-deny `warn!` spam in `validate_deny_overlaps` by aggregating conflicts into the fatal error with a full 5-item preview list; added regression assertion confirming old per-deny format is absent.

## Cherry-pick Manifest

| # | Upstream SHA | Fork SHA | Subject |
|---|-------------|----------|---------|
| 1 | `1122c315` | `b5164769` | fix: code review (sandbox/linux.rs — move port-0 early-return; rename test) |
| 2 | `4fa9f6a6` | `726d8380` | cli: quiet Landlock deny-overlap diagnostics on Linux |
| 3 | `e6215f8b` | `0cea214b` | review fix (PREVIEW_LIMIT=5; full preview list with overflow indicator) |

## Tasks Completed

| Task | Description | Commit | Files |
|------|-------------|--------|-------|
| 1 | Cherry-pick C5-01 (1122c315) — sandbox/linux.rs code review | b5164769 | crates/nono/src/sandbox/linux.rs |
| 2 | Cherry-pick C5-02 (4fa9f6a6) — diagnostic quieting | 726d8380 | crates/nono-cli/src/policy.rs, crates/nono-cli/tests/deny_overlap_run.rs |
| 3 | Cherry-pick C5-03 (e6215f8b) — PREVIEW_LIMIT=5 review fix | 0cea214b | crates/nono-cli/src/policy.rs |
| 4 | Close-gate matrix + PR section + SUMMARY | f6b63094 + final | .planning/phases/48-upst6-sync-execution/ |

## Key Changes

### C5-01: sandbox/linux.rs — port-0 check moved earlier

Moves the `open_port 0` (localhost TCP wildcard) rejection from inside the Landlock-net-capable ABI guard to the top level of `apply_with_abi`. Rejection now fires for any restricted network mode, not only Landlock-net-enabled ABIs. The associated test is renamed `test_reject_localhost_port_wildcard_zero_on_linux` and the ABI-level early-return guard is removed from the test body.

### C5-02: policy.rs — validate_deny_overlaps diagnostic quieting

Removes per-conflict `warn!("Landlock cannot enforce deny '{}'...")` calls from `validate_deny_overlaps`. Conflicts are now collected in `fatal_conflicts: Vec<String>` and summarized in a single fatal `SandboxInit` error. Also adds a regression assertion to `deny_overlap_run.rs` that confirms the old per-deny format is absent from stderr.

### C5-03: policy.rs — PREVIEW_LIMIT=5 constant + full preview list

Replaces the inline count/first/more arithmetic with:
- `const PREVIEW_LIMIT: usize = 5` named constant
- Full preview list (up to 5 conflicts with `"- {conflict}"` format)
- `remainder` overflow line (`"... and N more conflict(s)"`)

## Deviations from Plan

### Auto-fixed Issues

None — plan executed as written except for one conflict resolved per the plan's documented conflict resolution table.

### Conflict Resolutions

**1. linux.rs test section conflict (C5-01)**
- **Found during:** Task 1 cherry-pick
- **Issue:** HEAD had no test at that location (Plan 48-01 deleted `test_reject_localhost_port_wildcard_zero_under_landlock_net`). Upstream commit 1122c315 tried to rename that test. Cherry-pick created a conflict.
- **Fix:** Accepted upstream's new test body (`test_reject_localhost_port_wildcard_zero_on_linux` with unconditional network-mode check). Both histories agree the old test is gone; the new test is additive and correct.
- **Files modified:** crates/nono/src/sandbox/linux.rs
- **Commit:** b5164769

## Fork-Invariant Preservation

**PATTERNS.md row #7 — Phase 41 Class D deny-overlap protection:** PRESERVED. `validate_deny_overlaps` still returns `Err(NonoError::SandboxInit)` on any overlap; C5 changes only diagnostic format. Phase 41 regression test (`deny_overlap_run.rs`) passes (Linux-only, 0/0 on macOS as expected). Phase 44 D-44-C1 OR-assertion composes with new per-deny format check.

**PATTERNS.md row #1 — sandbox/linux.rs strictly allow-list:** PRESERVED. The moved `open_port 0` check is an early-return input-validation error (refuses to start with invalid config), not a Landlock deny rule. No deny-style code path introduced.

**D-48-E1 — Windows-only files invariant:** PRESERVED. Zero files touched under exec_strategy_windows/, nono-shell-broker/, or *_windows.rs suffix.

## Security Posture

- **T-48-04-01 (diagnostic quieting masks regression):** Mitigated. `validate_deny_overlaps` Err path preserved; Phase 41 Class D regression test re-run after each cherry-pick; Gate 5 PASS.
- **T-48-04-02 (sandbox/linux.rs allow-list regression):** Mitigated. Moved check is a pre-condition guard (input validation), not a Landlock deny rule; PATTERNS.md row #1 satisfied; Gate 6 PASS.

## Gate Summary

| Gate | Description | Result |
|------|-------------|--------|
| 1 | D-19 trailer completeness | PASS |
| 2 | Build clean (macOS) | PASS |
| 3 | Cross-target Linux clippy | PARTIAL (_environmental — cross-toolchain not installed) |
| 4 | Cross-target macOS clippy | PARTIAL (pre-existing Class-B debt; zero new C5 errors) |
| 5 | Phase 41 Class D deny-overlap regression | PASS (0/0 on macOS; protection invariant preserved) |
| 6 | PATTERNS.md row #1 allow-list invariant | PASS |
| 7 | Windows-only files invariant | PASS |
| 8 | Test suite (baseline comparison) | PASS (pre-existing 1 failure unchanged) |
| 9 | Baseline-aware CI (Pattern H) | DEFERRED (operator push to pre-merge required) |

**Overall: PASS** — all load-bearing gates pass; PARTIAL/DEFERRED gates are environmental or pre-existing Class-B debt not introduced by C5.

## Known Stubs

None.

## Threat Flags

None — C5 touches existing Linux policy code only. No new network endpoints, auth paths, file access patterns, or schema changes at trust boundaries.

## Self-Check: PASSED

- b5164769 exists in worktree log: FOUND
- 726d8380 exists in worktree log: FOUND
- 0cea214b exists in worktree log: FOUND
- f6b63094 exists in worktree log: FOUND
- 48-04-CLOSE-GATE.md: FOUND
- 48-04-PR-SECTION.md: FOUND
- crates/nono/src/sandbox/linux.rs: modified in b5164769
- crates/nono-cli/src/policy.rs: modified in 726d8380 + 0cea214b
- crates/nono-cli/tests/deny_overlap_run.rs: modified in 726d8380
