---
phase: 101
slug: verify-gate-hardening-azure-profile-confirmation
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-07-02
---

# Phase 101 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | None (Pester-free by convention) — imperative pwsh test scripts under `scripts/tests/`, mirroring `test_windows_*.ps1`; machine-verdict gates under `scripts/` (`verify-dark.ps1` style) |
| **Config file** | none — Wave 0 adds `scripts/tests/test_verify_authenticode.ps1` |
| **Quick run command** | `pwsh -File scripts/tests/test_verify_authenticode.ps1` |
| **Full suite command** | `gh workflow run trusted-signing-smoke.yml` then `gh run watch` (SIGN-03's live acceptance gate) |
| **Estimated runtime** | ~10s (unit script) / ~3–5 min (smoke workflow) |

---

## Sampling Rate

- **After every task commit:** Run `pwsh -File scripts/tests/test_verify_authenticode.ps1` (Windows host)
- **After every plan wave:** Dispatch `trusted-signing-smoke.yml` on `windows-latest`
- **Before `/gsd:verify-work`:** Smoke workflow GREEN **and** SIGN-01 operator checkpoint confirmed
- **Max feedback latency:** ~10 seconds (unit) — smoke is the integration gate, not the inner loop

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 101-W0 | 00 | 0 | SIGN-02 | — | Test harness scaffolds the 4 helper cases | scaffold | `pwsh -File scripts/tests/test_verify_authenticode.ps1` | ❌ W0 | ⬜ pending |
| helper-knownGood | helper | 1 | SIGN-02 | T-101-01 | `-Mode Strict` on known-good signed file → `Passed=$true`, quiet log | unit | `pwsh -File scripts/tests/test_verify_authenticode.ps1 -Case knownGood` | ❌ W0 | ⬜ pending |
| helper-whqlInfo | helper | 1 | SIGN-02 | T-101-01 | `-Mode Informational` on `.sys` WHQL driver logs status, never gates/throws | unit | `pwsh -File scripts/tests/test_verify_authenticode.ps1 -Case whqlInformational` | ❌ W0 | ⬜ pending |
| helper-transient | helper | 1 | SIGN-02 | T-101-02 | Simulated `RevocationStatusUnknown` retries then fails-closed after N attempts | unit (mocked chain) | `pwsh -File scripts/tests/test_verify_authenticode.ps1 -Case transientRetry` | ❌ W0 | ⬜ pending |
| helper-untrusted | helper | 1 | SIGN-02 | T-101-01 | Genuine `UntrustedRoot` fails closed with NO retry (distinct from transient) | unit (mocked chain) | `pwsh -File scripts/tests/test_verify_authenticode.ps1 -Case untrustedRootNoRetry` | ❌ W0 | ⬜ pending |
| failclosed-diff | wire | 2 | SIGN-02 | T-101-01 | Original `Status -ne 'Valid'` condition byte-for-byte preserved at all 3 sites | static / code-review | `git diff` review of extracted lines vs pre-refactor | N/A | ⬜ pending |
| smoke-green | smoke | 3 | SIGN-03 | — | Smoke run GREEN; embedded sig `Valid`, issuer `Microsoft ID Verified CS.*(EOC\|AOC) CA` | smoke (live CI) | `gh workflow run trusted-signing-smoke.yml` | ✅ (workflow exists) | ⬜ pending |
| profile-confirm | operator | — | SIGN-01 | — | `profileType == PublicTrust`; finding documented not guessed | manual (operator) | `az trustedsigning certificate-profile show ... --query profileType -o tsv` | N/A | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `scripts/tests/test_verify_authenticode.ps1` — new imperative pwsh test script exercising the 4 unit cases (`knownGood`, `whqlInformational`, `transientRetry`, `untrustedRootNoRetry`); no Pester. Follow `scripts/tests/test_windows_attach.ps1` convention (`$ErrorActionPreference = "Stop"`, `Write-Host`/`Write-Error` assertions, explicit exit code).
- [ ] No shared fixtures — knownGood/whqlInformational use a locally self-signed throwaway cert (or reuse `scripts/sign-poc-local.ps1`'s POC cert); transient/untrusted cases use **mocked** `X509ChainStatusFlags` inputs, never live revocation network state.
- [ ] Framework install: none — repo is deliberately Pester-free for PowerShell; do not introduce Pester this phase.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Azure profile type is `PublicTrust` | SIGN-01 | Operator-run Azure CLI against live subscription; not automatable in CI | `az trustedsigning certificate-profile show -g <rg> --account-name <acct> -n <profile> --query profileType -o tsv`; if `PublicTrustTest`, create a `PublicTrust` profile + update `TRUSTED_SIGNING_PROFILE` and FIC subject `repo:OscarMackJr/nono:environment:Development`; record profile type + issuer chain |
| Fail-closed condition unweakened | SIGN-02 (criterion 3) | Correctness of a security invariant is a diff/code-review judgment, not a runtime assertion | Review `git diff` of the extracted `Status -ne 'Valid'` lines at all 3 call sites — confirm the original condition is byte-for-byte present and all new logic is additive |

---

## Validation Sign-Off

- [ ] All tasks have automated verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references (`test_verify_authenticode.ps1`)
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s (unit loop)
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
