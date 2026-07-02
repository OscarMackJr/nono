# Phase 101: Verify-Gate Hardening + Azure Profile Confirmation - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-07-02
**Phase:** 101-verify-gate-hardening-azure-profile-confirmation
**Areas discussed:** signtool role, transient handling, helper interface, SIGN-01 operator flow, failure diagnostics

---

## signtool /pa role vs Get-AuthenticodeSignature

| Option | Description | Selected |
|--------|-------------|----------|
| Always-run, both-must-pass (AND) | Helper runs BOTH checks every time; Valid requires both. Strongest fail-closed. | ✓ |
| signtool as fallback only | Run signtool only when GAS is non-Valid, to disambiguate. GAS-Valid alone passes. | |
| signtool primary, GAS corroborates | signtool authoritative; GAS secondary. | |

**User's choice:** Always-run, both-must-pass (AND)
**Notes:** Two independent chain-build engines must agree; signtool is additive corroboration, original `Status -ne 'Valid'` condition preserved byte-for-byte.

---

## CRL/OCSP revocation-check transient behavior

| Option | Description | Selected |
|--------|-------------|----------|
| Retry-with-backoff, then fail-closed | Bounded retries on classified transient, then fail-closed. | ✓ |
| Immediate fail-closed + distinct diagnostic | No retry; fail immediately with transient classification. | |
| Treat revocation as offline-permissible | Offline chain-build as corroborating signal only. | |

**User's choice:** Retry-with-backoff, then fail-closed
**Notes:** Reduces flaky CI on genuine network blips (documented AOC/EOC CA rotation risk) without ever passing a non-Valid sig. Exact counts/timeouts left to planner.

---

## Helper interface / three-caller strictness (incl. .sys WHQL carve-out)

| Option | Description | Selected |
|--------|-------------|----------|
| One function + explicit mode param | `Assert-TrustedSignature -Path X -Mode Strict\|Informational`; explicit per call site. | ✓ |
| Strict-only helper; .sys stays inline | Helper strict-only; .sys informational log stays inline in release.yml. | |
| Extension-aware auto-classify | Helper branches on file extension (.sys → informational). | |

**User's choice:** One function + explicit mode param
**Notes:** No extension-based auto-classification — hiding the security-relevant strict/informational decision behind a filename check is a footgun per CLAUDE.md path-handling guidance. Preserves the .sys WHQL/cross-sign carve-out via explicit `-Mode Informational`.

---

## SIGN-01 operator flow + phase sequencing

| Option | Description | Selected |
|--------|-------------|----------|
| Ship hardening first; profile-fix is a checkpoint | Fork lands helper/wiring/diagnostics autonomously; operator profile confirm/fix is a gate before SIGN-03 smoke green. | ✓ |
| Block on operator profile confirmation first | Phase pauses at start until operator confirms/creates PublicTrust profile. | |
| Also check in a scripted `az` confirmation helper | Add checked-in `scripts/azure/confirm-profile-type.ps1` recording profileType/issuer. | |

**User's choice:** Ship hardening first; profile-fix is a checkpoint
**Notes:** Decouples fork work from the Azure ops step; the hardened verify + diagnostics are what let the operator settle root cause fast. Scripted `az` helper noted as optional (not selected, not forbidden).

---

## Failure diagnostics depth

| Option | Description | Selected |
|--------|-------------|----------|
| Full chain dump on failure | On non-Valid: every chain element, status flags, signtool /v verbatim, CRL/OCSP URLs + reachability. Noise only on failure path. | ✓ |
| Classified summary only | One-line classification + signer issuer CN, no full dump. | |
| Full dump always (pass and fail) | Full chain + probe detail every run including success. | |

**User's choice:** Full chain dump on failure
**Notes:** Serves SIGN-01's documented-not-guessed mandate — PublicTrustTest-vs-missing-intermediate-vs-revocation distinction must be settleable from the CI log alone. Happy path stays quiet.

---

## Claude's Discretion

- Exact retry count / backoff timings for the transient handler.
- Precise PowerShell function name/signature and dot-sourcing wiring at each of the three call sites (semantics fixed by D-01/D-03).
- Whether to add the optional scripted `az` confirmation/record helper.

## Deferred Ideas

- POC signing path retirement + smoke workflow deletion + stale `windows-signing-guide.mdx` fix → Phase 107 / CLOSE-01 (gated on clean-host UAT PASS).
- Azure VM IaC + `trusted-signed-assertion` / `broker-spawn-on-clean-host` gates reusing this helper → Phase 103.
