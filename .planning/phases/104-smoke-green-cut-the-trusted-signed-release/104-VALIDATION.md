---
phase: 104
slug: smoke-green-cut-the-trusted-signed-release
status: draft
nyquist_compliant: true
wave_0_complete: false
created: 2026-07-03
---

# Phase 104 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## External-Blocker Note (READ FIRST)

Phase 104's terminal SCs (SC1 smoke green, SC2 tag-push green, SC3 Verified publisher) are **externally blocked**: the signing chain terminates in `CN=Microsoft Enterprise ID Root CA 2021`, which is currently ABSENT from Microsoft's distributed Trusted Root Program (live-confirmed via `certutil -generateSSTFromWU`). No code change fixes this — it resolves only when Microsoft propagates the root. The AUTONOMOUS, dev-host-verifiable work is: (1) the D-04 diagnostic-flush fix, (2) a NoCheck-classification regression guard, (3) `publish-crates` neutralization + a grep guard, (4) a cheap local root-availability pre-check for the operator's poll-until-green loop. The release CUT itself is an operator-in-loop checkpoint gated on the external root propagation.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Pester-free imperative PowerShell harness (`scripts/tests/test_*.ps1`, explicit exit codes) |
| **Config file** | none |
| **Quick run command** | `pwsh -File scripts/tests/test_verify_authenticode.ps1 -Case All` |
| **Full suite command** | the PowerShell harness above + `scripts/verify-release-yml-publish-selectors.ps1` (Rust `make ci` unaffected by this phase) |
| **Estimated runtime** | harness <30s (all mocked, no network) |

---

## Sampling Rate

- **After every task commit:** `pwsh -File scripts/tests/test_verify_authenticode.ps1 -Case All` after any `verify-authenticode.ps1` edit
- **After every plan wave:** full PowerShell harness + `scripts/verify-release-yml-publish-selectors.ps1`
- **Before phase gate:** both automated gates green; SC1/SC2/SC3 require explicit operator confirmation (outward-facing, gated on external root propagation)
- **Max feedback latency:** <30s (mocked harness)

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 104-D04 | 104-01 Task 1/2 | 1 | REL-01 | T-104-01-05 | `Write-ChainDiagnostic` output appears BEFORE any terminating throw even under `$ErrorActionPreference='Stop'` (fix: `Write-Error -ErrorAction Continue`); fail-closed `Status -ne 'Valid'` UNCHANGED | unit (new case) | `pwsh -File scripts/tests/test_verify_authenticode.ps1 -Case diagnosticFlushUnderStop` | ❌ W0 | ⬜ pending |
| 104-NCK | 104-01 Task 1/2 | 1 | REL-01 | T-104-01-02 | a mocked untrusted-root chain classifies `IsUntrustedRoot=$true, IsTransient=$false` (NoCheck-mode confirms it's a genuine untrusted root, not a revocation transient) via the new `Merge-NoCheckOverride` function | unit (new case) | `pwsh -File scripts/tests/test_verify_authenticode.ps1 -Case noCheckOverridesTransient` | ❌ W0 | ⬜ pending |
| 104-PUB | 104-02 Task 1/2 | 1 | REL-01 | T-104-02-01, T-104-02-02 | `release.yml` has NO *enabled* job referencing `-p nono`/`-p nono-proxy`/`-p nono-cli` (publish-crates neutralized via job-level `if: false`, not a secret-consuming no-op) | static/grep | `pwsh -File scripts/verify-release-yml-publish-selectors.ps1` (the 3 stale lines sit under an `if: false` job; guard fails if they appear anywhere else) | ❌ W0 | ⬜ pending |
| 104-PRECHK | 104-02 Task 2 | 1 | REL-01 | T-104-02-03 | a local `certutil -generateSSTFromWU`-based pre-check tells the operator whether `Microsoft Enterprise ID Root CA 2021` (thumbprint `991D364E97882715B80ED978F53E1F35DC2F07C2`) has propagated BEFORE burning a CI smoke dispatch | integration (local) | `pwsh -File scripts/azure/check-trusted-signing-root.ps1` → exit 0 PRESENT / exit 1 ABSENT / exit 2 harness-error (currently expected ABSENT) | ❌ W0 | ⬜ pending |
| 104-DOC | 104-02 Task 3 | 1 | REL-01 | T-104-02-05 | `REQUIREMENTS.md` REL-01 and `ROADMAP.md` Phase 104 SC3 no longer state the disproven issuer-substring heuristic as the Verified-publisher gate condition | static/grep | `grep -n "Microsoft ID Verified CS" .planning/REQUIREMENTS.md .planning/ROADMAP.md` (2 matches expected in REQUIREMENTS.md: SIGN-03 + CHOST-02, unchanged; 1 match expected in ROADMAP.md: Phase 103 SC2, unchanged; REL-01/Phase 104 SC3 lines no longer match) | ❌ W0 | ⬜ pending |
| 104-SC1 | 104-03 Task 2 | 2 | REL-01 | T-104-03-01, T-104-03-03 | operator re-runs smoke → GREEN (only possible once root propagates); FIC-subject canary re-checked first | manual (operator, outward-facing) | N/A — operator checkpoint | N/A | ⬜ blocked-external |
| 104-SC2 | 104-03 Task 3 | 2 | REL-01 | T-104-03-01 | operator pushes `v0.66.1` → Release workflow green, all .exe + MSIs pass hardened verify | manual (operator, outward-facing) | N/A — operator checkpoint | N/A | ⬜ blocked-external |
| 104-SC3 | 104-03 Task 3 | 2 | REL-01 | T-104-03-02 | published artifacts show Verified publisher (Status=Valid + non-test signer; NOT gated on the `Microsoft ID Verified CS` issuer substring — disproven Phase 101 Finding B) | manual (operator, outward-facing) | N/A — operator checkpoint | N/A | ⬜ blocked-external |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky · blocked-external = gated on Microsoft root propagation*
*Plan/Wave assignments finalized by `/gsd:plan-phase 104` (2026-07-03): Wave 1 = 104-01 (verify-authenticode.ps1 hardening) + 104-02 (release.yml neutralization + guards + doc fix), both autonomous, no file overlap. Wave 2 = 104-03 (operator checkpoint), depends on both Wave 1 plans, `autonomous: false`.*

---

## Wave 0 Requirements

- [ ] `scripts/tests/test_verify_authenticode.ps1` — add `Test-DiagnosticFlushUnderStop` case (D-04 flush-order regression guard) — **Plan 104-01 Task 2**
- [ ] `scripts/tests/test_verify_authenticode.ps1` — add `Test-NoCheckOverridesTransient` case (NoCheck-mode classification guard) — **Plan 104-01 Task 2**
- [ ] A fast local grep-based guard asserting `release.yml` has no *enabled* job referencing `-p nono`/`-p nono-proxy`/`-p nono-cli` — **Plan 104-02 Task 2** (`scripts/verify-release-yml-publish-selectors.ps1`)
- [ ] A local root-availability pre-check (certutil-based) for the operator's poll-until-green loop — **Plan 104-02 Task 2** (`scripts/azure/check-trusted-signing-root.ps1`)

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Smoke green (SC1) | REL-01 | Outward-facing CI dispatch + gated on external Microsoft root propagation | Operator re-runs `trusted-signing-smoke.yml`; blocked until `Microsoft Enterprise ID Root CA 2021` propagates — Plan 104-03 Task 2 |
| Tag-push Release green (SC2) | REL-01 | Outward-facing, irreversible tag push + live signing | Operator pushes `v0.66.1` once smoke is green — Plan 104-03 Task 3 |
| Verified publisher (SC3) | REL-01 | Requires a published artifact + clean-host trust | Operator verifies published artifact shows Status=Valid + non-test signer — Plan 104-03 Task 3 (also re-verified independently in Phase 106) |

*The autonomous fixes (D-04 flush, NoCheck guard, publish-crates neutralization, pre-check, doc correction) are all dev-host-automated (Plans 104-01/104-02); only the release cut (Plan 104-03) is manual/blocked-external.*

---

## Validation Sign-Off

- [x] All autonomous tasks have `<automated>` verify (harness cases / grep guard / local pre-check)
- [x] Sampling continuity: harness after every verify-authenticode edit
- [x] Wave 0 covers all MISSING references (2 new harness cases + grep guard + pre-check)
- [x] No watch-mode flags
- [x] Feedback latency < 30s
- [x] `nyquist_compliant: true` set — plans 104-01/104-02/104-03 assign task IDs to concrete plan/wave/task locations above
- [x] Fail-closed `Status -ne 'Valid'` gate is NEVER loosened by any task (external blocker resolved only by poll-until-green, never code-side acceptance)

**Approval:** pending (plans authored; execution not yet started)
