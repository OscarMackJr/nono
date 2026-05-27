---
quick_id: 260527-mts
slug: sign-poc-local-script
date: 2026-05-27
---

# Quick Task: Local POC signing helper + doc

## Description

Codify the local POC Authenticode signing flow into `scripts/sign-poc-local.ps1` (one
command: self-signed cert → sign EXEs → rebuild MSIs → sign MSIs → export public `.cer`)
and document it in the Windows signing guide + a pointer from the POC handoff.

## Why

nono's Windows broker-spawn gate (`verify_broker_authenticode`, D-32-12) fail-closes when an
installed `nono.exe` is unsigned. For a controlled internal POC (5 desktop-support admins),
the sanctioned cheap path is a SELF-SIGNED code-signing cert + per-machine trust-store import
— no commercial CA / DigiCert needed (DigiCert is only the free RFC-3161 timestamp server).
The flow is fiddly (EXE-then-MSI ordering matters because the MSI embeds the EXEs), so a
single idempotent script removes copy-paste error.

## Tasks

1. **`scripts/sign-poc-local.ps1`** — new helper.
   - Params: `-VersionTag` (default `v0.57.3`), `-Scope` (machine/user/both, default both),
     `-CertSubject` (default `CN=nono POC Signing`), `-TimestampUrl`
     (default `http://timestamp.digicert.com`), `-Thumbprint` (reuse existing cert),
     `-SkipMsiRebuild` (re-sign only), `-OutputDir` (default `dist/windows`).
   - `Set-StrictMode -Version Latest`, `$ErrorActionPreference = "Stop"`, `$repoRoot` via
     `Split-Path -Parent $PSScriptRoot`, fail-closed throws (CLAUDE.md "Fail Secure").
   - Discover newest `Windows Kits\10\bin\<ver>\x64\signtool.exe`; throw directive if absent.
   - Create self-signed `CodeSigningCert` (SHA256, DigitalSignature, +2y, `Cert:\CurrentUser\My`)
     unless `-Thumbprint` given; else locate-or-throw by thumbprint.
   - **Order:** sign `nono.exe` + `nono-shell-broker.exe` (+ `nono-wfp-service.exe` when it
     exists AND machine scope) → rebuild MSIs via `build-windows-msi.ps1` (release.yml arg
     shape) unless `-SkipMsiRebuild` → sign MSIs.
   - Export PUBLIC `.cer` (no private key) to `<OutputDir>\nono-poc-signing.cer`.
   - `Get-AuthenticodeSignature` status reporting (NON-fatal; no `/pa` hard gate — it
     false-fails for self-signed pre-import).
   - Header comment: POC-only, broadens machine trust surface, NOT external-distribution,
     reversible; `.pfx`/private key never leaves the build machine.
   - Print per-machine next-steps (import `.cer` to `LocalMachine\Root` + `\TrustedPublisher`,
     install signed machine MSI, verify `Valid`).

2. **`docs/cli/development/windows-signing-guide.mdx`** — add `## Local POC signing
   (self-signed, internal use)` (points at the script, restates EXE-then-MSI ordering,
   per-machine Root+TrustedPublisher import, reversibility, "not for external distribution");
   add a pointer from the "Runtime symptom" section.

3. **`docs/cli/development/windows-poc-handoff.mdx`** — short pointer from the Broker.exe
   verification section to the helper so a POC operator finds it.

## Verification

- `pwsh -NoProfile -Command "..."` parses `sign-poc-local.ps1` with no syntax errors
  (AST parse via `[System.Management.Automation.Language.Parser]::ParseFile`).
- `-WhatIf`-style dry sanity: invoking with `-SkipMsiRebuild` against the discovery/cert
  paths does not throw on argument validation (full run requires real signing, out of scope
  to execute here — POC operator runs it).
- Docs render: headings present, links resolve to `scripts/sign-poc-local.ps1`.

## Notes

- `docs/cli/development/` is gitignored-but-tracked → stage `.mdx` with `git add -f`
  (plain `git add` exits 1 and breaks `&& git commit` chains) — [[feedback_docs_cli_dev_gitignored]].
- PowerShell/docs only; no Rust cfg-gated code → cross-target clippy gate N/A.
- Bash tool intermittently throws MSYS `add_item` fork errors on this host → retry once.
