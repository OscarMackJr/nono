---
phase: 97-release-engineering-leapfrog-pipeline-runbook
plan: "02"
subsystem: release-engineering
tags: [release, pipeline, msi, signing, homebrew, release.yml]
dependency_graph:
  requires: [97-01]
  provides: [msi-build-validated-0.66.0, release-yml-audited, signing-order-asserted, homebrew-url-fork-canonical]
  affects: [.github/workflows/release.yml, scripts/build-windows-msi.ps1]
tech_stack:
  added: []
  patterns: [sign-before-harvest, admin-extract-payload-gate, msi-contract-validation]
key_files:
  created: []
  modified:
    - .github/workflows/release.yml
    - scripts/build-windows-msi.ps1
decisions:
  - "3-crate publish set (nono, nono-proxy, nono-cli) is sufficient: nono-shell-broker appears ONLY under [target.'cfg(target_os = \"windows\")'.dev-dependencies] in crates/nono-cli/Cargo.toml — a version-pinned Windows dev-dep that cargo does not resolve during publish-verify and downstream consumers ignore entirely"
  - "Registry key paths updated from Software\\always-further\\nono to Software\\OscarMackJr\\nono — safe because MajorUpgrade handles the remove+reinstall cycle on upgrade; no registry migration needed"
  - "ARPURLINFOABOUT / ARPURLUPDATEINFO MSI properties corrected from always-further/nono to OscarMackJr/nono; these are user-facing ARP info URLs, not security surfaces"
metrics:
  duration: "~15 minutes"
  completed: "2026-06-26T18:30:00Z"
  tasks_completed: 2
  files_modified: 2
---

# Phase 97 Plan 02: Release Pipeline Audit and MSI Validation Summary

**One-liner:** Asserted the 0.66.0 release pipeline structural invariants — sign-before-harvest ordering, admin-extract payload gate, no reusable-workflow-call 0s-failure job, all five build legs, 3-crate publish set — and fixed the stale always-further/nono homebrew + MSI property URLs to the fork's canonical repo OscarMackJr/nono.

## What Was Built

- **Task 1:** Built 0.66.0 workspace release binaries (`cargo build --release --bin nono --bin nono-wfp-service`, `cargo build --release -p nono-shell-broker`). Drove `scripts/build-windows-msi.ps1` with `-VersionTag v0.66.0` for both machine and user scope into a scratch output dir, producing:
  - `nono-v0.66.0-x86_64-pc-windows-msvc-machine.msi` ✓
  - `nono-v0.66.0-x86_64-pc-windows-msvc-user.msi` ✓
  Both MSIs passed `scripts/validate-windows-msi-contract.ps1` (all assertions PASS). Confirmed in `release.yml` that the Phase 53 sign-before-harvest sequence is intact: "Sign Windows binaries (pre-package)" at line 167 precedes "Package (Windows)" at line 186, followed by "Sign Windows MSIs" and the admin-extract "Verify MSI payload signatures" gate at line 281 (fail-closed on non-Valid payload — the T-97-04 regression guard). Updated stale `always-further/nono` URLs in `scripts/build-windows-msi.ps1`:
  - `ARPURLINFOABOUT` / `ARPURLUPDATEINFO` MSI ARP properties → `https://github.com/OscarMackJr/nono`
  - Registry key paths `Software\always-further\nono\{machine,user}` → `Software\OscarMackJr\nono\{machine,user}`

- **Task 2:** Audited `release.yml` against all structural invariants. Fixed the stale `always-further/nono` download-url in the `update-homebrew-core` job to the fork's canonical release repo `OscarMackJr/nono` (T-97-07 mitigation). Verified YAML parses cleanly, all five build matrix legs are present, and no reusable-workflow-call `uses: ./.github/workflows/image-build.yml` job exists (the chronic 0s startup_failure cause — T-97-06 mitigation). Determined analytically from `crates/nono-cli/Cargo.toml` that the existing 3-crate publish set (nono, nono-proxy, nono-cli) is sufficient — `nono-shell-broker` is a Windows-only dev-dependency and does not force inclusion.

## Verification Results

- `grep -c 'always-further/nono' .github/workflows/release.yml` → **0** ✓
- `grep -c 'nolabs-ai/nono' .github/workflows/release.yml` → **0** ✓
- `grep -c 'OscarMackJr/nono/archive/refs/tags' .github/workflows/release.yml` → **1** ✓
- `grep -c 'target:' .github/workflows/release.yml` → **5** (all five build legs present) ✓
- No `uses: ./.github/workflows/image-build.yml` job (two matches are in the explanatory NOTE comment, not a job definition) ✓
- `pwsh -File scripts/validate-windows-msi-contract.ps1 ...` → exit 0 / PASS ✓
- `nono-v0.66.0-x86_64-pc-windows-msvc-machine.msi` produced ✓
- `nono-v0.66.0-x86_64-pc-windows-msvc-user.msi` produced ✓
- YAML valid (parsed by Python `yaml.safe_load`) ✓
- Sign-before-harvest order: line 167 (pre-package sign) < line 186 (Package) < line 281 (Verify payload) ✓

## Publish-Set Determination (manifest-derived, no 97-03 dependency)

`crates/nono-cli/Cargo.toml` declares `nono-shell-broker` under:
```toml
[target.'cfg(target_os = "windows")'.dev-dependencies]
nono-shell-broker = { path = "../nono-shell-broker", version = "0.66.0" }
```

This is a **dev-dependency** — cargo does not resolve dev-dependencies during a crate's publish-verify build, and downstream consumers ignore them entirely. `nono-shell-broker` therefore does **not** force `nono-cli` into a 4-crate publish set. The existing 3-crate set (`nono`, `nono-proxy`, `nono-cli`, published in dependency order with `sleep 30` gaps) is sufficient. This is decidable from the manifest alone.

Note: `nono-shell-broker` has no `publish = false` in its `Cargo.toml`, meaning it _could_ be published independently, but it is not required for the `nono-cli` publish to succeed.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 — Missing] Stale always-further/nono URLs in build-windows-msi.ps1**
- **Found during:** Task 1 read-first pass
- **Issue:** Three stale `always-further/nono` references in the WiX XML here-strings: `ARPURLINFOABOUT`, `ARPURLUPDATEINFO`, and two registry key paths
- **Fix:** Updated `ARPURLINFOABOUT`/`ARPURLUPDATEINFO` to `https://github.com/OscarMackJr/nono` and registry key paths to `Software\OscarMackJr\nono\{scope}`
- **Files modified:** `scripts/build-windows-msi.ps1`
- **Commit:** 0b7a5f73

## Commits

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Fix stale always-further/nono URLs in MSI build script | 0b7a5f73 | scripts/build-windows-msi.ps1 |
| 2 | Reconcile release.yml homebrew URL with fork repo | 98a06112 | .github/workflows/release.yml |

## Known Stubs

None.

## Threat Flags

None — changes are confined to URL/registry-key string updates and a one-line tarball download-url fix. No new network endpoints, auth paths, file access patterns, or schema changes introduced.

## Self-Check: PASSED

- [x] `scripts/build-windows-msi.ps1` no longer contains `always-further/nono` URLs ✓
- [x] `.github/workflows/release.yml` download-url points at `OscarMackJr/nono` ✓
- [x] `nono-v0.66.0-x86_64-pc-windows-msvc-machine.msi` was produced locally ✓
- [x] `nono-v0.66.0-x86_64-pc-windows-msvc-user.msi` was produced locally ✓
- [x] `validate-windows-msi-contract.ps1` exits 0 (PASS) ✓
- [x] Commit 0b7a5f73 exists in git log ✓
- [x] Commit 98a06112 exists in git log ✓
