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
| Profile name | `NonoCertProfile` — source: repo-scoped GitHub Actions variable `TRUSTED_SIGNING_PROFILE`, verified 2026-07-02. Account = `ArtifactNono`; endpoint = `https://eus.codesigning.azure.net/`. |

**Correction (2026-07-02):** the row above originally read "Not separately reported by the
operator." Independent verification found the real profile name recorded in the repo-scoped
GitHub Actions variable `TRUSTED_SIGNING_PROFILE`, alongside `TRUSTED_SIGNING_ACCOUNT=ArtifactNono`
and `TRUSTED_SIGNING_ENDPOINT=https://eus.codesigning.azure.net/` (all created 2026-06-04). See
also the corrected "Follow-Up Note" below.

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

## Follow-Up Note (material for Phase 104 planning): GitHub Trusted Signing Config — Corrected

**Correction (2026-07-02):** the section below originally reported that no GitHub Trusted
Signing config existed on the repo. Independent verification (`101-VERIFICATION.md`) found
this claim **factually wrong**, contradicted by live `gh` evidence in the same session:

- Repo-scoped variables `TRUSTED_SIGNING_ACCOUNT` (`ArtifactNono`), `TRUSTED_SIGNING_PROFILE`
  (`NonoCertProfile`), and `TRUSTED_SIGNING_ENDPOINT` (`https://eus.codesigning.azure.net/`)
  **do exist**, created 2026-06-04 (before this phase began).
- The OIDC federated credential **does exist and works** — run `28469674206` (2026-06-30)
  shows a successful OIDC login with the exact expected subject claim
  `repo:OscarMackJr/nono:environment:Development`, and the Sign step succeeded.

The most likely cause of the earlier false report: the operator checked only the GitHub
**environment-scoped** ("`Development`" environment) variables/secrets page, which is empty —
while the workflow's job specifies `environment: Development` but reads `${{ vars.TRUSTED_SIGNING_* }}`,
which GitHub Actions resolves from **repository-level** scope when no environment-level
override exists. No `Development` *environment* exists on the repo at all. **Reconciliation
CONFIRMED by the operator on 2026-07-02: the signing config is present and current** (not being
decommissioned) — the repo-scoped variables + OIDC federated credential are authoritative.

**Corrected consequences:**

- This plan's originally-anticipated sub-steps — "update the GitHub `TRUSTED_SIGNING_PROFILE`
  variable" and "correct the FIC subject to `repo:OscarMackJr/nono:environment:Development`" —
  remain **not needed**: the variable is already correctly `NonoCertProfile` and the FIC subject
  already matches the corrected form, confirmed working by the 2026-06-30 Sign step.
- The **SIGN-03 smoke dispatch (Plan 04)** is **BLOCKED**, but not on missing GitHub
  configuration. The sole remaining gates are: (1) Plan 02's hardened
  `trusted-signing-smoke.yml` is unpushed (all Phase 101 commits are local-only), so a dispatch
  today would run the pre-hardening workflow; (2) pushing + dispatching live Azure Trusted
  Signing CI is outward-facing and gated on explicit operator authorization under this fork's
  prepare-only/push-operator-gated posture; and (3) the prior real run's Verify `UnknownError`
  is still unresolved and must be re-diagnosed via Plan 01's D-04 chain diagnostics on the
  eventual hardened re-run. See the corrected `101-SIGN03-SMOKE-VERDICT.md` for full detail.
- **No fresh GitHub-variable or OIDC-FIC provisioning is expected to be necessary.** Do not
  re-create a second, possibly-misconfigured federated credential or duplicate variables
  against an Azure AD application that already has a working FIC — that would be needless
  config drift on a security-critical signing pipeline. The live names to keep using if any
  future provisioning is ever warranted: account `ArtifactNono`, resource group `RG_Nono`,
  profile `NonoCertProfile`, identity id `20cb70d3-2d17-4fdb-9121-963628df6b63` — **not** the
  go-live cookbook's example values (`nono-trusted-signing`, `rg-nono-signing`).

## SIGN-01 Requirement Satisfaction

Per `.planning/REQUIREMENTS.md` SIGN-01: "The Azure Trusted Signing certificate profile is
confirmed to be type `PublicTrust`... The finding is recorded (profile type + issuer chain)
so the root cause is documented, not guessed."

This finding satisfies that wording: `profileType == PublicTrust` is confirmed live (via
Azure Portal, since the `az` CLI extension was unavailable), and the finding is recorded
here — with the honest caveat that the previously-suspected root cause (profile type) is now
ruled out, and the true root cause of `UnknownError` remains open, routed to D-04
diagnostics on a future smoke run rather than guessed.
