---
phase: 117-fail-direction-contract-startup-self-attestation
plan: 26
subsystem: windows-fail-direction-contract
tags: [spec, discrepancy-ledger, sc4, documentation, citation-verification]

# Dependency graph
requires:
  - phase: 117-20
    provides: "CR-01/WR-01/WR-02/WR-03 disposition (AceFlags-aware, ownership-gated residue predicate)"
  - phase: 117-21
    provides: "CR-02/WR-04 disposition (D-28 private-channel gating; ClearStaleLayerResidue remediation wired)"
  - phase: 117-22
    provides: "WR-06/WR-11 disposition (corrected daemon doc premise; hardened discovery tests)"
  - phase: 117-23
    provides: "WR-07 disposition (3-state ancestor-guard coverage reporting)"
  - phase: 117-24
    provides: "WR-08/WR-10 disposition (symbol-form citation conversion; content-verified self-checks)"
  - phase: 117-25
    provides: "WR-05/WR-09 disposition (dead CI env block removed; HMAC chain fields re-privatized)"
provides:
  - "proj/SPEC-windows-fail-direction-contract.md with a complete iteration-4 discrepancy ledger (CR-01, CR-02, WR-01..WR-11), a citation table matching layer_registry.rs exactly, and corrected NR3-01/04/05 rows"
affects: [117-verification, phase-118-enforcement-receipts, phase-119-boundary-statement]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Ledger rows cite a literal, re-run grep/test command and its live hit count dated to the plan's execution — not a restated claim from the review artifact (D-16 evidence discipline)."

key-files:
  created: []
  modified:
    - proj/SPEC-windows-fail-direction-contract.md

key-decisions:
  - "WR-02's row is recorded as 'Accepted, not mechanically fixed' — 117-20 corrected the doc comment to state the code's true first-out-restores behavior rather than building a cross-session refcount, per the plan's own prescribed disposition. The ledger states this explicitly rather than implying the underlying race was closed."
  - "WR-05's row states the true scoped outcome (only the windows-layer-fault-injection job's dead block removed; the windows-security job's genuinely load-bearing copy was deliberately kept) rather than the plan's example command's literal zero-match implication, which would have misrepresented 117-25's actual, correct disposition."
  - "WR-06's row states the corrected doc rationale (under-granting vs. under-confining) rather than the plan's assumed 'wire a real downgrade path' framing — 117-22 deliberately did NOT reintroduce ProceedDowngraded, and the ledger reflects that true outcome."
  - "All 13 evidence commands in the new ledger rows were independently re-run against the current tree during this plan's own execution (2026-08-11), not transcribed from 117-REVIEW.md's pre-fix findings or from the disposing plans' SUMMARY.md prose."

requirements-completed: []

# Metrics
duration: ~50min
completed: 2026-08-11
---

# Phase 117 Plan 26: Reconcile the Discrepancy Ledger (SC4 Closure) Summary

**Closed SC4 by recording all 13 iteration-4 code-review findings (CR-01, CR-02, WR-01 through WR-11) in the standing `proj/SPEC-windows-fail-direction-contract.md` ledger with re-runnable, dated evidence and their TRUE (code-verified, not prose-trusted) disposition; synced the Layer registry table's 9 drifted citations to the post-117-24 symbol-form source; and forward-referenced the three prior-round rows (NR3-01, NR3-04, NR3-05) this round's fixes made stale.**

## Performance

- **Duration:** ~50 min
- **Completed:** 2026-08-11
- **Tasks:** 3
- **Files modified:** 1 (`proj/SPEC-windows-fail-direction-contract.md`)

## Accomplishments

- **Task 1:** All 9 drifted rows in the `## Layer registry` table (`RestrictedToken`, `MandatoryIntegrityLabel`, `AppContainerProfile`, `DaclPackageSidGrant`, `WfpEgressFilters`, `FirewallRulesEgress`, `JobObjectContainment`, `BrokerAuthenticodeTrustGate`, `InterpreterCoverageGate`) now cite the exact, current `call_sites` arrays from `layer_registry.rs::REGISTRY_ENTRIES`, verified symbol-by-symbol via direct grep of each symbol's declaration. The 4 already-correct rows (`DaclSessionSidGrant`, `DaclAncestorTraverse`, `DaclAncestorReadAttrs`, `MinifilterAbsence`) were confirmed unchanged by direct comparison, not assumed.
- **Task 2:** Appended 13 new rows to the `## Review-fix pass` table (`CR-01`, `CR-02`, `WR-01`..`WR-11`), each with a 1-3 sentence "What was wrong" summary condensed from `117-REVIEW.md` and a "What changed" cell citing at least one re-run `grep`/test command with its live hit count, dated 2026-08-11. Every command was independently re-executed against the current tree during this plan's own execution — not copied from a disposing plan's SUMMARY.md prose.
- **Task 3:** Appended one "Iteration 4 update" sentence to each of NR3-01, NR3-04, NR3-05, forward-referencing the new rows above rather than restructuring the historical record. Confirmed `.planning/REQUIREMENTS.md` is untouched (`git diff --stat` shows no changes) and CINT-01/02/03 remain `[ ] Pending` — this plan does not flip them to Complete.

## Task Commits

Each task was committed atomically:

1. **Task 1: Sync the Layer registry table's citations to post-117-24 code** - `62a20c87` (docs)
2. **Task 2: Append the 13 iteration-4 discrepancy rows to the Review-fix pass table** - `67129143` (docs)
3. **Task 3: Correct NR3-01/04/05 to forward-reference iteration-4; confirm REQUIREMENTS.md untouched** - `f1ede25e` (docs)

**Plan metadata:** (this commit, to follow)

## Files Created/Modified

- `proj/SPEC-windows-fail-direction-contract.md` — Layer registry table's 9 citation cells resynced to symbol form matching `layer_registry.rs`; 13 new rows appended to the Review-fix pass ledger; NR3-01/04/05 forward-referenced.

## Decisions Made

- Recorded WR-02 as "Accepted, not mechanically fixed" rather than implying the underlying first-session-out-restores race was closed — 117-20's disposition was a doc correction (stating the code's true behavior) per the plan's own prescribed fix, not a refcount mechanism. The ledger's wording makes this distinction explicit so a future reader does not assume the race is gone.
- Recorded WR-05's true, scoped outcome — the plan's own `<action>` text (`grep -c "NONO_CI_HAS_WFP" .github/workflows/ci.yml` → `0`) does not match what 117-25 actually and correctly did (kept the `windows-security` job's genuinely load-bearing copy, removed only the dead `windows-layer-fault-injection` block). Re-ran the command live: current count is `1`, not `0`. The ledger row states the true, correct disposition rather than the plan's assumed literal outcome, per this plan's own stated purpose (prevent exactly the kind of assumed-not-verified claim SC4 exists to catch).
- Recorded WR-06's true outcome (doc-comment correction to the under-granting-vs-under-confining rationale) rather than the review's suggested "wire a real downgrade path" fix — 117-22 deliberately did not reintroduce `ProceedDowngraded`, and its SUMMARY.md documents that as a considered, tested decision, not an incomplete fix.
- Every one of the 13 new rows' evidence commands was re-run against the current tree during this plan's own execution (2026-08-11) rather than trusted from the disposing plan's SUMMARY.md — per this plan's accuracy_note, code is the source of truth over prose. All counts matched or exceeded what the disposing plans' SUMMARY.md files claimed (no discrepancies found between claimed and actual code state across all 6 disposing plans' SUMMARYs).

## Deviations from Plan

None — plan executed exactly as written. The plan's own `<action>` text for Task 2 explicitly anticipated and authorized the WR-05/WR-06/WR-02 wording choices above ("For any finding where the disposing plan's actual SUMMARY shows a different outcome than this plan's `<interfaces>` mapping assumed ... write that TRUE outcome, not the assumed one"), so recording the corrected outcomes is fulfilling the plan's own instruction, not deviating from it.

## Issues Encountered

- The plan's Task 2 `<action>` example for WR-05 ("record `0`") did not match 117-25's actual, correct disposition (1 remaining, load-bearing occurrence). This was anticipated by the plan's own text as the exact scenario Task 2 exists to catch and was resolved by recording the true, re-verified outcome rather than the plan's illustrative example.
- One residual staleness noted but NOT fixed (out of this plan's file scope, `proj/SPEC-windows-fail-direction-contract.md` only): `crates/nono-cli/tests/layer_registry_selfcheck.rs::symbol_citation_extraction_finds_the_eight_converted_citations` is now a stale test name (117-24 converted 9 more citations, for a running total higher than "eight"), but the test itself still passes — flagged here for visibility, not filed as a new finding since it is a naming staleness in test code, not this plan's `files_modified` scope.

## Verification

- `grep -c "Iteration 4, gap-closure Plan 117-2" proj/SPEC-windows-fail-direction-contract.md` → `13` (matches the plan's stated verification).
- `grep -n "CR-01 (Iteration 4\|CR-02 (Iteration 4"` → both present.
- `grep -c "WR-0[1-9] (Iteration 4\|WR-1[01] (Iteration 4"` → `11`.
- `grep -c "Iteration 4 update"` → `3` (NR3-01, NR3-04, NR3-05).
- `git diff --stat .planning/REQUIREMENTS.md` → empty (no changes).
- `grep -n "CINT-01\|CINT-02\|CINT-03" .planning/REQUIREMENTS.md` → all three still `[ ] Pending`.
- All 9 registry-table citation cells verified symbol-by-symbol against `layer_registry.rs::REGISTRY_ENTRIES`'s current `call_sites` arrays; the 4 already-correct rows confirmed unchanged.
- `cargo test -p nono-sandbox-cli --test layer_registry_selfcheck` — 6/6 passed.
- `cargo test -p nono-sandbox-cli --test layer_registry_meta_test` — 9/9 passed.
- `cargo fmt --check` (workspace) — clean.
- Doc-only plan touching only `proj/SPEC-windows-fail-direction-contract.md` (a Markdown file, not compiled) — no cross-target clippy gate applies; no Rust source was modified by this plan.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- SC4 is closed: every iteration-4 finding (CR-01, CR-02, WR-01 through WR-11) is now recorded in the standing SPEC with a re-runnable, dated, code-verified disposition — nothing remains only in the ephemeral `117-REVIEW.md`.
- The registry citation table matches `layer_registry.rs` exactly for all 13 rows.
- The three prior-round rows this round's fixes made stale (NR3-01, NR3-04, NR3-05) now forward-reference the corrections.
- `REQUIREMENTS.md`'s CINT-01/CINT-02/CINT-03 remain `[ ] Pending` — untouched by this plan, as required; that determination belongs to re-verification.
- This is the last plan of Phase 117 Wave 11. No blockers for phase close or re-verification.

---
*Phase: 117-fail-direction-contract-startup-self-attestation*
*Completed: 2026-08-11*

## Self-Check: PASSED

- FOUND: `proj/SPEC-windows-fail-direction-contract.md`
- FOUND commit: `62a20c87` (Task 1)
- FOUND commit: `67129143` (Task 2)
- FOUND commit: `f1ede25e` (Task 3)
