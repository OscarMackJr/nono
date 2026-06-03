# Phase 61 Pre-Tag Readiness Record — v0.58.0 / v2.9

**Date:** 2026-06-03
**Plan:** 61-03
**Scope:** Go/no-go gate for Wave 3 tag plan (61-04); all four checks must be green before
pushing the v0.58.0 tag that triggers release.yml.

---

## D-08: Drain-Fix Ancestry (Untagged v2.7 Fixes Present in Release Tree)

Verified via `git merge-base --is-ancestor <sha> HEAD` from `/c/Users/OMack/Nono` on branch
`main` (confirmed CWD + branch before checks). All four commits are confirmed ancestors of
HEAD (`c4c92cc2`).

| SHA | Description | Ancestry |
|-----|-------------|----------|
| `d8b7ce00` | broker `CreateProcessAsUserW` GLE=87 — HANDLE_LIST dedup fix | ANCESTOR (ok) |
| `005b4c9e` | no-PTY relay stdout-echo — child stdout was swallowed, never echoed to console | ANCESTOR (ok) |
| `0cbeb3be` | WFP service-stop fix | ANCESTOR (ok) |
| `b852826b` | WFP MSI-uninstall custom action fix | ANCESTOR (ok) |

**Result: D-08 GREEN** — all 4 drain-fix commits are present in the release tree.

---

## D-07: v0.57.4 Superseded Release Absent (Distribution Hazard Cleared)

The v0.57.4 release shipped unsigned-payload MSIs (wrapper was Authenticode-signed but the
payload binaries were not signed before WiX harvest — a signing-ORDER defect fixed in Phase 53).

Verification commands:

```
gh release view v0.57.4  →  "release not found" (exit non-0)
gh release list --limit 5  →  v0.57.5  Latest  2026-05-29T13:23:30Z
```

**Finding:** v0.57.4 is ABSENT. Only v0.57.5 is present as the Latest public release.

**Result: D-07 GREEN** — no superseded unsigned-payload release is exposed to the public.

Action taken: none (delete-if-found did not apply; release was already absent).

---

## D-02: Signing Material Pre-Flight (CI Signing Secrets + Fail-Closed Guard)

### Fail-Closed Guard Confirmation

release.yml contains a "Check signing secrets (Windows)" step at line 124 that explicitly
fails the build if either secret is empty:

```yaml
- name: Check signing secrets (Windows)
    if ([string]::IsNullOrWhiteSpace($env:WINDOWS_SIGNING_CERT)) {
      Write-Error "WINDOWS_SIGNING_CERT and WINDOWS_SIGNING_CERT_PASSWORD must be set..."
    if ([string]::IsNullOrWhiteSpace($env:WINDOWS_SIGNING_CERT_PASSWORD)) {
      Write-Error "WINDOWS_SIGNING_CERT and WINDOWS_SIGNING_CERT_PASSWORD must be set..."
```

This means: if the signing secrets are absent at tag-push time, the release.yml pipeline
fails loudly and explicitly — it does NOT silently fall back to the self-signed POC cert.
Falling back to the POC cert for a public release is FORBIDDEN.

`grep -c "WINDOWS_SIGNING_CERT" .github/workflows/release.yml` = 10 references
(guard + 3 sign steps for binary, machine-MSI, user-MSI).

### Secret Presence Corroboration

`gh secret list -R oscarmackjr-twg/nono` confirms both secret NAMES are configured:

```
WINDOWS_SIGNING_CERT          2026-05-28T21:36:49Z
WINDOWS_SIGNING_CERT_PASSWORD 2026-05-28T21:36:58Z
```

NOTE: The CLI cannot read secret VALUES. The operator MUST confirm at/before the v0.58.0 tag
push that both secrets still contain valid signing material (the production cert, not the
POC cert `319E507E...`). A missing or expired cert will be caught by the fail-closed guard
above — but catching it during CI is preferable to discovering it post-tag.

**Result: D-02 GREEN** — fail-closed guard confirmed present; secret names confirmed
present; operator sign-off required on secret VALUES before the tag push.

---

## nono-wfp-driver.sys: Package Step Gate

release.yml Package step (line 193) fails closed if the checked-in pre-signed driver is absent:

```powershell
$driver = Join-Path $PWD "crates\nono-cli\data\windows\nono-wfp-driver.sys"
if (-not (Test-Path $driver)) {
    Write-Error "Missing WFP driver binary (checked-in pre-signed copy): $driver"
```

Verification: `test -f crates/nono-cli/data/windows/nono-wfp-driver.sys` → PRESENT

**Result: DRIVER-SYS GREEN** — checked-in driver placeholder is present; release.yml
Package step will not fail closed on this gate.

---

## Summary Checklist

| Check | Decision Ref | Status |
|-------|-------------|--------|
| D-08: All 4 drain-fix commits are ancestors of HEAD | D-08 | GREEN |
| D-07: v0.57.4 (unsigned-payload) release absent | D-07 | GREEN |
| D-02: release.yml fail-closed signing guard present | D-02 | GREEN |
| D-02: Signing secret NAMES confirmed in repo secrets | D-02 | GREEN (names only) |
| nono-wfp-driver.sys present in tree | — | GREEN |

**Operator action required before tag push:**
- Confirm secret VALUES (WINDOWS_SIGNING_CERT + WINDOWS_SIGNING_CERT_PASSWORD) contain
  valid production signing material, NOT the POC cert (`319E507E...`).

---

**READY TO TAG: YES** (all automated checks green; operator secret-value confirmation pending)

The Wave 3 plan (61-04) may proceed with the v0.58.0 tag push once the operator has
confirmed signing secret values are production-valid.
