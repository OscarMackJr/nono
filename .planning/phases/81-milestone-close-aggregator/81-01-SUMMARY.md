---
phase: 81-milestone-close-aggregator
plan: "01"
subsystem: scripts
tags: [dark-factory, aggregator, verify-dark, milestone-close, DARK-02]
dependency_graph:
  requires: [76-01, 77-01, 78-01, 79-01, 80-01]
  provides: [DARK-02, milestone-close-signal]
  affects: [scripts/verify-dark.ps1]
tech_stack:
  added: []
  patterns: [PowerShell-aggregator, gate-discovery, verdict-rollup, machine-readable-close-signal]
key_files:
  created: []
  modified:
    - scripts/verify-dark.ps1
decisions:
  - "Aggregate gate name is the literal '_aggregate' (hardcoded, not gate-supplied — prevents T-81-03 path-injection)"
  - "HARNESS_ERROR placeholder pushed to $gateResults for every gate-attempted-but-failed path so the gates array is always complete"
  - "Exit 0 for both PASS and PASS_WITH_SKIPS (SC3 CI-consumable — CI need not distinguish the two)"
  - "WR-04 ordering preserved: Persist-Verdict _aggregate runs BEFORE [Console]::Out.Write of the aggregate line"
metrics:
  duration: "~5 minutes"
  completed: "2026-06-18"
  tasks_completed: 1
  tasks_total: 1
  files_changed: 1
---

# Phase 81 Plan 01: Milestone-Close Aggregator Summary

Formalizes the all-run aggregator in `scripts/verify-dark.ps1`, replacing the minimal Phase-76 exit tail with a structured `{gates:[...],overall,timestamp}` rollup that is the single unattended machine-readable v2.13 close signal (DARK-02).

## What Was Built

The `else` branch (all-run path) of `scripts/verify-dark.ps1` now:

1. Tracks two new variables alongside `$anyFail`/`$harnessError`: `$anySkip` (set when any gate is `SKIP_HOST_UNAVAILABLE`) and `$gateResults` (a `List[object]` that accumulates every per-gate verdict or HARNESS_ERROR placeholder).
2. Pushes a HARNESS_ERROR placeholder to `$gateResults` on every error path (Test-Precondition-throws, Invoke-Gate-throws, Normalize-null, Persist-fails) so the aggregate `gates` array has an entry for every gate that was attempted.
3. Pushes the real `$verdictObj` to `$gateResults` after a successful per-gate emit (and also inside the SKIP fast-path after its emit).
4. After all gates have run, computes `$overallStr` (precedence: `HARNESS_ERROR` > `FAIL` > `PASS_WITH_SKIPS` > `PASS`), builds the aggregate object, serializes with `ConvertTo-Json -Depth 6 -Compress`, calls `Persist-Verdict -GateName '_aggregate'` (WR-04: file written before stdout), then emits via `[Console]::Out.Write`.
5. Exits 4 for harness-internal error, 2 for FAIL, 0 for PASS or PASS_WITH_SKIPS (SC3).

## Observed Run Results on Dev Host (2026-06-18)

`pwsh -File scripts\verify-dark.ps1` (no flags):

| Gate | Verdict | Reason |
|------|---------|--------|
| clean-host-install | SKIP_HOST_UNAVAILABLE | nono.exe detected under C:\Program Files\nono — host is not clean |
| copilot-e2e | SKIP_HOST_UNAVAILABLE | GitHub Copilot CLI access denied by org policy |
| harness-self-check | PASS | framework functional |
| wfp-egress-isolation | FAIL | blocked agent failed to launch through the daemon (no package SID in response; `nono agent` subcommand not recognised on installed stale binary) |

**Overall: FAIL** — exit code **2**

The FAIL is expected on this dev host: the installed `nono.exe` under `C:\Program Files\nono` is a stale build without the `agent` subcommand required by the wfp-egress-isolation gate. This is a pre-existing Phase 79 gate condition, not a harness defect.

`.nono-runtime\verdicts\_aggregate.json` written with `gateCount=4`, `overall="FAIL"`.

## Verification Steps Passed

1. Single-gate regression: `pwsh -File scripts\verify-dark.ps1 -Gate harness-self-check` exits 0, PASS verdict.
2. Structural assertions: `_aggregate`, `PASS_WITH_SKIPS`, `gateResults`, `anySkip`, `Depth 6` all present (Select-String returns True for each).
3. Stub exclusion: `scripts\gates\*.ps1` has exactly {clean-host-install, copilot-e2e, harness-self-check, wfp-egress-isolation} — no stub files.
4. End-to-end: 4 per-gate JSON lines + 1 aggregate JSON line; exit 2 (FAIL); `_aggregate.json` written.
5. Artifact shape: `overall=FAIL`, `gateCount=4`.
6. Gate names in aggregate: exactly the 4 real gate names, no stub names.

## Deviations from Plan

None — plan executed exactly as written. The advisory NIT about the SKIP fast-path's persist-failure branch was addressed: that path now pushes an `HARNESS_ERROR` placeholder to `$gateResults` before its `continue`, ensuring every attempted gate has a record.

## Threat Surface Scan

No new network endpoints, auth paths, or schema changes. The `_aggregate` gate name is hardcoded (literal string `'_aggregate'`) and cannot be influenced by gate-supplied input — T-81-03 mitigated as planned. Gate set is derived exclusively from `$discoveredGates` (built from `scripts/gates/*.ps1`), not from the `verdicts/` directory — T-81-04 mitigated.

## Self-Check: PASSED

- FOUND: `.planning/phases/81-milestone-close-aggregator/81-01-SUMMARY.md`
- FOUND commit: `666c81ea`
