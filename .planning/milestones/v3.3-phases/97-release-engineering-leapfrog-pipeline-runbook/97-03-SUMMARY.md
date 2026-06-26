---
phase: 97-release-engineering-leapfrog-pipeline-runbook
plan: "03"
subsystem: release-engineering
tags: [dry-run, crates-io, pypi, npm, publish-validation, release]
dependency_graph:
  requires: [97-01]
  provides: [release-dry-run-orchestrator, packaging-validation-findings]
  affects: [scripts/release-dry-run.ps1]
tech_stack:
  added: []
  patterns: [dry-run-validation, pre-publish-verification, fail-closed-orchestrator]
key_files:
  created:
    - scripts/release-dry-run.ps1
  modified: []
decisions:
  - "PRE_PUBLISH_REGISTRY_BLOCKED treated as expected pre-publish state (not hard failure): cargo publish --dry-run resolves all deps from the live crates.io index; downstream workspace crates fail until nono 0.66.0 is published; nono is the always-runnable core"
  - "maturin build failure in nono-py reported as hard FAIL: real compilation error (missing endpoint_policy field in RouteConfig initializers), not toolchain absence; nono-py is read-only for this plan"
  - "npm publish --dry-run: PASS — index.js + index.d.ts present in tarball manifest"
metrics:
  duration: "~35 minutes"
  completed: "2026-06-26T19:30:00Z"
  tasks_completed: 2
  files_modified: 1
---

# Phase 97 Plan 03: Dry-Run Publish Validation Summary

**One-liner:** Single fail-closed orchestrator validates all three publish paths at 0.66.0 — reveals two pre-release blockers: downstream crates need nono published first (expected), and nono-py has a stale RouteConfig struct missing the `endpoint_policy` field (real bug).

## What Was Built

**Task 1** (`70742aa0`): `scripts/release-dry-run.ps1` — crates.io section. Runs `cargo publish --dry-run` for all four publishable crates (nono, nono-proxy, nono-shell-broker, nono-cli) in dependency order. Detects `PRE_PUBLISH_REGISTRY_BLOCKED` when the error pattern "failed to select a version for the requirement" appears (downstream crates failing because nono 0.66.0 is not yet on crates.io). nono base crate: PASS. Exits 0 when only PRE_PUBLISH_REGISTRY_BLOCKED statuses are present; exits 1 on any hard packaging error.

**Task 2** (`3d61d931`): Extended with PyPI (maturin build + twine check) and npm (npm publish --dry-run) legs. PyPI leg: maturin is installed on the host; twine is absent (`SKIP_HOST_UNAVAILABLE`). However, `maturin build` itself exits 1 due to a real compilation error in nono-py (see Findings). npm leg: `npm publish --dry-run` exits 0 and confirms index.js + index.d.ts are in the tarball manifest (PASS). Safety greps: 0 `twine upload` occurrences, all `npm publish` calls use `--dry-run`, 0 registry credentials.

## Dry-Run Transcript

```
─ crates.io dry-run ────────────────────────────────────────────────────
  cargo publish --dry-run -p nono              OK
  cargo publish --dry-run -p nono-proxy        PRE_PUBLISH_REGISTRY_BLOCKED
  cargo publish --dry-run -p nono-shell-broker PRE_PUBLISH_REGISTRY_BLOCKED
  cargo publish --dry-run -p nono-cli          PRE_PUBLISH_REGISTRY_BLOCKED

─ PyPI dry-run ─────────────────────────────────────────────────────────
  maturin build                                FAILED (exit 1)
    error[E0063]: missing field `endpoint_policy` in initializer of
      nono_proxy::config::RouteConfig
    --> nono-py/src/policy.rs:743:9
    --> nono-py/src/proxy.rs:206:20

─ npm dry-run ──────────────────────────────────────────────────────────
  npm publish --dry-run                        OK (index.js + index.d.ts present)

─ Results ──────────────────────────────────────────────────────────────
  PASS    crates.nono
  BLOCKED crates.nono-proxy        (nono ^0.66.0 not yet on crates.io)
  BLOCKED crates.nono-shell-broker (nono ^0.66.0 not yet on crates.io)
  BLOCKED crates.nono-cli          (nono ^0.66.0 not yet on crates.io)
  FAIL    pypi.maturin_build       (maturin build exited 1)
  SKIP    pypi.twine_check         (twine absent: SKIP_HOST_UNAVAILABLE)
  PASS    npm.dry_run

FAIL: 1 dry-run check(s) failed.
```

Script exit code: 1 (fail-closed on the maturin hard failure).

## Deviations from Plan

### Auto-detected Issues (findings only — nono-py is read-only for this plan)

**1. [Rule 1 - Bug] nono-py RouteConfig missing endpoint_policy field**
- **Found during:** Task 2 (maturin build)
- **Issue:** `nono_proxy::config::RouteConfig` gained the `endpoint_policy: Option<EndpointPolicyConfig>` field in phase 95 absorb (CR-01 endpoint deny/approve policy). Two nono-py struct initializers were not updated:
  - `nono-py/src/policy.rs:743` — `From<PolicyRouteConfig> for RustRouteConfig` impl
  - `nono-py/src/proxy.rs:206` — `RouteConfig::new` constructor
- **Fix (operator action required):** Add `endpoint_policy: None,` to both initializers in nono-py before the actual release. Since nono-py's `Cargo.toml` has `publish = false`, this is only needed for the binding wheel — not for crates.io publishing.
- **Files to fix:** `C:\Users\OMack\nono-py\src\policy.rs:743`, `C:\Users\OMack\nono-py\src\proxy.rs:206`
- **Commit:** Not committed (nono-py is read-only for this plan)

### Plan Assumption Correction

**2. [Discovery] cargo publish --dry-run registry resolution for downstream workspace crates**
- **Issue:** The plan assumed `cargo publish --dry-run -p nono-proxy/shell-broker/cli` would exit 0 before nono 0.66.0 is on crates.io. This is incorrect: `cargo publish --dry-run` resolves all dependencies against the live crates.io index at packaging time. Since nono 0.66.0 has not been published, downstream crates get "failed to select a version for the requirement `nono = ^0.66.0`" and exit 101.
- **This is expected pre-publish behavior**, not a packaging error. The script surfaces it as `PRE_PUBLISH_REGISTRY_BLOCKED` (not a hard failure) and exits 0 when no other errors are present.
- **Re-run after publishing nono** to get PASS for all four crates.
- **Corroborates plan 97-02:** confirms the 3-crate publish set analysis; nono-cli's failure is on `nono` (the direct dep), not `nono-shell-broker` (the dev-dep) — consistent with the dev-dep not being resolved at publish-verify.

### Toolchain SKIPs

**3. twine: SKIP_HOST_UNAVAILABLE**
- `twine` is not installed on this host (neither as CLI nor as `python -m twine`).
- This SKIP is moot since `maturin build` itself failed; twine check is downstream of build.

## Known Stubs

None in the orchestrator script itself. The nono-py compilation failure blocks the PyPI path — tracked as operator-action item above, not a script stub.

## Pre-Release Blockers Surfaced

| Blocker | File | Fix Required |
|---------|------|-------------|
| nono-py RouteConfig missing endpoint_policy | nono-py/src/policy.rs:743, nono-py/src/proxy.rs:206 | Add `endpoint_policy: None,` to both initializers |
| twine absent | host | `pip install twine` or `uv add twine` in nono-py dev env |

## Threat Flags

None — this plan creates only a dry-run validation script. No new network endpoints, auth paths, file access patterns, or schema changes introduced. The script itself enforces no-upload invariants by construction.

## Commits

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | crates.io dry-run orchestrator | 70742aa0 | scripts/release-dry-run.ps1 |
| 2 | PyPI + npm dry-run legs wired | 3d61d931 | scripts/release-dry-run.ps1 |

## Self-Check: PASSED

- [x] `scripts/release-dry-run.ps1` exists ✓
- [x] Commit `70742aa0` exists in git log ✓
- [x] Commit `3d61d931` exists in git log ✓
- [x] `grep -c token scripts/release-dry-run.ps1` → 0 ✓
- [x] `grep -c 'twine upload' scripts/release-dry-run.ps1` → 0 ✓
- [x] All `npm publish` in script use `--dry-run` ✓
- [x] `cargo publish --dry-run -p nono` exits 0 (NONO_DRYRUN_OK confirmed) ✓
- [x] Script exits 1 on maturin build failure (fail-closed) ✓
