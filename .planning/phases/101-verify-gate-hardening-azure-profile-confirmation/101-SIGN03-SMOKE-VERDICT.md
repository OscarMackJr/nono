# SIGN-03 Smoke Verdict

**Cross-reference:** `.planning/REQUIREMENTS.md` — **SIGN-03**
**Date:** 2026-07-02
**Verdict: BLOCKED** (explicitly NOT a PASS — SIGN-03 is NOT satisfied)

## No Run Was Dispatched

No `gh workflow run trusted-signing-smoke.yml` dispatch was performed, and no run ID or run
URL exists for this attempt. This is a deliberate decision, not an oversight — see
"Why Task 1 Was Not Performed" below. Do not treat the absence of a run URL as missing data;
it is the correct, honest record of what happened (nothing was dispatched).

## Why Task 1 Was Not Performed

Plan 04's Task 1 calls for dispatching the hardened smoke workflow and asserting a GREEN,
correctly-chained result. That dispatch was **not attempted** because it is structurally
guaranteed to fail before it could ever produce a meaningful (or even a false-positive)
result, for five independent reasons confirmed live in this session:

1. **No OIDC federated credential exists.** `azure/login` (OIDC) has nothing to authenticate
   against, so `azure/trusted-signing-action@v0` cannot obtain a token. The run would fail at
   the SIGN step and never reach the verify step at all — a GREEN, correctly-chained result
   is impossible under these conditions, not merely unlikely.
2. **No GitHub Actions variables exist.** `TRUSTED_SIGNING_ACCOUNT`,
   `TRUSTED_SIGNING_PROFILE`, and `TRUSTED_SIGNING_ENDPOINT` are all absent from the repo (and
   the `Development` environment), so even if authentication somehow succeeded, the signing
   action would receive empty/undefined inputs.
3. **Plan 02's hardened workflow has not been pushed.** The dot-sourced
   `Assert-TrustedSignature` wiring into `trusted-signing-smoke.yml` (Plan 02) exists only in
   the local working tree on this branch, not on GitHub's default branch. A dispatch today
   would run the pre-hardening workflow regardless of local state, making any result
   unrepresentative of the SIGN-02 hardening this gate exists to validate.
4. **`gh` resolves to the wrong repo.** On this host, `gh` currently resolves the repo
   context to `nolabs-ai/nono` (the upstream remote), not `OscarMackJr/nono` (the fork and the
   plan's intended dispatch target). A dispatch would either target the wrong repository or
   fail outright on permissions.
5. **Dispatching live Azure Trusted Signing CI is outward-facing**, and this fork is
   currently under a documented prepare-only / push-operator-gated release posture. Triggering
   a live signing round-trip against Azure infrastructure is not consistent with that posture
   without explicit operator action to re-provision the missing configuration first.

Any one of reasons 1-4 alone would make a correctly-chained GREEN result impossible. All five
holding simultaneously means there is no partial or exploratory value in dispatching anyway —
per Plan 04's own Task 2 rule ("If Task 1's run was NOT a correctly-chained success, this file
records that failure verbatim rather than a false PASS"), the honest record is that Task 1 was
not run, and this verdict is BLOCKED rather than a fabricated FAIL or PASS artifact.

## Unblock Preconditions for a Future SIGN-03 Attempt

Before any future dispatch of `trusted-signing-smoke.yml` can produce a meaningful verdict,
all of the following must be true:

1. **OIDC federated credential (FIC) provisioned** on the Azure AD application used for
   GitHub OIDC login, with subject `repo:OscarMackJr/nono:environment:Development` (the
   corrected subject — not the stale `260603-i31` cookbook's `oscarmackjr-twg` + `ref:` form,
   which causes `AADSTS700213`).
2. **GitHub Actions variables re-provisioned** using the **live Azure names** confirmed in
   `101-SIGN01-FINDING.md` — account `ArtifactNono`, resource group `RG_Nono` — **not** the
   go-live cookbook's example values (`nono-trusted-signing`, `rg-nono-signing`):
   - `TRUSTED_SIGNING_ACCOUNT` = `ArtifactNono`
   - `TRUSTED_SIGNING_ENDPOINT` = (the account's live endpoint URL)
   - `TRUSTED_SIGNING_PROFILE` = (the live PublicTrust profile name under `ArtifactNono`; not
     separately captured yet per `101-SIGN01-FINDING.md` — must be confirmed before use)
3. **Plan 02's hardened `trusted-signing-smoke.yml`** (dot-sourcing `Assert-TrustedSignature`)
   pushed to the `OscarMackJr/nono` default branch so a dispatch actually exercises the
   hardened verify path, not the pre-hardening workflow.
4. **`gh` confirmed to target `OscarMackJr/nono`** (not `nolabs-ai/nono`) before any dispatch
   or read command is run against "the repo."

Only once all four preconditions hold does a dispatch have any chance of producing a
correctly-chained result — and even then, per `101-SIGN01-FINDING.md`, the residual
`UnknownError` observed on the prior live smoke run (`28467925298`, 2026-06-30) is **not**
resolved by this provisioning alone; it must be investigated via Plan 01's D-04 chain
diagnostics on that eventual re-run.

## Relevant Context Carried Forward (Plan 03's Finding)

Per `101-SIGN01-FINDING.md`, the live Azure certificate profile is confirmed
`profileType == PublicTrust` (account `ArtifactNono`, resource group `RG_Nono`) — the
profile-type hypothesis for the 2026-06-30 `UnknownError` is **ruled out**. Once the GitHub
config above is re-provisioned with the correct live names, the signing side of a future
dispatch should chain to a public root; the previously observed `Status: UnknownError` /
`Issuer: CN=Microsoft Enterprise ID Verified Policy AOC CA 01` residual remains unresolved and
open, to be diagnosed via Plan 01's `Assert-TrustedSignature` D-04 chain-introspection output
on that future run — not guessed at here.

## SIGN-03 Requirement Status

Per `.planning/REQUIREMENTS.md` **SIGN-03**: "The Trusted Signing Smoke Test workflow runs
GREEN on GitHub's clean windows-latest runner... proving the signing path is live end-to-end
before any release is cut."

**This requirement is NOT satisfied by this plan.** No run was dispatched, no GREEN result
exists, and none is claimed. SIGN-03 remains open, marked **Blocked/Deferred** in
`.planning/REQUIREMENTS.md`, pending the unblock preconditions above.

## Hand-off

This is **not** a failure of the SIGN-02 hardening — that work (Plans 01-02) is complete and
live in the working tree, and the shared `Assert-TrustedSignature` helper is fully wired into
all fail-closed verify sites. SIGN-03 is a **deferred blocker** on missing GitHub-side Azure
configuration, discovered during Plan 03's operator confirmation pass, not a defect in this
phase's own deliverables.

The GitHub Trusted Signing re-provisioning (OIDC FIC + variables) and the eventual live smoke
dispatch are handed off to **Phase 104** (Smoke Green + Cut the Trusted-Signed Release), whose
own success criterion #1 already requires "the operator re-runs the hardened Trusted Signing
Smoke Test workflow and confirms it is GREEN" immediately before the release tag push. Phase
104 must re-provision the GitHub config described above before it can perform that re-run.
