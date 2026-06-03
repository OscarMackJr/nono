---
quick_id: 260603-i31
slug: cosign-sigstore-signing-authority
status: complete
date: 2026-06-03
verdict: NO (as Authenticode signing authority) / YES (as additive supply-chain provenance)
---

# Determination — sigstore/cosign as the v2.9 release signing authority

## Verdict

**No — sigstore/cosign CANNOT be the signing authority for this release, and it does NOT
unblock D-02.** It is the wrong trust model for what this release ships.

**But yes — `cosign sign-blob` is a worthwhile *additive* supply-chain provenance layer** you
can bolt on alongside Authenticode (not instead of it). It cannot replace the Authenticode
requirement that D-02 is blocked on.

## Why — the two are different trust mechanisms solving different problems

This release ships **Windows MSIs, `.exe` binaries (nono.exe, broker), and the
`nono-wfp-service.exe` / `nono-wfp-driver.sys` kernel-adjacent components.** For Windows to
*trust* these at install / run / driver-load time, they must carry an **Authenticode**
signature chained to a certificate Windows already trusts (a CA-issued OV/EV code-signing cert,
or a managed cloud-signing service). Authenticode is what drives:

- SmartScreen reputation and the UAC "Verified publisher" vs "Unknown publisher" prompt
- The MSI **payload** signature checks (release.yml already gates on these: `Verify MSI payload
  signatures` admin-extracts each MSI and runs `Get-AuthenticodeSignature` on every payload
  `.exe`/`.sys`)
- Kernel/driver code-signing trust for the WFP component

**`cosign sign-blob --yes my-built-binary.tar.gz`** does something fundamentally different:

- It produces a **detached sigstore signature** over an opaque *blob* using keyless signing
  (ambient OIDC identity → Fulcio short-lived cert → logged in the Rekor transparency log).
- The output is a `.sig` + `.pem` (+ Rekor proof). Verification is **out-of-band** via
  `cosign verify-blob` — a separate tool the consumer must run on purpose.
- **Windows does not know about, consume, or trust sigstore signatures for code execution.**
  A `cosign`-signed `nono-v0.58.0.tar.gz` still contains an `.exe` that Windows sees as
  *unsigned* once extracted. It will still show "Unknown publisher", still fail the release.yml
  MSI-payload Authenticode gate, and still cannot load a signed driver.

So `sign-blob` proves *"this archive was produced by this CI identity and logged in Rekor"*
(provenance / anti-tamper for the download). It says nothing Windows can act on about the
trustworthiness of the binaries *inside* it. Signing the `.tar.gz` does not sign the `.exe`
or `.msi` in any OS-recognized way.

## Repo facts that ground this (verified 2026-06-03)

- `cosign` is available here: `/c/tools/bin/cosign`.
- `release.yml` Windows signing is **Authenticode**: `signtool` + `Set-AuthenticodeSignature`,
  keyed on the `WINDOWS_SIGNING_CERT` PFX secret, with a fail-closed secret check and a
  `Verify MSI payload signatures` admin-extract gate (lines ~124–332). This is exactly the
  path D-02 is blocked on.
- The repo **already uses sigstore** for a *different* purpose: it ships `trusted_root.json`
  (Phase 49 REQ-POC-TRUST) and `SHA256SUMS.txt` as release assets, and the CLI links
  `sigstore-rs` (`sigstore-verify`/`sigstore-sign`) for attestation/verification. A `cosign`
  blob-signing step would align with that existing provenance direction — but it lives in the
  provenance lane, not the Windows-code-signing lane.

## What this means for the blocked release

- Adding the proposed cosign step **will not unblock v2.9.** The blocker is Authenticode, and
  cosign is not Authenticode. Shipping with only a cosign signature would leave every Windows
  user on "Unknown publisher" and would still fail the pipeline's own MSI-payload Authenticode
  verification.
- The honest D-02 remediation paths are all Authenticode:
  1. **Azure Trusted Signing** (recommended — cheapest/fastest modern route; ~$10/mo Microsoft
     managed cloud Authenticode signing, integrates with `signtool`, MS-trusted root, no PFX to
     custody). Best fit to replace the POC PFX with minimal pipeline change.
  2. A purchased **OV/EV code-signing certificate** (DigiCert / Sectigo / etc.) with the key in
     an HSM or cloud KMS; feed it as `WINDOWS_SIGNING_CERT[_PASSWORD]`.
  3. For the **WFP kernel driver** specifically, the bar is higher — an **EV** cert plus the
     Microsoft Partner Center / attestation-signing portal (WHQL/attestation). (Today the
     shipped `nono-wfp-driver.sys` is a checked-in pre-signed placeholder; a real driver ship
     would need this.)

## Recommended use of cosign (optional, additive — does not gate v2.9)

Once D-02 is solved with Authenticode, you *may* add a cosign provenance layer on top:

- `cosign sign-blob --yes` the distribution archives and the digest manifest
  (`*.tar.gz`, `*.zip`, `SHA256SUMS.txt`, and even the `.msi` as a blob), publishing the
  `.sig`/`.pem` (or a `.bundle`) alongside each asset.
- Use the GitHub Actions OIDC identity for keyless signing (no long-lived key), and document
  `cosign verify-blob --certificate-identity ... --certificate-oidc-issuer ...` in the release
  notes so downloaders can verify provenance + transparency-log inclusion.
- Label it clearly as **supply-chain provenance / attestation**, NOT as code-signing — so it is
  never mistaken for the OS-trust signature. It complements `trusted_root.json` rather than
  replacing Authenticode.

## Bottom line

cosign/sigstore = verifiable **provenance** of the download. Authenticode = the OS **trust** of
the executable/installer/driver. v2.9 is blocked on the latter; cosign cannot substitute for it.
Pursue Azure Trusted Signing (or an OV/EV cert) to clear D-02; optionally add `cosign sign-blob`
afterward as a provenance bonus.

## Self-Check: PASSED
- Claims grounded against `release.yml` (Authenticode signing + MSI-payload gate) and the
  Phase 49 `trusted_root.json` asset; cosign confirmed available at `/c/tools/bin/cosign`.
- No code changed (research-determination task).
