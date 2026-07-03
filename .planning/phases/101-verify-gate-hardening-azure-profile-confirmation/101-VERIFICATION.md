---
phase: 101-verify-gate-hardening-azure-profile-confirmation
verified: 2026-07-03T00:00:00Z
status: human_needed
score: 3/4 must-haves verified (1 legitimately deferred to Phase 104)
overrides_applied: 0
deferred:
  - truth: "The 'Trusted Signing Smoke Test' workflow runs GREEN on windows-latest with an issuer chaining to the public Microsoft ID Verified CS EOC/AOC CA root (ROADMAP Phase 101 Success Criterion #4 / SIGN-03)"
    addressed_in: "Phase 104"
    evidence: "Phase 104 Success Criterion #1: '[Operator-in-loop] Immediately before the tag push, the operator re-runs the hardened Trusted Signing Smoke Test workflow and confirms it is GREEN'"
human_verification:
  - test: "Reconcile the factually incorrect 'GitHub Trusted Signing config does not exist' claim in 101-SIGN01-FINDING.md / 101-SIGN03-SMOKE-VERDICT.md / STATE.md before Phase 104 planning proceeds"
    expected: "Operator confirms whether TRUSTED_SIGNING_ACCOUNT/_ENDPOINT/_PROFILE repo-level variables and the OIDC federated credential (both of which this verification found live and already working) are in fact usable for a future smoke dispatch, and corrects the phase's hand-off artifacts accordingly"
    why_human: "This verifier found live, repo-level GitHub Actions variables (TRUSTED_SIGNING_ACCOUNT=ArtifactNono, TRUSTED_SIGNING_ENDPOINT, TRUSTED_SIGNING_PROFILE=NonoCertProfile, all created 2026-06-04) and a working OIDC federated credential (smoke run 26925847471 succeeded 2026-06-04; run 28469674206 on 2026-06-30 logged a successful OIDC login with the exact expected subject claim repo:OscarMackJr/nono:environment:Development and a successful Sign step) — directly contradicting Plan 03/04's recorded claim that 'no GitHub Actions variables or secrets... for Trusted Signing' and 'no OIDC federated credential' exist. Only a human/operator can authoritatively reconcile which record is stale (see Discrepancy section below) and decide whether Phase 104 needs to re-provision anything at all, versus simply pushing Plan 02's local branch and re-dispatching.
---

# Phase 101: Verify-Gate Hardening + Azure Profile Confirmation Verification Report

**Phase Goal:** The Trusted Signing verify path is provably fixed — root cause disambiguated and documented, the fail-closed verify hardened without ever loosening it — and the smoke workflow runs GREEN on GitHub's clean `windows-latest` runner, proving the signing chain is live before any release is cut.

**Verified:** 2026-07-03
**Status:** human_needed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | [SIGN-01] Azure Trusted Signing certificate profile type confirmed via az/portal and documented, not guessed | ✓ VERIFIED | `101-SIGN01-FINDING.md` records `profileType: PublicTrust` (account `ArtifactNono`, RG `RG_Nono`), confirmed via Azure Portal (az CLI extension unavailable — documented reason), cross-references SIGN-01, honestly notes the residual `UnknownError` is NOT a profile-type issue |
| 2 | [SIGN-02] Shared `scripts/verify-authenticode.ps1` helper exists, dot-sourced by both fail-closed verify sites in `release.yml` and `trusted-signing-smoke.yml`, adds `signtool verify /pa /v` deep-check + classifies chain-build failure distinctly from untrusted root | ✓ VERIFIED | File exists (359 lines), defines `Assert-TrustedSignature` exactly once + 7 supporting functions; dual-engine AND-gate confirmed by code read (`$gasOk -and $signToolOk`); dot-sourced at all 5 call sites (4 in `release.yml`, 1 in `trusted-signing-smoke.yml`); **behaviorally re-ran the test harness myself** (`pwsh -File scripts/tests/test_verify_authenticode.ps1 -Case All`) — exit code 0, all 4 named cases (`knownGood`, `whqlInformational`, `transientRetry`, `untrustedRootNoRetry`) PASSED |
| 3 | [SIGN-02] The fail-closed `Status -ne 'Valid'` contract is provably unweakened — diff review confirms the condition is unchanged; all new logic is additive, not a loosened pass condition | ✓ VERIFIED | `grep -n "Status -ne 'Valid'"` on `verify-authenticode.ps1` returns exactly one match (line 341), inside the `Mode -eq 'Strict'` branch, gating a `throw`; `grep -c "sig.Status -ne"` returns `0` in both `release.yml` and `trusted-signing-smoke.yml` — no duplicated/weakened inline gate remains anywhere; `.sys` WHQL carve-out preserved (`-Mode Informational`, "SEPARATE signing regime" comment intact, `msiexec` admin-extract block untouched); both YAML files parse cleanly via `yaml.safe_load` |
| 4 | [SIGN-03 / ROADMAP SC#4] The "Trusted Signing Smoke Test" workflow runs GREEN on `windows-latest` with an issuer chaining to the public `Microsoft ID Verified CS EOC/AOC CA` root | ✗ NOT MET — **deferred to Phase 104** | `101-SIGN03-SMOKE-VERDICT.md` honestly records `Verdict: BLOCKED` — no dispatch was attempted, no fabricated PASS/FAIL. `REQUIREMENTS.md` correctly marks SIGN-03 `Blocked/Deferred`, not Complete. Phase 104 Success Criterion #1 explicitly requires this operator re-run before the release tag push — this is the designated closure point, per Step 9b deferred-item filtering. **However, see Discrepancy below**: the *stated reasons* for the block are materially inaccurate. |

**Score:** 3/4 truths verified; 1 deferred (not a gap against this phase — see `deferred:` in frontmatter)

### Deferred Items

| # | Item | Addressed In | Evidence |
|---|------|-------------|----------|
| 1 | Smoke Test workflow runs GREEN with correct issuer chain | Phase 104 | Phase 104 SC#1: "the operator re-runs the hardened Trusted Signing Smoke Test workflow and confirms it is GREEN" — immediately before the release tag push |

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `scripts/verify-authenticode.ps1` | `Assert-TrustedSignature` + 7 supporting functions, ≥150 lines | ✓ VERIFIED | 359 lines; `Find-Signtool`, `Invoke-SignToolVerify`, `Get-FlagClassification`, `Get-ChainClassification`, `Get-CertificateRevocationUrls`, `Write-ChainDiagnostic`, `Invoke-VerifyWithTransientRetry`, `Assert-TrustedSignature` all present and grep-verified |
| `scripts/tests/test_verify_authenticode.ps1` | 4-case imperative test harness, ≥80 lines | ✓ VERIFIED | 269 lines; all 4 named cases present; **ran it live — exit 0, all PASSED** |
| `.github/workflows/release.yml` | Both verify sites (+ a 3rd/4th) dot-source and call the shared helper | ✓ VERIFIED | 4 call sites, all pass explicit `-Mode`; `msiexec` admin-extract block and `.sys` carve-out comment both byte-for-byte preserved |
| `.github/workflows/trusted-signing-smoke.yml` | Verify step dot-sources and calls the shared helper | ✓ VERIFIED | 1 call site (`-Mode Strict`); ambient `Status:`/`Signer:`/`Issuer:` echo lines preserved |
| `.planning/phases/.../101-SIGN01-FINDING.md` | Documented `profileType` + issuer chain finding, cross-referencing SIGN-01 | ✓ VERIFIED (core claim) — ⚠️ follow-up note factually wrong | `profileType: PublicTrust` field present, not a placeholder; SIGN-01 cross-reference present. The document's separate "GitHub Trusted Signing Config Does Not Exist Yet" follow-up section is contradicted by live evidence (see Discrepancy) |
| `.planning/phases/.../101-SIGN03-SMOKE-VERDICT.md` | Recorded BLOCKED verdict, no fabricated PASS | ✓ VERIFIED (honesty) — ⚠️ stated reasons factually wrong | No run URL/conclusion/issuer fabricated; `Verdict: BLOCKED` recorded plainly. 2 of its 5 stated "independent reasons" are contradicted by live `gh` evidence (see Discrepancy) |

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| `scripts/tests/test_verify_authenticode.ps1` | `scripts/verify-authenticode.ps1` | dot-source | ✓ WIRED | `. (Join-Path $PSScriptRoot "..\verify-authenticode.ps1")` present; behaviorally confirmed by running the harness (it dot-sources successfully and all cases execute against the real functions) |
| `.github/workflows/release.yml` (4 sites) | `scripts/verify-authenticode.ps1` | dot-source in `run:` block | ✓ WIRED | `. (Join-Path $PWD "scripts\verify-authenticode.ps1")` at Site 1 (~L263), Site 2 `.exe`/`.sys` (~L281), and the zip-payload verify site (~L333); all call `Assert-TrustedSignature` with explicit `-Mode` |
| `.github/workflows/trusted-signing-smoke.yml` | `scripts/verify-authenticode.ps1` | dot-source in `run:` block | ✓ WIRED | `. (Join-Path $PWD "scripts\verify-authenticode.ps1")` at L65, `Assert-TrustedSignature -Path "smoke\nono-smoke.exe" -Mode Strict` at L70 |

### Data-Flow Trace (Level 4)

Not applicable in the conventional (rendered-UI) sense — this is CI/PowerShell infrastructure, not a data-rendering component. Instead, the equivalent check is **behavioral execution of the actual verify logic**, which was performed directly (see Behavioral Spot-Checks below) rather than merely inspected as static code.

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| `Assert-TrustedSignature -Mode Strict` on a known-good signed file returns `Passed=$true`, quiet happy path | `pwsh -File scripts/tests/test_verify_authenticode.ps1 -Case All` | `PASS: knownGood` — real csc.exe-compiled + self-signed-cert-signed exe verified with dual-engine pass, no `ChainStatus:` diagnostic line printed | ✓ PASS |
| `-Mode Informational` never throws regardless of signature status, no retry cost | (same run) | `PASS: whqlInformational` — unsigned file, `GAS=NotSigned`, `Passed=$null`, completed in well under 5s (no `@(2,4,8)` backoff paid) | ✓ PASS |
| Classified transient retries exactly 3 attempts, still fails closed | (same run) | `PASS: transientRetry` — log shows "attempt 1/3... retrying in 2s", "attempt 2/3... retrying in 4s", 3rd attempt exhausts, `Passed=$false` preserved | ✓ PASS |
| Genuine untrusted-root classification never retries (1 attempt only) | (same run) | `PASS: untrustedRootNoRetry` — mock counter = 1, `Passed=$false` | ✓ PASS |
| Both workflow YAMLs still parse after edits | `python -c "import yaml; yaml.safe_load(open(...))"` on both files | `yaml ok` for both `release.yml` and `trusted-signing-smoke.yml` | ✓ PASS |

**Full harness output (captured live, this session):**
```
Authenticode OK (dual-engine): ...\knowngood.exe
PASS: knownGood
Authenticode (informational): ...\unsigned.exe GAS=NotSigned signtool_exit=1
PASS: whqlInformational
Transient chain/revocation failure (attempt 1/3) - retrying in 2s
Transient chain/revocation failure (attempt 2/3) - retrying in 4s
PASS: transientRetry
PASS: untrustedRootNoRetry
test_verify_authenticode: all cases PASSED
EXIT_CODE=0
```

### Probe Execution

No `scripts/*/tests/probe-*.sh` probes declared or discovered for this phase; the phase's own TDD test harness (`scripts/tests/test_verify_authenticode.ps1`) served as the equivalent runnable verification and was executed directly (see above), not merely trusted from SUMMARY narration.

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| SIGN-01 | 101-03 | Azure Trusted Signing profile type confirmed, documented not guessed | ✓ SATISFIED | `101-SIGN01-FINDING.md`, `profileType: PublicTrust`; REQUIREMENTS.md marks Complete |
| SIGN-02 | 101-01, 101-02 | CI Authenticode verify hardened into shared helper, fail-closed contract unweakened | ✓ SATISFIED | Code + behavioral test run, all grep gates pass, all call sites wired |
| SIGN-03 | 101-04 | Smoke Test workflow runs GREEN, correct issuer chain | ✗ NOT SATISFIED (honestly recorded, deferred to Phase 104) | `101-SIGN03-SMOKE-VERDICT.md` Verdict: BLOCKED; REQUIREMENTS.md marks Blocked/Deferred |

No orphaned requirements found — SIGN-01/02/03 are the only requirements mapped to Phase 101 in REQUIREMENTS.md's traceability table, and all three appear in a plan's `requirements:` frontmatter.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| — | — | None found | — | `grep -n -E "TBD\|FIXME\|XXX"` and `grep -n -iE "TODO\|HACK\|PLACEHOLDER\|not yet implemented\|coming soon"` return zero matches across `scripts/verify-authenticode.ps1`, `scripts/tests/test_verify_authenticode.ps1`, `release.yml`, `trusted-signing-smoke.yml` |

### Discrepancy Found (Escalated — Requires Human Reconciliation)

**This is the material finding of this verification pass.** SIGN-03's BLOCKED verdict is *itself* honestly recorded (no fabricated PASS) — that part of the phase's own anti-false-PASS discipline worked correctly. But the **stated rationale** for the block, repeated across `101-SIGN01-FINDING.md`, `101-SIGN03-SMOKE-VERDICT.md`, `REQUIREMENTS.md`'s SIGN-01 annotation, and `STATE.md`'s decision log, contains claims that live evidence directly contradicts:

| Claimed reason (101-SIGN03-SMOKE-VERDICT.md) | This verification's live finding |
|---|---|
| "No OIDC federated credential exists... azure/login (OIDC) has nothing to authenticate against" | **FALSE.** `gh run view 28469674206 --repo OscarMackJr/nono --log` (2026-06-30) shows: `Azure CLI login succeeds by using OIDC` with `subject claim - repo:OscarMackJr/nono:environment:Development` — the *exact* expected subject. An even earlier run (`26925847471`, 2026-06-04) completed with `conclusion: success`. |
| "No GitHub Actions variables exist. TRUSTED_SIGNING_ACCOUNT, TRUSTED_SIGNING_PROFILE, and TRUSTED_SIGNING_ENDPOINT are all absent" | **FALSE.** `gh variable list --repo OscarMackJr/nono` shows all three present at repo scope: `TRUSTED_SIGNING_ACCOUNT=ArtifactNono`, `TRUSTED_SIGNING_ENDPOINT=https://eus.codesigning.azure.net/`, `TRUSTED_SIGNING_PROFILE=NonoCertProfile`, all created 2026-06-04 (before this phase started). The same 2026-06-30 run's Sign step used them and logged `Number of errors: 0`. |
| "`gh` resolves to the wrong repo (`nolabs-ai/nono`)" | **Not reproducible.** In this verification session, `gh repo view --json nameWithOwner` and `gh repo set-default --view` both correctly resolve to `OscarMackJr/nono` (the `origin` remote, `gh-resolved = base` in `.git/config`). |
| "Plan 02's hardened workflow has not been pushed" | **TRUE and confirmed.** `git log origin/main..HEAD` shows all Phase 101 commits (including `5bd7567c`/`51d75786`, the wiring commits) are local-only, ahead of `origin/main`. |

**Likely root cause of the discrepancy:** the GitHub **environment-scoped** secrets/variables for the `Development` environment ARE empty (`gh api repos/OscarMackJr/nono/environments/Development/variables` and `/secrets` both return `total_count: 0`) — but the workflow's job specifies `environment: Development` while reading `${{ vars.TRUSTED_SIGNING_* }}` / `${{ secrets.AZURE_* }}`, which GitHub Actions resolves from **repository-level** scope when no environment-level override exists. The operator (per 101-03/101-04's SUMMARY) appears to have checked only the environment-scoped secrets/variables page and concluded nothing exists — missing that the repo-level values (already provisioned, already exercised by a prior GREEN OIDC+Sign run) are what the workflow actually uses.

**Why this matters (security-critical release-signing context):** this repo's Phase 104 hand-off, and the phase's own audit trail (STATE.md, REQUIREMENTS.md), currently direct the operator to "re-provision GitHub config from scratch" before any future smoke attempt. Per this verification, that is very likely unnecessary — the only confirmed, verified blocker is that **Plan 02's hardened workflow branch has not been pushed to `origin/main`**. Proceeding on the "re-provision from scratch" assumption risks wasted operator effort and, more importantly, risks the operator re-creating a *second*, possibly-misconfigured OIDC federated credential or duplicate variables against an Azure AD app that already has a working FIC — a needless attack-surface/config-drift risk in a security-critical signing pipeline.

**This does not retroactively fail SIGN-01 or SIGN-02** — both of those requirements' own deliverables (profile-type confirmation; the hardened helper + its wiring) are independently verified above and stand on their own merits. It also does not mean SIGN-03 should be marked satisfied — no GREEN smoke run has actually occurred, and the pushed-vs-unpushed-branch gap is real. The issue is narrowly that **the documented reasoning for the block is inaccurate**, which is exactly the kind of drift that goal-backward verification exists to catch before it propagates into Phase 104 planning.

**Suggested correction (for human decision, not applied by this verifier):** update `101-SIGN01-FINDING.md`'s "Follow-Up Note" and `101-SIGN03-SMOKE-VERDICT.md`'s reasons 1–2 to reflect that repo-level GitHub Trusted Signing config already exists and previously worked (OIDC+Sign GREEN on 2026-06-30 per auto-memory `azure_trusted_signing_golive.md`), and narrow Phase 104's actual precondition to: push Plan 02's hardened `trusted-signing-smoke.yml` to `origin/main`, then re-dispatch — no fresh OIDC/variable provisioning required unless the operator has independent reason to rotate them.

### Human Verification Required

### 1. Reconcile the GitHub Trusted Signing config discrepancy before Phase 104

**Test:** Compare this verification's live findings (`gh variable list --repo OscarMackJr/nono`, `gh run view 28469674206 --repo OscarMackJr/nono --log`) against `101-SIGN01-FINDING.md` / `101-SIGN03-SMOKE-VERDICT.md`'s claim that no GitHub Trusted Signing variables/OIDC FIC exist.
**Expected:** Operator confirms the repo-level variables and FIC are indeed live/usable (as this verification found), and updates the phase's hand-off documents so Phase 104 does not attempt unnecessary re-provisioning.
**Why human:** This is an operator-owned Azure AD / GitHub org-settings judgment call (which record is stale, and whether to rotate the FIC/vars regardless) that only the repo/Azure admin can authoritatively resolve — a verifier can observe the contradiction but cannot decide the correct remediation path for a security-critical credential surface.

## Gaps Summary

Phase 101's two autonomous, code-level deliverables — SIGN-01 (Azure profile-type confirmation, documented not guessed) and SIGN-02 (the shared `Assert-TrustedSignature` fail-closed verify helper, wired into all 5 verify call sites with the original `Status -ne 'Valid'` contract byte-for-byte preserved) — are **solidly verified**, including a live re-run of the TDD test harness (not merely trusted from SUMMARY claims), which passed all 4 cases with real behavioral evidence (retry counts, backoff timing, quiet-happy-path, mode gating).

SIGN-03 (smoke workflow GREEN) is honestly **not** claimed complete — the phase correctly recorded a BLOCKED verdict rather than fabricating a PASS, and this gap is properly deferred to Phase 104's own success criterion #1, per this verification's Step 9b filtering.

The one substantive issue this verification surfaces is **not a missing deliverable but a documentation-accuracy gap**: the BLOCKED verdict's stated justification (no OIDC FIC, no GitHub variables) is contradicted by live `gh` evidence and this repo's own prior successful smoke runs. This should be reconciled by the operator before Phase 104 begins, so that phase does not spend effort re-provisioning Azure/GitHub configuration that already exists and already works — the actual, confirmed remaining blocker is simply pushing Plan 02's local commits to `origin/main` and re-dispatching.

---

*Verified: 2026-07-03*
*Verifier: Claude (gsd-verifier)*
