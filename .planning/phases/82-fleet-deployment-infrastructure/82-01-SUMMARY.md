---
phase: 82-fleet-deployment-infrastructure
plan: "01"
subsystem: msi-fleet-deploy
tags: [msi, wix, admx, gpo, cert-trust, fleet-deploy, windows]
dependency_graph:
  requires: []
  provides:
    - dist/windows/nono-poc-signing.pem
    - dist/windows/nono.admx
    - dist/windows/nono.adml
    - machine WXS: ProgramData root + sentinel key + DER+PEM cert + cert CA + nono CLI Event Log source
  affects:
    - scripts/build-windows-msi.ps1
    - scripts/validate-windows-msi-contract.ps1
    - .planning/REQUIREMENTS.md
tech_stack:
  added:
    - nono.admx / nono.adml (GPO ADMX template, dist/windows/)
    - nono-poc-signing.pem (PEM copy of POC root cert for Node NODE_EXTRA_CA_CERTS, dist/windows/)
  patterns:
    - WiX here-string machine-only component block (gated by $Scope -eq "machine")
    - Deferred SYSTEM CustomAction pattern: Directory=INSTALLFOLDER, Execute=deferred, Impersonate=no, Return=ignore
    - PowerShell EmitOnly+[xml] contract validator assertion idiom (extended)
key_files:
  created:
    - dist/windows/nono-poc-signing.pem
    - dist/windows/nono.admx
    - dist/windows/nono.adml
  modified:
    - scripts/build-windows-msi.ps1
    - scripts/validate-windows-msi-contract.ps1
    - .planning/REQUIREMENTS.md
decisions:
  - "D-Discretion: nono CLI Event Log source registered in Plan 01 (not deferred to Plan 04 / Phase 84) to eliminate an extra install step and de-risk Phase 84 SecurityEventLayer; mirrors existing cmpEventLogSource pattern"
  - "PEM mechanism: certutil -encode used at build time (committed nono-poc-signing.pem alongside the DER .cer); a committed .pem is preferred so builds are reproducible without relying on certutil availability"
  - "XML comment sanitization: removed '--' from XML comment text in here-strings (XML comments cannot contain '--'); ExeCommand attribute value 'setup --trust-root' is an attribute value (not a comment) so it is valid"
  - "DEPLOY-04 traceability split: row updated to Phase 82 (template) / Phase 83 (reader) to prevent milestone audit treating the ADMX as a Phase-82 gap or Phase-83 duplicate"
metrics:
  duration_minutes: 25
  completed: "2026-06-18T19:16:45Z"
  tasks_completed: 3
  files_changed: 6
---

# Phase 82 Plan 01: Machine-Global MSI Fleet Deployment Infrastructure Summary

**One-liner:** Machine MSI extended with ProgramData root + sentinel key + non-fatal cert CA (nono.exe setup --trust-root, both stores via Rust) + DER+PEM cert staging + nono CLI Event Log source; GPO ADMX/ADML template shipping separate AllowedSuffixes/AllowedHosts policies; contract validator extended with full machine-only assertions and static-CRT flag check.

## Tasks Completed

| Task | Name | Commit | Key Files |
|------|------|--------|-----------|
| 1 | Add machine-only MSI components (ProgramData, sentinel, DER+PEM cert, cert CA, Event Log) | `216a95ba` | scripts/build-windows-msi.ps1, dist/windows/nono-poc-signing.pem, dist/windows/nono-machine.wxs |
| 2 | Extend MSI contract validator; verify static-CRT flag | `8b8e0d30` | scripts/validate-windows-msi-contract.ps1, scripts/build-windows-msi.ps1 (XML comment fix) |
| 3 | Ship ADMX/ADML template; fix DEPLOY-04 traceability | `cff0b39b` | dist/windows/nono.admx, dist/windows/nono.adml, .planning/REQUIREMENTS.md |

## What Was Built

### Task 1: Machine-only MSI components (`scripts/build-windows-msi.ps1`)

**New machine-scope variables** (`$machineOnlyComponentsXml`, `$certImportCustomActionXml`) initialized unconditionally for all machine-scope MSI builds (not gated on `$serviceBinaryFullPath`).

**New components in machine WXS:**
- `cmpProgramDataNono`: Creates `C:\ProgramData\nono\` via `CommonAppDataFolder`/`PROGRAMDATANONO` standard directory. Per Pitfall 4 / D-08: NEVER creates `%LOCALAPPDATA%` user scratch (SYSTEM-context install would write to `C:\Windows\system32\config\systemprofile\...` and fail every user R-B3 guard).
- `cmpPocCertDer`: Stages DER-format `nono-poc-root.cer` under `INSTALLFOLDER` for the cert CA `certutil -addstore` import path.
- `cmpPocCertPem`: Stages PEM-format `nono-poc-root.pem` under `PROGRAMDATANONO` (`%PROGRAMDATA%\nono\nono-poc-root.pem`) for Node's `NODE_EXTRA_CA_CERTS` (Pitfall 13 / D-05: Node cannot read DER `.cer`).
- `cmpPolicySentinel`: Creates `HKLM\SOFTWARE\Policies\nono` with `InstalledByMsi=1` placeholder for Phase 83 reader + `nono health` presence probe.
- `cmpNonoCliEventLogSource`: Registers `SYSTEM\CurrentControlSet\Services\EventLog\Application\nono` (EventMessageFile + TypesSupported=7) for Phase 84 SecurityEventLayer (Claude's Discretion: registered now to de-risk Phase 84).

**Cert-import CustomAction** `CaImportTrustRoot`:
- `Directory="INSTALLFOLDER"`, `Execute="deferred"`, `Impersonate="no"`, `Return="ignore"` (non-fatal, per D-04)
- `ExeCommand="nono.exe setup --trust-root nono-poc-root.cer"` — single Rust source of truth for Root+TrustedPublisher stores (T-82-01/02)
- Conditioned: `NOT (REMOVE="ALL") AND NOT UPGRADINGPRODUCTCODE` (fresh install only)

**PEM cert mechanism:** `certutil -encode dist/windows/nono-poc-signing.cer dist/windows/nono-poc-signing.pem` at build time; committed `nono-poc-signing.pem` alongside the DER `.cer` so builds are reproducible without certutil on PATH.

**Cert thumbprint (SHA-256):** `a9a95ac9c3b7a774bf5d6968a2c61577fa6f745ed820f951ba9351b0b8c18fff`
**Cert SHA-1:** `319e507e950472d490f56f7c4cd94437c013cc06`

**Installed PEM path:** `%PROGRAMDATA%\nono\nono-poc-root.pem` — Plan 02 must point `NODE_EXTRA_CA_CERTS` at this exact path.

**User MSI:** unaffected. All new components are machine-scope only; user `.wxs` verified to contain none of: `setup --trust-root` CA, Policies\nono key, CommonAppDataFolder, PEM file, nono CLI Event Log source.

### Task 2: Contract validator (`scripts/validate-windows-msi-contract.ps1`)

New assertions added (unconditional; not gated on service binary):
- **(a)** Cert CA `ExeCommand` contains `setup --trust-root`; `Execute=deferred`, `Impersonate=no`, `Return=ignore`
- **(b)** Machine doc contains `HKLM\SOFTWARE\Policies\nono` sentinel key
- **(c)** Machine doc contains `CommonAppDataFolder`; does NOT contain `LocalAppDataFolder` in machine block
- **(d)** Machine doc has both PEM `.pem` File component (Blocker-1 guard) AND DER `.cer` File component
- **(e)** Machine doc has nono CLI Event Log source (`EventLog\Application\nono`)
- **(f)** User doc contains NONE of the machine-only elements
- **Static-CRT flag:** asserts `.cargo/config.toml` contains `crt-static` under `[target.x86_64-pc-windows-msvc]` (D-01/D-02 verify-only; caveat: CI step-level RUSTFLAGS override silently drops this stanza)

Validator exits 0 against Task 1 generator output (verified).

**Static-CRT caveat (documented in `.cargo/config.toml` and in SUMMARY):**
The `[target.x86_64-pc-windows-msvc] rustflags` stanza is silently dropped when the `RUSTFLAGS` environment variable is set at the process level (a known Cargo rustflags source-precedence behavior). CI and `release.yml` use step-level `RUSTFLAGS` env overrides; the `.cargo/config.toml` flag covers local dev builds where `RUSTFLAGS` is not set. This is already documented in the config file itself.

### Task 3: GPO ADMX/ADML template

**`dist/windows/nono.admx`:**
- Valid `policyDefinitions` root with `policyNamespaces`, `resources`, `categories`
- `AllowedSuffixes` policy: `class="Machine"`, `key="SOFTWARE\Policies\nono"`, `REG_MULTI_SZ` list via `<list>` element for wildcard FQDN suffix entries (e.g., `.anthropic.com`)
- `AllowedHosts` policy: `class="Machine"`, `key="SOFTWARE\Policies\nono"`, `REG_MULTI_SZ` list for exact-match hostname entries (e.g., `api.github.com`)
- Pitfall 3 enforced at schema level: separate policies for suffix vs exact match; never one flat list
- Top-of-file comment: Intune OMA-URI (ADMXInstall, AllowedSuffixes, AllowedHosts), `KEY_WOW64_64KEY`/`[RegistryView]::Registry64` requirement (Pitfall 7), policy lifecycle note, AI-provider presets

**`dist/windows/nono.adml`:**
- `policyDefinitionResources` root; supplies all `displayName`, `explainText`, `presentation` IDs referenced by the ADMX
- AI-provider preset names documented in explainText: `.anthropic.com`, `.openai.com`, `.githubusercontent.com`, `api.github.com`, `models.inference.ai.azure.com`
- Pitfall 3 separation warnings in both policy descriptions

**`REQUIREMENTS.md` DEPLOY-04 traceability:**
Changed `| DEPLOY-04 | Phase 83 | Pending |` to `| DEPLOY-04 | Phase 82 (template) / Phase 83 (reader) | Pending |` to prevent milestone audit flagging DEPLOY-04 as a Phase-82 gap or Phase-83 duplicate. Only this single row was modified.

## Verification Results

All verification criteria from the plan verified:

- `pwsh -File scripts/build-windows-msi.ps1 -Scope machine -EmitOnly` regenerates machine `.wxs` containing: cert CA (`setup --trust-root`), DER+PEM File components, sentinel key, ProgramData root, nono CLI Event Log source. PASS.
- `pwsh -File scripts/validate-windows-msi-contract.ps1` exits 0. PASS.
- `.cargo/config.toml` carries `target-feature=+crt-static` for the windows-msvc target. PASS.
- `nono.admx` + `nono.adml` parse as valid XML, target `SOFTWARE\Policies\nono`, carry policyNamespaces/resources/categories + 2 Machine policies (AllowedSuffixes + AllowedHosts). PASS.
- `REQUIREMENTS.md` DEPLOY-04 traceability reads `Phase 82 (template) / Phase 83 (reader)`. PASS.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] XML comment `--` characters in WiX here-strings caused XML parse failure**
- **Found during:** Task 2 (validator run)
- **Issue:** PowerShell's `[xml]` parser rejected the generated `.wxs` because the here-string comments in `$machineOnlyComponentsXml` and `$certImportCustomActionXml` contained `--` (e.g., `nono.exe setup --trust-root) accepts DER` as comment text). XML 1.0 prohibits `--` inside `<!-- ... -->` comments.
- **Fix:** Rewrote all XML comment text in the here-string to avoid `--`. The `ExeCommand` attribute value `nono.exe setup --trust-root nono-poc-root.cer` is valid XML (attribute values can contain `--`); only the surrounding comments were affected.
- **Files modified:** `scripts/build-windows-msi.ps1`
- **Commit:** `8b8e0d30` (included in Task 2 commit)

### Claude's Discretion Applied

**Event Log source for nono CLI registered in Plan 01 (not deferred to Phase 84):**
The CONTEXT.md (lines 100-104) explicitly delegated this as Claude's Discretion. The nono CLI Event Log source is registered now because: (a) it mirrors the proven `cmpEventLogSource` pattern byte-for-byte with only Id/Key/Value changes; (b) registering now eliminates an extra Phase 84 install step; (c) the Phase 84 SecurityEventLayer must write to this source regardless of when it is registered. The validator enforces this with assertion (e). Decision recorded in SUMMARY frontmatter.

## Known Stubs

None. The machine MSI components create real registry keys and file components. The ADMX/ADML template is a valid, admin-usable template (not a placeholder). The `setup --trust-root` verb referenced in the CustomAction `ExeCommand` is authored in Plan 02 Task 1; the MSI emits the correct call but the Rust implementation is Plan 02's scope (deliberately deferred — the MSI runs `Return="ignore"` so a missing verb is non-fatal until Plan 02 ships).

## Threat Flags

No new unplanned threat surface introduced. All threat register items (T-82-01 through T-82-SC) were addressed as designed:
- T-82-01/02: cert CA uses relative `nono.exe` under `INSTALLFOLDER` (Admins-only writable); cert pinned in MSI cab
- T-82-03: PATH `Environment System=yes` + `[INSTALLFOLDER]` + `Part="last"` unchanged from shipped element
- T-82-04: `Return="ignore"` (cert CA) + `Vital="no"` (service); static-CRT eliminates 0xC0000135
- T-82-06: ADMX separate AllowedSuffixes/AllowedHosts policies (Pitfall 3 enforced)
- T-82-07: PEM ships as MSI `<File>` under `%PROGRAMDATA%\nono\` (Admins-only writable ACL)
- T-82-SC: No package-manager installs; certutil is in-box Windows tool

## Self-Check

Files created exist:
- `dist/windows/nono-poc-signing.pem`: EXISTS (committed `216a95ba`)
- `dist/windows/nono.admx`: EXISTS (committed `cff0b39b`)
- `dist/windows/nono.adml`: EXISTS (committed `cff0b39b`)

Commits exist:
- `216a95ba`: feat(82-01): add machine-global MSI components for fleet deployment
- `8b8e0d30`: feat(82-01): extend MSI contract validator with Phase 82 machine-only assertions
- `cff0b39b`: feat(82-01): ship GPO ADMX/ADML template for HKLM policy spine; fix DEPLOY-04 traceability

## Self-Check: PASSED
