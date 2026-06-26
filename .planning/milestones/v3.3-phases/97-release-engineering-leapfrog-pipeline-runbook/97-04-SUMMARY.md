---
phase: 97-release-engineering-leapfrog-pipeline-runbook
plan: "04"
subsystem: release-engineering
tags: [release, readiness-gate, verify-dark, runbook, prepare-only]
dependency_graph:
  requires: [97-01, 97-02, 97-03]
  provides: [release-readiness-gate, operator-push-runbook]
  affects:
    - scripts/gates/release-readiness.ps1
    - .planning/phases/97-release-engineering-leapfrog-pipeline-runbook/RELEASE-RUNBOOK.md
tech_stack:
  added: []
  patterns: [verify-dark-gate-contract, fail-closed-policy-assertions, prepare-only-runbook]
key_files:
  created:
    - scripts/gates/release-readiness.ps1
    - .planning/phases/97-release-engineering-leapfrog-pipeline-runbook/RELEASE-RUNBOOK.md
  modified: []
decisions:
  - "Policy violations (wrong version, staged private path) return FAIL verdict; infrastructure failures (git/cargo missing, JSON parse error) throw — distinction enforced per plan threat model T-97-11/12/13"
  - "git rev-parse --show-toplevel used for repo root in Invoke-Gate rather than $PSScriptRoot arithmetic — unambiguous when dot-source caller context shifts $PSScriptRoot to verify-dark.ps1 directory"
  - "Runbook documents 4-crate publish set (nono, nono-proxy, nono-shell-broker, nono-cli) not 3-crate — nono-shell-broker has no publish=false and is publishable independently even though it is a Windows dev-dep"
metrics:
  duration: "~3 minutes"
  completed: "2026-06-26T13:50:00Z"
  tasks_completed: 2
  files_modified: 2
---

# Phase 97 Plan 04: Release-Readiness Gate and Operator Runbook Summary

**One-liner:** Structurally enforced release-readiness gate (verify-dark auto-discovered, PASS on prepared tree) plus a checklist-gated operator runbook that makes the PUBLIC-repo safety invariants machine-verifiable before any push.

## What Was Built

**Task 1** (`0ea9454a`): `scripts/gates/release-readiness.ps1` — auto-discovered by
`scripts/verify-dark.ps1`. Exports `Test-Precondition` (returns `$null` — static check, any
Win11 host) and `Invoke-Gate` (asserts five release-readiness invariants). Policy violations
return a FAIL verdict with per-check detail; infrastructure failures throw (classified as
harness-internal errors by the runner). Never calls `exit`.

Five assertions in `Invoke-Gate`:

| Assertion | What | Notes |
|-----------|------|-------|
| (a) version-family | cargo metadata reports nono, nono-cli, nono-proxy, nono-shell-broker, nono-fltmgr-client, nono-ffi all at 0.66.0 | 6 crates |
| (b) no-stale-0.62.2 | No 0.62.2 in any tracked workspace Cargo.toml | git ls-files + content scan |
| (c) leapfrog | 0.66.0 > upstream highest 0.65.1 | [Version] comparison |
| (d) no-private-paths | No build_notes/ or .gsd/ in staging area or tracked set | git diff --cached + git ls-files |
| (e) cargo-lock | Cargo.lock contains 0.66.0 entries | regex scan |

Verified: PASS on prepared tree (exit 0); FAIL when `build_notes/` dummy staged (exit 2, then
unstaged and cleaned); verdict persisted to `.nono-runtime/verdicts/release-readiness.json`.

**Task 2** (`47c0cab7`): `RELEASE-RUNBOOK.md` — operator one-step push guide. Embeds the
verbatim PUBLIC-repo pre-push checklist (build_notes/.gsd check, leapfrog >= 0.66.0,
operator-push-is-sole-action), makes both scripts/release-dry-run.ps1 and
`-Gate release-readiness` mandatory Step 1/2 before any push, and documents:

- Full push sequence: `git push origin <branch> && git push origin v0.66.0` (tag triggers
  release.yml — 5-leg build matrix, sign-before-harvest, admin-extract payload gate)
- crates.io publish order: nono → nono-proxy → nono-shell-broker → nono-cli
- PyPI: `maturin publish` from `../nono-py` (blocker documented: RouteConfig struct fix needed)
- npm: `npm publish` from `../nono-ts`
- Known pre-release blockers table (nono-py RouteConfig missing endpoint_policy field)
- "PREPARE-ONLY" stated at top and bottom; no command executed by this plan

## Deviations from Plan

None — plan executed exactly as written.

## Known Stubs

None. The gate performs all five checks against live workspace state. The runbook is complete
documentation; the nono-py RouteConfig blocker is an operator-action item explicitly
documented in the runbook, not a stub in the artifacts.

## Threat Flags

None — no new network endpoints, auth paths, file access patterns, or schema changes
introduced. The gate and runbook are a PowerShell script and a markdown document respectively.

## Threat Model Mitigations Applied

| Threat | Mitigation | Where |
|--------|-----------|-------|
| T-97-11: build_notes/.gsd leaked into PUBLIC push | Gate asserts no private paths staged/tracked (FAIL otherwise); runbook pre-push checklist step 0 | scripts/gates/release-readiness.ps1 assertion (d); RELEASE-RUNBOOK.md |
| T-97-12: operator skips dry-run/readiness gate | Runbook makes both the mandatory first two steps before any push | RELEASE-RUNBOOK.md Steps 1+2 |
| T-97-13: accidental live publish from runbook | Runbook is documentation only; all commands are future operator actions; milestone is PREPARE-ONLY | RELEASE-RUNBOOK.md |

## Commits

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | release-readiness verify-dark gate | 0ea9454a | scripts/gates/release-readiness.ps1 |
| 2 | operator one-step-push runbook | 47c0cab7 | .planning/phases/97-release-engineering-leapfrog-pipeline-runbook/RELEASE-RUNBOOK.md |

## Self-Check: PASSED

- [x] `scripts/gates/release-readiness.ps1` exists ✓
- [x] `RELEASE-RUNBOOK.md` exists ✓
- [x] `.nono-runtime/verdicts/release-readiness.json` written by runner ✓
- [x] Commit `0ea9454a` exists in git log ✓
- [x] Commit `47c0cab7` exists in git log ✓
- [x] `pwsh -File scripts/verify-dark.ps1 -Gate release-readiness` exits 0 (PASS) on prepared tree ✓
- [x] Gate exits 2 (FAIL) when build_notes/ dummy staged, then unstaged (clean) ✓
- [x] `grep -c 'build_notes' RELEASE-RUNBOOK.md` = 3 (>= 1) ✓
- [x] `grep -c 'release-readiness' RELEASE-RUNBOOK.md` = 4 (>= 1) ✓
- [x] `grep -c '0.66.0' RELEASE-RUNBOOK.md` = 12 (>= 1) ✓
- [x] Gate file contains no `exit` statement (only `exit` in error message strings) ✓
- [x] Gate defines exactly `Test-Precondition` and `Invoke-Gate` ✓
