---
phase: 96-cross-target-toolchain
plan: 03
subsystem: testing
tags: [cross-target, clippy, cross, cargo-zigbuild, verification-protocol, docs]

# Dependency graph
requires:
  - phase: 96-01
    provides: linux-gnu cross clippy gate GREEN + recorded pinned cross image tag
  - phase: 96-02
    provides: apple-darwin cargo-zigbuild clippy gate GREEN + LOCAL-RUNNABLE disposition + invocation-form nuance
provides:
  - Rewritten cross-target-verify-checklist.md retiring the auto-PARTIAL default per-gate (evidence-based)
  - Canonical local-runnable invocations documented for both gates (cross clippy + direct-binary cargo-zigbuild clippy)
  - One-line CLAUDE.md pointer to the checklist (single source of truth, no duplicated runbook)
affects: [97-release, cross-target-verification, gsd-verify-phase]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Per-gate, evidence-based retirement of PARTIAL->CI default: PARTIAL is the fallback for a documented runner failure only"
    - "Single-source-of-truth doc home (checklist) + one-line CLAUDE.md pointer (D-06)"

key-files:
  created:
    - .planning/phases/96-cross-target-toolchain/96-03-SUMMARY.md
  modified:
    - .planning/templates/cross-target-verify-checklist.md
    - CLAUDE.md

key-decisions:
  - "Both gates documented as LOCAL-RUNNABLE (linux-gnu via cross clippy, apple-darwin via direct-binary cargo-zigbuild clippy), reflecting the actual Wave 1/Wave 2 GREEN verdicts — not the pre-execution PARTIAL assumption."
  - "apple-darwin Q3 FLIPS to MUST-run-locally per the 96-02 record handoff flag (D-04 clean-exit branch), not PARTIAL->CI."
  - "PARTIAL->CI demoted from default to documented-runner-failure fallback; stopped daemon and absent-but-installable tool explicitly excluded."
  - "cargo-zigbuild direct-binary 'clippy' form documented (NOT 'cargo zigbuild clippy', which mis-parses)."

patterns-established:
  - "Anti-pattern 5: defaulting to PARTIAL->CI for an unrun gate is forbidden — run the gate."
  - "Anti-pattern 6: 'cargo zigbuild clippy' mis-parses; use direct-binary 'cargo-zigbuild clippy'."

requirements-completed: [XTGT-04]

# Metrics
duration: 2min
completed: 2026-06-26
---

# Phase 96 Plan 03: Cross-Target Verification Protocol Rewrite Summary

**Cross-target verification protocol rewritten so both Unix gates are documented as local-runnable on the Windows dev host (linux-gnu via `cross clippy`, apple-darwin via direct-binary `cargo-zigbuild clippy`), demoting PARTIAL→CI from the default to a documented-runner-failure fallback while preserving the security mandate.**

## Performance

- **Duration:** ~2 min
- **Started:** 2026-06-26T16:21:07Z
- **Completed:** 2026-06-26T16:22:42Z
- **Tasks:** 2
- **Files modified:** 2 (1 created: this SUMMARY)

## Accomplishments
- Rewrote `.planning/templates/cross-target-verify-checklist.md`: Q2 (linux-gnu) now names the canonical containerized `cross clippy` form (bare `cargo clippy --target` removed as the runnable command); Q3 (apple-darwin) flipped to MUST-run-locally via the direct-binary `cargo-zigbuild clippy` form, matching the 96-02 Wave 2 GREEN record.
- Retired the auto-default-to-PARTIAL per-gate: PARTIAL→CI is now the fallback for a *documented* runner failure only — a stopped Docker daemon and an absent-but-installable tool are explicitly excluded.
- Moved the setup + canonical invocations into the checklist (D-06 single source of truth), including the recorded pinned cross image `ghcr.io/cross-rs/x86_64-unknown-linux-gnu:0.2.5@sha256:9e5b39c0...` and the `zig 0.16.0` + `cargo-zigbuild 0.23.0` runner.
- Collapsed the CLAUDE.md cross-target bullet to a one-line pointer carrying both commands + the demoted-default disposition; preserved the "Windows-host `cargo check` is NOT a substitute" warning.
- Added anti-patterns 5 (default-to-PARTIAL for an unrun gate) and 6 (`cargo zigbuild clippy` mis-parse).

## Task Commits

Each task was committed atomically:

1. **Task 1: Rewrite the checklist — cross-form invocation, setup home, per-gate evidence-based retirement** - `df10eef5` (docs)
2. **Task 2: Collapse the CLAUDE.md cross-target bullet to a one-line pointer** - `d5147daa` (docs)

**Plan metadata:** (final docs commit — SUMMARY + STATE + ROADMAP)

## Files Created/Modified
- `.planning/templates/cross-target-verify-checklist.md` - Rewritten decision tree, per-gate runner setup with pinned image tag, narrowed PARTIAL fallback, new anti-patterns 5/6.
- `CLAUDE.md` - § Coding Standards cross-target bullet collapsed to a one-line pointer carrying both local-runnable commands + demoted PARTIAL default.
- `.planning/phases/96-cross-target-toolchain/96-03-SUMMARY.md` - This summary.

## Decisions Made
- **Both gates documented as LOCAL-RUNNABLE** (not the plan's pre-execution PARTIAL assumption). The plan's Q3 interface allowed either a PARTIAL→CI hard-blocker OR a local-runnable flip; the 96-02 record committed apple-darwin to the clean-exit (local-runnable) branch, so the checklist flips Q3 to MUST-run-locally and the CLAUDE.md bullet states both gates as runnable.
- **Security mandate preserved.** The cross-target verification requirement is unchanged; only HOW it is satisfied was updated (local now possible) and the PARTIAL→CI *default* was retired — not the requirement. The "Windows `cargo check` is NOT a substitute" warning and the no-`#[allow]`-silencing anti-pattern were retained.

## Deviations from Plan

None - plan executed exactly as written. The plan's Q3 was conditional on the Wave 2 outcome; the 96-02 record's clean-exit verdict selected the local-runnable branch, which both edits encode. No code changes were required (docs-only plan, as expected).

## Issues Encountered
None.

## User Setup Required
None - no external service configuration required. (The checklist documents optional one-time host tool installs — `cross`/Docker and `zig`/`cargo-zigbuild` — already present on this dev host.)

## Next Phase Readiness
- XTGT-04 closed; this completes Phase 96 (3/3 plans). Both cross-target gates are provably local-runnable and the verification protocol Phase 97's release tree inherits now reflects that.
- Phase 97 (release prepare-only: crate leapfrog ≥0.65.0, signed pipeline) is unblocked.

## Self-Check: PASSED

- FOUND: `.planning/templates/cross-target-verify-checklist.md`
- FOUND: `CLAUDE.md`
- FOUND: `.planning/phases/96-cross-target-toolchain/96-03-SUMMARY.md`
- FOUND commit: `df10eef5` (Task 1 — checklist rewrite)
- FOUND commit: `d5147daa` (Task 2 — CLAUDE.md pointer)

---
*Phase: 96-cross-target-toolchain*
*Completed: 2026-06-26*
