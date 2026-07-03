# SIGN-03 Smoke Verdict

**Cross-reference:** `.planning/REQUIREMENTS.md` — **SIGN-03**
**Date:** 2026-07-02
**Corrected:** 2026-07-02 (same-day factual-correction pass, post-verification)
**Verdict: BLOCKED** (explicitly NOT a PASS — SIGN-03 is NOT satisfied)

## Correction (2026-07-02)

An independent verifier (`101-VERIFICATION.md`) found that the original version of this
document's stated blockers rested on **false premises**, contradicted by live `gh`/`git`
evidence gathered in the same session. This section retracts those premises explicitly so
the record does not silently drift. The corrected reasoning follows in the next section.

**Retracted (false) claims from the original verdict:**

- ~~"No OIDC federated credential exists."~~ **FALSE.** The FIC exists and works. Run
  `28469674206` (2026-06-30) shows a successful OIDC login with the exact expected subject
  claim `repo:OscarMackJr/nono:environment:Development`, and the Sign step succeeded
  ("Number of errors: 0"). An earlier run (`26925847471`, 2026-06-04) also completed
  successfully.
- ~~"No GitHub Actions variables exist."~~ **FALSE.** Repo-scoped variables `TRUSTED_SIGNING_ACCOUNT`
  (`ArtifactNono`), `TRUSTED_SIGNING_PROFILE` (`NonoCertProfile`), and `TRUSTED_SIGNING_ENDPOINT`
  (`https://eus.codesigning.azure.net/`) all exist, created 2026-06-04 — before this phase
  started — and were exercised successfully by the same 2026-06-30 run above.
- ~~"`gh` resolves to the wrong repo (`nolabs-ai/nono`)."~~ **FALSE / not reproducible.**
  `origin` is `https://github.com/OscarMackJr/nono.git`, and `gh repo view` correctly resolves
  to `OscarMackJr/nono`. The earlier report of a wrong-repo resolution came from an operator/
  orchestrator query that mistakenly hit a non-existent `--env Development` scope (producing a
  404 against a different lookup path), not an actual `gh` misconfiguration.

**Root cause of the discrepancy:** there is no GitHub "Environment" named `Development` on this
repo — the environment-scoped variables/secrets page for it is empty. The workflow's job
specifies `environment: Development` but reads `${{ vars.TRUSTED_SIGNING_* }}`, which GitHub
Actions resolves from **repository-level** scope when no environment-level override exists.
The operator's earlier "no env vars anywhere" report is very likely explained by checking only
the environment-scoped page and not the repo-scoped one. **This reconciliation is still
pending explicit operator confirmation** — if the operator is intentionally decommissioning
this configuration (e.g., deliberately rotating credentials, or these values are stale/
unexpected for some other reason), this verdict must be revisited accordingly. Absent that
signal, the live evidence above is treated as authoritative.

## Corrected Reasoning: Why No Dispatch Was Performed

No `gh workflow run trusted-signing-smoke.yml` dispatch was performed, and no run ID or run
URL exists for this attempt — that remains an honest, deliberate absence, not an oversight.
The genuine reasons are narrower than originally stated:

1. **Plan 02's hardened `trusted-signing-smoke.yml` is unpushed.** All 14 Phase 101 commits
   are local-only (`git log origin/main..HEAD` lists them; none exist on `origin/main`). A
   dispatch today would run the **pre-hardening** version of the workflow on GitHub, not the
   `Assert-TrustedSignature`-wired version this phase built — making any result unrepresentative
   of the SIGN-02 hardening this gate exists to validate. **[GENUINE]**
2. **Dispatching live Azure Trusted Signing CI is outward-facing.** This fork is currently
   under a documented prepare-only / push-operator-gated release posture. Pushing the Phase
   101 branch and triggering a live signing round-trip against Azure infrastructure requires
   explicit operator authorization before either action is taken. **[GENUINE — operator gate,
   not a technical defect]**
3. **The prior real smoke run's Verify failure is unresolved.** Run `28469674206` (2026-06-30)
   succeeded through OIDC login and Sign, but failed the Verify step with `Status: UnknownError`
   (issuer `CN=Microsoft Enterprise ID Verified Policy AOC CA 01`). This is a real, open root
   cause — not fixed merely by the GitHub config being confirmed present. It must be
   re-investigated via Plan 01's D-04 chain-introspection diagnostics (the `Assert-TrustedSignature`
   helper's failure-path chain dump) on the eventual hardened re-run. **[GENUINE — the actual
   open question this gate exists to resolve]**

Per Plan 04's own Task 2 rule ("If Task 1's run was NOT a correctly-chained success, this file
records that failure verbatim rather than a false PASS"), the honest record remains that Task 1
was not run, and this verdict is BLOCKED rather than a fabricated FAIL or PASS artifact — only
the *rationale* for the block has been corrected.

## Unblock Preconditions for a Future SIGN-03 Attempt (Corrected)

Before any future dispatch of `trusted-signing-smoke.yml` can produce a meaningful verdict:

1. **Push the Phase 101 commits** (including Plan 02's hardened `trusted-signing-smoke.yml`
   wiring, e.g. `5bd7567c`/`51d75786`) to the `OscarMackJr/nono` default branch, so a dispatch
   actually exercises the hardened verify path (`Assert-TrustedSignature -Mode Strict`), not
   the pre-hardening workflow.
2. **Operator authorization to push + dispatch.** Because this is outward-facing (a live Azure
   Trusted Signing round-trip against real infrastructure), the operator must explicitly
   authorize both the push and the dispatch — this is a posture gate, not a missing-config
   gate.
3. **No re-provisioning of GitHub OIDC/variables is expected to be necessary** — they already
   exist and already worked on 2026-06-30 (Sign step GREEN). Do not create a second, possibly
   duplicate OIDC federated credential or duplicate variables against the same Azure AD
   application; that would be needless config drift/attack-surface on a security-critical
   signing pipeline. If the operator's reconciliation (above) concludes otherwise, this
   precondition must be revised.

Once the push + operator authorization above are satisfied, dispatch
`gh workflow run trusted-signing-smoke.yml --repo OscarMackJr/nono` and observe:

- If the hardened D-04 diagnostics + bounded transient-retry (Plan 01) resolve the
  `UnknownError` — because it was a transient CRL/OCSP revocation-check failure — the run goes
  GREEN and SIGN-03 is satisfied.
- If it was not transient, the hardened verify fails closed (never loosened) with a full
  chain-diagnostic dump identifying the true root cause, which must then be addressed before
  any release is cut.

## Relevant Context Carried Forward (Plan 03's Finding)

Per `101-SIGN01-FINDING.md`, the live Azure certificate profile is confirmed
`profileType == PublicTrust` (account `ArtifactNono`, resource group `RG_Nono`, profile name
`NonoCertProfile`) — the profile-type hypothesis for the 2026-06-30 `UnknownError` is **ruled
out**. The previously observed `Status: UnknownError` / `Issuer: CN=Microsoft Enterprise ID
Verified Policy AOC CA 01` residual remains unresolved and open, to be diagnosed via Plan 01's
`Assert-TrustedSignature` D-04 chain-introspection output on the eventual hardened re-run — not
guessed at here.

## SIGN-03 Requirement Status

Per `.planning/REQUIREMENTS.md` **SIGN-03**: "The Trusted Signing Smoke Test workflow runs
GREEN on GitHub's clean windows-latest runner... proving the signing path is live end-to-end
before any release is cut."

**This requirement is NOT satisfied by this plan.** No run was dispatched, no GREEN result
exists, and none is claimed. SIGN-03 remains open, marked **Blocked/Deferred** in
`.planning/REQUIREMENTS.md`, pending the corrected unblock preconditions above.

## Hand-off

This is **not** a failure of the SIGN-02 hardening — that work (Plans 01-02) is complete and
live in the working tree, and the shared `Assert-TrustedSignature` helper is fully wired into
all fail-closed verify sites. SIGN-03 is a **deferred blocker** on (a) the hardened workflow
being unpushed and (b) the operator-gated posture around pushing/dispatching live signing CI,
plus (c) the still-open `UnknownError` root cause — not a defect in this phase's own
deliverables, and (as of this correction) not a missing-GitHub-config problem.

The push + dispatch of the hardened smoke workflow are handed off to **Phase 104** (Smoke Green
+ Cut the Trusted-Signed Release), whose own success criterion #1 already requires "the
operator re-runs the hardened Trusted Signing Smoke Test workflow and confirms it is GREEN"
immediately before the release tag push. Phase 104 must push Plan 02's local branch (with
operator authorization) before it can perform that re-run — it does **not** need to
re-provision GitHub OIDC/variables absent new evidence to the contrary.
