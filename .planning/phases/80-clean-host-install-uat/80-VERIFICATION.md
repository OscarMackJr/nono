---
phase: 80-clean-host-install-uat
verified: 2026-06-18T00:00:00Z
status: human_needed
score: 5/6 must-haves verified
overrides_applied: 0
human_verification:
  - test: "Run the clean-host-install gate on a fresh Win11 VM (no prior nono, no VC++ runtime) with the MSI rebuilt after Plan 80-01"
    expected: "Gate exits 0, verdict PASS, detail.versionOutput contains nono version string, detail.installExitCode is 0 or 3010, detail.wfpServiceState is 'Running' or 'Stopped' (non-fatal)"
    why_human: "Requires operator-provisioned clean Win11 VM/snapshot — SKIP_HOST_UNAVAILABLE is the structurally-correct result on the dirty dev host. Live PASS is the only path to confirming SC #1, #4 (no VC++ rollback, static-CRT proof, PATH propagation on clean host). Correctly scoped as D-01 by the phase; this is a host-gated item, not a defect."
---

# Phase 80: Clean-Host Install UAT — Verification Report

**Phase Goal:** Verify the machine MSI installs and runs cleanly on a fresh Win11 host with no manual prerequisite steps, closing the Phase 67 v2.11 carry-forward with an unattended scripted gate rather than an interactive human UAT.
**Verified:** 2026-06-18
**Status:** human_needed
**Re-verification:** No — initial verification

---

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | D-03: All Windows MSVC build paths include +crt-static (local dev, CI, release) | VERIFIED | `.cargo/config.toml` line 10 sets `rustflags = ["-C", "target-feature=+crt-static"]` under `[target.x86_64-pc-windows-msvc]`; `ci.yml` line 337 has step-level `RUSTFLAGS: -Dwarnings -C target-feature=+crt-static` on the windows-packaging build step; `release.yml` lines 86-87 and 92-93 have the same on both "Build (Windows — static CRT)" and "Build broker (Windows)" steps |
| 2 | D-03: Linux and macOS CI/release builds are unaffected by +crt-static | VERIFIED | `ci.yml` global env block (lines 12-15) retains `RUSTFLAGS: -Dwarnings` only; `release.yml` non-Windows "Build" step (`if: matrix.target != 'aarch64-unknown-linux-gnu' && runner.os != 'Windows'`, line 81) has no `+crt-static` in its env |
| 3 | D-04: `nono-wfp-service` start failure is non-fatal — install does not roll back | VERIFIED | `scripts/build-windows-msi.ps1` `$serviceComponentXml` here-string contains `Vital="no"` (line 239) and `ErrorControl="ignore"` (line 237) on the ServiceInstall element; `ErrorControl="normal"` is absent (0 matches); no lowercase `vital` present (0 matches) |
| 4 | D-04: Contract validator locks Vital="no" + ErrorControl="ignore" against future revert | VERIFIED | `scripts/validate-windows-msi-contract.ps1` lines 216-219 contain both `Assert-Equal -Actual $machineServiceInstall.ErrorControl -Expected "ignore"` and `Assert-Equal -Actual $machineServiceInstall.Vital -Expected "no"`; neither asserts on `$machineServiceControl` (which has no Vital attribute in WiX v4 XSD) |
| 5 | The clean-host gate exists, exports Test-Precondition + Invoke-Gate, and emits SKIP_HOST_UNAVAILABLE on the dirty dev host | VERIFIED | `scripts/gates/clean-host-install.ps1` exists (188 lines); `grep -c "function Test-Precondition"` = 1; `grep -c "function Invoke-Gate"` = 1; no bare `exit` calls (0 matches on `^\s*exit\b`); `Persist-Verdict` appears only in header comment (rule doc), not as a call; `.nono-runtime/verdicts/clean-host-install.json` contains `"verdict":"SKIP_HOST_UNAVAILABLE"` with reason "nono.exe detected under C:\Program Files\nono" |
| 6 | Gate emits PASS on a fresh Win11 VM with the rebuilt MSI (ROADMAP SC #1 and #4: no VC++ rollback, nono --version succeeds in a new session) | HOST-GATED | Correctly deferred to operator-provided clean VM per D-01; SKIP_HOST_UNAVAILABLE is the only achievable result on the dirty dev host. This is structural design, not a defect. |

**Score:** 5/6 truths verified on the dev host. Truth #6 is host-gated per D-01 (clean VM required).

---

### Deferred Items

Items not yet met but explicitly scoped to operator-provisioned environment, not a later phase.

| # | Item | Status | Evidence |
|---|------|--------|----------|
| 1 | Live PASS verdict on clean Win11 VM | HOST-GATED (D-01) | Gate correctly emits SKIP_HOST_UNAVAILABLE on dirty dev host. PASS path: operator provisions fresh Win11 snapshot, stages rebuilt MSI, runs `pwsh scripts/verify-dark.ps1 --gate clean-host-install` elevated. |
| 2 | Publicly-trusted code signing for broker/supervised path (DIST-SIGN-01) | DEFERRED TO ENTERPRISE MILESTONE | Out of scope per D-05; not tested or fixed in Phase 80. POC-cert broker refusal on clean host is a known documented limitation. |

---

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `.cargo/config.toml` | `[target.x86_64-pc-windows-msvc]` + `rustflags = ["-C", "target-feature=+crt-static"]` | VERIFIED | File exists (11 lines); correct stanza confirmed; header comment documents mutual-exclusion caveat |
| `.github/workflows/ci.yml` | Step-level `RUSTFLAGS: -Dwarnings -C target-feature=+crt-static` on windows-packaging build step; global env unchanged | VERIFIED | Line 337 confirmed; global env lines 12-15 unchanged (`-Dwarnings` only) |
| `.github/workflows/release.yml` | "Build (Windows — static CRT)" and "Build broker (Windows)" steps both gated `if: runner.os == 'Windows'` with step-level RUSTFLAGS; non-Windows "Build" step excluded | VERIFIED | Lines 84-93 confirmed; non-Windows guard on line 81 (`runner.os != 'Windows'`) confirmed; 2 matches for crt-static |
| `scripts/build-windows-msi.ps1` | `Vital="no"` + `ErrorControl="ignore"` on ServiceInstall in `$serviceComponentXml` here-string; `ErrorControl="normal"` removed; ServiceControl unchanged; `Start="install"` preserved | VERIFIED | All conditions confirmed; `Start="install"` has 2 occurrences (both ServiceControl and ServiceInstall parameter — correct); no lowercase `vital` |
| `scripts/validate-windows-msi-contract.ps1` | Two new `Assert-Equal` calls on `$machineServiceInstall.ErrorControl` and `$machineServiceInstall.Vital`; no assertion on `$machineServiceControl.vital` | VERIFIED | Lines 216-219 confirmed; `machineServiceControl.vital` count = 0 |
| `scripts/gates/clean-host-install.ps1` | Test-Precondition + Invoke-Gate; no `exit` calls; no `Persist-Verdict` calls; `LiteralPath` used; `Start-Process` ≥3; no `$LASTEXITCODE` | VERIFIED | All contract gates pass: Test-Precondition=1, Invoke-Gate=1, bare exit=0, Persist-Verdict in comment only, LiteralPath=4, Start-Process=5, LASTEXITCODE=0 |

---

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `.cargo/config.toml` | `x86_64-pc-windows-msvc` local dev builds | `[target.x86_64-pc-windows-msvc]` rustflags | WIRED | Correct stanza; comment documents that RUSTFLAGS env var takes precedence (CI) |
| `.github/workflows/ci.yml` | windows-packaging CI build | Step-level RUSTFLAGS env on "Build Windows release binaries" step | WIRED | Line 337; overrides global `-Dwarnings` for that step only |
| `.github/workflows/release.yml` | Windows MSI release builds | Separate Windows-only steps with step-level RUSTFLAGS; non-Windows step excluded | WIRED | "Build (Windows — static CRT)" line 84-87; "Build broker (Windows)" line 90-93; non-Windows step uses different `if:` guard |
| `scripts/build-windows-msi.ps1` | `dist/windows/nono-machine.wxs` (generated) | `Write-Utf8NoBomCompat` call from `$serviceComponentXml` here-string | WIRED | `Vital="no"` and `ErrorControl="ignore"` in the here-string; generated `.wxs` not directly modified (git diff clean) |
| `scripts/validate-windows-msi-contract.ps1` | `scripts/build-windows-msi.ps1` | Assert-Equal reads ServiceInstall.Vital + ErrorControl from generated .wxs XML | WIRED | Lines 216-219; asserts on `$machineServiceInstall` (correct node) |
| `scripts/gates/clean-host-install.ps1` | `scripts/verify-dark.ps1` | Dot-source auto-discovery (runner globs `scripts/gates/*.ps1`) | WIRED | Gate exports `Test-Precondition` and `Invoke-Gate` at module level; runner invoked with `-Gate clean-host-install`; exit 3 confirmed |
| `scripts/gates/clean-host-install.ps1` | `.nono-runtime/verdicts/clean-host-install.json` | Runner `Persist-Verdict` after `Invoke-Gate` returns | WIRED | Verdict file confirmed present with correct JSON shape; gate does NOT call Persist-Verdict (only in comment) |

---

### Data-Flow Trace (Level 4)

Not applicable — Phase 80 delivers build pipeline changes and a gate script, not a component that renders dynamic data. The gate's data flow is verified structurally: verdict JSON exists on disk with correct content.

---

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Gate emits SKIP_HOST_UNAVAILABLE (exit 3) on dirty dev host | `pwsh scripts/verify-dark.ps1 -Gate clean-host-install` (direct invocation) | exit 3; verdict file contains `"verdict":"SKIP_HOST_UNAVAILABLE"`, reason "nono.exe detected under C:\Program Files\nono" | PASS |
| `.cargo/config.toml` has correct target-scoped stanza | `grep -c "target.x86_64-pc-windows-msvc" .cargo/config.toml` | 1 | PASS |
| `Vital="no"` present in MSI build script | `grep -c 'Vital="no"' scripts/build-windows-msi.ps1` | 1 | PASS |
| Old `ErrorControl="normal"` fully removed | `grep -c 'ErrorControl="normal"' scripts/build-windows-msi.ps1` | 0 | PASS |
| Gate has no bare exit calls | `grep -n "^\s*exit\b" scripts/gates/clean-host-install.ps1` | 0 matches | PASS |
| Gate emits PASS on clean VM | Requires operator-provisioned clean Win11 VM | NOT RUN — host-gated | SKIP |

---

### Probe Execution

No probe scripts declared or applicable for Phase 80. The SUMMARY documents the verified dark-factory gate invocation result in place of a formal probe.

---

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| INST-01 | 80-01, 80-02 | Clean-host MSI install closure — Phase 67 carry-forward | PARTIALLY SATISFIED (host-gated) | Build fix (D-03 +crt-static, D-04 Vital="no") fully wired; gate structural contract satisfied; live PASS requires operator clean-VM run per D-01 |

---

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| None | — | — | — | — |

Scanned files: `.cargo/config.toml`, `.github/workflows/ci.yml`, `.github/workflows/release.yml`, `scripts/build-windows-msi.ps1`, `scripts/validate-windows-msi-contract.ps1`, `scripts/gates/clean-host-install.ps1`.

No TBD/FIXME/XXX markers. No stubs or placeholder implementations. No hardcoded empty returns in gate logic. All Start-Process calls use `-Wait -PassThru` with `.ExitCode` reads (never `$LASTEXITCODE`). The `Persist-Verdict` mention on line 8 of the gate is a contract documentation comment explaining what the gate must NOT do — not a call.

**One operational caveat (from 80-02-SUMMARY Verification Note, not a defect):** `pwsh -Command "scripts/verify-dark.ps1 -Gate clean-host-install"` exits 1, not 3, due to a pre-existing PowerShell `-Command` flag quirk that swallows `exit N` from dot-sourced scripts. The correct invocations (`pwsh -File scripts/verify-dark.ps1 -Gate clean-host-install` or direct bare path) exit 3 correctly. This affects every gate equally and is documented as an operational guidance item for the Phase 81 aggregator, not a clean-host-install defect.

---

### Human Verification Required

#### 1. Live PASS on Clean Win11 VM (ROADMAP SC #1, #2, #4)

**Test:** On a fresh Win11 VM (no prior nono install, no VC++ runtime). Stage the MSI rebuilt AFTER Plan 80-01 lands (`dist\windows\nono-machine.msi`). From an ELEVATED PowerShell, run:
```
pwsh scripts/verify-dark.ps1 -Gate clean-host-install
```

**Expected:**
- Exit code 0
- `.nono-runtime/verdicts/clean-host-install.json` contains `"verdict": "PASS"`
- `detail.versionOutput` contains the nono version string (e.g., `nono 0.x.y`)
- `detail.installExitCode` is 0 or 3010
- `detail.wfpServiceState` is `"Running"` or `"Stopped"` (either is acceptable — service start is non-fatal per D-04)
- `detail.uninstallExitCode` is 0 (cleanup successful)

**Why human:** Requires operator-provisioned clean Win11 VM or snapshot with no prior nono install and no VC++ runtime. Structurally correct to defer (D-01) — the dirty dev host cannot emulate this condition.

**Note:** The MSI must be REBUILT after Plan 80-01 commits (`a517284b`, `cd856641`) to incorporate the `+crt-static` binary linkage and `Vital="no"` in the generated `.wxs`. A pre-80-01 MSI would still fail on a clean host.

---

### Gaps Summary

No blocking gaps found. All structurally-verifiable must-haves pass on the dev host. The only remaining item is the host-gated live PASS on a clean Win11 VM, which is correctly scoped as an operator-run step (D-01) and confirmed SKIP_HOST_UNAVAILABLE on the dirty host — the designed outcome.

The phase delivers:
1. A two-pronged `+crt-static` wiring that covers every Windows MSVC build path without affecting Linux/macOS.
2. `Vital="no"` + `ErrorControl="ignore"` on `ServiceInstall` in the MSI generator, with contract assertions locking the non-fatal posture.
3. A fully-compliant Phase 76 gate (`Test-Precondition` / `Invoke-Gate`) that emits `SKIP_HOST_UNAVAILABLE` on a dirty host and is wired to emit `PASS` on a clean one.

The phase goal is structurally achieved on the dev host. Live closure of ROADMAP SC #1/#4 requires the operator clean-VM run.

---

_Verified: 2026-06-18_
_Verifier: Claude (gsd-verifier)_
