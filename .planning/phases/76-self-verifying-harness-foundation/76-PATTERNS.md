# Phase 76: Self-Verifying Harness Foundation - Pattern Map

**Mapped:** 2026-06-16
**Files analyzed:** 3 (2 new PowerShell files + 1 .gitignore edit)
**Analogs found:** 3 / 3

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `scripts/verify-dark.ps1` (NEW) | runner / dispatcher | request-response (CLI in → JSON verdict + exit code out) | `scripts/windows-test-harness.ps1` (param/dispatch/spawn idioms) + `scripts/check-upstream-drift.ps1` (JSON emit) | role-match (composite) |
| `scripts/gates/harness-self-check.ps1` (NEW) | gate plugin (dot-sourced contract) | transform (assert → verdict object) | `scripts/test-windows-shell-write-deny.ps1` (exit-code-classify idiom) | role-match |
| `.gitignore` (MODIFIED — add `.nono-runtime/`) | config | n/a | existing scratch-dir ignore rules (`ci-logs-local/`, `.tmp/`, `.bg-shell/`) | exact |

**Auto-discovery note (D-04):** `scripts/gates/` does NOT exist yet — this phase creates the directory plus its first member. No existing `scripts/gates/*.ps1` analog; the gate-contract shape is defined fresh from CONTEXT.md D-05/D-06/D-11.

**.gitignore status (verified):** `.nono-runtime/` is NOT currently in `.gitignore` (read full file — lines 1-103). The dir is untracked-but-unignored. A rule MUST be added (do not assume it is already ignored).

---

## Pattern Assignments

### `scripts/verify-dark.ps1` (runner, request-response)

**Composite analog** — borrow the script header + param idiom from `windows-test-harness.ps1`, the exit-code-classify idiom from `test-windows-shell-write-deny.ps1`, and the JSON-emit idiom from `check-upstream-drift.ps1`.

**Script header pattern** — `scripts/windows-test-harness.ps1:7-10`. Apply verbatim because a gate's `Invoke-Gate` may shell out to cargo/msiexec (D-09) and benign native stderr must not promote to a terminating error:
```powershell
$ErrorActionPreference = "Stop"
# Cargo and other native tools write normal progress output to stderr.
# Keep that from being promoted into terminating PowerShell errors while we tee logs.
$PSNativeCommandUseErrorActionPreference = $false
```
NOTE divergence: `test-windows-shell-write-deny.ps1:52` deliberately uses `$ErrorActionPreference = 'Continue'` instead, *because it wants non-zero native exits to flow to `$LASTEXITCODE` rather than throw*. The runner's own dispatch wants `Stop` (harness-internal robustness), but inside `Invoke-Gate` a gate that shells out and reads `$LASTEXITCODE` should locally set `Continue` or wrap in try/catch — mirror the write-deny script's reasoning, documented at its lines 49-51.

**Param / mode-selection idiom** — `scripts/windows-test-harness.ps1:1-5`. The established house idiom is `param([ValidateSet(...)]...)`. Per CONTEXT.md D-04, gates are auto-discovered so do NOT hardcode a `ValidateSet`; use a plain `[string]$Gate` and validate against the globbed gate list (unknown `--gate` = harness-internal error / exit 1+, per D-05). Reference shape:
```powershell
param(
    [ValidateSet("build", "smoke", "integration", "security", "regression", "all")]
    [string]$Suite = "all",
    [string]$LogDir = "ci-logs"
)
```
Adapt to: `param([string]$Gate, [switch]$All)` — accept "no `-Gate`" as the all-run (Claude's discretion per CONTEXT.md).

**Gate auto-discovery (glob)** — house glob idiom is `Get-ChildItem`/`Join-Path`. Build the gate list from `Join-Path $PSScriptRoot "gates"` then `Get-ChildItem -Filter *.ps1`; gate name = `[System.IO.Path]::GetFileNameWithoutExtension($_.Name)` (D-05). Dot-source each selected gate file (`. $gateFile`) so `Test-Precondition` / `Invoke-Gate` enter scope. (No existing analog dot-sources sibling scripts; the `& (Join-Path $PSScriptRoot ...)` invoke pattern at `windows-test-harness.ps1:168` and `validate-windows-msi-contract.ps1:67` is the closest call-a-sibling-script idiom — but the contract requires dot-sourcing, not `&`, so functions persist.)

**Precondition → SKIP / dispatch order** (D-06): call `Test-Precondition` BEFORE `Invoke-Gate`. If it returns a non-null string, emit `SKIP_HOST_UNAVAILABLE` (exit 3) with that string as `reason` and never enter the gate body.

**Verdict JSON emit pattern** — `scripts/check-upstream-drift.ps1:224-243`. This is the ONLY existing JSON-emitting script; mirror its house style exactly:
```powershell
$result = [ordered]@{
    range                = "${From}..${To}"
    ...
}
# -Depth 6 (NOT default 2!) so nested arrays don't serialize as "System.Object[]".
# -Compress matches bash printf no-pretty-print. Use [Console]::Out.Write + explicit
# LF to match bash's printf output byte-for-byte (PS Write-Output appends CRLF on Windows).
$json = ($result | ConvertTo-Json -Depth 6 -Compress)
[Console]::Out.Write($json + "`n")
```
Adapt for the verdict object (D-01): use `[ordered]@{}` to lock key order `gate, verdict, reason, detail, timestamp`; `detail` is a nested object so keep `-Depth` ≥ 5; `[Console]::Out.Write($json + "`n")` to avoid CRLF. Timestamp idiom from `test-windows-shell-write-deny.ps1:60`: `Get-Date -Format 'yyyy-MM-ddTHH:mm:ss.fffZ'` (matches the ISO-ish example in CONTEXT.md "specifics").

**Exit-code mapping (verdict → exit)** — classify idiom from `scripts/test-windows-shell-write-deny.ps1:172-176` and `:225-234`:
```powershell
$writeDenyResult = switch ($shellExit) {
    42 { "PASS" }
    1  { "FAIL" }
    default { "INDETERMINATE" }
}
```
Apply the D-02 three-way map for a single `-Gate` run: `PASS → exit 0`, `FAIL → exit 2`, `SKIP_HOST_UNAVAILABLE → exit 3`, harness-internal error → exit 1 (or 4+). Use a trailing `if/elseif/else ... exit N` block exactly as `:225-234` does. Reserve 1/4+ so a harness crash never reads as a gate FAIL (D-02).

**Verdict persistence (D-08)** — directory-create idiom is `New-Item -ItemType Directory -Force -Path ... | Out-Null` (`windows-test-harness.ps1:12`, `test-windows-shell-write-deny.ps1:55`). Create `.nono-runtime/verdicts/` then write `<gate>.json`. House write idiom uses `Tee-Object`/`Add-Content`; for a single JSON artifact, write the same `$json` string captured for stdout to `Join-Path $verdictDir "$gate.json"` (one file per gate = single source of truth, D-08). Resolve the runtime dir relative to repo root via `Split-Path -Parent $PSScriptRoot` (root-resolution idiom from `validate-windows-msi-contract.ps1:44`).

**Child-process / cargo-wrapping helper (for future gates, D-09)** — `scripts/windows-test-harness.ps1:14-43` `Invoke-LoggedCargo`. The runner itself does NOT shell out, but document/leave-room for gates to mirror this `Start-Process -NoNewWindow -Wait -PassThru -RedirectStandardOutput/Error` + `$process.ExitCode` idiom. Per D-10 do NOT reuse the file or fold into the harness; reuse the *idiom* only. Key excerpt:
```powershell
$process = Start-Process -FilePath "cargo" `
    -ArgumentList $CargoArgs -NoNewWindow -Wait -PassThru `
    -RedirectStandardOutput $stdoutPath -RedirectStandardError $stderrPath
...
if ($process.ExitCode -ne 0) { throw "Cargo command failed ... $($process.ExitCode)" }
```

---

### `scripts/gates/harness-self-check.ps1` (gate plugin, transform)

**No existing `scripts/gates/` analog** — this is the first gate and defines the contract for phases 77-80. Shape is fixed by CONTEXT.md D-05/D-06/D-11 plus the contract sketch in `76-CONTEXT.md:167-177`.

**Required contract (D-05):** the file exports exactly two functions, dot-sourced by the runner:
```powershell
function Test-Precondition {
    # -> $null (preconditions met → run Invoke-Gate)
    # -> "reason string" (host unavailable → runner emits SKIP_HOST_UNAVAILABLE)
    return $null   # D-11: harness-self-check ALWAYS runs on any Win11 host
}

function Invoke-Gate {
    # -> a verdict (PASS/FAIL); may shell out (D-09).
    # D-11: trivially verify framework wiring — emit + persist + JSON round-trip.
}
```

**Self-check assertion idiom (round-trip)** — borrow the `ConvertTo-Json` house style from `check-upstream-drift.ps1:241` and round-trip it back with `ConvertFrom-Json`, asserting field equality. Assertion-helper idiom from `validate-windows-msi-contract.ps1:100-129`:
```powershell
function Assert-Equal {
    param($Actual, $Expected, [string]$Message)
    if ($Actual -ne $Expected) { throw "$Message. Expected '$Expected', got '$Actual'." }
}
function Assert-True {
    param([bool]$Condition, [string]$Message)
    if (-not $Condition) { throw $Message }
}
```
Use these to verify: (a) a verdict object serializes (`ConvertTo-Json -Depth 5`), (b) `ConvertFrom-Json` returns matching `gate`/`verdict` fields, (c) the persistence file at `.nono-runtime/verdicts/harness-self-check.json` exists and round-trips. On success return PASS (`reason = "framework functional"`, `detail = @{}` per CONTEXT.md:152). A thrown assertion should surface to the runner as FAIL or harness-internal error per D-07 (the runner — not the gate — owns exit-code mapping; the gate returns/throws).

**Exit/return convention:** the gate returns the verdict to the runner (does NOT call `exit` itself — only the runner maps verdict→exit, D-02). This diverges from `test-windows-shell-write-deny.ps1` which is standalone and calls `exit` directly; the gate is a dot-sourced plugin so it must return, not exit.

---

### `.gitignore` (MODIFIED)

**Analog:** existing scratch-dir ignore rules in the same file — `ci-logs-local/` (`.gitignore:42`), `.tmp/` (`.gitignore:36`), `.bg-shell/` / `.gsd/` (`.gitignore:59-60`). All follow `# <comment>` + `<dir>/` form.

**Add** (D-08 — `.nono-runtime/verdicts/<gate>.json` is regenerable runtime output, not source):
```
# Dark-factory verdict runtime artifacts (Phase 76 DARK-01 — verify-dark.ps1
# writes .nono-runtime/verdicts/<gate>.json; regenerable, not committed)
.nono-runtime/
```
Place it near the other runtime-scratch rules (after `ci-logs-local/` ~line 42, or in the GSD/scratch block ~line 58). Match the existing comment-then-rule house style.

---

## Shared Patterns

### Native-tool-safe script header
**Source:** `scripts/windows-test-harness.ps1:7-10`
**Apply to:** `scripts/verify-dark.ps1` (runner top); gates that shell out set `Continue` locally per the `test-windows-shell-write-deny.ps1:49-53` reasoning.
```powershell
$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $false
```

### JSON emission (house style)
**Source:** `scripts/check-upstream-drift.ps1:229-242` (the ONLY existing JSON emitter)
**Apply to:** verdict object in the runner AND the round-trip self-check.
- `[ordered]@{}` to lock key order
- `ConvertTo-Json -Depth N -Compress` (N ≥ 5 for nested `detail`)
- `[Console]::Out.Write($json + "`n")` to avoid PowerShell CRLF

### Exit-code classification (switch + trailing if/elseif/else exit)
**Source:** `scripts/test-windows-shell-write-deny.ps1:172-176`, `:225-234`
**Apply to:** runner's verdict→exit mapping (D-02 three-way: 0/2/3, reserve 1/4+).

### Directory creation + repo-root resolution
**Source:** `New-Item -ItemType Directory -Force ... | Out-Null` (`windows-test-harness.ps1:12`); `Split-Path -Parent $PSScriptRoot` (`validate-windows-msi-contract.ps1:44`)
**Apply to:** creating `.nono-runtime/verdicts/` relative to repo root before persisting.

### Assertion helpers
**Source:** `scripts/validate-windows-msi-contract.ps1:100-129` (`Assert-Equal`, `Assert-True` — throw on failure)
**Apply to:** `harness-self-check.ps1` round-trip assertions; optionally the runner for internal invariants.

### ISO timestamp
**Source:** `scripts/test-windows-shell-write-deny.ps1:60` — `Get-Date -Format 'yyyy-MM-ddTHH:mm:ss.fffZ'`
**Apply to:** the `timestamp` field of every verdict object.

### Child-process spawn + exit-code check (deferred to feature gates)
**Source:** `scripts/windows-test-harness.ps1:14-43` `Invoke-LoggedCargo`
**Apply to:** NOT this phase's runner; reuse the *idiom* (not the file, D-10) in phases 77-80/78 gates that wrap cargo/msiexec.

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `scripts/gates/harness-self-check.ps1` (gate-contract shape) | gate plugin | transform | No `scripts/gates/*.ps1` exists yet; the `Test-Precondition`/`Invoke-Gate` dot-sourced-plugin contract is net-new (defined by CONTEXT.md D-05/D-06/D-11). Internal idioms (JSON, assertions, timestamp) are covered by the analogs above; only the two-function plugin *contract* has no precedent. |

## Metadata

**Analog search scope:** `scripts/` (all PowerShell), `.gitignore`
**Files scanned:** `windows-test-harness.ps1`, `validate-windows-msi-contract.ps1`, `test-windows-shell-write-deny.ps1`, `check-upstream-drift.ps1`, `.gitignore`; globbed `scripts/gates/*.ps1` (empty), grepped `ConvertTo-Json` across `scripts/` (1 hit)
**Pattern extraction date:** 2026-06-16
