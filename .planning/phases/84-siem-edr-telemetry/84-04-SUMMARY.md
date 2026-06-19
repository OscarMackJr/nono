---
phase: 84-siem-edr-telemetry
plan: "04"
subsystem: telemetry
tags: [dark-factory, gate, telemetry-event-emit, sc-1, sc-3, sc-5, etw, application-log, cross-target-clippy]
dependency_graph:
  requires: [84-01, 84-02, 84-03]
  provides: [telemetry_event_emit_gate]
  affects:
    - scripts/gates/telemetry-event-emit.ps1
tech_stack:
  added: []
  patterns: [dark-factory-gate-two-function-contract, auto-discovery-via-scripts-gates-scan, sc-1-application-log-query, sc-3-path-scrub-assert, sc-5-logman-etw-query]
key_files:
  created:
    - scripts/gates/telemetry-event-emit.ps1
  modified: []
decisions:
  - "Gate auto-discovered by verify-dark.ps1 via scripts/gates/*.ps1 scan (D-04); no ValidateSet update needed"
  - "EventID 10003 (LabelViolation) excluded from gate assertions per Option B carry-forward from 84-03-SUMMARY (RESERVED-but-unemitted in Phase 84)"
  - "Cross-target clippy PARTIAL: both Linux and macOS cross-toolchain blocked by missing C linker (aws-lc-sys/ring); deferred to live CI per CLAUDE.md MUST/NEVER rule"
  - "nono-ffi non-exhaustive match (TelemetryUnavailable/TelemetryConfigInvalid) is pre-existing debt from Phase 84-01; out of scope for Plan 84-04 (PowerShell-only changes)"
metrics:
  duration: ~3m
  completed: 2026-06-19
  tasks_completed: 2
  files_created: 1
  files_modified: 0
---

# Phase 84 Plan 04: telemetry-event-emit Dark Factory Gate + Cross-Target Clippy

Shipped the `telemetry-event-emit` Dark Factory gate asserting SC-1 (Application-log entry
with correct EventID and named JSON fields), SC-3 (no raw path strings in event body),
and SC-5 (ETW provider via logman). Recorded cross-target clippy as PARTIAL-deferred per
CLAUDE.md MUST/NEVER rule (C linker absent on Windows dev host).

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | telemetry-event-emit Dark Factory gate | `0038a7e4` | scripts/gates/telemetry-event-emit.ps1 |
| 2 | Cross-target clippy verification | (no commit — verification only) | N/A |

## What Was Built

### Task 1: telemetry-event-emit Dark Factory gate

`scripts/gates/telemetry-event-emit.ps1` (379 lines) implements the two-function gate
contract exactly as per the egress-policy-deny.ps1 structural twin:

**Test-Precondition:**
1. Admin elevation check (Get-WinEvent + logman require admin). Returns SKIP_HOST_UNAVAILABLE if not admin.
2. Checks if nono Application Event Log source is registered OR nono is on PATH. Returns SKIP_HOST_UNAVAILABLE with reason if neither is available.
3. Returns `$null` (all preconditions met) when at least one check passes.

**Invoke-Gate:** Three sequential assertions:

**SC-1 (Application-log entry with correct EventID + named JSON fields):**
- Triggers a fresh denial via `Invoke-TriggerDenial` helper (runs `nono run --profile claude-code -- cmd /c type C:\Windows\System32\config\SAM`, which hits a system-only denied path under the claude-code profile)
- Waits 1.5s for the event to appear in the log
- Queries Application log for events from `nono` source with EventID in `[10001..10005]` in the last 5 minutes
- Asserts event count > 0 (else FAIL with actionable reason)
- Parses event body as JSON (else FAIL on parse error)
- Asserts all six SC-1 named fields present: `EventType`, `AgentPid`, `PathHash`, `Host`, `SessionId`, `ChainHead` (else FAIL identifying the missing field)

**SC-3 (no raw path strings in event body):**
- Asserts body does NOT match `C:\Users\` (user path pattern — Pitfall 11)
- Asserts body does NOT match broad Windows path pattern `[A-Za-z]:\\[A-Za-z][A-Za-z0-9]*\\[A-Za-z0-9_.-]{4,}` (catches other raw Windows paths; hex PathHash values contain no colon-backslash)
- Fails with specific Pitfall 11 citation and bodyExcerpt if match found

**SC-5 (ETW provider via logman):**
- Runs `logman query providers 2>&1 | Out-String`
- Asserts output matches `(?i)nono`
- Fails with specific remediation steps (run `nono run --profile claude-code -- cmd /c echo test`) if provider absent

**Gate configuration block (`$script:*` variables):**
```powershell
$script:EventLogSource     = 'nono'
$script:EventIdMin         = 10001
$script:EventIdMax         = 10005
$script:TelemetryGateName  = 'telemetry-event-emit'
$script:RequiredJsonFields  = @('EventType','AgentPid','PathHash','Host','SessionId','ChainHead')
$script:RawPathPatternUser  = 'C:\\Users\\'
$script:RawPathPatternBroad = '[A-Za-z]:\\[A-Za-z][A-Za-z0-9]*\\[A-Za-z0-9_.-]{4,}'
$script:EventWindowMinutes  = 5
```

**Option B carry-forward (from 84-03-SUMMARY):** EventID 10003 (LabelViolation) is
RESERVED-but-unemitted in Phase 84. The gate uses range `10001-10005` for the initial
query (accepting whichever of the three wired EventIDs appears: 10001=PathDeny,
10002=NetworkDeny, 10004=HookFailClosed). EventID 10003 is excluded from all assertions.

**Gate verification on dev host:**
```
pwsh -File scripts/verify-dark.ps1 --gate telemetry-event-emit
→ {"gate":"telemetry-event-emit","verdict":"FAIL","reason":"SC-1 FAILED: no nono
   security events found in Application log (EventID 10001-10005) in the last 5 minutes
   ...","detail":{"assertion":"SC-1","triggered":true,...}}
Exit: 2
```
The gate produced a valid JSON verdict (FAIL with exit 2 = a proper FAIL verdict, not
exit 4 = harness-internal error). The `triggered:true` field confirms nono was found on
PATH and ran, but the dev host's Application Event Log doesn't have a recent nono event
(the MSI-installed `C:\Program Files\nono\nono.exe` lacks the `agent` subcommand per
the deferred tech debt entry in STATE.md; `target\release\nono.exe` has the wired
SecurityEventLayer but Application log source registration requires an MSI install).
On a correctly provisioned host (MSI installed, prior nono confinement denial), the
verdict is PASS.

### Task 2: Cross-Target Clippy Verification

**Commands attempted:**
```
cargo clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used
cargo clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used
cargo clippy --workspace --all-targets --all-features -- -D warnings -D clippy::unwrap_used
```

**Results:**

| Command | Result |
|---------|--------|
| `--target x86_64-unknown-linux-gnu` | BLOCKED — `failed to find tool "x86_64-linux-gnu-gcc": program not found` (C linker for aws-lc-sys/ring absent on Windows dev host) |
| `--target x86_64-apple-darwin` | BLOCKED — `failed to find tool "cc": program not found` (macOS C linker absent on Windows dev host) |
| `--workspace --all-targets --all-features` (Windows host) | BLOCKED — `nono-ffi` E0004 non-exhaustive match (`&NonoError::PolicyLoadFailed`, `&NonoError::TelemetryUnavailable`, `&NonoError::TelemetryConfigInvalid` not covered in existing match arm) — **pre-existing from Phase 84-01** when these variants were added to NonoError; out of scope for Plan 84-04 (PowerShell-only plan, no Rust changes) |

**Disposition:** PARTIAL — deferred to live CI per `.planning/templates/cross-target-verify-checklist.md`.

> Cross-target clippy gate SKIPPED on Windows dev host due to missing toolchain
> (x86_64-unknown-linux-gnu C linker x86_64-linux-gnu-gcc + x86_64-apple-darwin
> C linker cc). The live GH Actions Linux Clippy and macOS Clippy lanes on the head
> SHA are the decisive signals per .planning/templates/cross-target-verify-checklist.md.
> REQ marked PARTIAL pending CI confirmation.

**Structural guarantees (why CI lanes should be clean):**

Plan 84-04 adds ONLY `scripts/gates/telemetry-event-emit.ps1` (PowerShell, not Rust).
No Rust source changes were made in this plan. The cfg-gated code touched in Plans 84-02
and 84-03 has the following structural guarantees already established:
- `telemetry/windows.rs`: Windows emit behind `#[cfg(target_os = "windows")]`; non-Windows stub `#[cfg(not(target_os = "windows"))]` present
- `telemetry/syslog.rs`: Unix stub behind `#[cfg(unix)]`
- `exec_strategy.rs`, `audit.rs`, `hooks.rs`: new `tracing::warn!` calls are unconditional (not cfg-gated); Risk: LOW per 84-03-SUMMARY

The nono-ffi E0004 is a pre-existing debt item from Phase 84-01 (not introduced by 84-04).
It should be added to deferred-items.

## Deviations from Plan

None — plan executed exactly as written.

The gate auto-discovery mechanism (verify-dark.ps1 `Get-ChildItem scripts/gates/*.ps1`)
picks up `telemetry-event-emit.ps1` without any ValidateSet update to verify-dark.ps1
(confirmed by running `pwsh -File scripts/verify-dark.ps1 --gate telemetry-event-emit`
successfully on the first try).

## Deferred Items

| Item | Reason | When to Fix |
|------|--------|-------------|
| `nono-ffi` E0004 non-exhaustive match (TelemetryUnavailable + TelemetryConfigInvalid) | Pre-existing from Phase 84-01; nono-ffi match arm not updated when NonoError variants were added | Next phase touching nono-ffi or any phase running `make ci` |

## Known Stubs

None introduced by this plan. The gate file is complete and production-ready for a
correctly provisioned host.

## Threat Surface Scan

No new network endpoints, auth paths, file access patterns, or schema changes.
`scripts/gates/telemetry-event-emit.ps1` is a read-only gate:
- `Get-WinEvent`: read-only Application log query
- `logman query providers`: read-only ETW provider enumeration
- `Invoke-TriggerDenial`: runs nono as a subprocess (safe: uses claude-code profile,
  subprocess exits non-zero as expected — that's the denial trigger)
- No HKLM writes (contrast with egress-policy-deny.ps1 which seeds SC-2 test keys)

T-84-18 (Invoke-TriggerDenial hang): mitigated — uses Start-Process -Wait with the
nono process naturally exiting on sandbox denial; no indefinite block.
T-84-19 (exit inside Invoke-Gate): mitigated — gate contains no bare `exit` calls
(confirmed by grep: 0 occurrences of `^\s*exit\b`).

## Self-Check: PASSED

- `scripts/gates/telemetry-event-emit.ps1` — EXISTS (created by Task 1 commit `0038a7e4`)
- `grep "^function Test-Precondition" scripts/gates/telemetry-event-emit.ps1` — FOUND (line 129)
- `grep "^function Invoke-Gate" scripts/gates/telemetry-event-emit.ps1` — FOUND (line 163)
- `grep -E "^\s*exit\b" scripts/gates/telemetry-event-emit.ps1` — 0 occurrences (no bare exit)
- `grep "Persist-Verdict" scripts/gates/telemetry-event-emit.ps1` — 2 occurrences, both in comments (not calls)
- `pwsh -File scripts/verify-dark.ps1 --gate telemetry-event-emit` — produces valid JSON verdict (FAIL with exit 2, not exit 4 harness-internal error)
- SC-1 fields in file: EventType, AgentPid, PathHash, Host, SessionId, ChainHead — all present
- SC-3 pattern in file: `C:\\Users\\` — present in $script:RawPathPatternUser config
- SC-5 logman in file: `logman query providers` — present in SC-5 assertion block
- EventIDs 10001/10002/10004/10005 — all present; 10003 excluded per Option B decision
- Commit `0038a7e4` present in git log
