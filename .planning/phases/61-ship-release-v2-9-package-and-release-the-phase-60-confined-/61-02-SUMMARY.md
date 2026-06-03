---
phase: 61-ship-release-v2-9-package-and-release-the-phase-60-confined
plan: 02
subsystem: security
tags: [windows, hook, cwd-guard, deny-dotclaude, claude-code, sandbox, release-notes]

# Dependency graph
requires:
  - phase: 60-confined-coding-loop
    provides: Phase 60-03 hook-level CWD guard implementation (ddb711dc, fe832dfc, 309c94a4)
provides:
  - D-09 verification record with test results, scope documentation, and release-notes paragraph
  - Resolved pending todo (moved pending → done with Phase 60-03 resolution note)
affects: [61-04-release-notes, future-release-plans]

# Tech tracking
tech-stack:
  added: []
  patterns: ["verify-and-document pattern: run existing tests, write scoped verification record, resolve originating todo"]

key-files:
  created:
    - ".planning/phases/61-ship-release-v2-9-package-and-release-the-phase-60-confined-/61-D09-VERIFICATION.md"
    - ".planning/todos/done/2026-05-29-claude-tools-runner-deny-dotclaude-regardless-of-cwd.md"
  modified: []

key-decisions:
  - "D-09 is hook-layer enforcement (not OS-level deny-within-allow): the guard at claude_code_hook.rs:204 fires before any --allow-cwd runner command is emitted"
  - "Bare-CLI gap (nono run --allow-cwd ~/.claude outside hooked loop) accepted as T-61-04, documented limitation, not a Phase 61 code task"
  - "Windows OS label backend has no deny-within-allow primitive; add_deny_access is a no-op for the allow-overlap case on Windows"

patterns-established:
  - "verify-and-document plan: test first, document results + scope + limitation, resolve originating tracking artifact"

requirements-completed: [REQ-RLS-04]

# Metrics
duration: 15min
completed: 2026-06-03
---

# Phase 61 Plan 02: D-09 deny-`~/.claude` Hook Guard Verification Summary

**D-09 pre-ship security blocker proven closed: all 16 hook guard tests pass on v0.58.0, with honest hook-layer scope and bare-CLI limitation documented for release notes**

## Performance

- **Duration:** ~15 min
- **Started:** 2026-06-03T~16:30:00Z
- **Completed:** 2026-06-03
- **Tasks:** 2
- **Files modified:** 2 created + 1 renamed (todo moved pending → done)

## Accomplishments

- Ran `cargo test -p nono-cli --bin nono -- claude_code_hook` on the v0.58.0 build: 16/16 tests pass, including all CWD guard assertions (`windows_cwd_guard_*`, `pre_tool_use_file_tools_deny`, `windows_write_arm_cwd_guard_fires_before_ps_cmd`)
- Created `61-D09-VERIFICATION.md` documenting enforcement location (claude_code_hook.rs:204), Phase 60-03 commits, test results, honest scope (hook-layer only), bare-CLI documented limitation, and a ready-to-paste release-notes paragraph for plan 61-04
- Moved the originating pending todo to `todos/done/` via `git mv` with a Phase 60-03 resolution note citing commits `ddb711dc`, `fe832dfc`, `309c94a4`

## Task Commits

Task 1 (test run verification) produced no file changes — verification only, no source modifications.

1. **Task 2: Write D-09 verification record + resolve pending todo** - `5245e964` (docs)

**Plan metadata:** (final commit — this SUMMARY + STATE/ROADMAP)

## Files Created/Modified

- `.planning/phases/61-ship-release-v2-9-package-and-release-the-phase-60-confined-/61-D09-VERIFICATION.md` — D-09 closure record: enforcement location, test results, honest scope framing, bare-CLI limitation, release-notes paragraph
- `.planning/todos/done/2026-05-29-claude-tools-runner-deny-dotclaude-regardless-of-cwd.md` — moved from `pending/` with Phase 60-03 resolution note appended

## Decisions Made

- **D-09 is hook-layer enforcement.** The guard at `claude_code_hook.rs:204` fires before any `--allow-cwd` runner command is emitted, using `path_covers` (component comparison, not string `starts_with`) to prevent the CR-01 fail-open. This is the correct enforcement boundary for the Windows label backend, which lacks deny-within-allow semantics.
- **Bare-CLI gap accepted as T-61-04 (documented limitation, not a Phase 61 code task).** A direct `nono run --allow-cwd ~/.claude` invocation outside the hooked Claude loop bypasses the hook. Resolving this would require an OS deny-within-allow primitive (unavailable on Windows) or a kernel minifilter (v3.0 deferral). Consistent with the Phase 60 "defense-in-depth, not full isolation" verdict.

## Deviations from Plan

None — plan executed exactly as written. Task 1 discovered the test command needed `--bin nono` instead of `--lib` (nono-cli has no lib target), but this is a test-invocation detail, not a deviation from plan intent. All 16 tests passed on first run.

## Issues Encountered

Minor: `cargo test -p nono-cli --lib claude_code_hook` returned "no library targets found" — nono-cli is a binary crate. Corrected to `--bin nono` on the same run. No retry needed; tests passed immediately.

## User Setup Required

None — no external service configuration required. This was a verification-only plan.

## Next Phase Readiness

- D-09 is proven closed on the 0.58.0 build — the pre-ship security blocker is satisfied.
- `61-D09-VERIFICATION.md` contains a ready-to-paste release-notes paragraph for plan 61-04 (release notes authoring).
- The deny-`~/.claude` todo is resolved; no pending items related to D-09.
- Next plan in Wave 1: 61-03 (pre-ship readiness checks — version bump, release.yml live-verify, v0.57.4 annotation).

## Threat Flags

None — this plan made no source code changes. No new network endpoints, auth paths, file access patterns, or schema changes were introduced.

## Self-Check: PASSED

- `61-D09-VERIFICATION.md`: FOUND
- `.planning/todos/done/2026-05-29-claude-tools-runner-deny-dotclaude-regardless-of-cwd.md`: FOUND
- `.planning/todos/pending/2026-05-29-claude-tools-runner-deny-dotclaude-regardless-of-cwd.md`: CONFIRMED ABSENT
- Commit `5245e964`: PRESENT (Task 2)

---
*Phase: 61-ship-release-v2-9-package-and-release-the-phase-60-confined*
*Completed: 2026-06-03*
