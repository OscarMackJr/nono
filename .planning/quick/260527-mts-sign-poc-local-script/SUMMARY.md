---
quick_id: 260527-mts
slug: sign-poc-local-script
date: 2026-05-27
status: complete
---

# Summary: Local POC signing helper + doc

## What changed

- **`scripts/sign-poc-local.ps1`** (new) — one-command local POC Authenticode signing:
  discover `signtool.exe` → create/reuse self-signed `CodeSigningCert` → **sign EXEs** →
  **rebuild MSIs** (via `build-windows-msi.ps1`) → **sign MSIs** → export public `.cer`.
  Params: `-VersionTag` (default `v0.57.3`), `-Scope` (machine/user/both, default both),
  `-CertSubject`, `-TimestampUrl` (default free DigiCert RFC-3161), `-Thumbprint` (reuse),
  `-SkipMsiRebuild`, `-OutputDir` (default `dist/windows`). `Set-StrictMode -Version Latest`,
  `$ErrorActionPreference = "Stop"`, fail-closed throws throughout.
- **`docs/cli/development/windows-signing-guide.mdx`** — new `## Local POC signing
  (self-signed, internal use)` section + reworked the "Runtime symptom" closing paragraph to
  point at it (the old text flatly discouraged self-signed; now it distinguishes a single dev
  machine — prefer dev-layout — from a controlled internal POC — sanctioned, reversible).
- **`docs/cli/development/windows-poc-handoff.mdx`** — pointer from the Broker.exe verification
  section to the helper + the signing-guide section.

## Key decisions

- **EXE-then-MSI ordering enforced** — the MSI embeds copies of the EXEs and the broker gate
  (`verify_broker_authenticode`, D-32-12) checks the *installed* `nono.exe`, so EXEs must be
  signed before the MSI is (re)built. `-SkipMsiRebuild` re-signs only and warns.
- **No `signtool verify /pa` hard gate** — it false-fails for self-signed certs on the build
  machine (cert not yet in a trusted root). The script reports `Get-AuthenticodeSignature`
  status non-fatally and explains that non-`Valid` is expected pre-import.
- **Private key never leaves the build machine** — only the public `.cer` + signed MSI are
  distributed; each target machine imports the `.cer` to `LocalMachine\Root` +
  `\TrustedPublisher` (admin, one-time). Framed explicitly as POC-only / reversible / not
  external-distribution.
- **You do not need DigiCert as a CA** — documented; DigiCert appears only as the free
  timestamp server.

## Verification

- `[System.Management.Automation.Language.Parser]::ParseFile` on `sign-poc-local.ps1` → **PARSE OK**
  (no syntax errors).
- Full signing run NOT executed here (it creates a cert + signs real artifacts) — the POC
  operator runs `pwsh -File scripts/sign-poc-local.ps1`. Logic reviewed against
  `build-windows-msi.ps1`'s arg shape + scope-coherence guards (machine needs both
  service+driver or neither).
- Docs: new section heading + anchor `#local-poc-signing-self-signed-internal-use` referenced
  by both the in-page pointer and the handoff doc link.

## Notes / follow-ups

- `docs/cli/development/` is gitignored-but-tracked → staged the two `.mdx` with `git add -f`
  ([[feedback_docs_cli_dev_gitignored]]).
- PowerShell + docs only; no Rust cfg-gated code → cross-target clippy gate N/A.
- Not pushed (local `main`).
