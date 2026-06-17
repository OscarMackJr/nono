---
phase: 76-self-verifying-harness-foundation
reviewed: 2026-06-17T00:00:00Z
depth: standard
files_reviewed: 3
files_reviewed_list:
  - scripts/verify-dark.ps1
  - scripts/gates/harness-self-check.ps1
  - .gitignore
findings:
  critical: 2
  warning: 4
  info: 3
  total: 9
status: issues_found
---

# Phase 76: Code Review Report

**Reviewed:** 2026-06-17
**Depth:** standard
**Files Reviewed:** 3
**Status:** issues_found

## Summary

Reviewed the Dark-Factory verdict runner (`verify-dark.ps1`), its first gate
(`gates/harness-self-check.ps1`), and the `.gitignore` change that excludes the
verdict runtime directory.

The **single-gate dispatch path** (`Invoke-SingleGate`) honors the stated
verdict→exit contract correctly: PASS=0, FAIL=2, SKIP_HOST_UNAVAILABLE=3,
thrown/null/unknown-verdict → harness-internal exit 4, unknown gate → exit 1.
This path is sound.

However the **all-run path** (the `else` branch, lines 178–237) — which is the
*default* mode when neither `-Gate` nor `-All` is supplied (lines 96–98) —
violates the core contract in two distinct ways, and both produce exactly the
failure modes the contract was written to prevent: a thrown `Invoke-Gate`
exception reads as a gate FAIL, and an *unknown* verdict value reads as a silent
PASS. Because the default invocation `verify-dark.ps1` (no args) routes through
this branch, these are not edge cases — they are the common path.

A latent robustness gap also exists in both paths: the runner trusts that
`Invoke-Gate` returns a single dictionary, but a gate that leaks any stray
pipeline output turns the return value into an `Object[]`, which defeats the
null-check and the `$verdictObj['verdict']` lookup. Since this gate file is the
declared "reference contract for phases 77–80," that fragility will propagate.

## Critical Issues

### CR-01: All-run mode conflates a thrown Invoke-Gate exception with a gate FAIL (exit 2)

**File:** `scripts/verify-dark.ps1:205-212`, `225-236`
**Issue:** The contract states: *"a thrown Invoke-Gate exception must NEVER read
as a gate FAIL (exit 2) and NEVER as a silent PASS."* The single-gate path obeys
this (line 136–141: catch → `exit 4`). The all-run loop does **not**. When
`Invoke-Gate` throws (lines 205–212), the catch block sets `$anyFail = $true` and
`continue`s. At the end of the loop (lines 232–236) `$anyFail` maps to **exit 2**
— the FAIL exit code. The same conflation applies to the null-verdict case (lines
214–218: also `$anyFail = $true`). A harness-internal crash is therefore reported
to the unattended Dark-Factory verdict consumer as a legitimate gate FAILURE,
indistinguishable from a real FAIL. Because no-arg invocation defaults to all-run
(lines 96–98), this is the default behavior.

**Fix:** Track harness-internal errors separately from gate FAILs and map them to
a reserved code (1/4+), never to 2. Minimal change:

```powershell
$anyFail = $false
$harnessError = $false
foreach ($gateName in ($discoveredGates.Keys | Sort-Object)) {
    ...
    try {
        $verdictObj = Invoke-Gate
    }
    catch {
        [Console]::Error.WriteLine("[verify-dark] harness-internal error: Invoke-Gate threw for '$gateName': $_")
        $harnessError = $true
        continue
    }
    if ($null -eq $verdictObj) {
        [Console]::Error.WriteLine("[verify-dark] harness-internal error: Invoke-Gate returned null for '$gateName'")
        $harnessError = $true
        continue
    }
    ...
}
if ($harnessError) { exit 4 }
elseif ($anyFail)  { exit 2 }
else               { exit 0 }
```

### CR-02: All-run mode treats an unknown/garbage verdict value as a silent PASS (exit 0)

**File:** `scripts/verify-dark.ps1:225-236`
**Issue:** The single-gate path validates the verdict string against the known
set {PASS, FAIL, SKIP_HOST_UNAVAILABLE} and routes anything else to `exit 4`
(lines 158–168). The all-run loop only checks `if ($verdictObj['verdict'] -eq
'FAIL')` (lines 225–227). Any verdict that is not literally `'FAIL'` —
including a typo, an empty string, `$null`, or a corrupted object whose
`['verdict']` lookup yields `[]` (see CR-01/WR-01) — fails the FAIL test and
therefore contributes to the `$anyFail = $false` → **exit 0 (PASS)** outcome.
This is precisely the "silent PASS" failure mode the contract forbids: a gate
returning a malformed or unrecognized verdict is reported as success. A false
PASS defeats the entire purpose of the harness.

**Fix:** Validate each verdict against the known set in the loop, exactly as the
single-gate path does, and treat anything unrecognized as a harness-internal
error rather than silently counting it as non-FAIL:

```powershell
switch ($verdictObj['verdict']) {
    'PASS'                 { }
    'SKIP_HOST_UNAVAILABLE' { }
    'FAIL'                 { $anyFail = $true }
    default {
        [Console]::Error.WriteLine("[verify-dark] harness-internal error: unexpected verdict value '$($verdictObj['verdict'])' from gate '$gateName'")
        $harnessError = $true
    }
}
```

## Warnings

### WR-01: Runner does not defend against Invoke-Gate returning an array (stray pipeline output)

**File:** `scripts/verify-dark.ps1:134-146`, `205-218`; `scripts/gates/harness-self-check.ps1:66-126`
**Issue:** The runner assumes `Invoke-Gate` returns a single dictionary. In
PowerShell, any uncaptured pipeline output inside `Invoke-Gate` is *appended* to
the return value, producing an `Object[]`. Verified empirically: a gate that
emits a stray string plus `return $candidate` yields `$verdictObj.GetType() ==
Object[]`, `Count == 2`. The runner's `$null -eq $verdictObj` check passes (the
array is not null), and `$verdictObj['verdict']` on an `Object[]` returns an
empty result rather than the verdict string. In single-gate mode this lands in
the `else` (exit 4 — tolerable), but in all-run mode it is treated as non-FAIL →
**exit 0 silent PASS** (compounds CR-02). The harness-self-check gate happens to
use `| Out-Null` on its only emitting call (`New-Item`, line 106), but it is the
explicit "reference contract for phases 77–80" — copy-paste authors will not all
be so careful, and the runner provides no guardrail.

**Fix:** Normalize and type-check the return in the runner before use, in both
paths:

```powershell
$verdictObj = Invoke-Gate
if ($verdictObj -is [array]) { $verdictObj = $verdictObj[-1] }  # last object is the returned dict
if ($null -eq $verdictObj -or -not ($verdictObj -is [System.Collections.IDictionary])) {
    [Console]::Error.WriteLine("[verify-dark] harness-internal error: Invoke-Gate did not return a single verdict object for '$GateName'")
    exit 4
}
```

### WR-02: Thrown Test-Precondition aborts the entire all-run loop instead of skipping one gate

**File:** `scripts/verify-dark.ps1:118`, `194`
**Issue:** `Test-Precondition` is called outside any try/catch in both paths
(lines 118 and 194). With `$ErrorActionPreference = "Stop"` (line 12), a gate
whose `Test-Precondition` throws produces an uncaught terminating error. Verified:
this exits with code 1 (contract-safe, in the reserved range). But in all-run
mode it aborts the *entire* sweep — every gate after the throwing one is never
run, and the partial run exits 1 with no per-gate verdict for the remaining
gates. For an unattended verdict runner that is supposed to produce a verdict per
gate, one misbehaving precondition silently drops coverage of all subsequent
gates.

**Fix:** Wrap `Test-Precondition` in try/catch (treat a throw as a
harness-internal error for that gate and `continue` in all-run mode):

```powershell
try {
    $preconditionReason = Test-Precondition
} catch {
    [Console]::Error.WriteLine("[verify-dark] harness-internal error: Test-Precondition threw for '$gateName': $_")
    $harnessError = $true   # single-gate path: exit 4
    continue                # all-run path
}
```

### WR-03: `-All` is silently ignored when `-Gate` is also supplied

**File:** `scripts/verify-dark.ps1:96-98`, `175`
**Issue:** The dispatch at line 175 is `if ($Gate) { Invoke-SingleGate } else { ... }`.
If the operator runs `verify-dark.ps1 -Gate foo -All`, the `-All` switch is
silently discarded and only the single gate runs. There is no error or warning.
For a verdict harness whose output drives automated gating decisions, silently
ignoring an explicit run-mode flag is a correctness hazard (the operator may
believe all gates ran).

**Fix:** Detect the conflicting combination and fail as a harness-internal error,
or document that `-Gate` takes precedence and warn:

```powershell
if ($Gate -and $All) {
    [Console]::Error.WriteLine("[verify-dark] harness-internal error: -Gate and -All are mutually exclusive")
    exit 1
}
```

### WR-04: Verdict directory creation is unchecked; a failed New-Item silently proceeds toward a missing-file read

**File:** `scripts/verify-dark.ps1:67-69`; `scripts/gates/harness-self-check.ps1:106-113`
**Issue:** `Persist-Verdict` (runner) and the gate's assertion (c) both call
`New-Item -ItemType Directory -Force` then `Set-Content`. With
`$ErrorActionPreference = "Stop"` a failure (e.g., the path collides with an
existing *file* named `.nono-runtime`, or ACLs deny write) would terminate — but
the failure semantics differ by path: in `Persist-Verdict` a throw after
`Emit-Verdict` has already written JSON to stdout means the verdict was emitted
to the consumer but never persisted, and (in single-gate mode) the script
terminates *after* emit but *before* reaching the exit-mapping at lines 158–168,
exiting 1. The emitted-but-not-persisted-and-no-mapped-exit combination is an
inconsistent state for downstream consumers that read the persisted file. Persist
should either run before emit, or its failure should be explicitly classified as
harness-internal with a clear exit, and the gate's own redundant persistence
(assertion c) duplicates this logic in a second place that can drift.

**Fix:** Persist before emit (so the file-of-record exists before the consumer
sees the line), and wrap persistence so a write failure is an explicit
harness-internal error rather than a bare terminating error mid-mapping. Consider
removing the gate-side persistence duplication (assertion c) in favor of asserting
on the value round-trip only, leaving persistence solely to the runner.

## Info

### IN-01: Gate-side persistence duplicates runner persistence to the same file

**File:** `scripts/gates/harness-self-check.ps1:95-119`
**Issue:** The gate writes `harness-self-check.json` itself (assertion c, lines
102–119), then the runner writes the same file again via `Persist-Verdict`
(line 155 / 223). The gate writes the *candidate* JSON (with gate/timestamp it
stamped), the runner then overwrites with the *runner-stamped* JSON. Two
independent path-resolution chains (`Split-Path` twice in each file) compute the
same `.nono-runtime\verdicts` path; if one is ever edited and the other not, they
silently diverge. The duplication is harmless today but is dead weight in the
reference contract that phases 77–80 will copy.

**Fix:** Have assertion (c) write to a temp file (e.g. `New-TemporaryFile`) for
the round-trip proof rather than the real verdict path, leaving the canonical
verdict file owned solely by the runner.

### IN-02: Hardcoded literal verdict/exit values scattered across both branches

**File:** `scripts/verify-dark.ps1:159-167`, `225`, `232-235`
**Issue:** The verdict strings (`'PASS'`, `'FAIL'`, `'SKIP_HOST_UNAVAILABLE'`)
and their exit codes are repeated as magic literals in the single-gate mapping
and again (partially, see CR-02) in the all-run loop. The divergence between the
two copies is the direct cause of CR-01 and CR-02. A single shared mapping
function would have made the two paths consistent by construction.

**Fix:** Extract a `Get-ExitForVerdict`/`Resolve-Verdict` helper used by both
paths so the verdict→exit contract lives in exactly one place.

### IN-03: Backslash path separators in Join-Path arguments

**File:** `scripts/verify-dark.ps1:65`; `scripts/gates/harness-self-check.ps1:102`
**Issue:** `Join-Path $repoRoot ".nono-runtime\verdicts"` embeds a backslash in
the child segment. This works on Windows (the only supported host per D-11) but
`Join-Path` treats the embedded `\` as a literal segment rather than composing
two path components, so it is not portable and is stylistically inconsistent with
the project's cross-platform posture. Prefer nested `Join-Path` calls:
`Join-Path (Join-Path $repoRoot '.nono-runtime') 'verdicts'`.

**Fix:** Use nested `Join-Path` for each component, or `[IO.Path]::Combine`.

---

_Reviewed: 2026-06-17_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
