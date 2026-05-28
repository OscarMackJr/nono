---
phase: 53-release-drain
plan: "03"
subsystem: release-tooling
tags: [release, signing, ci, windows, github-actions, documentation]
dependency_graph:
  requires:
    - phase: 53-release-drain
      plan: "01"
      provides: workspace-at-0.57.4
  provides:
    - release.yml-semver-only-trigger
    - sign-poc-local-v0.57.4-default
    - windows-signing-guide-ca-ready-and-fresh-cert-sections
  affects: [53-04-wfp-uat, release-workflow-live-run]
tech_stack:
  added: []
  patterns: [semver-only tag glob (v*.*.*), fresh-cert-per-release discipline (D-53-02)]
key_files:
  created: []
  modified:
    - .github/workflows/release.yml
    - scripts/sign-poc-local.ps1
    - docs/cli/development/windows-signing-guide.mdx
key_decisions:
  - "Use v*.*.* glob (not v*) so milestone tags like v2.8 do not fire the release workflow (D-53-06)"
  - "Fresh self-signed cert per v2.x release; old cert trust does not carry over to new binaries (D-53-02)"
  - "CA swap is a secrets-only change — release.yml signing path is identical for self-signed and CA-issued certs (D-53-01)"
  - "Signing gate (WINDOWS_SIGNING_CERT fail-closed check, lines 124-139) preserved unchanged (D-53-04)"
requirements-completed:
  - REQ-RLS-01
  - REQ-RLS-02
duration: ~10 minutes
completed: "2026-05-28"
tasks_completed: 2
tasks_total: 3
---

# Phase 53 Plan 03: Release Trigger Fix + Signing Guide Expansion Summary

**Semver-only release.yml tag trigger (v*.*.*), updated sign-poc-local.ps1 default to v0.57.4, and windows-signing-guide.mdx expanded with CA-ready commercial path and 6-step fresh-cert generation + redistribution procedure; operator checkpoint pending for GitHub secret injection and main push.**

## Performance

- **Duration:** ~10 minutes
- **Started:** 2026-05-28T00:00:00Z
- **Completed:** 2026-05-28
- **Tasks:** 2 of 3 (Task 3 is a blocking human-verify checkpoint — awaiting operator)
- **Files modified:** 3

## Accomplishments

- Fixed `release.yml` `on.push.tags` from `'v*'` to `'v*.*.*'` — `v0.57.4` fires CI; `v2.8` (two-segment milestone tag) does not (D-53-06)
- Updated `sign-poc-local.ps1` `$VersionTag` default from `v0.57.3` to `v0.57.4`
- Added "Production signing path (CA-ready)" section to `windows-signing-guide.mdx` documenting commercial CA / Azure Trusted Signing as a secrets-only swap (D-53-01)
- Added "Fresh CI cert: generation, encoding, and trust redistribution" section with complete 6-step procedure including D-53-02 rationale, trust-store import commands, and dual-tag (`v0.57.4` + `v2.8`) cutting sequence (D-53-02)

## Task Commits

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Fix release.yml trigger glob and update sign-poc-local.ps1 VersionTag default | d3e875d4 | .github/workflows/release.yml, scripts/sign-poc-local.ps1 |
| 2 | Expand windows-signing-guide.mdx with CA-ready path, fresh-cert procedure, and operator instructions | eceac886 | docs/cli/development/windows-signing-guide.mdx |
| 3 | Operator prep checkpoint — generate fresh cert, set secrets, push main | PENDING | (human action — see below) |

## Files Created/Modified

- `.github/workflows/release.yml` — `on.push.tags` changed from `'v*'` to `'v*.*.*'`; signing gate (lines 124-139) untouched
- `scripts/sign-poc-local.ps1` — `$VersionTag` default changed from `v0.57.3` to `v0.57.4`
- `docs/cli/development/windows-signing-guide.mdx` — two new sections appended (staged with `git add -f`); all existing content preserved

## Decisions Made

- Use `v*.*.*` glob: matches three-component semver tags (`v0.57.4`) but not two-component milestone tags (`v2.8`), eliminating wasted CI runs on milestone markers (D-53-06).
- Fresh cert per release (D-53-02): each v2.x release uses a new self-signed cert; the new cert's public `.cer` must be redistributed to every trusted machine. Old cert trust does not carry over automatically.
- CA-ready path is secrets-only (D-53-01): the `release.yml` signing path (signing gate → `sign-windows-artifacts.ps1`) works identically for self-signed and CA-issued certs — only `WINDOWS_SIGNING_CERT` + `WINDOWS_SIGNING_CERT_PASSWORD` secrets need updating.

## Deviations from Plan

None — plan executed exactly as written. All three acceptance criteria for Tasks 1 and 2 met.

## Known Stubs

None.

## Threat Flags

None — this plan performs text-only edits to YAML, PowerShell, and MDX files. No new network endpoints, auth paths, file access patterns, or schema changes introduced. Signing gate (T-53-03-02 mitigant, lines 124-139) confirmed unchanged.

## Operator Checkpoint Pending (Task 3)

Task 3 is a `checkpoint:human-verify gate=blocking` that requires:

1. Generate a fresh self-signed CI cert on an elevated PowerShell session (see `windows-signing-guide.mdx` §"Fresh CI cert: generation, encoding, and trust redistribution")
2. Export the PFX, base64-encode it, and set `WINDOWS_SIGNING_CERT` and `WINDOWS_SIGNING_CERT_PASSWORD` as GitHub repository secrets at https://github.com/always-further/nono/settings/secrets/actions
3. Export the public `.cer` and import it on all trusted machines (`Cert:\LocalMachine\Root` + `Cert:\LocalMachine\TrustedPublisher`)
4. Push local `main` to origin: `git push origin main` (local main is ~34 commits ahead of origin after Phase 53 plans 01/02/03)

The cert thumbprint generated during operator execution should be noted for future reference.

**Resume signal:** "secrets set and main pushed"

## Self-Check: PASSED

- [x] `.github/workflows/release.yml` — confirmed `on.push.tags` contains `'v*.*.*'`, bare `'v*'` gone; signing gate lines 124-139 intact with `WINDOWS_SIGNING_CERT` references unchanged
- [x] `scripts/sign-poc-local.ps1` — confirmed `$VersionTag = "v0.57.4"` at line 79
- [x] `docs/cli/development/windows-signing-guide.mdx` — confirmed 2 new section headings, 11+ `WINDOWS_SIGNING_CERT` / trust-store references, 13+ `v0.57.4` references; staged with `git add -f`
- [x] Task 1 commit `d3e875d4` exists with DCO sign-off
- [x] Task 2 commit `eceac886` exists with DCO sign-off
- [x] No unexpected file deletions in either commit
