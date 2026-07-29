# Phase 104 Plan 03 — Task 2 verdict: FAILED (RED, diagnosed)

**Date:** 2026-07-29
**Run:** [`30460949560`](https://github.com/OscarMackJr/nono/actions/runs/30460949560) — `Trusted Signing Smoke Test`, ref `milestone/v2.13-carryforward-closeout`, conclusion **failure**
**Verdict:** Task 2 (SC1) **not satisfied**. Task 3 (tag push) correctly **not attempted**.

## What happened

The failure moved to a **different step** than every prior run. Previously Sign succeeded and
Verify failed. This time:

| Step | Result |
|---|---|
| Azure login (OIDC) | ✅ success |
| Compile fresh unsigned binary | ✅ success (`Pre-sign status: NotSigned`) |
| **Sign with Azure Trusted Signing** | ❌ **failure — HTTP 403** |
| Verify the embedded signature | — never ran |

```
"Metadata": {
  "Endpoint": "https://eus.codesigning.azure.net/",
  "CodeSigningAccountName": "ArtifactNono",
  "CertificateProfileName": "NonoPublicTrust"
}
Submitting digest for signing...
Unhandled managed exception
Azure.RequestFailedException: Service request failed.
Status: 403 (Forbidden)
   at Azure.CodeSigning.CertificateProfileRestClient.SignAsync(...)
Error information: "Error: SignerSign() failed." (-2147467259/0x80004005)
SignTool Error: An unexpected internal error has occurred.
```

This is **authorization at the data-plane sign call**, not authentication and not a certificate-chain
problem. OIDC login succeeded, so the federated credential and the `environment: Development`
subject claim are both fine.

## Ruled out

| Hypothesis | Evidence against |
|---|---|
| Profile reverted to the old private one | Sign metadata shows `CertificateProfileName: NonoPublicTrust`; `gh variable list` confirms the var |
| Profile is the wrong type again | ARM GET: `profileType = PublicTrust`, `identityValidationId = 20cb70d3-…` (the **public** validation) |
| Profile disabled / failed provisioning | ARM GET: `status = Active`, `provisioningState = Succeeded` |
| RBAC signer role removed | `Artifact Signing Certificate Profile Signer` **is** assigned at **account** scope (`…/codeSigningAccounts/ArtifactNono`) to app `eaab6a83-ad20-44b8-9431-7248b16ee8f8` (`Nono_Windows_Native`). Account scope covers all profiles. |
| Chain / root propagation | Not reached — Verify never ran. Also permanently retired as a theory (see the plan's `<amendment>`). |

## The lead: certificate rotation stopped 2026-07-15

Trusted Signing issues short-lived (3-day) certificates and rotates them continuously while the
account is healthy. The profile's inventory:

- **12 certificates total — 3 `Active`, 9 `Expired`**
- Oldest: created 2026-07-04 (the day the `NonoPublicTrust` fix landed)
- **Newest: created 2026-07-15 02:56, expired 2026-07-18 02:56**

Today is 2026-07-29. **No certificate has been issued in 14 days, and even the newest one expired
11 days ago.** A profile that is `Active` but has stopped minting certificates, combined with a 403
on the sign call, is the signature of the **underlying identity validation having lapsed** — the
public identity validation is time-bound and requires renewal, and when it lapses Trusted Signing
stops issuing and refuses to sign.

Timeline fits: the working smoke run was 2026-07-04, rotation continued to 2026-07-15, then stopped.

## Not confirmed from this host

The identity validation's own status/expiry could **not** be read here:

- ARM has no `identityValidations` resource type at this path —
  `ResourceTypeRegistrationNotFound` on `…/codeSigningAccounts/identityValidations`
  (it is a data-plane/portal concept, not ARM).
- The `az trustedsigning` CLI extension is not installed, and installing it fails on this host's
  corporate TLS interception to `aka.ms` (pre-existing, documented in Phase 101).

So the identity-validation hypothesis is **strongly evidenced but not directly confirmed**. It must
be checked in the portal.

## Operator action required

1. **Azure Portal → Trusted Signing → `ArtifactNono` → Identity validations.** Find validation
   `20cb70d3-2d17-4fdb-9121-963628df6b63` and read its **status and expiry**. If it is expired,
   lapsed, or needs re-attestation, renew it. Renewal is org legal-identity verification with a
   Microsoft review — hours to days, bounded and self-service.
2. If the validation is healthy, the 403 is instead an RBAC principal mismatch: confirm the
   `AZURE_CLIENT_ID` repo secret resolves to app `eaab6a83-ad20-44b8-9431-7248b16ee8f8`
   (`Nono_Windows_Native`), the principal that actually holds the signer role. The role assignment
   itself is present and correctly scoped, so a mismatch would have to be on the secret's side.
3. Re-dispatch the smoke workflow. Only on a genuine GREEN may Plan 104-03 Task 3 (tag push) proceed.

## What must NOT be done

- **Do not loosen the fail-closed gate.** It is behaving correctly: the release aborted rather than
  shipping unsigned or badly-signed artifacts.
- **Do not push tag `v0.66.1`.** Signing is currently non-functional; the Release workflow would
  fail at the same 403, and the tag is irreversible.
- **Do not resurrect the root-CTL propagation theory** or gate on
  `scripts/azure/check-trusted-signing-root.ps1` — see the plan's `<amendment>`.

## Status of plan 104-03

| Task | Status |
|---|---|
| Task 1 — automated pre-flight | ✅ complete (release-readiness PASS, publish-selector PASS, root pre-check moot) |
| Task 2 — poll-until-green smoke (SC1) | ❌ **FAILED — 403 at Sign**, diagnosed, operator action required |
| Task 3 — tag push (SC2 + SC3) | ⛔ **not attempted** — correctly blocked by Task 2 |

REL-01 remains unsatisfied. Phase 104 stays open.
