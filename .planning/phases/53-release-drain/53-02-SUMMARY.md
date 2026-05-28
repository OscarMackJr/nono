---
phase: 53-release-drain
plan: "02"
subsystem: planning-artifacts
tags: [drain, todos, requirements, backlog, deferred]
dependency_graph:
  requires: []
  provides: [REQ-DENY-PREFLIGHT-01, REQ-UNDO-TOCTOU-01]
  affects: [.planning/REQUIREMENTS.md, .planning/todos/done/]
tech_stack:
  added: []
  patterns: [backlog-promotion, disposition-notes]
key_files:
  created:
    - .planning/todos/done/44-class-d-validator-preflight-investigation.md
    - .planning/todos/done/44-validate-restore-target-fd-relative-hardening.md
  modified:
    - .planning/REQUIREMENTS.md
decisions:
  - "D-53-08: Todos 2 and 3 are promoted to v2 Deferred backlog rather than done in-phase; security equivalence proven for preflight investigation; fd-relative TOCTOU hardening warrants dedicated security-scoped phase"
metrics:
  duration_minutes: 8
  completed_date: "2026-05-28"
  tasks_completed: 2
  tasks_total: 2
  files_changed: 3
---

# Phase 53 Plan 02: Todo Re-disposition with REQUIREMENTS.md Backlog Promotion Summary

**One-liner:** Promoted Todos 2 and 3 from pending/ to done/ with D-53-08 rationale; added REQ-DENY-PREFLIGHT-01 and REQ-UNDO-TOCTOU-01 to REQUIREMENTS.md v2 Deferred section.

## What Was Built

Satisfied REQ-DRN-02 by re-dispositioning the two outstanding carry-forward todos from the v2.7 close. Neither todo was done in-phase; both received committed written rationale (D-53-08) explaining why they belong in the backlog rather than a drain phase:

- **REQ-DENY-PREFLIGHT-01**: The deny-overlap validator preflight investigation requires a Linux host with RUST_LOG=trace + strace capability. Security equivalence is already proven by the either-or assertion in the Class D test (both paths deny the read, neither leaks the secret), making this a low-priority latent-diagnostic investigation, not an active security gap.

- **REQ-UNDO-TOCTOU-01**: Full fd-relative TOCTOU hardening of `validate_restore_target` requires ~2-3 weeks of focused cross-platform work spanning O_NOFOLLOW + openat/mkdirat/renameat/fchmodat on Linux/macOS and NtCreateFile-with-OBJ_DONT_REPARSE or defense-in-depth on Windows. A doc note was already shipped in Phase 44. Full closure warrants a dedicated security-scoped phase.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Add backlog entries to REQUIREMENTS.md v2 Deferred section | c03ec85f | .planning/REQUIREMENTS.md |
| 2 | Move todo files from pending/ to done/ with disposition notes | 5b6fb2f2 | .planning/todos/done/44-class-d-validator-preflight-investigation.md, .planning/todos/done/44-validate-restore-target-fd-relative-hardening.md |

## Verification Results

- `ls .planning/todos/pending/` — neither 44-class-d nor 44-validate-restore file remains (only the WFP UAT todo remains)
- `ls .planning/todos/done/` — both done/ files exist with Disposition (Phase 53) sections
- `grep "REQ-DENY-PREFLIGHT-01\|REQ-UNDO-TOCTOU-01" .planning/REQUIREMENTS.md` — 4 matches (2 body entries + 2 traceability rows)
- Source file references present in both backlog entries (2 matches)
- REQ-DRN-02 acceptance condition met: both todos have committed written rationale

## Deviations from Plan

None — plan executed exactly as written.

## Decisions Made

**D-53-08 (recorded):** Todos 2 and 3 are not done in-phase. Both are promoted to the REQUIREMENTS.md v2 Deferred backlog:
- Todo 2 (deny-overlap validator preflight): security equivalence proven; Linux-host-gated investigation, low-priority.
- Todo 3 (fd-relative TOCTOU hardening): standalone security-scoped phase warranted; ~2-3 weeks cross-platform effort.

## Known Stubs

None — this is a documentation-only plan with no source code changes.

## Threat Flags

None — no new network endpoints, auth paths, file access patterns, or schema changes introduced. Planning artifacts only.

## Self-Check: PASSED

- .planning/REQUIREMENTS.md — FOUND
- .planning/todos/done/44-class-d-validator-preflight-investigation.md — FOUND
- .planning/todos/done/44-validate-restore-target-fd-relative-hardening.md — FOUND
- Task 1 commit c03ec85f — FOUND
- Task 2 commit 5b6fb2f2 — FOUND
