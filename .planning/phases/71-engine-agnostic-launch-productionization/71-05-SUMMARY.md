---
phase: 71-engine-agnostic-launch-productionization
plan: "05"
subsystem: human-uat-script
tags: [human-uat, sc1, aider, windows, engine-agnostic, pending-operator]
dependency_graph:
  requires: [71-01-engine-profiles, 71-02-coverage-gate, 71-03-rb3-gate, 71-04-cli-integration]
  provides: [71-HUMAN-UAT.md]
  affects: []
tech_stack:
  added: []
  patterns: [human-uat-runbook]
key_files:
  created:
    - .planning/phases/71-engine-agnostic-launch-productionization/71-HUMAN-UAT.md
  modified: []
decisions:
  - "Task 1 (author UAT script) is complete and committed; Task 2 (live SC1 run on a real Win11 host) is PENDING OPERATOR — not executed, not fabricated"
  - "Script models prior phase HUMAN-UAT format (Phase 60) with preconditions, numbered SC steps, expected outcomes, ENG-02 spot-checks, SC5 per-engine fit table, and operator pass/fail capture"
metrics:
  duration_minutes: 15
  completed_date: "2026-06-14"
  tasks_completed: 1
  tasks_total: 2
  files_changed: 1
---

# Phase 71 Plan 05: SC1 UAT Script — Summary

## One-liner

Authored `71-HUMAN-UAT.md` (345 lines) providing the operator-runnable SC1 acceptance script
for Aider end-to-end confinement on a real Win11 host — script committed, live UAT run pending
operator.

## Status: PENDING OPERATOR (Task 2 not executed)

Plan 71-05 has TWO tasks:

| Task | Type | Status |
|------|------|--------|
| Task 1: Author 71-HUMAN-UAT.md | auto | COMPLETE (commit `3a122a1c`) |
| Task 2: SC1 live run on a real Win11 host | checkpoint:human-verify (gate=blocking-human) | PENDING OPERATOR |

Task 2 requires a real Win11 host, a real Aider install, and the `BrokerLaunchNoPty` arm
exercised with a real Low-IL relabel + AppContainer path. It cannot be run in this dev
environment and has NOT been executed. No SC1 pass/fail result is claimed here.

## Tasks Completed

| Task | Name | Commit | Key Files |
|------|------|--------|-----------|
| 1 | Author 71-HUMAN-UAT.md SC1 acceptance script | `3a122a1c` | 71-HUMAN-UAT.md |

## What Was Built

### Task 1 — `71-HUMAN-UAT.md` (345 lines, plan minimum: 60)

The UAT script contains, in order:

**Preconditions block (5 items):**
- P-1: dev-layout `target\release\nono.exe` (R-B4 broker Authenticode trust gate; Program Files
  install is REFUSED — run from source tree)
- P-2: Real PowerShell console — not git-bash/MSYS (else `CreateProcessAsUserW` GLE=87)
- P-3: User-owned `%USERPROFILE%\nono-work` workspace (R-B3 WRITE_OWNER pre-launch gate;
  admin-owned dirs lack WRITE_OWNER even when the user is the NTFS owner)
- P-4: Aider via `pipx install aider-chat`; `langchain-python` + raw `python.exe` fallback
  if Aider install is problematic
- P-5: AppContainer/CLR gotchas to watch: CreateAppContainerProfile vs derive-only;
  `SystemRoot`/`windir`/`SystemDrive` env baseline; `\\?\` prefix stripping

**SC1 primary test (three numbered steps):**
- Step 1: `nono run --profile aider --workspace %USERPROFILE%\nono-work -- aider.exe <minimal-args>` —
  write INSIDE workspace LANDS; exact verify command included
- Step 2: Write OUTSIDE workspace (relative `..\outside.txt` AND absolute `C:\outside.txt`)
  is DENIED with `NO_WRITE_UP`/`UnauthorizedAccessException`; both files must not exist
- Step 3: `python.exe` subprocess initiated write DENIED transitively — proves parent-and-confine
  (not per-tool hook); T-71-14 proof; file must not exist

**SC2 CWD assertion:**
- Relative engine write resolves INSIDE `$ws`, not `C:\` — the PowerShell-to-C:\ trap closed
  by Plan 04's `lpCurrentDirectory` wiring (commit `2001bf97`)

**ENG-02 fail-secure spot-checks (recommended):**
- Spot-Check A: admin-owned workspace → named R-B3 SandboxInit error before spawn (names
  WRITE_OWNER, suggests `%USERPROFILE%`, states "nono will NOT take ownership automatically")
- Spot-Check B: engine with uncovered interpreter → named coverage refusal naming the exact
  `python.exe` path and the `--allow` fix

**SC5 per-engine fit table (5 rows):**
- Aider, LangChain-Python, Copilot CLI: launch-and-confine via `BrokerLaunchNoPty`
- Claude Code PreToolUse hook: per-tool hook (defense-in-depth, legacy path, NOT isolation)
- Cursor: WSL-only (GUI host limitation on native Windows)

**Operator pass/fail capture section:**
- Date, host OS build, nono version, Aider version, python.exe path, workspace used
- Per-step outcome table (SC1-1, SC1-2a, SC1-2b, SC1-3, SC2, ENG-02-A, ENG-02-B)
- NO_WRITE_UP confirmation checkbox
- Overall PASS/FAIL verdict with failure description field
- Resume signal instruction for Task 2 continuation

## Verification (Task 1)

```
test -f .planning/phases/71-engine-agnostic-launch-productionization/71-HUMAN-UAT.md \
  && grep -q "NO_WRITE_UP" 71-HUMAN-UAT.md \
  && grep -q "\-\-profile aider \-\-workspace" 71-HUMAN-UAT.md \
  && echo OK
```

Result: **OK**

Additional checks:
- Line count: 345 (>= 60 minimum)
- `grep "nono run --profile aider --workspace"` → 6 matches (SC1 steps, table)
- `grep "NO_WRITE_UP"` → 5 matches
- `grep "python.exe"` → present (SC1 step 3, ENG-02-B, SC5 table)
- `grep "per-engine"` → present (SC5 table header and description)
- All three preconditions named: dev-layout/R-B4, real PowerShell console, user-owned
  `%USERPROFILE%` workspace (R-B3)

## Deviations from Plan

None. Task 1 executed exactly as specified. Task 2 is a `checkpoint:human-verify` with
`gate="blocking-human"` and has not been run (correct per the sequential executor's mandate).

## Known Stubs

None in the authored script. All content is precise runbook instructions with exact commands,
expected outcomes, and verify steps. The pass/fail capture table is intentionally blank —
it is the operator's recording surface for the live UAT.

## Threat Flags

None. This plan modifies only a planning doc. No new source code, network endpoints, auth paths,
file access patterns, or schema changes.

## ROADMAP Status

Plan 71-05 is NOT marked complete. Task 2 (live SC1 run) is PENDING OPERATOR. The ROADMAP
plan-progress for 71-05 will be updated to complete only after the operator executes Task 2 and
types the resume signal.

## Self-Check: PASSED

Files created:
- `.planning/phases/71-engine-agnostic-launch-productionization/71-HUMAN-UAT.md` — FOUND (345 lines)

Commits verified:
- `3a122a1c` — FOUND (docs(71-05): author 71-HUMAN-UAT.md SC1 acceptance script)
