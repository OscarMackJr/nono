---
phase: 76-self-verifying-harness-foundation
plan: "01"
subsystem: scripts/harness
tags: [dark-factory, harness, powershell, gitignore]
dependency_graph:
  requires: []
  provides: [scripts/verify-dark.ps1, .nono-runtime/ gitignore rule]
  affects: [phases/77, phases/78, phases/79, phases/80, phases/81]
tech_stack:
  added: []
  patterns: [gate auto-discovery, Test-Precondition/Invoke-Gate dot-source contract, verdict JSON emit, exit-code mapping D-02]
key_files:
  created:
    - scripts/verify-dark.ps1
  modified:
    - .gitignore
decisions:
  - "param([string]$Gate, [switch]$All) with no hardcoded ValidateSet — gates auto-discovered from scripts/gates/*.ps1 glob (D-04)"
  - "Unknown gate = exit 1 (harness-internal error), not exit 2 (FAIL verdict) and not exit 0 (D-05)"
  - "Test-Precondition called before Invoke-Gate; non-null return = SKIP_HOST_UNAVAILABLE + exit 3 (D-06)"
  - "Invoke-Gate crash maps to exit 4 via try/catch, never PASS, never swallowed (D-07)"
  - "[ordered]@{gate,verdict,reason,detail,timestamp} with ConvertTo-Json -Depth 6 -Compress + [Console]::Out.Write LF (D-01)"
  - ".nono-runtime/ gitignore rule placed after ci-logs-local/ block with house comment-then-rule form"
  - "All-run loop left reusable without rollup/overall/PASS_WITH_SKIPS — deferred to Phase 81 (D-03)"
metrics:
  duration: "~25 minutes"
  completed_date: "2026-06-17"
  tasks_completed: 2
  tasks_total: 2
  files_created: 1
  files_modified: 1
---

# Phase 76 Plan 01: Dark-Factory Harness Runner Summary

**One-liner:** Dark-factory harness runner `scripts/verify-dark.ps1` with gate auto-discovery, Test-Precondition/Invoke-Gate dot-source contract, typed D-01 verdict JSON, D-02 three-way exit mapping (0/2/3, reserve 1/4+), and `.nono-runtime/verdicts/<gate>.json` persistence.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Add .nono-runtime/ gitignore rule | d76a5a0f | .gitignore |
| 2 | Write verify-dark.ps1 runner | 283ec8c6 | scripts/verify-dark.ps1 |

## Verification Results

| Check | Result |
|-------|--------|
| `pwsh -NoProfile -File scripts/verify-dark.ps1 -Gate does-not-exist` exits 1 | PASS (exit 1, harness-internal error) |
| SKIP stub gate produces `"verdict":"SKIP_HOST_UNAVAILABLE"` + exit 3 | PASS |
| SKIP stub verdict persisted to `.nono-runtime/verdicts/<stub>.json` | PASS |
| `git check-ignore .nono-runtime/verdicts/x.json` exits 0 | PASS (line 46 of .gitignore) |
| `git check-ignore .gitignore` exits non-zero | PASS (exit 1, no over-broad rule) |
| `[ordered]@{}` key order gate,verdict,reason,detail,timestamp | PASS |
| `ConvertTo-Json -Depth 6 -Compress` + `[Console]::Out.Write LF` | PASS (no CRLF) |

## Deviations from Plan

None — plan executed exactly as written.

## Decisions Made

1. **Gate auto-discovery without ValidateSet** — `param([string]$Gate, [switch]$All)` with runtime validation against the globbed gate list. Phases 77-80 add files, never edit the runner.

2. **Unknown gate = exit 1** — Confirmed harness-internal error semantics: print diagnostic to stderr, `exit 1`. Not a FAIL verdict (exit 2), not silent PASS (exit 0).

3. **Test-Precondition before Invoke-Gate** — Precondition returning non-null string emits SKIP_HOST_UNAVAILABLE (exit 3) without entering gate body. Invoke-Gate wrapped in try/catch; throws map to exit 4.

4. **All-run loop stub** — Minimal per-gate loop without rollup/overall/PASS_WITH_SKIPS aggregation. Phase 81 (DARK-02) formalizes the `{gates:[...], overall}` shape.

5. **`.gitignore` placement** — Rule added after `ci-logs-local/` block with two-line comment matching house style.

## Known Stubs

None — the runner is complete. The `scripts/gates/` directory is empty (untracked); it receives its first gate (`harness-self-check.ps1`) in Plan 76-02.

## Threat Surface Scan

No new network endpoints, auth paths, file access patterns, or schema changes. The runner reads from `scripts/gates/*.ps1` (repo-tracked source, accepted threat T-76-03) and writes to `.nono-runtime/verdicts/<gate>.json` (gitignored, T-76-04 accepted for Phase 81).

## Self-Check: PASSED

- `scripts/verify-dark.ps1` exists: FOUND
- `.gitignore` contains `.nono-runtime/`: FOUND (line 46)
- Commit d76a5a0f: FOUND (`chore(76-01): add .nono-runtime/ gitignore rule`)
- Commit 283ec8c6: FOUND (`feat(76-01): add dark-factory harness runner scripts/verify-dark.ps1`)
