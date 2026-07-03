---
phase: 104
slug: smoke-green-cut-the-trusted-signed-release
status: draft
nyquist_compliant: false
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
| **Full suite command** | the PowerShell harness above + the new grep-based `publish-crates` selector guard (Rust `make ci` unaffected by this phase) |
| **Estimated runtime** | harness <30s (all mocked, no network) |

---

## Sampling Rate

- **After every task commit:** `pwsh -File scripts/tests/test_verify_authenticode.ps1 -Case All` after any `verify-authenticode.ps1` edit
- **After every plan wave:** full PowerShell harness + the `publish-crates` selector grep guard
- **Before phase gate:** both automated gates green; SC1/SC2/SC3 require explicit operator confirmation (outward-facing, gated on external root propagation)
- **Max feedback latency:** <30s (mocked harness)

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 104-D04 | TBD | 1 | REL-01 | verify-bypass | `Write-ChainDiagnostic` output appears BEFORE any terminating throw even under `$ErrorActionPreference='Stop'` (fix: `Write-Error -ErrorAction Continue`); fail-closed `Status -ne 'Valid'` UNCHANGED | unit (new case) | `pwsh -File scripts/tests/test_verify_authenticode.ps1 -Case diagnosticFlushUnderStop` | ❌ W0 | ⬜ pending |
| 104-NCK | TBD | 1 | REL-01 | untrusted-root-accept | a mocked untrusted-root chain classifies `IsUntrustedRoot=$true, IsTransient=$false` (NoCheck-mode confirms it's a genuine untrusted root, not a revocation transient) | unit (new case) | `pwsh -File scripts/tests/test_verify_authenticode.ps1 -Case noCheckOverridesTransient` | ❌ W0 | ⬜ pending |
| 104-PUB | TBD | 1 | REL-01 | partial-publish | `release.yml` has NO *enabled* job referencing `-p nono`/`-p nono-proxy`/`-p nono-cli` (publish-crates neutralized via `if: false`, not a secret-consuming no-op) | static/grep | grep guard: no enabled `cargo publish -p nono(-proxy|-cli)?` in release.yml (the 3 lines sit under an `if: false` job) | ❌ W0 | ⬜ pending |
| 104-PRECHK | TBD | 1 | REL-01 | false-confidence | a local `certutil -generateSSTFromWU`-based pre-check tells the operator whether `Microsoft Enterprise ID Root CA 2021` has propagated BEFORE burning a CI smoke dispatch | integration (local) | run the pre-check script → reports root PRESENT/ABSENT (currently ABSENT) | ❌ W0 | ⬜ pending |
| 104-SC1 | TBD | 2 | REL-01 | — | operator re-runs smoke → GREEN (only possible once root propagates) | manual (operator, outward-facing) | N/A — operator checkpoint | N/A | ⬜ blocked-external |
| 104-SC2 | TBD | 2 | REL-01 | — | operator pushes `v0.66.1` → Release workflow green, all .exe + MSIs pass hardened verify | manual (operator, outward-facing) | N/A — operator checkpoint | N/A | ⬜ blocked-external |
| 104-SC3 | TBD | 2 | REL-01 | — | published artifacts show Verified publisher (Status=Valid + non-test signer; NOT gated on the `Microsoft ID Verified CS` issuer substring — disproven Phase 101) | manual (operator, outward-facing) | N/A — operator checkpoint | N/A | ⬜ blocked-external |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky · blocked-external = gated on Microsoft root propagation*
*Task IDs are placeholders until the planner assigns plan/wave numbers.*

---

## Wave 0 Requirements

- [ ] `scripts/tests/test_verify_authenticode.ps1` — add `Test-DiagnosticFlushUnderStop` case (D-04 flush-order regression guard)
- [ ] `scripts/tests/test_verify_authenticode.ps1` — add `Test-NoCheckOverridesTransient` case (NoCheck-mode classification guard)
- [ ] A fast local grep-based guard asserting `release.yml` has no *enabled* job referencing `-p nono`/`-p nono-proxy`/`-p nono-cli`
- [ ] A local root-availability pre-check (certutil-based) for the operator's poll-until-green loop

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Smoke green (SC1) | REL-01 | Outward-facing CI dispatch + gated on external Microsoft root propagation | Operator re-runs `trusted-signing-smoke.yml`; blocked until `Microsoft Enterprise ID Root CA 2021` propagates |
| Tag-push Release green (SC2) | REL-01 | Outward-facing, irreversible tag push + live signing | Operator pushes `v0.66.1` once smoke is green |
| Verified publisher (SC3) | REL-01 | Requires a published artifact + clean-host trust | Operator/Phase-106 verifies published artifact shows Status=Valid on a clean host |

*The autonomous fixes (D-04 flush, NoCheck guard, publish-crates neutralization, pre-check) are all dev-host-automated; only the release cut is manual/blocked-external.*

---

## Validation Sign-Off

- [x] All autonomous tasks have `<automated>` verify (harness cases / grep guard / local pre-check)
- [x] Sampling continuity: harness after every verify-authenticode edit
- [x] Wave 0 covers all MISSING references (2 new harness cases + grep guard + pre-check)
- [x] No watch-mode flags
- [x] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set once plans assign task IDs
- [x] Fail-closed `Status -ne 'Valid'` gate is NEVER loosened by any task (external blocker resolved only by poll-until-green, never code-side acceptance)

**Approval:** pending
