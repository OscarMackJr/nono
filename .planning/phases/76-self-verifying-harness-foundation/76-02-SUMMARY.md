---
phase: 76-self-verifying-harness-foundation
plan: "02"
subsystem: scripts/harness
tags: [dark-factory, harness, powershell, gate-contract, DARK-01]
dependency_graph:
  requires:
    - phase: 76-01
      provides: scripts/verify-dark.ps1 runner with Test-Precondition/Invoke-Gate dot-source contract
  provides:
    - scripts/gates/harness-self-check.ps1 (first gate; canonical contract reference for phases 77-80)
    - Verified PASS run of verify-dark.ps1 --gate harness-self-check on Win11 (ROADMAP SC5)
    - All five ROADMAP Phase 76 success criteria demonstrated with recorded exit codes + JSON
  affects: [phases/77, phases/78, phases/79, phases/80, phases/81]
tech_stack:
  added: []
  patterns:
    - "harness-self-check gate: Test-Precondition returns null; Invoke-Gate runs emit+persist+round-trip assertions and returns PASS verdict object"
    - "Gate returns verdict object to runner; never calls exit (D-02 exit/return convention)"
    - "Assert-Equal/Assert-True throw-on-failure helpers inside Invoke-Gate (D-07 semantics)"
    - "Persistence file round-trip: Set-Content -> Get-Content -> ConvertFrom-Json -> field equality assertion"
key_files:
  created:
    - scripts/gates/harness-self-check.ps1
  modified: []
key_decisions:
  - "Gate RETURNS verdict to runner, never calls exit — diverges from standalone scripts that call exit directly (D-02, PATTERNS exit/return convention)"
  - "Assertion (c) resolves repo root from PSScriptRoot via Split-Path -Parent twice (scripts/gates/ -> scripts/ -> repo root), matching the runner's own Split-Path -Parent $PSScriptRoot logic"
  - "PSScriptRoot inside a dot-sourced file is the file's own directory (scripts/gates/), not the runner's scripts/ directory — two levels of Split-Path are required to reach repo root"
patterns-established:
  - "Two-function gate contract (Test-Precondition + Invoke-Gate) — phases 77-80 copy this shape verbatim"
  - "Self-check assertions: ConvertTo-Json -> string non-empty; ConvertFrom-Json -> field equality; persistence file -> round-trip"
requirements-completed: [DARK-01]
duration: ~20min
completed: 2026-06-17
---

# Phase 76 Plan 02: Harness-Self-Check Gate Summary

**First gate file `scripts/gates/harness-self-check.ps1` delivered and proven: `verify-dark.ps1 --gate harness-self-check` exits 0 with `"verdict":"PASS"` on Win11 (ROADMAP SC5, DARK-01 satisfied); all five Phase 76 success criteria recorded with observed exit codes and JSON output.**

## Performance

- **Duration:** ~20 min
- **Started:** 2026-06-17T07:25:00Z
- **Completed:** 2026-06-17T07:35:00Z
- **Tasks:** 2
- **Files created:** 1 (`scripts/gates/harness-self-check.ps1`)

## Accomplishments

- Created `scripts/gates/harness-self-check.ps1` with the canonical `Test-Precondition` (returns `$null`) + `Invoke-Gate` (returns PASS after emit+persist+round-trip assertions) contract for phases 77-80 to copy.
- Proved framework end-to-end: `verify-dark.ps1 --gate harness-self-check` exits 0 with JSON `"verdict":"PASS"` on Win11.
- Demonstrated all five ROADMAP Phase 76 success criteria with recorded evidence (exit codes + JSON).
- Confirmed SKIP path (SC4): temporary `_skip-probe.ps1` stub emitted `SKIP_HOST_UNAVAILABLE` + exit 3; stub deleted, leaving exactly one file in `scripts/gates/`.
- Confirmed harness-internal error path: `--gate does-not-exist` exits 1 (not a FAIL verdict).

## Task Commits

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Write the harness-self-check gate file | f7e7bfe9 | scripts/gates/harness-self-check.ps1 |
| 2 | Prove five ROADMAP SC end-to-end (acceptance run) | (no new production file — evidence recorded in SUMMARY) | — |

## Verification Results

All verification performed on the Win11 host (Windows 11 Enterprise 10.0.26200). Observed exit codes and emitted JSON:

### SC1 — No interactive prompts; machine-readable JSON verdict + structured exit code

```
pwsh -NoProfile -File scripts/verify-dark.ps1 -Gate harness-self-check
{"gate":"harness-self-check","verdict":"PASS","reason":"framework functional","detail":{},"timestamp":"2026-06-17T07:30:10.297Z"}
Exit: 0
```

### SC2 — Verdict is one of PASS/FAIL/SKIP_HOST_UNAVAILABLE (typed gate contract)

Observed: `"verdict":"PASS"` — one of the three allowed values. Contract satisfied.

### SC3 — `--gate harness-self-check` exercises exactly one gate in isolation

The runner's gate auto-discovery (`Get-ChildItem scripts/gates/ -Filter *.ps1`) lists only `harness-self-check` at execution time. Single gate ran; no other gate output appeared.

### SC4 — SKIP path: stub gate produces `SKIP_HOST_UNAVAILABLE` + exit 3; stub deleted

Temporary stub `scripts/gates/_skip-probe.ps1` created with `Test-Precondition` returning `'skip-probe: host unavailable by design (SC4 SKIP path test)'`:

```
pwsh -NoProfile -File scripts/verify-dark.ps1 -Gate _skip-probe
{"gate":"_skip-probe","verdict":"SKIP_HOST_UNAVAILABLE","reason":"skip-probe: host unavailable by design (SC4 SKIP path test)","detail":{},"timestamp":"2026-06-17T07:30:23.126Z"}
Exit: 3
```

Stub deleted. `scripts/gates/` listing after deletion: `harness-self-check.ps1` only. (SC4: PASS)

### SC5 — `--gate harness-self-check` exits 0 with PASS verdict

Exit: 0, `"verdict":"PASS"` — confirmed (same run as SC1/SC2/SC3 above).

### Negative case — `--gate does-not-exist` exits 1 (harness-internal error, not FAIL)

```
pwsh -NoProfile -File scripts/verify-dark.ps1 -Gate does-not-exist
[verify-dark] harness-internal error: unknown gate 'does-not-exist'. Discovered gates: harness-self-check
Exit: 1
```

Confirmed: exit 1 (harness-internal error), not exit 2 (FAIL verdict). D-05/D-07 satisfied.

### Persistence file round-trip

`.nono-runtime/verdicts/harness-self-check.json` after the PASS run:

```json
{"gate":"harness-self-check","verdict":"PASS","reason":"framework functional","detail":{},"timestamp":"2026-06-17T07:30:10.297Z"}
```

Parses as JSON with `verdict = "PASS"`. Persistence and round-trip confirmed.

### Composite automated verification (plan-specified)

```
$a=0 (harness-self-check PASS); $b=1 (does-not-exist harness-internal error)
Condition: $a -eq 0 -and ($b -eq 1 -or $b -ge 4) -> True
Composite exit: 0
```

## Files Created/Modified

- `scripts/gates/harness-self-check.ps1` — First gate file; exports `Test-Precondition` (returns `$null`, D-11) and `Invoke-Gate` (returns PASS after three round-trip assertions); canonical contract reference for phases 77-80.

## Decisions Made

1. **Gate returns, never exits** — `Invoke-Gate` returns the candidate verdict `[ordered]@{}` to the runner and makes no `exit` call. The runner in `Invoke-SingleGate` stamps the final `gate` and `timestamp` fields before emitting (lines 150-151 of `verify-dark.ps1`), so the gate redundantly includes them for readability without conflict.

2. **PSScriptRoot in dot-sourced gate = file's own directory** — Inside `harness-self-check.ps1`, `$PSScriptRoot` is `scripts/gates/` (not the runner's `scripts/`). Two levels of `Split-Path -Parent` are needed to resolve repo root: `Split-Path -Parent (Split-Path -Parent $PSScriptRoot)`. This matches the runner's own `Split-Path -Parent $PSScriptRoot` from within `scripts/`. Documented in the gate file's assertion (c) comment.

3. **Assertion (c) uses a minimal Set-Content + Get-Content + ConvertFrom-Json round-trip** — The gate verifies it can write and read back the persistence file, asserting `gate` and `verdict` fields. The runner's `Persist-Verdict` function then overwrites this file as its normal operation (no conflict — the runner's emit happens after Invoke-Gate returns).

## Deviations from Plan

None — plan executed exactly as written.

## Known Stubs

None. `scripts/gates/harness-self-check.ps1` is complete and fully functional. The `_skip-probe.ps1` stub from Task 2 was deleted as required by the acceptance criteria.

## Threat Surface Scan

No new network endpoints, auth paths, file access patterns, or schema changes introduced. The gate file is a dot-sourced plugin; it reads `$PSScriptRoot` (its own directory path) and writes to `.nono-runtime/verdicts/` (gitignored). T-76-05 (false PASS on broken framework) mitigated by three throw-on-failure assertions. T-76-06 (gate calls exit) mitigated: confirmed no top-level `exit` in the file. T-76-07 (leftover stub pollutes --all) mitigated: stub deleted, `scripts/gates/` contains exactly one file.

## Self-Check: PASSED

- `scripts/gates/harness-self-check.ps1` exists: FOUND
- `function Invoke-Gate` in the file: FOUND
- `function Test-Precondition` in the file: FOUND
- No top-level `exit` calls in the file: CONFIRMED (grep count = 0)
- `Test-Precondition` returns `$null`: CONFIRMED (`. scripts/gates/harness-self-check.ps1; ($null -eq (Test-Precondition))` = `True`)
- `verify-dark.ps1 --gate harness-self-check` exits 0: CONFIRMED (exit 0)
- `"verdict":"PASS"` in output: CONFIRMED
- `.nono-runtime/verdicts/harness-self-check.json` exists and parses: CONFIRMED
- Commit f7e7bfe9: FOUND (`feat(76-02): add harness-self-check gate (Test-Precondition + Invoke-Gate)`)
- `scripts/gates/` contains exactly `harness-self-check.ps1`: CONFIRMED
