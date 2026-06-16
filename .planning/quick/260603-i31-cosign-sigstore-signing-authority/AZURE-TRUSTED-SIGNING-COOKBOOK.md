# Cookbook — Azure Trusted Signing for the nono v2.9 Windows release (clears D-02)

> Goal: replace the self-signed POC cert (the D-02 blocker) with Microsoft-trusted Authenticode
> signing so released MSIs/`.exe`s show **Verified publisher** and pass the release.yml payload
> gate — without custodying a private key or a PFX.
>
> ⚠ **Accuracy note:** authored against Azure Trusted Signing as of knowledge cutoff Jan 2026.
> Azure changes UI/pricing/eligibility often — confirm each Azure-side step against the current
> docs: <https://learn.microsoft.com/azure/trusted-signing/>. The GitHub-side and release.yml
> wiring below is exact for this repo.

---

## 0. The mental model (why this is NOT a PFX)

Trusted Signing is a **managed cloud signing service**. The private key lives in Microsoft's
HSM; you never download a cert or PFX. You authenticate to Azure (a service principal / OIDC),
and `signtool` calls the service via a small **dlib** to sign each file. So:

- ❌ There is no `WINDOWS_SIGNING_CERT` base64 PFX to update (the current guide is wrong on this).
- ✅ Auth is Azure RBAC: an identity with the **"Trusted Signing Certificate Profile Signer"**
  role calls your **Trusted Signing account → certificate profile**.
- ✅ The resulting signature chains to a **Microsoft-operated, Windows-trusted root** → real
  SmartScreen/UAC "Verified publisher".

---

## 1. ⚠ FIRST — check eligibility BEFORE investing time

Trusted Signing **Public Trust** certificate profiles require Microsoft **identity validation**:

- **Organization** validation needs a verifiable legal business entity. Historically Microsoft
  required the org to have existed **3+ years**, or else additional documentation.
- **Individual** validation is supported but also goes through verification.

This matters here: `oscarmackjr-twg/nono` is a personal fork. If there's no established legal
entity, you may hit the org-age wall and need the **individual** validation path (or a different
Authenticode route — an OV/EV cert from a CA). **Do step 1 before steps 2+** so you don't build
the whole pipeline against an account you can't validate.

> If validation is a blocker: fall back to a purchased **OV code-signing cert** (DigiCert /
> Sectigo / SSL.com), key in an HSM/USB token or cloud KMS. That keeps the existing PFX-style
> `sign-windows-artifacts.ps1` path — just swap the POC PFX for the real one. (EV is only needed
> to skip SmartScreen warm-up reputation, or for kernel drivers — see §6.)

---

## 2. Azure one-time setup (portal or CLI)

Prereqs: an Azure subscription + the Azure CLI (`az`) logged in (`az login`).

```bash
# Register the resource provider (once per subscription)
az provider register --namespace Microsoft.CodeSigning

# Create a resource group (or reuse one)
az group create -n rg-nono-signing -l eastus

# Create the Trusted Signing account (pick a region near your CI; note its endpoint)
#   Basic SKU ~ $9.99/mo (verify current pricing). The account endpoint looks like
#   https://eus.codesigning.azure.net  (region-prefixed).
az trustedsigning create \
  -n nono-trusted-signing \
  -g rg-nono-signing \
  -l eastus \
  --sku Basic
# (If the `az trustedsigning` extension isn't present: `az extension add -n trustedsigning`.
#  Portal path: "Trusted Signing accounts" → Create.)
```

Then in the portal (or CLI), under the account:

1. **Identity validation** → submit your org or individual validation. **Wait for Approved**
   (can take hours-to-days). Nothing below works until this is approved.
2. **Certificate profiles** → Create → type **Public Trust** (this is the public-release one;
   "Test" = a Microsoft test root, not publicly trusted; "Private Trust" / "Public Trust CI" are
   for internal/driver scenarios). Give it a name, e.g. `nono-public`.
3. Record three values you'll need for CI:
   - **Endpoint** (e.g. `https://eus.codesigning.azure.net`)
   - **Account name** (`nono-trusted-signing`)
   - **Certificate profile name** (`nono-public`)

---

## 3. Create the CI identity (GitHub OIDC — no stored secret key)

Create an Entra ID app registration + service principal, and federate it to GitHub Actions OIDC
so CI gets a short-lived token (no client secret to store):

```bash
# App registration + service principal
az ad app create --display-name "nono-ci-trusted-signing"
APP_ID=$(az ad app list --display-name "nono-ci-trusted-signing" --query "[0].appId" -o tsv)
az ad sp create --id "$APP_ID"

# Federated credential: trust GitHub Actions for this repo.
# Use ref:refs/tags/* if you want to scope it to tag pushes (this release triggers on tags).
az ad app federated-credential create --id "$APP_ID" --parameters '{
  "name": "github-nono-release",
  "issuer": "https://token.actions.githubusercontent.com",
  "subject": "repo:oscarmackjr-twg/nono:ref:refs/heads/main",
  "audiences": ["api://AzureADTokenExchange"]
}'
# Add a second federated credential for tag refs if you sign on tag push:
#   "subject": "repo:oscarmackjr-twg/nono:ref:refs/tags/v0.58.0"  (or use an environment subject)
```

Grant the SP the signer role **scoped to the Trusted Signing account**:

```bash
ACCOUNT_ID=$(az resource show -g rg-nono-signing -n nono-trusted-signing \
  --resource-type Microsoft.CodeSigning/codeSigningAccounts --query id -o tsv)

az role assignment create \
  --assignee "$APP_ID" \
  --role "Trusted Signing Certificate Profile Signer" \
  --scope "$ACCOUNT_ID"
```

### ⚠ API permissions: NONE — it's RBAC, not API permissions (common trap)

Do **not** add anything on the app registration's **API permissions** blade. Trusted Signing
authorization is an Azure **RBAC data action** (`Microsoft.CodeSigning/.../sign`) granted by the
`az role assignment create` above — it is **not** an OAuth/Graph "API permission". Specifically:

- **App registration → API permissions:** leave empty. The default `User.Read` (Microsoft Graph)
  that Azure auto-adds is unnecessary for a signing-only service principal; leave or remove it,
  no effect either way. **No admin consent** is needed.
- **What the app actually carries** is: (1) the **federated credential** above (who the app is —
  not an API permission), and (2) the **`Trusted Signing Certificate Profile Signer`** RBAC role
  on the account (what it may do). That role alone is sufficient for CI signing.
- `Trusted Signing Identity Verifier` is a *different* built-in role for managing identity
  **validation**, not signing — skip it for CI.
- **Your own** rights to *make* the role assignment: you need **Owner** or **User Access
  Administrator** / **Role Based Access Control Administrator** on the account scope (this is
  about the admin running setup, not the app).

Mental model: *federated credential* (identity) + *RBAC role* (authorization). No API
permissions in the loop.

---

## 4. GitHub repo configuration

Add these **repository secrets** (Settings → Secrets and variables → Actions) — note: only IDs,
no private key:

| Secret | Value |
|--------|-------|
| `AZURE_CLIENT_ID` | the `$APP_ID` from §3 |
| `AZURE_TENANT_ID` | `az account show --query tenantId -o tsv` |
| `AZURE_SUBSCRIPTION_ID` | `az account show --query id -o tsv` |

Optionally make the endpoint/account/profile **repository variables** (not secret):
`TRUSTED_SIGNING_ENDPOINT`, `TRUSTED_SIGNING_ACCOUNT`, `TRUSTED_SIGNING_PROFILE`.

The signing job needs OIDC permission — ensure the release job has:

```yaml
permissions:
  id-token: write   # required for azure/login OIDC
  contents: write   # already needed for the gh-release publish
```

---

## 5. Wire it into THIS repo's release.yml (exact step-level changes)

The current Windows signing region (verified at `.github/workflows/release.yml`) is:

| Line | Step | Change |
|------|------|--------|
| ~124 | `Check signing secrets (Windows)` (fail-closed on `WINDOWS_SIGNING_CERT`) | **Replace** the check with an `azure/login@v2` step (fail-closed naturally if OIDC creds absent) |
| ~146 | `Sign Windows binaries (pre-package)` → `sign-windows-artifacts.ps1 -CertBase64 …` over nono.exe/broker/wfp-service | **Replace** with `azure/trusted-signing-action` over the 3 binaries |
| ~166 | `Package (Windows)` | **unchanged** — still harvests the now-signed binaries into both MSIs |
| ~218 | `Sign Windows MSIs` → `sign-windows-artifacts.ps1 -CertBase64 …` over the 2 MSIs | **Replace** with `azure/trusted-signing-action` over the 2 MSIs |
| ~236 | `Verify Authenticode signatures (Windows)` (`Get-AuthenticodeSignature` Status=Valid) | **unchanged** — cert-agnostic; passes with the real MS-trusted cert |
| ~255 | `Verify MSI payload signatures` (admin-extract + per-payload Authenticode) | **unchanged** — this is the D-06 gate; it just starts passing for real |

Preserve the **signing ORDER** that Phase 53 fixed (sign binaries → package MSI → sign MSI
wrappers → verify). Only the *signing primitive* changes; the order and the verify gates stay.

### 5a. Add the Azure login step (replaces the secret-check)

```yaml
      - name: Azure login (OIDC) for Trusted Signing
        if: runner.os == 'Windows'
        uses: azure/login@v2
        with:
          client-id: ${{ secrets.AZURE_CLIENT_ID }}
          tenant-id: ${{ secrets.AZURE_TENANT_ID }}
          subscription-id: ${{ secrets.AZURE_SUBSCRIPTION_ID }}
```

### 5b. Replace the binary-signing step

```yaml
      - name: Sign Windows binaries (pre-package) — Trusted Signing
        if: runner.os == 'Windows'
        uses: azure/trusted-signing-action@v0
        with:
          endpoint: ${{ vars.TRUSTED_SIGNING_ENDPOINT }}          # e.g. https://eus.codesigning.azure.net
          trusted-signing-account-name: ${{ vars.TRUSTED_SIGNING_ACCOUNT }}   # nono-trusted-signing
          certificate-profile-name: ${{ vars.TRUSTED_SIGNING_PROFILE }}       # nono-public
          files: |
            ${{ github.workspace }}\target\${{ matrix.target }}\release\${{ matrix.artifact }}
            ${{ github.workspace }}\target\${{ matrix.target }}\release\nono-shell-broker.exe
            ${{ github.workspace }}\target\${{ matrix.target }}\release\nono-wfp-service.exe
          file-digest: SHA256
          timestamp-rfc3161: http://timestamp.acs.microsoft.com
          timestamp-digest: SHA256
```

### 5c. Replace the MSI-signing step (after Package)

```yaml
      - name: Sign Windows MSIs — Trusted Signing
        if: runner.os == 'Windows'
        uses: azure/trusted-signing-action@v0
        with:
          endpoint: ${{ vars.TRUSTED_SIGNING_ENDPOINT }}
          trusted-signing-account-name: ${{ vars.TRUSTED_SIGNING_ACCOUNT }}
          certificate-profile-name: ${{ vars.TRUSTED_SIGNING_PROFILE }}
          files: |
            ${{ github.workspace }}\artifact_staging\nono-${{ env.RELEASE_TAG }}-x86_64-pc-windows-msvc-machine.msi
            ${{ github.workspace }}\artifact_staging\nono-${{ env.RELEASE_TAG }}-x86_64-pc-windows-msvc-user.msi
          file-digest: SHA256
          timestamp-rfc3161: http://timestamp.acs.microsoft.com
          timestamp-digest: SHA256
```

> **Alternative (keep `sign-windows-artifacts.ps1`):** instead of the action, install the
> Trusted Signing dlib (`dotnet tool install --global Azure.CodeSigning.Client` or the
> `Microsoft.Trusted.Signing.Client` NuGet) and have the script call
> `signtool sign /v /fd SHA256 /tr http://timestamp.acs.microsoft.com /td SHA256 /dlib <Azure.CodeSigning.Dlib.dll> /dmdf <metadata.json> <files...>`,
> where `metadata.json` = `{ "Endpoint": "...", "CodeSigningAccountName": "...",
> "CertificateProfileName": "..." }`. Auth comes from the `azure/login` token (the dlib uses
> `DefaultAzureCredential`). The action is simpler; the script keeps one signing entrypoint.

### 5d. Retire the POC secrets

Once green, delete the `WINDOWS_SIGNING_CERT` / `WINDOWS_SIGNING_CERT_PASSWORD` repo secrets and
drop their `env:` blocks from the replaced steps so there's no POC-cert fallback path left.

---

## 6. ⚠ The WFP kernel driver caveat (`nono-wfp-driver.sys`)

Trusted Signing **Public Trust** signs user-mode `.exe`/`.msi` and (via a separate **Public Trust
CI / driver** profile) certain driver scenarios — but **kernel-mode driver loading** on modern
Windows still generally requires the **Microsoft Partner Center attestation/WHQL** flow (EV
identity), not a plain code-signing cert. In this repo the machine MSI ships
`crates/nono-cli/data/windows/nono-wfp-driver.sys` as a **checked-in pre-signed placeholder**, so
v2.9 does **not** need driver signing to ship — the user-mode `nono-wfp-service.exe` is what
Trusted Signing covers. A real WFP-callout driver ship later is a separate EV+attestation track;
keep it out of the D-02 critical path.

---

## 7. Verify after the first signed run

On a Windows host, on a downloaded MSI and an extracted payload:

```powershell
(Get-AuthenticodeSignature .\nono-v0.58.0-...-machine.msi).Status          # Valid
(Get-AuthenticodeSignature .\nono.exe).SignerCertificate.Subject            # CN=<your validated name>, ...
# Issuer should chain to a Microsoft ID Verified CS root (publicly trusted), NOT 319E507E… (POC)
```

Then the release.yml `Verify MSI payload signatures` gate passes for real, and this is the moment
D-02 is cleared — re-run the tag step (`git tag -a v0.58.0 …` → push) to ship v2.9.

---

## 8. Cost + time summary

| Item | Note |
|------|------|
| Trusted Signing **Basic** SKU | ~**$9.99/mo** (confirm current pricing; includes a monthly signature quota ample for releases) |
| Identity validation | the long pole — hours to days; **do §1 first** |
| Private key custody | none — Microsoft HSM, keyless from CI via OIDC |
| Pipeline change | ~3 steps swapped; verify gates + signing order unchanged |

## 9. Also fix the stale guide

`docs/cli/development/windows-signing-guide.mdx` currently implies Trusted Signing means
"update the `WINDOWS_SIGNING_CERT` PFX secret" (lines ~244–251). That's incorrect (Trusted
Signing is keyless/dlib, no PFX). When you wire this up, correct that section to point at this
cookbook. (Editing that file needs `git add -f` — it's gitignored-but-tracked.)
