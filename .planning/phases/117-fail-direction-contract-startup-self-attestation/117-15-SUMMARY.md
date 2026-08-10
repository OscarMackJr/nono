---
phase: 117-fail-direction-contract-startup-self-attestation
plan: 15
subsystem: infra
tags: [rust, windows, layer-registry, self-attestation, fail-open, citation-drift]

# Dependency graph
requires:
  - phase: 117-fail-direction-contract-startup-self-attestation
    provides: "layer_registry.rs (Plan 01) and layer_registry_selfcheck.rs (Plan 03) as the code-resident registry + drift gate this plan closes gaps in"
provides:
  - "dacl_session_sid_grant_is_not_claimed_anywhere + its converse every_consulting_row_with_an_expectancy_has_a_reported_field, closing the DaclSessionSidGrant fail-open landmine (NR3-03)"
  - "symbol-form (\"file.rs::Symbol\") call_sites citations for the four rows whose line-number citations had drifted ~145 lines (NR3-08)"
  - "registry_call_sites_exist extended to verify symbol-form citations by file content, not merely file existence"
affects: [117-18-verification, 118-receipts]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Symbol-form call-site citations (\"file.rs::Type::method\") as the immune-to-line-drift alternative to \"file:line\" citations"
    - "Discovery-based converse test: for every ConfiguredOnly/ConfirmedByEnforcingComponentReport row with an expected:true cell, an all-positive AppliedLayers must not classify NotApplicable"

key-files:
  created: []
  modified:
    - crates/nono-cli/src/exec_strategy_windows/layer_registry.rs
    - crates/nono-cli/tests/layer_registry_selfcheck.rs

key-decisions:
  - "Did not touch AppliedLayers::status()'s hardcoded NotApplicable for DaclSessionSidGrant or widen DACL_SESSION_SID_GRANT_EXPECTANCY — RF-03's operator decision stays open; the new tests only make silently reopening it loud instead of silent."
  - "Symbol split uses split_once (FIRST \"::\") not rsplit_once, since the symbol half itself contains \"::\" for Type::method notation."
  - "resolve_citation_path's four-way prefix match was refactored into a shared resolve_file_part(&str) helper so line-form and symbol-form resolution share one implementation."

patterns-established:
  - "Content-verified (not merely existence-verified) citation checking: a citation naming a real file but a since-renamed/removed symbol now fails the build."

requirements-completed: []  # CINT-01 progressed but not solely completed by this plan; CINT-02 spans 117-13..117-17 and stays Pending per orchestrator instruction.

duration: 60min
completed: 2026-08-10
---

# Phase 117 Plan 15: Close NR3-03 fail-open landmine + NR3-08 citation drift in layer_registry.rs Summary

**Added a canary test (plus its discovery-based converse) that fails the build if `DaclSessionSidGrant`'s empty expectancy is ever widened without `AppliedLayers::status()` also reporting a real field, and converted 8 stale `"file:line"` call-site citations across 4 registry rows to `"file.rs::Symbol"` citations that `registry_call_sites_exist` now verifies by file content.**

## Performance

- **Duration:** ~60 min (across a session interruption/resume)
- **Started:** 2026-08-10T18:05:00+01:00 (approx)
- **Completed:** 2026-08-10T18:19:12+01:00
- **Tasks:** 3
- **Files modified:** 2

## Accomplishments
- **NR3-03 closed:** `layer_registry.rs`'s `DACL_SESSION_SID_GRANT_EXPECTANCY` doc comment no longer cites a nonexistent test. `dacl_session_sid_grant_is_not_claimed_anywhere` now exists and fails the build if `DaclSessionSidGrant`'s expectancy is widened without `status()` also being updated. Its converse, `every_consulting_row_with_an_expectancy_has_a_reported_field`, is a discovery-based test (iterates `all_entries()`, names no `LayerId`) protecting every OTHER `ConfiguredOnly`/`ConfirmedByEnforcingComponentReport` row with an `expected: true` cell from the same fail-open shape.
- **NR3-08 closed for the four flagged rows:** `MandatoryIntegrityLabel`, `DaclPackageSidGrant`, `DaclAncestorTraverse`, and `DaclAncestorReadAttrs` now cite `"file.rs::Symbol"` call sites (e.g. `"dacl_guard.rs::AppliedDaclGrantsGuard::snapshot_and_apply"`) instead of the 8 stale `"file:line"` citations that had drifted ~145 lines across three consecutive review passes. The unrelated citations on these rows (`nono-shell-broker/src/main.rs:615-644`, the two `agent_daemon/launch.rs:*` lines) were left untouched, matching the plan's exact scope.
- `registry_call_sites_exist` (in `layer_registry_selfcheck.rs`) now additionally resolves every symbol-form citation's file and asserts the file's content contains the cited symbol — a renamed or removed function now fails the build, not just a moved-and-deleted file. `resolve_citation_path`'s four-way prefix match was refactored into a shared `resolve_file_part` helper reused by both citation shapes. A new floor test, `symbol_citation_extraction_finds_the_eight_converted_citations`, proves the new extraction path is non-vacuous (asserts >= 8, not exactly 8, so future additions don't force an edit here).

## Task Commits

Each task was committed atomically:

1. **Task 1: Close the DaclSessionSidGrant fail-open landmine (NR3-03)** - `0fd55d51` (test)
2. **Task 2: Symbol-form call-site citations for the four drifted rows (NR3-08)** - `fb06ac07` (fix)
3. **Task 3: Extend registry_call_sites_exist to verify symbol citations by content** - `8411ea6e` (test)

**Plan metadata:** (this commit)

## Files Created/Modified
- `crates/nono-cli/src/exec_strategy_windows/layer_registry.rs` — corrected the `DACL_SESSION_SID_GRANT_EXPECTANCY` doc comment; added 2 new tests; converted 8 call-site citations across 4 rows to symbol form.
- `crates/nono-cli/tests/layer_registry_selfcheck.rs` — added `extract_symbol_citations` + `split_symbol_citation`; refactored `resolve_citation_path` to share `resolve_file_part`; extended `registry_call_sites_exist` with content-based symbol verification; added `symbol_citation_extraction_finds_the_eight_converted_citations`.

## Decisions Made
- Left `AppliedLayers::status()`'s hardcoded `NotApplicable` for `DaclSessionSidGrant` and the empty `DACL_SESSION_SID_GRANT_EXPECTANCY` untouched — RF-03's operator decision (whether `WriteRestricted` should grant `config.session_sid`) is explicitly out of scope for this plan; the new tests exist to make its eventual resolution loud rather than silent.
- Chose `split_once("::")` (first occurrence) over `rsplit_once` for symbol citations, since `Type::method` notation means the symbol half legitimately contains further `::`.
- Refactored the shared prefix-resolution logic into `resolve_file_part` rather than duplicating the four-way `strip_prefix` chain a second time for symbol citations.

## Deviations from Plan

None — plan executed exactly as written, including both prescribed non-vacuity counterfactual proofs (see below).

**Counterfactual proofs performed manually, not left as permanent code changes (per plan's acceptance criteria):**
- **Task 1:** Temporarily widened `DACL_SESSION_SID_GRANT_EXPECTANCY` to a 1-element array without changing `status()`; `dacl_session_sid_grant_is_not_claimed_anywhere` failed with the expected fail-open message; reverted (confirmed via `git diff` showing zero unintended change afterward).
- **Task 3:** Temporarily edited the `DaclPackageSidGrant` citation in `layer_registry.rs` to name a nonexistent symbol (`snapshot_and_apply_renamed`) without changing `dacl_guard.rs`; `registry_call_sites_exist` failed naming the missing symbol at the resolved path; reverted. (An initial attempt renaming the actual production symbol in `dacl_guard.rs` was abandoned — it left the old symbol text present in nearby doc comments and test-module call sites, producing a false pass via `content.contains`; editing the citation directly is the cleaner, self-contained counterfactual and was reverted cleanly.)

## Issues Encountered

Session was interrupted mid-Task-3 by an API connection error after Tasks 1 and 2 were already committed. Resumed by re-reading the uncommitted working-tree state of `layer_registry_selfcheck.rs`, confirming it matched the pre-refactor original, and completing the remaining refactor + extension + non-vacuity test from there. No rework was needed on Tasks 1/2.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- `layer_registry.rs` is closer to the enumerable, self-verifying state Plan 18 (verify gate) needs: all 13 rows now have citations that are either symbol-form (immune to line drift) or explicitly scoped-and-unchanged.
- NR3-03 and NR3-08 are closed for the rows flagged in `117-VERIFICATION.md`; no other rows were touched, matching the plan's exact scope boundary.
- CINT-02 (spanning 117-13..117-17) remains `Pending` in REQUIREMENTS.md per orchestrator instruction — not marked complete by this plan alone.

---
*Phase: 117-fail-direction-contract-startup-self-attestation*
*Completed: 2026-08-10*
