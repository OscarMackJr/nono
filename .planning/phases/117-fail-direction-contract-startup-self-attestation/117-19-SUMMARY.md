---
phase: 117-fail-direction-contract-startup-self-attestation
plan: 19
subsystem: infra
tags: [windows, spec, documentation, layer-registry, fail-direction-contract, SC4]

# Dependency graph
requires:
  - phase: 117-fail-direction-contract-startup-self-attestation
    provides: "Every wave-6/7 gap-closure plan's shipped fix (117-13..117-18) — this plan's only job is to make the standing SPEC ledger describe them accurately"
provides:
  - "Review-fix pass table rows for NR-04, NR-05, NR-06 (iteration 2) and NR3-01, NR3-02, NR3-03, NR3-04, NR3-05, NR3-08 (iteration 3), each with its written Honest-limit/deviation caveat"
  - "Corrected RF-14 row describing the shipped three-defense staging guard, not the pre-NR-06 single-defense guard"
  - "Symbol-form registry-table citations for MandatoryIntegrityLabel/DaclPackageSidGrant/DaclAncestorTraverse/DaclAncestorReadAttrs, matching layer_registry.rs's current source"
  - "Manual-verification section reduced to the 3 rows Plan 117-18 left un-automated, each explicitly framed as this session's operator-accepted downgrade"
affects: [118-receipts, 119-boundary-statement, future-CINT-01-citation-audits]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Standing discrepancy ledger extended by table-append (new # identifiers), never renumbering prior rows — preserves every prior row's citation stability"

key-files:
  created: []
  modified:
    - proj/SPEC-windows-fail-direction-contract.md

key-decisions:
  - "Left REQUIREMENTS.md's CINT-01 status as Pending, not Complete — spot-checked a citation this plan did NOT touch (`restricted_token.rs:55`) and confirmed it, too, no longer points at the enforcing call site; NR3-08's citation drift was flagged and fixed for only 4 of 13 rows this gap-closure round, so CINT-01's full 'cites the enforcing call site' bar remains unmet across the whole registry, even though this plan's own scoped rows are now accurate"
  - "Left CINT-03 as Pending per orchestrator instruction — this plan makes no registry-row-count change, only ledger/citation accuracy"
  - "RF-14's corrected text cross-references NR-06 rather than duplicating the three-defense description, matching the plan's own instruction and the ledger's existing convention (RF-15/RF-16 cross-reference the 'Downgrade banner behavior' section rather than repeating it)"

patterns-established:
  - "NR3-* row identifiers are explicitly labelled '(Iteration 3, gap-closure Plan 117-1N)' so a reader can trace which plan landed the fix without leaving the SPEC document"

requirements-completed: []  # CINT-01 spans citation accuracy across all 13 rows; this plan closes SC4 (the ledger) and 4 of 13 rows' citations, not the full registry — left Pending per this plan's own verified spot-check (see key-decisions)

# Metrics
duration: ~50min
completed: 2026-08-10
---

# Phase 117 Plan 19: SC4 Gap Closure — Review-Fix Ledger Ported Current + Registry Citations Corrected Summary

**Ported NR-04/NR-05/NR-06 (iteration 2) and NR3-01/02/03/04/05/08 (iteration 3) into the standing Review-fix pass table with their Honest-limit caveats, corrected the stale RF-14 row, converted 4 registry-table citations to the symbol-form strings 117-15 landed in source, and reduced the manual-verification section to Plan 117-18's 3-row operator-accepted remainder.**

## Performance

- **Duration:** ~50 min
- **Started:** 2026-08-10
- **Completed:** 2026-08-10
- **Tasks:** 3/3
- **Files modified:** 1

## Accomplishments

- Closed the SC4 gap `117-VERIFICATION.md` named directly: the SPEC's own "Review-fix pass" table — the standing artifact D-15 exists specifically to prevent "quietly reconciled" drift — now has a row for every iteration-2 finding (NR-04, NR-05, NR-06) and every iteration-3 finding this gap-closure round fixed (NR3-01, NR3-02, NR3-03, NR3-04, NR3-05, NR3-08), none reading "quietly reconciled"
- Corrected RF-14's "What changed" text, which described the pre-NR-06 single-defense guard while the shipped code has three ordered defenses (traversal-component rejection, component test, canonicalize-both-sides)
- Converted the `## Layer registry` table's citations for `MandatoryIntegrityLabel`, `DaclPackageSidGrant`, `DaclAncestorTraverse`, `DaclAncestorReadAttrs` from stale `"file:line"` strings to the exact `"file.rs::Symbol"` citations Plan 117-15 landed in `layer_registry.rs`'s `REGISTRY_ENTRIES` (read fresh from the current source, not reused from the plan's own `<interfaces>` block, which itself predated 117-15)
- Reduced the `### LayerId-scoped rows` manual-verification table from 11 to the exact 3 rows Plan 117-18's `ALSO_AUTOMATED` mechanism left un-automated (`DaclSessionSidGrant`, `MinifilterAbsence`, `BrokerAuthenticodeTrustGate`), each now explicitly framed as an OPERATOR-ACCEPTED DOWNGRADE citing the 2026-08-10 gap-closure session's locked decision; added a 10-of-13 count sentence above the table so the section states its own total
- All 10 combined `layer_registry_selfcheck`/`layer_registry_meta_test` tests re-run green against the fully-edited document

## Task Commits

Each task was committed atomically:

1. **Task 1: Port NR-04/NR-05/NR-06 into the Review-fix pass table; fix RF-14** - `dafae5c1` (docs)
2. **Task 2: Add NR3-01..NR3-05/NR3-08 rows; update registry table citations** - `215b611f` (docs)
3. **Task 3: Reduce and re-frame the manual-verification section** - `c8fdf259` (docs)

## Files Created/Modified

- `proj/SPEC-windows-fail-direction-contract.md` — Extended the `## Review-fix pass` table with 9 new rows (NR-04, NR-05, NR-06, NR3-01, NR3-02, NR3-03, NR3-04, NR3-05, NR3-08), each carrying its Honest-limit/deviation caveat; corrected RF-14's stale text; converted 4 registry-table rows' citations to symbol form; reduced the manual-verification `### LayerId-scoped rows` table from 11 to 3 rows with OPERATOR-ACCEPTED DOWNGRADE framing and a stated 10-of-13 total.

## Decisions Made

- Read every source document fresh rather than trusting the plan's own `<interfaces>`/`<action>` worked examples, per the `<prior_work>` warning that the plan's snapshot predates 117-15's citation conversion — confirmed the current `layer_registry.rs` source directly before writing the four corrected citation cells
- Sourced each NR3-* row's "What changed" text from the corresponding plan's SUMMARY.md (what actually shipped), not from the review's suggested fix — this surfaced one genuine deviation worth recording explicitly: NR3-05 (Plan 117-17) deleted `DaemonAttestationDecision::ProceedDowngraded` rather than wiring a real downgrade path, because the daemon's own guards are fail-closed end-to-end with no partial-coverage state to represent; the SPEC row states this choice and its reasoning rather than implying the review's literal suggestion was followed
- See `key-decisions` in frontmatter for the CINT-01/CINT-03 status decisions

## Deviations from Plan

None — plan executed as written across all three tasks. No Rule 1-4 triggers encountered; all edits matched the plan's `<action>` specifications and all acceptance criteria were met on the first pass.

## Issues Encountered

None.

## Verification Performed

- `cargo test -p nono-sandbox-cli --test layer_registry_selfcheck` — 4/4 pass (`spec_matches_registry`, `registry_call_sites_exist`, `symbol_citation_extraction_finds_the_eight_converted_citations`, `call_site_extraction_ignores_backtick_doc_comment_citations`)
- `cargo test -p nono-sandbox-cli --test layer_registry_meta_test` — 6/6 pass (`every_registry_row_has_a_test`, `host_gated_rows_are_loud`, `manual_verification_section_excludes_the_registry_table`, `security_assumptions_are_loud`, `pascal_to_snake_case_matches_expected_shapes`, `also_automated_entries_are_non_vacuous`)
- `cargo test -p nono-sandbox-cli --test layer_registry_selfcheck --test layer_registry_meta_test` (combined final check per the plan's `<verification>` section) — 10/10 pass
- `grep -n "| NR-04 |\|| NR-05 |\|| NR-06 |"` and the NR3-01..NR3-08 equivalents — all 9 new rows present
- `grep -n "refuses any path that is not a strict subdirectory"` — zero matches (stale RF-14 text gone)
- `grep -c "Honest limit"` — 3 (one per NR-04/NR-05/NR-06 row)
- Manual-verification table row count spot-checked via `awk` range extraction — exactly 3 data rows plus header
- `git add -f proj/SPEC-windows-fail-direction-contract.md` used for every commit (plain `git add` would exit 1 per the project's `proj/`-gitignored-but-tracked convention)
- Cross-target clippy: N/A — this plan touches only a markdown file, no Rust source

## Known Stubs

None.

## Threat Flags

None — this plan edits only `proj/SPEC-windows-fail-direction-contract.md` (a documentation artifact); it introduces no new network endpoints, auth paths, file access patterns, or schema changes at trust boundaries.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- SC4 is closed for this gap-closure round: every discrepancy this phase found and fixed (iteration-2's NR-04/NR-05/NR-06, iteration-3's NR3-01..NR3-05/NR3-08) is now recorded in the standing ledger with re-runnable evidence.
- The registry table's citations for the 4 rows NR3-08 flagged match the current source exactly, machine-verified by `registry_call_sites_exist`.
- The manual-verification section accurately reflects Plan 117-18's final 3-row accepted-downgrade set, explicitly citing the operator's locked decision.
- **Not closed by this plan:** CINT-01's citation-accuracy bar is not fully met across the whole 13-row registry — a spot-check of `restricted_token.rs:55` (a citation this plan did not touch, since NR3-08 only flagged 4 rows) found it, too, no longer points at the enforcing call site. A future pass converting the remaining line-number citations to symbol form (matching the pattern 117-15 established) would close this residual, but it is outside this plan's and this gap-closure round's stated scope.
- This is the final plan of Phase 117's gap-closure wave (wave 8, plan 19 of 19). Phase-level completion, ROADMAP checkbox, and CINT-01/CINT-03 requirement status are left for the orchestrator's verification pass, per this plan's explicit instructions.

---
*Phase: 117-fail-direction-contract-startup-self-attestation*
*Completed: 2026-08-10*

## Self-Check: PASSED

- FOUND: `proj/SPEC-windows-fail-direction-contract.md`
- FOUND: `.planning/phases/117-fail-direction-contract-startup-self-attestation/117-19-SUMMARY.md`
- FOUND commit `dafae5c1` (Task 1)
- FOUND commit `215b611f` (Task 2)
- FOUND commit `c8fdf259` (Task 3)
