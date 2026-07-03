# SIGN-03 Smoke Verdict

**Cross-reference:** `.planning/REQUIREMENTS.md` — **SIGN-03**
**Date:** 2026-07-02
**Corrected:** 2026-07-02 (same-day factual-correction pass, post-verification)
**Live smoke-run update:** 2026-07-02/03 UTC (operator authorized push + live dispatch; two runs executed)
**Verdict: FAIL** (RED — explicitly NOT a PASS; SIGN-03 is NOT satisfied)

This is a **rewrite** of the prior BLOCKED verdict. The workflow is no longer blocked-unpushed:
the operator authorized pushing the branch and dispatching the hardened smoke workflow live.
Two runs executed. The first exposed a real defect in this phase's own wiring (fixed same
session); the second is the authoritative diagnostic run and it FAILED at Verify. The record
below documents the accurate outcome — a real, precisely-diagnosed failure — not the earlier
"never dispatched" state.

## Live Smoke-Run Evidence (2026-07-02/03 UTC)

- Branch `milestone/v2.13-carryforward-closeout` was pushed to `origin/OscarMackJr/nono`
  (commits through `83eefe11`), satisfying the "push the hardened workflow" precondition from
  the prior verdict.
- **Run #1 — `28636000664`**
  (https://github.com/OscarMackJr/nono/actions/runs/28636000664): **FAILED**, but not at
  Verify — the smoke workflow had **no `actions/checkout` step**, so Plan 02's dot-source of
  `scripts\verify-authenticode.ps1` could not resolve the repo-relative path on the runner
  ("The term '...\scripts\verify-authenticode.ps1' is not recognized"). Sign succeeded before
  this failure. This was a **real defect Plan 02 introduced** — wiring a repo-file dot-source
  into a workflow that never checked the repo out — and static grep/YAML-shape gates could not
  have caught it; it only surfaces on an actual dispatch.
- **FIX — commit `83eefe11`**: added `actions/checkout@v4` as the first step of
  `trusted-signing-smoke.yml`.
- **Run #2 — `28636133664`**
  (https://github.com/OscarMackJr/nono/actions/runs/28636133664): checkout succeeded, the
  hardened `Assert-TrustedSignature` verify path ran for real, and it **still FAILED**. This is
  the **authoritative diagnostic run** for SIGN-03. Verbatim verify-step log:

  ```
  Status: UnknownError
  Signer: CN=TWGGLOBAL.onmicrosoft.com, O=TWGGLOBAL.onmicrosoft.com, OU=Information Technology
  Issuer: CN=Microsoft Enterprise ID Verified Policy AOC CA 02, O=Microsoft Corporation, C=US
  Transient chain/revocation failure (attempt 1/3) - retrying in 2s
  Assert-TrustedSignature: ... Authenticode verification failed for smoke\nono-smoke.exe with status UnknownError.
  ```

  Step conclusions for Run #2: Checkout success, Azure login (OIDC) success, Compile success,
  Sign success ("Number of errors: 0"), **Verify FAILURE**.

## Findings

### Finding (A) — SIGN-03 root cause, diagnosed

`Get-AuthenticodeSignature` returns `UnknownError` for a **genuinely-signed** binary (a real
org-validated certificate, `CN=TWGGLOBAL.onmicrosoft.com`, issued from the operator-confirmed
`PublicTrust` profile — see `101-SIGN01-FINDING.md`). The shared helper's bounded retry
correctly classified this as a **TRANSIENT** condition (chain-build/revocation-status, not
`UntrustedRoot`), retried once, and — still unable to validate the chain — **threw fail-closed**
per its Strict-mode contract. **This is correct security behavior**: the gate did not accept
`UnknownError`, and it was never loosened to do so (per SIGN-02's invariant).

The failure itself is an **environmental chain-build / revocation-validation problem on the
`windows-latest` GitHub-hosted runner** — most likely the runner's revocation-check endpoint
(CRL/OCSP) being unreachable or slow, or the `Microsoft Enterprise ID Verified Policy AOC CA 02`
intermediate/root not yet fully distributed to the runner's trust store — **NOT** the
certificate profile type, and **NOT** a defect in the SIGN-02 gate/code itself.

### Finding (B) — Research issuer-naming heuristic DISPROVEN

`101-RESEARCH.md` (§4 and the SIGN-01/SIGN-03 evidence section) proposed an issuer-substring
differentiator: issuer containing `Enterprise ID Verified Policy` ⇒ likely `PublicTrustTest`;
issuer matching `Microsoft ID Verified CS.*(EOC|AOC) CA` ⇒ `PublicTrust`. Plan 04's acceptance
criteria for SIGN-03 encoded the same tell (reject `Enterprise ID Verified Policy`, expect a
`Microsoft ID Verified CS EOC/AOC CA NN` issuer string).

The live run **disproves this heuristic**: the operator-confirmed `PublicTrust` profile
(account `ArtifactNono`, resource group `RG_Nono`, profile `NonoCertProfile` — Plan 03,
`101-SIGN01-FINDING.md`) produced a real signature whose issuer is
`CN=Microsoft Enterprise ID Verified Policy AOC CA 02` — the exact substring both the research
and Plan 04 labeled as the `PublicTrustTest` tell. **`PublicTrust` can and does chain through an
`Enterprise ID Verified Policy AOC CA` issuer.** Had Plan 04's issuer-regex assertion been
applied literally to this run, it would have **wrongly rejected a legitimate `PublicTrust`
signature** as if it were `PublicTrustTest`. Any future gate must not resurrect this
differentiator as a pass/fail condition.

### Finding (C) — D-04 diagnostic-flush gap

The failure-path `Write-ChainDiagnostic` full chain dump and the `signtool verify /pa`
StdOut/StdErr capture (Plan 01's D-04 chain-introspection diagnostics) did **not** appear in the
CI log before the terminating `throw` in Run #2 — only the single retry line and the throw
message surfaced. This means we still cannot see, from this run, whether the `signtool verify
/pa` arm would have **passed** where `Get-AuthenticodeSignature` returned `UnknownError` — which
the research explicitly flagged as plausible (signtool walks a different chain/policy
validation path than GAS). This is a real legibility gap in D-04 that undercuts its stated
purpose (giving the operator fast, evidence-based root-cause attribution) and needs a follow-up
fix: ensure `Write-ChainDiagnostic` output is flushed to the CI log **before** the terminating
throw, and surface the `signtool /pa` arm's result explicitly rather than only the GAS status.

## SIGN-03 Requirement Status

Per `.planning/REQUIREMENTS.md` **SIGN-03**: "The Trusted Signing Smoke Test workflow runs
GREEN on GitHub's clean windows-latest runner... proving the signing path is live end-to-end
before any release is cut."

**This requirement is NOT satisfied.** The workflow was pushed and dispatched twice with full
operator authorization; the first run failed on a real wiring defect (fixed); the second,
authoritative run failed at the fail-closed Verify gate with `UnknownError`, correctly
diagnosed per Findings (A)-(C) above. No GREEN result exists, and none is claimed. SIGN-03
remains open, marked **Failed/Deferred** (RED, diagnosed — not BLOCKED, not a fabricated PASS)
in `.planning/REQUIREMENTS.md`.

## Hand-off to Phase 104

This is **not** a failure of the SIGN-02 hardening — that work (Plans 01-02) is complete, the
shared `Assert-TrustedSignature` helper is fully wired into all fail-closed verify sites, and it
did exactly what a fail-closed gate should do when it cannot validate a chain: it retried the
transient case and then refused to pass. SIGN-03 hands off to **Phase 104** (Smoke Green + Cut
the Trusted-Signed Release), which must:

1. **Resolve the runner-side chain/revocation validation of the `AOC CA 02` chain.** Candidates:
   re-run on a clean/updated `windows-latest` image (Microsoft periodically refreshes runner
   images and CA trust stores); investigate whether the runner can reach the relevant CRL/OCSP
   endpoints; and/or treat the `signtool verify /pa` arm's result as authoritative when GAS
   returns `UnknownError` instead of (or in addition to) `Get-AuthenticodeSignature` — do NOT
   loosen the fail-closed `Status -ne 'Valid'` gate itself; if `signtool /pa` is to be trusted
   over GAS for this class of error, that must be an explicit, documented gate redesign, not a
   silent softening.
2. **Fix Plan 04's issuer-regex acceptance criteria** to stop rejecting issuers containing
   `Enterprise ID Verified Policy` — Finding (B) disproves that heuristic empirically. A
   `PublicTrust` profile chaining through `Microsoft Enterprise ID Verified Policy AOC CA NN` is
   a legitimate, expected outcome, not a `PublicTrustTest` tell.
3. **Close the D-04 diagnostic-flush gap** (Finding C) — make `Write-ChainDiagnostic` output and
   the `signtool /pa` result visible in the CI log before any terminating throw, so the next
   dispatch produces a fully legible diagnostic instead of a bare retry-then-throw.

**Never recommend loosening the fail-closed `Status -ne 'Valid'` gate itself to accept
`UnknownError`** — per SIGN-02's invariant and `.planning/REQUIREMENTS.md`'s explicit
Out-of-Scope entry, that would be a security regression, not a fix.
