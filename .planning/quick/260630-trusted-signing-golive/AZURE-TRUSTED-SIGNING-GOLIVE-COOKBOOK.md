# Cookbook — Azure Trusted Signing GO-LIVE (identity validation approved)

> **For:** a junior dev/ops engineer who has Azure subscription access + GitHub repo-admin on
> `OscarMackJr/nono`. Identity validation is **Approved** — this runbook takes you from "approved"
> to "a published release whose Windows binaries show **Verified publisher** on a clean host."
>
> **This is the operational go-live runbook.** The deep Azure account-creation guide is the
> companion: `.planning/quick/260603-i31-cosign-sigstore-signing-authority/AZURE-TRUSTED-SIGNING-COOKBOOK.md`.
> The CI is **already implemented** (`.github/workflows/release.yml` + `trusted-signing-smoke.yml`,
> commit `20cd68d9`) — you are not editing pipeline code, you are wiring credentials and pressing buttons.
>
> ⚠ **Accuracy note:** Azure portal UI/pricing/role names drift. Verify each Azure-side step against
> <https://learn.microsoft.com/azure/trusted-signing/>. The GitHub-side wiring below is **exact** for
> this repo as the workflows are written today.

---

## 0. The one thing the old cookbook gets wrong (read this first)

The companion setup cookbook (§3) shows the GitHub federated-credential subject as
`repo:oscarmackjr-twg/nono:ref:refs/heads/main`. **That is stale on two counts** and will cause
`azure/login` to fail with `AADSTS700213 / no matching federated identity record`:

1. The repo is now **`OscarMackJr/nono`** (capital O/M), not `oscarmackjr-twg/nono`.
2. Both implemented workflows authenticate from a job pinned to **`environment: Development`**, so
   the OIDC token's subject is an **environment** subject, not a **ref** subject.

✅ **The federated credential subject MUST be exactly:**

```
repo:OscarMackJr/nono:environment:Development
```

This is verified against `release.yml:29` and `trusted-signing-smoke.yml:20`. Get this one string
right and 90% of go-live failures disappear.

---

## 1. What you need before you start (credentials checklist)

| You need | Why | How to confirm |
|----------|-----|----------------|
| Azure CLI (`az`) logged in | run the setup commands | `az account show` |
| **Owner** or **User Access Administrator** / **RBAC Administrator** on the Trusted Signing account | only these can create the role assignment in §3 | `az role assignment list --assignee <you> --scope <account-id>` |
| GitHub **repo admin** on `OscarMackJr/nono` | set secrets/variables, run workflows | repo Settings is visible to you |
| Identity validation = **Approved** | required before a Public Trust profile can sign | Azure portal → Trusted Signing account → Identity validations |

If you are missing the Azure RBAC-admin right, you can still do everything except the
`az role assignment create` in §3 — hand that one command to whoever holds Owner.

---

## 2. Azure: create the Public Trust certificate profile + record 3 values

Identity validation is approved, so you can now create the **Public Trust** profile (this is the
publicly-trusted one; "Test" = a Microsoft test root that is NOT publicly trusted — do not use it
for releases).

```bash
# Variables — fill these in from your account
RG=rg-nono-signing
ACCOUNT=nono-trusted-signing          # your Trusted Signing account name
LOCATION=eastus

# Create the Public Trust certificate profile (portal: account → Certificate profiles → Create)
az trustedsigning certificate-profile create \
  -g "$RG" \
  --account-name "$ACCOUNT" \
  -n nono-public \
  --profile-type PublicTrust \
  --identity-validation-id <APPROVED_VALIDATION_ID>
# (CLI extension: `az extension add -n trustedsigning`. If the verb differs in your CLI version,
#  do it in the portal: Certificate profiles → Create → type "Public Trust".)
```

**Record these three values — you will paste them into GitHub in §4:**

| Value | Example | Where to find it |
|-------|---------|------------------|
| **Endpoint** (region-prefixed) | `https://eus.codesigning.azure.net` | account Overview blade |
| **Account name** | `nono-trusted-signing` | the `$ACCOUNT` you created |
| **Certificate profile name** | `nono-public` | the profile from above |

---

## 3. Azure: CI identity (OIDC app + federated credential + signer role)

If the `nono-ci-trusted-signing` app already exists from the original setup, **skip to 3b** to fix
the federated-credential subject. If it does not exist, do 3a first.

### 3a. App registration + service principal (only if not already created)

```bash
az ad app create --display-name "nono-ci-trusted-signing"
APP_ID=$(az ad app list --display-name "nono-ci-trusted-signing" --query "[0].appId" -o tsv)
az ad sp create --id "$APP_ID"
echo "AZURE_CLIENT_ID = $APP_ID"
```

### 3b. Federated credential — the corrected ENVIRONMENT subject

```bash
APP_ID=$(az ad app list --display-name "nono-ci-trusted-signing" --query "[0].appId" -o tsv)

az ad app federated-credential create --id "$APP_ID" --parameters '{
  "name": "github-nono-environment-development",
  "issuer": "https://token.actions.githubusercontent.com",
  "subject": "repo:OscarMackJr/nono:environment:Development",
  "audiences": ["api://AzureADTokenExchange"]
}'
```

> If an **old** federated credential with a `ref:refs/...` or `oscarmackjr-twg` subject exists,
> delete it to avoid confusion:
> `az ad app federated-credential list --id "$APP_ID" -o table`
> then `az ad app federated-credential delete --id "$APP_ID" --federated-credential-id <name>`.

### 3c. Grant the signer role (RBAC — NOT API permissions)

```bash
APP_ID=$(az ad app list --display-name "nono-ci-trusted-signing" --query "[0].appId" -o tsv)
ACCOUNT_ID=$(az resource show -g rg-nono-signing -n nono-trusted-signing \
  --resource-type Microsoft.CodeSigning/codeSigningAccounts --query id -o tsv)

az role assignment create \
  --assignee "$APP_ID" \
  --role "Trusted Signing Certificate Profile Signer" \
  --scope "$ACCOUNT_ID"
```

⚠ **Do NOT touch the app's "API permissions" blade.** Trusted Signing auth is an RBAC data action,
not a Graph/OAuth permission. The federated credential = *who the app is*; the role = *what it may
do*. No admin consent, no `User.Read` needed.

---

## 4. GitHub: secrets, variables, and the Development environment

Repo → **Settings → Secrets and variables → Actions**.

**Secrets** (Repository secrets tab) — IDs only, no private key ever:

| Secret | Value (how to get it) |
|--------|-----------------------|
| `AZURE_CLIENT_ID` | the `$APP_ID` from §3 |
| `AZURE_TENANT_ID` | `az account show --query tenantId -o tsv` |
| `AZURE_SUBSCRIPTION_ID` | `az account show --query id -o tsv` |

**Variables** (Variables tab) — from §2:

| Variable | Value |
|----------|-------|
| `TRUSTED_SIGNING_ENDPOINT` | `https://eus.codesigning.azure.net` (your region) |
| `TRUSTED_SIGNING_ACCOUNT` | `nono-trusted-signing` |
| `TRUSTED_SIGNING_PROFILE` | `nono-public` |

**Environment** — repo → Settings → Environments. A `Development` environment already exists
(confirmed). **Leave it WITHOUT required-reviewer / wait-timer protection rules** — `release.yml`
pins its whole build matrix to this environment, so a protection rule would pause every OS leg, not
just the Windows signing leg (see the comment at `release.yml:27-31`).

---

## 5. GATE 1 — Smoke test (non-destructive; do this BEFORE any release)

This is the cheapest, safest proof the whole chain is live. It compiles a throwaway exe, signs it
via Trusted Signing, and asserts Authenticode = `Valid`. **It publishes nothing.**

1. Repo → **Actions** → **"Trusted Signing Smoke Test"** → **Run workflow** (it is
   `workflow_dispatch`-only).
   - CLI equivalent: `gh workflow run trusted-signing-smoke.yml`
2. Watch the run. The final step prints:
   ```
   OK: Trusted Signing smoke test PASSED — the D-02 signing path is live.
   ```
3. In the "Verify the embedded signature" step, confirm:
   - `Status: Valid`
   - `Issuer:` chains to a **Microsoft ID Verified CS** root (publicly trusted), NOT the old POC
     root `319E507E…`.

**If this fails, STOP and fix it here** — do not cut a release until smoke is green. See
Troubleshooting below.

> **⚠ FIELD NOTE (first live run, 2026-06-30, run `28467925298`):** OIDC login + the Sign step
> **succeeded** (auth, RBAC role, and endpoint/account/profile vars all correct; signer =
> `CN=TWGGLOBAL.onmicrosoft.com`). But **Verify failed with `Status: UnknownError`**, issuer
> `CN=Microsoft Enterprise ID Verified Policy AOC CA 01`. Two things to settle before a release:
> 1. **Confirm the certificate profile is type _Public Trust_.** The publicly-trusted code-signing
>    chain is named `Microsoft ID Verified **CS** EOC/AOC CA NN`. An issuer reading
>    `…Enterprise ID Verified Policy AOC CA…` is *not* that naming — verify the profile type in
>    Azure and, if needed, create a Public Trust profile and update `TRUSTED_SIGNING_PROFILE`.
> 2. **`UnknownError` ≠ `UntrustedRoot`** — it usually means the chain could not be *built* on that
>    machine (new AOC intermediate/root absent on the `windows-latest` runner, or a revocation/CRL
>    fetch timed out). Re-verify on a fully-updated clean **Win11** host with
>    `signtool verify /pa /v <file>`; if it is `Valid` there, the signing is fine and only the
>    runner-side gate is stale.
>
> Note `release.yml:259` uses the **same** `Get-AuthenticodeSignature -ne 'Valid'` check (fail-closed),
> so a release tag will also abort at verify until this resolves to `Valid`.

---

## 6. GATE 2 — Cut the first trusted-signed release

The newest *published* artifact is still `v0.62.2`, which predates the migration and is POC-signed.
The first tag pushed after smoke-green produces trusted-signed binaries.

> 🔗 **Tie-in:** this is exactly the work scoped in **Phase 100** (v3.4 — version leapfrog to
> `0.66.1` + release-pipeline reconcile). Prefer cutting this release **as the Phase 100 release
> step** rather than a stray manual tag, so the version bump and the first trusted-signed release
> happen together.

When ready:

```bash
# from a clean checkout of the release commit
git tag -a v0.66.1 -m "v0.66.1 — first Azure Trusted Signing release"
git push origin v0.66.1
# (or: Actions → Release → Run workflow → input tag v0.66.1)
```

The `Release` workflow then automatically:
1. `azure/login` (OIDC, environment:Development)
2. Signs all top-level `.exe` in `target/<target>/release` — `nono.exe`, `nono-shell-broker.exe`,
   `nono-wfp-service.exe` (`release.yml:167`)
3. Builds the machine + user MSIs from those signed binaries
4. Signs both MSI wrappers (`release.yml:241`)
5. **Fail-closed verify** (`release.yml:259`): `Get-AuthenticodeSignature` must be `Valid` on
   `nono.exe`, the broker, and both MSIs — else the release aborts and uploads nothing.

A green Release run = the binaries and MSIs are published, trusted-signed.

---

## 7. GATE 3 — Clean-host UAT (the real acceptance test)

This is host-gated — it must be done on a **fresh Windows 11 box** (or a clean VM snapshot) that has
**never** had the POC cert imported.

1. Download the published machine MSI: `nono-v0.66.1-x86_64-pc-windows-msvc-machine.msi`.
2. Confirm the signature is trusted out-of-the-box:
   ```powershell
   (Get-AuthenticodeSignature .\nono-v0.66.1-x86_64-pc-windows-msvc-machine.msi).Status
   #   expect: Valid
   (Get-AuthenticodeSignature .\nono.exe).SignerCertificate.Issuer
   #   expect: a Microsoft ID Verified CS root — NOT CN=nono Test Signing
   ```
3. Install, then run the supervised path **with NO manual cert-trust step**:
   ```powershell
   nono run --profile claude-code -- <some command>
   ```
   **Pass = the broker spawns.** Previously (POC-signed) the D-32-12 broker self-trust gate
   (`verify_broker_authenticode`, full-chain `WinVerifyTrust`) refused to spawn the broker on a
   clean host. With trusted signing it spawns with no `LocalMachine\Root` import.

This is the acceptance criterion in `.planning/todos/pending/20260611-poc-cert-broker-clean-host.md`.

---

## 8. Close-out (after Gate 3 passes)

1. **Retire the POC secrets** (if still present): delete repo secrets `WINDOWS_SIGNING_CERT` /
   `WINDOWS_SIGNING_CERT_PASSWORD` so no self-signed fallback path can ever run.
2. **Delete the smoke workflow** — its own header says "Delete once the real release is green":
   `git rm .github/workflows/trusted-signing-smoke.yml`
3. **Fix the stale guide** — `docs/cli/development/windows-signing-guide.mdx` (~lines 244-251) still
   implies "update the `WINDOWS_SIGNING_CERT` PFX". Correct it to point at this cookbook. (That file
   is gitignored-but-tracked — edit needs `git add -f`.)
4. **Move the todo** `20260611-poc-cert-broker-clean-host.md` from `pending/` → `resolved/`, and
   update `STATE.md` (FUT-02 / DIST-SIGN-01 cleared).

---

## Troubleshooting

| Symptom | Likely cause | Fix |
|---------|--------------|-----|
| `azure/login` → `AADSTS700213` "no matching federated identity record" | FIC subject wrong | Must be **exactly** `repo:OscarMackJr/nono:environment:Development` (§3b). Check case + `environment:` not `ref:`. |
| `azure/login` succeeds, signing step → `403 / AuthorizationFailed` | SP missing the signer role, or role scoped wrong | Re-run §3c; scope must be the **account** resource ID, role name exactly `Trusted Signing Certificate Profile Signer`. |
| Signing → `profile not found` / `account not found` | Wrong `TRUSTED_SIGNING_*` variables | Re-check §4 against the §2 recorded values; endpoint is region-prefixed (`https://eus.…`). |
| Smoke `Status: NotSigned` after sign step | wrong endpoint region, or profile is "Test" not "Public Trust" | Confirm profile type = **Public Trust**; confirm endpoint region matches the account. |
| Sign succeeds but Verify `Status: UnknownError` (chain can't be built) | profile not Public Trust, OR runner lacks the new AOC/EOC intermediate/root, OR revocation check timed out | (1) Confirm profile type = **Public Trust** (issuer should read `Microsoft ID Verified **CS** EOC/AOC CA NN`); (2) re-verify on an updated clean Win11 host with `signtool verify /pa /v`; if Valid there, the signing is fine. Seen on first live run 2026-06-30. |
| Signature `Valid` but "Windows protected your PC" on first run | new `AOC CA` intermediate has no SmartScreen reputation yet | Expected for new profiles; reputation accrues over time/volume. Validity is unaffected. |
| Clean-host: MSI `Valid` but `nono run` won't spawn broker | broker `.exe` wasn't signed by the same profile | The pre-package step signs all top-level `.exe` (incl. broker) — confirm `nono-shell-broker.exe` was in `target/<target>/release` at sign time (`release.yml:159-163`). |
| Release run pauses on every OS matrix leg | protection rule added to `Development` environment | Remove required-reviewer / wait-timer from the `Development` environment (§4). |

---

## Quick reference card (fill in, keep handy)

```
Repo:                 OscarMackJr/nono   (PUBLIC)
FIC subject:          repo:OscarMackJr/nono:environment:Development
GH environment:       Development        (no protection rules)

Secrets:   AZURE_CLIENT_ID         = <app id>
           AZURE_TENANT_ID         = <tenant id>
           AZURE_SUBSCRIPTION_ID   = <subscription id>
Variables: TRUSTED_SIGNING_ENDPOINT = https://eus.codesigning.azure.net
           TRUSTED_SIGNING_ACCOUNT  = nono-trusted-signing
           TRUSTED_SIGNING_PROFILE  = nono-public
SP role:   Trusted Signing Certificate Profile Signer  (scope = account)

Gates:  1) Actions → "Trusted Signing Smoke Test"  (publishes nothing)
        2) push tag vX.Y.Z → Release workflow green
        3) clean Win11: MSI Valid + `nono run --profile claude-code` spawns broker, no cert import
```
