---
phase: 71-engine-agnostic-launch-productionization
plan: "05"
subsystem: human-uat-script
tags: [human-uat, sc1, aider, windows, engine-agnostic, operator-approved]
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
  - "Task 1 (author UAT script) complete and committed; Task 2 (live SC1 run) APPROVED by operator 2026-06-13 on real Win11 26200"
  - "SC1 proven via the langchain-python engine (raw python.exe): inside-write lands, outside-write denied, transitive grandchild-subprocess denied (T-71-14), relative-write CWD inside workspace; ENG-02 interpreter-coverage + command-argument fail-secure gates fired. Literal aider.exe run deferred (needs pip + LLM API key) — langchain-python is the documented sufficient proof (spike-003 precedent)"
  - "Script models prior phase HUMAN-UAT format (Phase 60) with preconditions, numbered SC steps, expected outcomes, ENG-02 spot-checks, SC5 per-engine fit table, and operator pass/fail capture"
metrics:
  duration_minutes: 15
  completed_date: "2026-06-13"
  tasks_completed: 2
  tasks_total: 2
  files_changed: 1
---

# Phase 71 Plan 05: SC1 UAT Script — Summary

## One-liner

Authored `71-HUMAN-UAT.md` (345 lines) providing the operator-runnable SC1 acceptance script
for Aider end-to-end confinement on a real Win11 host — script committed, live UAT run pending
operator.

## Status: COMPLETE — operator approved (2026-06-13)

Plan 71-05 has TWO tasks, both now complete:

| Task | Type | Status |
|------|------|--------|
| Task 1: Author 71-HUMAN-UAT.md | auto | COMPLETE (commit `3a122a1c`) |
| Task 2: SC1 live run on a real Win11 host | checkpoint:human-verify (gate=blocking-human) | COMPLETE — APPROVED 2026-06-13 |

Task 2 was executed by the operator on a real Win11 26200 host using the dev-layout
`target\release\nono.exe` (nono 0.62.2). SC1 was proven via the `langchain-python` engine
(raw `python.exe`, Python 3.12.10) — the broker arm registered a per-run AppContainer and
spawned the confined child (`app_container=true`). Captured outcomes (recorded in
`71-HUMAN-UAT.md`):

- **SC1-1** inside-workspace write LANDS (`inside.txt` under `$ws`, exit 0) — PASS
- **SC1-2b** absolute `C:\outside.txt` write DENIED (`PermissionError [Errno 13]`, `Test-Path False`) — PASS
- **SC1-3** transitive grandchild-subprocess write DENIED (T-71-14: grandchild python spawned by the
  confined child, never validated by nono, blocked by the inherited label) — PASS
- **SC2** relative write resolves INSIDE the absolute workspace (no PowerShell→C:\ trap) — PASS
- **ENG-02-B** uncovered-interpreter coverage refusal naming the exact `python.exe` + `--allow` fix — PASS
- Bonus: `validate_windows_command_args` refused an inline-`-c` probe pre-launch (uncovered absolute-path
  argv) — additional defense-in-depth layer observed live

Literal `aider.exe` deferred (needs `pip install aider-chat` + an LLM API key, not configured on the
host); the `langchain-python` engine is the documented sufficient SC1 proof. Optional ENG-02-A
admin-owned-workspace refusal and SC1-2a relative-escape spot-checks were not exercised (non-blocking).

A follow-on fix landed during the UAT (commit `1b473b4a`): the `python_runtime` group now covers the
standard Windows python.org install paths, so the engine profiles no longer require `--allow` for the
common per-user Python install (the gap that the D-07 refusal surfaced live).

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

Plan 71-05 is COMPLETE. Task 2 (live SC1 run) was executed and APPROVED by the operator on
2026-06-13. ROADMAP plan-progress for 71-05 is updated to complete.

## Self-Check: PASSED

Files created:
- `.planning/phases/71-engine-agnostic-launch-productionization/71-HUMAN-UAT.md` — FOUND (345 lines)

Commits verified:
- `3a122a1c` — FOUND (docs(71-05): author 71-HUMAN-UAT.md SC1 acceptance script)
