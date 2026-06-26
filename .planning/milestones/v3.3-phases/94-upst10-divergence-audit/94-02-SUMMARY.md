---
phase: 94-upst10-divergence-audit
plan: "02"
subsystem: planning-docs
tags: [upstream-relocation, git-remote, project-meta]
dependency_graph:
  requires: []
  provides: [nolabs-ai/nono-upstream-remote, PROJECT.md-future-cycles-stub]
  affects: [.planning/PROJECT.md, git-remote-config]
tech_stack:
  added: []
  patterns: [git-remote-rename, upstream-relocation-provenance]
key_files:
  created: []
  modified:
    - .planning/PROJECT.md
decisions:
  - "D-06: upstream remote repointed to nolabs-ai/nono; always-further/nono retained as upstream-legacy"
  - "D-07: PROJECT.md Upstream Parity Process canonical-source line added (nolabs-ai/nono)"
  - "D-08: Future Cycles trigger = next v* tag past v0.65.1 from nolabs-ai/nono (not drift-count, not time-based)"
metrics:
  duration: "~10 minutes"
  completed: "2026-06-25"
  tasks_completed: 2
  tasks_total: 2
  files_changed: 1
---

# Phase 94 Plan 02: Upstream Relocation Record Summary

**One-liner:** Git upstream remote repointed from always-further/nono to nolabs-ai/nono with provenance-safe legacy rename; PROJECT.md Upstream Parity Process annotated with canonical source and next-cycle trigger stub.

## What Was Built

This plan recorded the upstream relocation (`always-further/nono` → `nolabs-ai/nono`) via two config/docs-only changes. No source, build, or test files were modified.

### Task 1: Repoint upstream remote (D-06)

- Renamed `upstream` → `upstream-legacy` (retaining `https://github.com/always-further/nono.git`)
- Added `upstream` → `https://github.com/nolabs-ai/nono.git` (new canonical)
- Final state: three remotes — `origin` / `upstream` (nolabs-ai) / `upstream-legacy` (always-further)
- Config-only change; no fetch, cherry-pick, or build

### Task 2: PROJECT.md edits (D-07, D-08)

In `.planning/PROJECT.md` `## Upstream Parity Process`:

- Added canonical-source line: `nolabs-ai/nono` named as the upstream this process tracks (D-07)
- No `always-further/nono` introduced in the section (regression guard satisfied)
- Historical milestone footers left intact (they legitimately name always-further/nono as point-in-time records)
- Added `### Future Cycles` subsection (D-08):
  - High-water mark: v0.65.1 (SHA `1d1c88c9`); Phase 94 window fully audited
  - Next trigger: next `v*` tag past v0.65.1 from nolabs-ai/nono (observable via `git ls-remote --tags`)
  - Explicitly states: next-`v*`-tag, NOT drift-count, NOT time-based
  - Notes the drain-then-sync per-tag-window cadence

## Commits

| Task | Commit | Type | Description |
|------|--------|------|-------------|
| 1 | `b5572aa9` | chore | Repoint upstream remote to nolabs-ai/nono; retain upstream-legacy |
| 2 | `f4bd6877` | docs | Update PROJECT.md Upstream Parity Process + Future Cycles stub |

## Verification Results

- `git remote get-url upstream` → `https://github.com/nolabs-ai/nono.git` ✓
- `git remote get-url upstream-legacy` → `https://github.com/always-further/nono.git` ✓
- `git remote -v` shows exactly three remotes (origin, upstream, upstream-legacy) ✓
- Section awk-range contains `nolabs-ai/nono` ✓
- Section awk-range contains no `always-further/nono` ✓
- `Future Cycles` stub present ✓
- `v0.65.1` high-water mark recorded ✓
- No source/build/test files modified ✓

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Removed always-further/nono from section canonical-source line**
- **Found during:** Task 2 automated verify gate
- **Issue:** First draft of the canonical-source line included "(relocated from `always-further/nono`; see Phase 94, D-06)" — this introduced exactly the string the acceptance criteria prohibits in the section.
- **Fix:** Reworded to "(see Phase 94, D-06 for relocation record)" — the relocation fact is preserved via the phase cross-reference without embedding the legacy org string in the section body.
- **Files modified:** `.planning/PROJECT.md`
- **Commit:** `f4bd6877` (the fix was incorporated before committing Task 2)

## Known Stubs

None — this plan's deliverables are complete as-is. The `### Future Cycles` section is intentionally a stub (it is a trigger-condition record, not a deferral of unfinished work).

## Threat Flags

None — no new network endpoints, auth paths, file access patterns, or schema changes introduced. The remote rename is a config-only change; T-94-04 (typosquat guard on exact org string) is satisfied by the verified URL `https://github.com/nolabs-ai/nono.git`.

## Self-Check: PASSED

- `.planning/PROJECT.md` modified and committed at `f4bd6877` ✓
- Commit `b5572aa9` exists (Task 1 remote config) ✓
- Commit `f4bd6877` exists (Task 2 PROJECT.md) ✓
- `git remote get-url upstream` → nolabs-ai/nono ✓
- `git remote get-url upstream-legacy` → always-further/nono ✓
- No STATE.md or ROADMAP.md modified (orchestrator owns those) ✓
