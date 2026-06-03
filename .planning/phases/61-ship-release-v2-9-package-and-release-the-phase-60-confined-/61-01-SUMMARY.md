---
phase: 61-ship-release-v2-9-package-and-release-the-phase-60-confined
plan: 01
subsystem: release
tags: [cargo, versioning, changelog, roadmap, requirements, workspace-bump]

requires:
  - phase: 60-confined-coding-loop
    provides: Phase 60 confined Write/Edit/MultiEdit code + 0.57.5 crate baseline
  - phase: 62-wfp-kernel-network-enforcement
    provides: Phase 62 out-of-box WFP enforcement code, UAT PASS 5/5

provides:
  - All 5 workspace crates and 6 internal path-dep pins at 0.58.0
  - Cargo.lock regenerated at 0.58.0 (no stale 0.57.5 pins)
  - CHANGELOG.md ## [0.58.0] v2.9 section (Phase 60 + Phase 62, honest POC framing)
  - ROADMAP.md Phase 61 list entry corrected to 0.58.0-off-main + Phase 62 WFP framing
  - REQ-RLS-03 and REQ-RLS-04 verified present in REQUIREMENTS.md (no reconciliation needed)

affects: [61-02-tag-and-release, any subsequent cargo publish steps]

tech-stack:
  added: []
  patterns:
    - "Lockstep workspace version bump: all 5 crates + all 6 path-dep pins bumped together"

key-files:
  created: []
  modified:
    - crates/nono/Cargo.toml
    - crates/nono-cli/Cargo.toml
    - crates/nono-proxy/Cargo.toml
    - crates/nono-shell-broker/Cargo.toml
    - bindings/c/Cargo.toml
    - Cargo.lock
    - CHANGELOG.md
    - .planning/ROADMAP.md

key-decisions:
  - "tools/sign-fixture excluded from bump (independent 0.1.0, no nono path-dep, publish=false)"
  - "Root Cargo.toml not touched (no [workspace.package].version field exists)"
  - ".wxs files not touched (MSI ProductVersion derived from git tag by build-windows-msi.ps1)"
  - "REQUIREMENTS.md no-change: REQ-RLS-03 and REQ-RLS-04 already registered and mapped to Phase 61"
  - "Cross-target Unix clippy deferred to CI (Windows host cannot run --target x86_64-unknown-linux-gnu)"

patterns-established:
  - "Version bump gated by cargo build --workspace + grep -rn version = old zero-match check"

requirements-completed: [REQ-RLS-03]

duration: 12min
completed: 2026-06-03
---

# Phase 61 Plan 01: Version Bump + CHANGELOG + ROADMAP Fix Summary

**Lockstep 0.57.5 -> 0.58.0 workspace bump across 5 crates + 6 path-dep pins, v2.9 CHANGELOG section, and ROADMAP D-05 stale-wording fix**

## Performance

- **Duration:** ~12 min
- **Started:** 2026-06-03T~14:45Z
- **Completed:** 2026-06-03
- **Tasks:** 2
- **Files modified:** 8

## Accomplishments

- All 5 crate Cargo.toml version literals and all 6 internal path-dep version pins bumped from 0.57.5 to 0.58.0 in lockstep
- `cargo build --workspace` passes cleanly at 0.58.0 (all crates compile: nono, nono-cli, nono-ffi, nono-proxy, nono-shell-broker); Cargo.lock regenerated
- `grep -rn 'version = "0.57.5"' crates/ bindings/` returns zero matches — no stale pins survive
- CHANGELOG.md `## [0.58.0] - v2.9` section added above the [Unreleased] entry, describing Phase 60 confined coding-loop POC and Phase 62 out-of-box WFP kernel network enforcement with honest POC/defense-in-depth framing
- ROADMAP.md Phase 61 list entry corrected from the stale 0.57.5/Phase-60-only wording (D-05) to the 0.58.0-off-main + dual-tag + Phase 62 WFP framing
- REQ-RLS-03 and REQ-RLS-04 verified present in REQUIREMENTS.md with Phase 61 Pending mappings (no reconciliation needed)

## Task Commits

1. **Task 1: Lockstep-bump all 5 crate versions + 6 path-dep pins to 0.58.0 and regenerate Cargo.lock** - `536e7c4e` (chore)
2. **Task 2: Add CHANGELOG [0.58.0] section; fix stale ROADMAP Phase 61 list entry** - `48a047db` (docs)

## Files Created/Modified

- `crates/nono/Cargo.toml` — version 0.57.5 -> 0.58.0
- `crates/nono-cli/Cargo.toml` — version + 3 path-dep pins 0.57.5 -> 0.58.0
- `crates/nono-proxy/Cargo.toml` — version + 1 path-dep pin 0.57.5 -> 0.58.0
- `crates/nono-shell-broker/Cargo.toml` — version + 1 path-dep pin 0.57.5 -> 0.58.0
- `bindings/c/Cargo.toml` — version + 1 path-dep pin (nono-ffi) 0.57.5 -> 0.58.0
- `Cargo.lock` — regenerated; all 5 crates resolve at 0.58.0
- `CHANGELOG.md` — new ## [0.58.0] - v2.9 section at top (Phase 60 + Phase 62 release description)
- `.planning/ROADMAP.md` — Phase 61 list entry rewritten from stale 0.57.5/Phase-60-only to 0.58.0-off-main + Phase 62 WFP framing

## Decisions Made

- **tools/sign-fixture excluded:** version 0.1.0, publish=false, no nono path-dep — out of scope per plan interfaces block
- **Root Cargo.toml not touched:** no `[workspace.package].version` field; nothing to bump there
- **.wxs not touched:** MSI ProductVersion is derived from the git tag by `build-windows-msi.ps1` (generated artifact, not hand-edited per `[[windows_msi_wxs_is_generated]]`)
- **REQUIREMENTS.md no-change:** REQ-RLS-03 (CI-signed public GitHub release with 0.58.0 bump, dual tag, signed MSIs, release notes) and REQ-RLS-04 (CWD guard preventing `--allow-cwd` credential exposure) were already registered with Phase 61 Pending mapping in both definitions and Traceability table — verified 2026-06-03; reconciliation not needed
- **Cross-target Unix clippy deferred to CI:** Windows host cannot run `--target x86_64-unknown-linux-gnu`; Cargo.toml changes are version strings only (no cfg-gated code added); deferred per plan scope

## Deviations from Plan

None - plan executed exactly as written.

REQUIREMENTS.md was pre-populated (2026-06-03) — the plan correctly anticipated this with "VERIFY-AND-RECONCILE, NOT create" framing. No edit was required.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Workspace is at 0.58.0; Cargo.lock is consistent; CHANGELOG carries the release section
- ROADMAP Phase 61 list entry correctly describes the 0.58.0-off-main + Phase 62 WFP + dual-tag framing
- Ready for Plan 61-02: tag v2.9 + v0.58.0 off current main and trigger release.yml

---
*Phase: 61-ship-release-v2-9-package-and-release-the-phase-60-confined*
*Completed: 2026-06-03*

## Self-Check: PASSED

- [x] `crates/nono/Cargo.toml` contains `version = "0.58.0"` — FOUND
- [x] `crates/nono-cli/Cargo.toml` contains `version = "0.58.0"` (crate + 3 path-dep pins) — FOUND
- [x] `CHANGELOG.md` contains `## [0.58.0]` — FOUND (grep count 1)
- [x] `.planning/ROADMAP.md` does NOT contain "0.57.5 binaries" — CONFIRMED (grep count 0)
- [x] `.planning/REQUIREMENTS.md` contains REQ-RLS-03 (count 3) and REQ-RLS-04 (count 2) — FOUND
- [x] Task 1 commit `536e7c4e` — FOUND
- [x] Task 2 commit `48a047db` — FOUND
