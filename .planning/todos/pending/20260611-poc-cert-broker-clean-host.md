# TODO: cut a trusted-signed release + clean-host UAT (broker spawns out-of-the-box)

**Captured:** 2026-06-11 (Phase 66 WR-02 EDR UAT, clean-host install) — **headline finding**
**Narrowed:** 2026-06-25 — signing-pipeline fix + docs DONE; remaining = release-cut + host-gated UAT
**Severity:** medium (was high) — the fix is implemented in CI; only an actual trusted-signed release + clean-host confirmation remain
**Source:** `.planning/phases/66-wr-02-edr-human-uat/66-HUMAN-UAT.md` (findings)
**Resolves phase:** 67 — Clean-Host Windows Install (v2.11; TRUST-01/TRUST-02 interim trust path only — real publicly-trusted signing is DIST-SIGN-01, deferred to the enterprise milestone)
**Resolves phase (v3.0):** 82 — Fleet Deployment Infrastructure (DEPLOY-05 silent POC root-cert install so the broker works on a clean fleet host; real publicly-trusted signing remains DIST-SIGN-01, out of scope)

## Problem
v0.62.2 is Authenticode-signed with a **self-signed `CN=nono Test Signing` POC cert**. On any
clean host that cert is untrusted → `Get-AuthenticodeSignature` returns `InvalidSignature`
(`0x800B0109` CERT_E_UNTRUSTEDROOT). The D-32-12 broker self-trust gate (correctly, fail-secure)
then **refuses to spawn the broker**, so `nono run --profile claude-code` (the supervised path)
**fails out-of-the-box**. We could only proceed in the UAT by manually importing the POC cert into
LocalMachine\Root + TrustedPublisher.

## Implication
The fork's public releases are effectively **dev/POC artifacts** for the supervised path — they
do not run on a clean machine without the operator trusting a self-signed cert. This is the most
material WR-02 finding.

## Fix options
- Sign releases with a **publicly-trusted** code-signing cert (EV/OV, e.g. via Azure Trusted
  Signing — note the existing `trusted-signing-smoke.yml` workflow), so Authenticode is `Valid`
  on any host, OR
- If keeping the POC cert for now: **document** the limitation prominently + ship the cert and an
  import step, and/or provide a `nono setup --trust-broker` helper. Cross-reference
  `docs/cli/development/windows-signing-guide.mdx`.

## Acceptance
`nono run --profile claude-code -- ...` spawns the broker on a clean Win11 host with no manual
cert-trust step (or the limitation is explicitly documented + a supported trust path exists).

## Status — 2026-06-25 (narrowed): signing-pipeline fix + docs DONE; release + UAT remain

**DONE — fix branch 1 implemented at the CI level:**
- release.yml migrated to **Azure Trusted Signing** (keyless OIDC, Microsoft-trusted root) — commit
  **`20cd68d9`** *"ci(release): switch Windows Authenticode signing to Azure Trusted Signing (D-02)"*
  (quick-task `260603-i31`), replacing the self-signed `CN=nono Test Signing` POC cert.
- The Trusted Signing step signs **all** top-level release `.exe` (`files-folder-filter: exe`,
  no recurse) — so `nono.exe` AND `nono-shell-broker.exe` are signed by the **same** profile →
  matching signer subject + thumbprint → the D-32-12 broker self-trust gate
  (`verify_broker_authenticode`, full-chain `WinVerifyTrust`) passes on a clean host.
- Limitation + trust path **documented** in `docs/cli/development/windows-signing-guide.mdx`
  ("Runtime symptom: unsigned install cannot spawn the broker" + dev-layout workaround).

**REMAINING (release-cut + host-gated — cannot be done from a Windows dev host):**
1. **Cut a release post-migration.** The latest *published* artifact is still `v0.62.2`, which
   predates the migration and is **still POC-signed**. The pipeline is ready but unrealized until
   the next `v*.*.*` tag push produces trusted-signed binaries.
2. **Confirm Azure Trusted Signing is live** — depends on repo secrets/vars (`AZURE_CLIENT_ID`,
   `AZURE_TENANT_ID`, `AZURE_SUBSCRIPTION_ID`, `TRUSTED_SIGNING_ENDPOINT/ACCOUNT/PROFILE`) being
   configured + the SP holding the "Trusted Signing Certificate Profile Signer" role. The
   release.yml comment flags this as not-yet-verified-live ("verify against the action's README
   when wiring live").
3. **Clean-host UAT** — on a fresh Win11 box with a trusted-signed release, confirm
   `nono run --profile claude-code` spawns the broker with NO manual cert-trust step.

Real publicly-trusted signing was originally tracked as DIST-SIGN-01 (enterprise milestone); the
pipeline work landed early via `260603-i31`. The dev-layout binary remains the supported trust path
for local/dev use until a trusted-signed release ships.
