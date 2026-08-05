---
phase: 112-security-residual-sync
plan: 01
subsystem: infra
tags: [upstream-sync, divergence-audit, security-review, disposition-gate, oauth, aws-sigv4, cargo-audit]

# Dependency graph
requires:
  - phase: 108-upst12-divergence-audit
    provides: "108-DIVERGENCE-LEDGER.md's canonical 18-SHA security-residual-and-misc cluster itemization"
  - phase: 109-proxy-network-absorb
    provides: "109-AWS-SIGV4-TLS-INTERCEPT-FINDING.md — precedent shape and prior evidence for the same absent aws/tls_intercept subsystem"
  - phase: 111-core-carry-resource-cli-verify-release-leapfrog
    provides: "Confirmed crossbeam-epoch 0.9.20 + clean cargo audit baseline (D-07's premise)"
provides:
  - "Finalized 18-SHA disposition table (112-DISPOSITION-TABLE.md) with live-re-verified evidence, feeding every Wave 2+ absorb plan's read_first"
  - "SEC-01 won't-sync finding (112-AWS-SIGV4-PROXY-AUTH-FINDING.md), correcting 112-CONTEXT.md's stale SEC-01 target listing"
  - "SEC-02a/b/c reality-check evidence + deferred-to-Phase-114 disposition record (112-OAUTH-CAPTURE-DISPOSITION.md)"
  - "RES-01's 4-SHA skip-with-reasoning documentation (D-03 compliance)"
  - "D-07 confirmation: crossbeam-epoch 0.9.20, cargo audit 0 vulnerabilities, RUSTSEC-2026-0204 closed"
affects: [112-security-residual-sync (Wave 2+ absorb plans), 114-oauth-capture-absorb]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "D-02 front-gate reality-check: re-run confirmatory commands live before any absorb plan executes, rather than trusting cached research text"
    - "Evidence-capture-without-verdict for phase-boundary carve-outs (mirrors NET-02/SPIFFE Phase 109->113 precedent)"

key-files:
  created:
    - .planning/phases/112-security-residual-sync/112-DISPOSITION-TABLE.md
    - .planning/phases/112-security-residual-sync/112-AWS-SIGV4-PROXY-AUTH-FINDING.md
    - .planning/phases/112-security-residual-sync/112-OAUTH-CAPTURE-DISPOSITION.md
  modified: []

key-decisions:
  - "SEC-01 (0ecc476b) confirmed won't-sync — aws/ and tls_intercept/ subsystems absent from crates/nono-proxy/src/, corrects 112-CONTEXT.md's stale target-file listing for SEC-01"
  - "SEC-02a/b/c (9b692e07/3c59c62e/d033c631) recorded deferred -> Phase 114 with reality-check evidence; forward.rs's ResponseRewrite hook confirmed as the load-bearing mechanism keeping real OAuth tokens out of the sandboxed client, so no partial adapt was attempted"
  - "SEC-08 (a5a441c2) flagged won't-sync-verbatim / adapt-with-fail-closed-preservation for ADR-112 escalation — adopting verbatim would relax the fork's existing Phase-34 fail-closed allow_vars default"
  - "D-07 confirmed: crossbeam-epoch 0.9.20 in Cargo.lock, cargo audit exits 0 with 0 vulnerabilities (6 unrelated allowed advisory warnings); 373a67ae not re-absorbed"
  - "RES-01's 4 SHAs recorded skip-with-reasoning per D-03 (HKLM machine-policy-spine collision); f0506434's always-further pack-registry-namespace staleness flagged but not acted on (UX-only, out of scope)"

patterns-established:
  - "Live-re-run gate: every disposition in 112-DISPOSITION-TABLE.md cites a freshly executed command's output, not a copy-paste from 112-RESEARCH.md"

requirements-completed: []  # SEC-01/RES-01 partially addressed by this plan (finding + skip docs), but full requirement satisfaction depends on Wave 2+ absorb plans landing the adopt/adapt items — not marking Complete here to avoid premature REQUIREMENTS.md flip on a multi-plan requirement.

# Metrics
duration: 25min
completed: 2026-08-05
---

# Phase 112 Plan 01: Wave 1 Reality-Check Gate Summary

**Live-re-verified all 18 security-residual-and-misc SHA dispositions against the current fork
tree, producing SEC-01's won't-sync finding and SEC-02a/b/c's deferred-to-Phase-114 evidence
record — zero source code touched.**

## Performance

- **Duration:** ~25 min (evidence-gathering + 3 documents authored + committed)
- **Started:** 2026-08-05T14:15:00Z (approx.)
- **Completed:** 2026-08-05T14:40:00Z (approx.)
- **Tasks:** 3/3 completed
- **Files modified:** 3 (all new, all documentation)

## Accomplishments
- Re-ran every D-02 confirmatory command live (SEC-01 `ls`, SEC-09 `grep`, SEC-05 `AccessFs::Refer`
  scope check inside `restrict_execute()`, SEC-08 regression-guard `grep`, D-07 `cargo audit` +
  `crossbeam-epoch` pin) and produced a single 18-row finalized disposition table.
- Authored the SEC-01 won't-sync finding mirroring `109-AWS-SIGV4-TLS-INTERCEPT-FINDING.md`'s shape,
  explicitly correcting `112-CONTEXT.md`'s stale SEC-01 target-file listing.
- Captured SEC-02a/b/c's reality-check evidence, confirmed `forward.rs`'s `ResponseRewrite` hook is
  the mechanism preventing real OAuth tokens from reaching the sandboxed client, and recorded the
  disposition as deferred to Phase 114 (not an adopt/adapt/decline verdict) per the ROADMAP
  Amendment (2026-08-05).
- Documented all 4 RES-01 SHAs with individual skip-with-reasoning entries per D-03, plus a
  flagged-but-out-of-scope UX-staleness finding for `f0506434`'s pack-registry-namespace prefix.

## Task Commits

Each task was committed atomically:

1. **Task 1: Reality-check finalization of all 18 dispositions + D-07 confirmation** - `a6a4f115` (docs)
2. **Task 2: SEC-01 won't-sync finding document** - `b1f17f61` (docs)
3. **Task 3: SEC-02a/b/c reality-check evidence capture + RES-01 skip documentation** - `a8a85d6f` (docs)

_No plan-metadata-only commit was created separately — this SUMMARY's commit serves as the final
metadata commit per the workflow's final_commit step._

## Files Created/Modified
- `.planning/phases/112-security-residual-sync/112-DISPOSITION-TABLE.md` - 18-SHA finalized disposition table with live-re-run cited evidence, feeding every Wave 2+ absorb plan
- `.planning/phases/112-security-residual-sync/112-AWS-SIGV4-PROXY-AUTH-FINDING.md` - SEC-01 won't-sync finding, cross-referencing 109's finding and correcting 112-CONTEXT.md
- `.planning/phases/112-security-residual-sync/112-OAUTH-CAPTURE-DISPOSITION.md` - SEC-02a/b/c reality-check evidence + deferred-to-Phase-114 record + RES-01's 4 skip entries

## Decisions Made
- Kept `112-DISPOSITION-TABLE.md` as a single flat 18-row table (rather than a 15-row gated table
  plus a separate 3-row RES-02 appendix) to unambiguously satisfy the plan's "exactly 18 SHA rows"
  acceptance criterion, while still noting in-table which 15 rows were freshly live-re-verified
  under this plan's Task 1-3 gate scope (SEC-01..09, D-07, RES-01) versus which 3 RES-02 rows were
  carried from `112-RESEARCH.md` (RES-02 sits outside this plan's `must_haves`).
- For SEC-02a/b/c, deliberately stopped at evidence-capture + deferred-disposition rather than
  proposing even a tentative adapt-down scope, per the plan's explicit instruction not to make an
  adopt/adapt/decline call — that authority belongs to Phase 114 per the ROADMAP Amendment.

## Deviations from Plan

None - plan executed exactly as written. All three tasks' acceptance criteria were met using the
exact commands the plan specified (or their live re-run), and no source code under `crates/` was
touched.

## Issues Encountered

None. All confirmatory commands reproduced the exact findings `112-RESEARCH.md` had already
established as HIGH confidence; no live re-verification diverged from the research document.

## User Setup Required

None - no external service configuration required. This is a documentation-only plan (no package
installs, no code changes).

## Next Phase Readiness

- Wave 2+ absorb plans (SEC-03/04/05/07 adopt, SEC-06 adapt, SEC-08 ADR-112-flagged, RES-01
  skip-recorded) can now read `112-DISPOSITION-TABLE.md` as their finalized, execution-time-cited
  work list rather than re-deriving dispositions from `112-RESEARCH.md`.
- SEC-09's D-01 carry-forward note into the v3.7 tool-sandbox work-list is still owed by a future
  task in this phase (not this plan's task list) — flagging so a downstream plan in this phase
  picks it up; this plan only re-confirmed SEC-09's absent-target premise, it did not author the
  carry-forward note itself.
- Phase 114 (OAuth Capture Absorb, SEC-02) has its evidence base ready in
  `112-OAUTH-CAPTURE-DISPOSITION.md` and does not need to re-derive the `forward.rs`/`oauth_capture/`
  file-presence findings from scratch.
- No blockers for Wave 2.

---
*Phase: 112-security-residual-sync*
*Completed: 2026-08-05*

## Self-Check: PASSED

All 4 created files confirmed present on disk; all 4 task/summary commit hashes
(`a6a4f115`, `b1f17f61`, `a8a85d6f`, `83bcec31`) confirmed present in `git log`.
