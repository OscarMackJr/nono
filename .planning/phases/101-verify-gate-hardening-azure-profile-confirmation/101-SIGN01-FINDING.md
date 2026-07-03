# SIGN-01 Finding: Azure Trusted Signing Certificate Profile Confirmation

**Cross-reference:** `.planning/REQUIREMENTS.md` — **SIGN-01**
**Confirmed:** 2026-07-02
**Confirmed by:** Operator (live Azure session)

## Confirmation Route

The operator attempted `az trustedsigning certificate-profile show` from their own
authenticated Azure session, but the `trustedsigning` CLI extension failed with a
corporate-TLS SSL error while fetching the extension index (a local environment/network
constraint, not an Azure-side failure). The operator therefore confirmed the profile type
via the **Azure Portal / management plane directly** instead of the CLI. This is a valid
substitute confirmation route per the plan's intent ("confirmed via the `az` CLI (or
portal)") — the profile type below is a live, operator-observed fact, not an inference from
issuer-string pattern-matching.

## profileType: `PublicTrust`

| Field | Value |
|-------|-------|
| Resource group | `RG_Nono` |
| Trusted Signing (code signing) account | `ArtifactNono` |
| Certificate profile type | **`PublicTrust`** (operator statement: "this is the public trust") |
| Identity id associated with the public-trust profile | `20cb70d3-2d17-4fdb-9121-963628df6b63` |
| Profile name | Not separately reported by the operator — not invented here. Account = `ArtifactNono`; the distinct profile name under that account was not captured in this confirmation pass. |

## Fix Status

**No fix was applied.** The profile is already `PublicTrust` — there was no
`PublicTrustTest` profile to replace, so none of the cookbook §2 (`certificate-profile
create --profile-type PublicTrust`) or §3b (FIC-subject correction) remediation steps were
executed. This is expected: those steps are conditional on the profile being
`PublicTrustTest`, which it is not.

## Prior Smoke-Run Evidence (on record, unchanged)

From live smoke run `28467925298` (2026-06-30), recorded in auto-memory
`azure_trusted_signing_golive.md`:

- OIDC login + Sign: **GREEN** (signer `CN=TWGGLOBAL.onmicrosoft.com`)
- Verify: **FAILED** — `Status: UnknownError`
- Observed issuer: `CN=Microsoft Enterprise ID Verified Policy AOC CA 01`

## Discrepancy — Documented Honestly, Not Resolved by Guessing

The phase's working hypothesis (per `101-CONTEXT.md` and `REQUIREMENTS.md` SIGN-01 wording)
was that the `…Enterprise ID Verified Policy AOC CA…` issuer naming was the tell for a
`PublicTrustTest` profile. **This live confirmation contradicts that hypothesis**: the
profile is confirmed `PublicTrust`, yet the smoke run still produced `UnknownError` with
that issuer string.

**Conclusion:** the residual `UnknownError` observed on 2026-06-30 is **NOT a profile-type
issue**. The root cause must be something else in the taxonomy the requirements doc already
anticipates (chain-build staleness from the March-2026 AOC/EOC CA rotation, or a CRL/OCSP
revocation-check transient) — not resolved here. This is explicitly deferred: it should be
investigated via **Plan 01's D-04 full chain-introspection diagnostics** (the shared
`Assert-TrustedSignature` helper's failure-path chain dump) on a **future smoke run** (Plan
04 / Phase 104), not attributed to, or guessed as, a profile-type defect. Do not re-open the
profile-type hypothesis without new evidence.

## Follow-Up Note (material for Phase 104 planning): GitHub Trusted Signing Config Does Not Exist Yet

The operator confirmed there are currently **NO GitHub Actions variables or secrets** —
repo-level or environment-scoped — for Trusted Signing on the `OscarMackJr/nono` GitHub
repo. Specifically:

- `TRUSTED_SIGNING_ACCOUNT`, `TRUSTED_SIGNING_PROFILE`, `TRUSTED_SIGNING_ENDPOINT` do not
  exist.
- The OIDC federated credential (FIC) does not exist.

**Consequences:**

- This plan's originally-anticipated sub-steps — "update the GitHub
  `TRUSTED_SIGNING_PROFILE` variable" and "correct the FIC subject to
  `repo:OscarMackJr/nono:environment:Development`" — are **DEFERRED**, because there is
  currently no existing GitHub configuration to update or correct.
- The **SIGN-03 smoke dispatch (Plan 04)** is **BLOCKED** until the GitHub Trusted Signing
  configuration (variables + OIDC federated credential) is re-provisioned from scratch.
- Any future GitHub-variable provisioning work **must use the live Azure names confirmed
  here** — account `ArtifactNono`, resource group `RG_Nono`, identity id
  `20cb70d3-2d17-4fdb-9121-963628df6b63` — **not** the go-live cookbook's example values
  (`nono-trusted-signing`, `rg-nono-signing`). The live names differ from the cookbook
  examples and this is a known, intentional divergence to carry forward, not a defect.

## SIGN-01 Requirement Satisfaction

Per `.planning/REQUIREMENTS.md` SIGN-01: "The Azure Trusted Signing certificate profile is
confirmed to be type `PublicTrust`... The finding is recorded (profile type + issuer chain)
so the root cause is documented, not guessed."

This finding satisfies that wording: `profileType == PublicTrust` is confirmed live (via
Azure Portal, since the `az` CLI extension was unavailable), and the finding is recorded
here — with the honest caveat that the previously-suspected root cause (profile type) is now
ruled out, and the true root cause of `UnknownError` remains open, routed to D-04
diagnostics on a future smoke run rather than guessed.
