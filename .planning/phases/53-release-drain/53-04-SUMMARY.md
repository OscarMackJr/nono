---
phase: 53-release-drain
plan: "04"
subsystem: release-uat
tags: [release, uat, human-verify, windows, wfp, signing, authenticode, ci]
dependency_graph:
  requires:
    - phase: 53-release-drain
      plan: "01"
      provides: workspace-at-0.57.4
    - phase: 53-release-drain
      plan: "03"
      provides: release.yml-semver-only-trigger
  provides:
    - uat-a-ci-release-pass
    - uat-c-wfp-uninstall-pass
    - req-drn-01-closed
    - release-yml-signing-order-fix
  affects: [req-rls-01-reopened, v2.8-tag-pending-resigned-build]
tech_stack:
  added: []
  patterns: [sign-before-MSI-harvest, MSI-payload Authenticode verification gate]
key_files:
  created: []
  modified:
    - .planning/todos/done/2026-05-27-wix-auto-uninstall-wfp-custom-action-plus-live-uat.md
    - .planning/REQUIREMENTS.md
    - .github/workflows/release.yml
key_decisions:
  - "UAT-A verified objectively from CI evidence (tags already cut + pushed in a prior session); operator confirmed the live legs B + C"
  - "UAT-B FAIL: MSI payload binaries Authenticode NotSigned (signing-order defect in release.yml) — REQ-RLS-01 reopened"
  - "UAT-C PASS: all WFP stop/uninstall fixes confirmed on Win11 26200 — REQ-DRN-01 closed, Todo 1 moved to done/"
  - "Operator chose to fix release.yml signing-order in-session rather than defer"
  - "release.yml fixed, re-released as v0.57.5; UAT-B re-run PASS — REQ-RLS-01 closed"
status: complete
---

# Plan 53-04 SUMMARY — Operator-gated HUMAN-UAT (REQ-RLS-01/02, REQ-DRN-01)

Three operator-gated HUMAN-UAT checkpoints executed on a live Windows 11 (build
26200) host. UAT-A and UAT-C passed against `v0.57.4`. UAT-B exposed a
release-blocking Authenticode signing-order defect in `release.yml`; it was fixed
in-session, re-released as **v0.57.5**, and **UAT-B re-run PASSED** — all three
checkpoints are now green and REQ-RLS-01/02 + REQ-DRN-01 are closed.

## UAT Results

| UAT | Requirement | Result | Summary |
|-----|-------------|--------|---------|
| A | REQ-RLS-02 | ✅ PASS | `release.yml` ran clean on the `v0.57.4` tag; signed artifacts uploaded |
| B | REQ-RLS-01 | ✅ PASS (on re-run) | First attempt (v0.57.4) FAILED — payload `NotSigned`; fixed + re-released as v0.57.5; re-run install confirms payloads `Valid`, version `0.57.5` |
| C | REQ-DRN-01 | ✅ PASS | Elevated WFP stop/uninstall leaves nothing behind |

### UAT-A — CI release run (REQ-RLS-02) — PASS

Tags `v0.57.4` + `v2.8` were already cut and pushed to `origin` (both at HEAD
`6c6f3b25`) in a prior session. Verified from CI evidence (run `26615804419`):

- All 5 build jobs green, incl. `Build x86_64-pc-windows-msvc` (11m8s).
- `Create Release` green — signed artifacts uploaded.
- Sign step log: `All artifacts signed and verified.`
- Verify step log: `Authenticode OK:` for `nono.exe`, `nono-shell-broker.exe`,
  `…-machine.msi`, `…-user.msi` (+ zip payloads).
- Release page (https://github.com/oscarmackjr-twg/nono/releases/tag/v0.57.4) has
  `…-machine.msi`, `…-user.msi`, `…-msvc.zip`, `SHA256SUMS.txt`.
- **No `startup_failure`** (the old 0s failures were pre-fix 2026-05-27 wip commits).
- `v2.8` (two-segment tag) did **not** fire a CI run — confirms the D-53-06 `v*.*.*`
  trigger works as designed.
- The overall run shows "failure" only because of two non-blocking cosmetic fork jobs:
  `Publish to crates.io` (HTTP 303, no `CARGO_REGISTRY_TOKEN`) and `Bump Homebrew Core
  Formula`. Both are the explicitly-acceptable fork failures called out in the plan.

### UAT-B — signed MSI install + version + no-PTY path (REQ-RLS-01) — FAIL

Operator installed `nono-v0.57.4-…-machine.msi` (after importing the POC cert
`CN=nono Test Signing`, thumbprint `F45DBEA6…`, into `LocalMachine\Root` +
`TrustedPublisher`). Diagnostics:

| Check | Result |
|-------|--------|
| MSI package signature `(Get-AuthenticodeSignature …machine.msi).Status` | `Valid` |
| Installed `C:\Program Files\nono\nono.exe` signature | **`NotSigned`** |
| Installed `C:\Program Files\nono\nono-shell-broker.exe` signature | **`NotSigned`** |
| `nono --version` | `nono 0.57.4` (correct) |
| `(Get-Command nono).Source` | `C:\Program Files\nono\nono.exe` |

**Root cause — `release.yml` signing-order defect.** The MSI *wrapper* is signed
(`Valid`), but the binaries it installs are unsigned. The Windows build job ran
`Package (Windows)` (which invokes `build-windows-msi.ps1` to harvest
`nono.exe` / `nono-shell-broker.exe` / `nono-wfp-service.exe` into the MSIs)
**before** `Sign Windows artifacts`. So the loose `.exe` assets, the zip payload,
and the MSI wrappers all verify `Valid`, but the binaries embedded in the MSI were
never signed. CI never caught this because the verify steps checked only the loose
binaries + the MSI wrapper, never the MSI *payload*.

**No-PTY supervised path — not validated (separate gate).**
`nono run --profile claude-code -- nono --version` failed *before* reaching the
broker with: `Platform not supported: Windows filesystem policy does not cover the
executable path required for launch: C:\Program Files\nono\nono.exe`. This is the
executable-coverage gate behaving correctly — the `claude-code` profile does not
cover `C:\Program Files\nono`, so launching `nono.exe` as the sandboxed child is
refused. The plan's "use `nono` as the child" proxy test does not work for a
Program-Files install. (Moot for this build anyway: the broker is unsigned, so the
D-32-12 self-trust-anchor gate would reject it.) The no-PTY relay fix
(`d8b7ce00` + `005b4c9e`) remains validated only at dev-layout (per v2.7 close), not
on a signed MSI install.

### UAT-C — elevated WFP stop/uninstall (REQ-DRN-01) — PASS

All 5 steps passed in an elevated PowerShell on Windows 11 build 26200:

1. **Fix #1 (SERVICE_CONTROL_STOP):** `sc.exe stop nono-wfp-service` → accepted,
   `STOPPABLE`, no fast-fail. ✓
2. **Fix #2a (`nono setup --uninstall-wfp`):** both `nono-wfp-service` +
   `nono-wfp-driver` removed; `sc query` on each → `1060 does not exist`. ✓
3. No residual filters: `netsh wfp show filters | Select-String nono` → empty. ✓
4. **Fix #2b (CaUninstallWfpServices WiX CA):** `msiexec /x …machine.msi` → no
   service, no driver, `Test-Path "C:\Program Files\nono"` = `False`, no filters. ✓
5. **Upgrade guard (`NOT UPGRADINGPRODUCTCODE`):** double `msiexec /i` → service
   still `RUNNING`, not torn down. ✓

No CA-fallback (immediate-CA + CustomActionData) was needed. **REQ-DRN-01 satisfied.**
Todo 1 (`2026-05-27-wix-auto-uninstall-wfp-custom-action-plus-live-uat.md`) moved to
`.planning/todos/done/` with a PASS Disposition footer (commit `bbcb5f97`).

## release.yml signing-order fix (applied in-session)

Per operator decision ("Fix release.yml signing-order now"), `release.yml` was
restructured so the MSI payload is signed:

- **New step `Sign Windows binaries (pre-package)`** — signs `nono.exe`,
  `nono-shell-broker.exe`, and `nono-wfp-service.exe` **before** `Package (Windows)`,
  so `build-windows-msi.ps1` harvests already-signed binaries.
- **`Sign Windows artifacts` → `Sign Windows MSIs`** — now signs only the two MSI
  package wrappers (the payload binaries are already signed).
- **New step `Verify MSI payload signatures (Windows)`** — administrative-extracts
  each MSI (`msiexec /a … TARGETDIR=…`) and runs `Get-AuthenticodeSignature` over
  every extracted `.exe`/`.sys`, failing closed on any non-`Valid` status. This is
  the regression guard for the blind spot that shipped this bug — CI now verifies the
  *payload*, not just the loose binaries and the wrapper.

The checked-in `nono-wfp-driver.sys` remains the WHQL-pre-signed copy (unchanged).

### Fix iteration (two CI rounds)

1. **v0.57.4 → v0.57.5 bump** (`ae5c3358`) + the reorder above. CI run `26638508604`
   proved the reorder works (`MSI payload Authenticode OK: nono-shell-broker.exe`)
   but the new payload-verify step over-reached: it gated on
   `nono-wfp-driver.sys`, the WHQL/cross-signed kernel driver, whose chain returns
   `UnknownError` under `Get-AuthenticodeSignature` on the CI runner → build failed.
2. **Verify-gate narrowed to `.exe`** (`a3927be0`): strict-verify only the
   executables (`nono.exe`, broker, wfp-service); log the driver `.sys` status
   informationally. CI run `26639177584` Windows job green; `Create Release`
   published **v0.57.5** with all MSI payloads `MSI payload Authenticode OK`.

`v0.57.5` is tagged at `a3927be0`; `v2.8` was force-moved to the same commit. Both
CI "failures" are the cosmetic fork jobs only (`crates.io` HTTP 303, homebrew bump).

### UAT-B re-run (v0.57.5) — PASS

Operator installed `nono-v0.57.5-…-machine.msi`:

| Check | v0.57.4 (fail) | v0.57.5 (pass) |
|-------|----------------|-----------------|
| `nono.exe` Authenticode | `NotSigned` | **`Valid`** |
| `nono-shell-broker.exe` Authenticode | `NotSigned` | **`Valid`** |
| `nono-wfp-service.exe` Authenticode | (n/a) | **`Valid`** |
| `nono --version` | `0.57.4` | `0.57.5` |

**REQ-RLS-01 satisfied.** No-PTY supervised-path leg remains a documented test-design
gap on Program-Files installs (the `claude-code` profile does not cover
`C:\Program Files\nono`); the relay fix stays validated at dev-layout (v2.7 close).

## Requirement status

| Requirement | Status | Note |
|-------------|--------|------|
| REQ-RLS-01 | ✅ Complete | UAT-B PASS on v0.57.5 (payloads signed); no-PTY leg = dev-layout-validated |
| REQ-RLS-02 | ✅ Complete | UAT-A PASS |
| REQ-DRN-01 | ✅ Complete | UAT-C PASS; Todo 1 closed |
| REQ-DRN-02 | ✅ Complete | (Plan 53-02) |

## Follow-ups (post-phase, non-blocking)

1. **`v0.57.4` GitHub Release still has unsigned-payload MSIs** — a distribution
   hazard; delete or annotate it now that v0.57.5 supersedes it.
2. **No-PTY-on-installed-MSI** is unvalidated due to the executable-coverage gate;
   exercise it with a profile/child whose path is policy-covered, or accept the
   dev-layout validation from v2.7.

## Self-Check: PASS

All three HUMAN-UAT checkpoints PASS (UAT-A + UAT-C on v0.57.4; UAT-B on v0.57.5
after the in-session `release.yml` signing-order fix). REQ-RLS-01, REQ-RLS-02,
REQ-DRN-01 all closed; Todo 1 in `todos/done/`. Phase 53 ready to complete.
