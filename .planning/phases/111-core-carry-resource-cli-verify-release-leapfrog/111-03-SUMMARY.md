---
phase: 111-core-carry-resource-cli-verify-release-leapfrog
plan: 03
subsystem: docs
tags: [adr, library-boundary, resource-limits, divergence-ledger, upstream-sync]

# Dependency graph
requires:
  - phase: 108-upst12-divergence-audit
    provides: "CORE cluster per-commit table classifying e6d26871/#1269 and 34c2c975/#1403 as adapt, plus the cross-crate-reexport Threat Flag this plan resolves"
  - phase: 111-02
    provides: "corrected cli.rs/flags.mdx help text — the other half of CORE-02"
provides:
  - "proj/ADR-111-resource-limits-boundary.md — the standalone, citable ADR settling CORE-02's library-boundary disposition (ADAPT-not-adopt) with an explicit ADR-86 carve-out non-extension argument"
  - "108-DIVERGENCE-LEDGER.md Phase 111 Standing Divergence Addendum — a locked exception to the ledger's own 'closed' declaration, recording e6d26871/34c2c975 as a permanent (not deferred) divergence"
  - "CORE-02 marked Complete in .planning/REQUIREMENTS.md (both split halves — 111-02 help-text + 111-03 ADR/ledger — now landed)"
affects: [112-security-residual-sync, future-upstream-syncs-touching-resource-module]

# Tech tracking
tech-stack:
  added: []
  patterns: ["standalone per-decision ADR mirroring ADR-108's shape (Context/Decision/Consequences/References)", "standing-divergence ledger addendum as a locked exception to a closed document"]

key-files:
  created:
    - proj/ADR-111-resource-limits-boundary.md
  modified:
    - .planning/phases/108-upst12-divergence-audit/108-DIVERGENCE-LEDGER.md
    - .planning/REQUIREMENTS.md

key-decisions:
  - "D-01 confirmed and recorded durably: ADAPT not adopt upstream's core `resource` module — crates/nono/src/lib.rs will never gain `pub mod resource;`"
  - "D-03: ADR-111 explicitly explains (not merely asserts) why ADR-86's audit/diagnostics carve-out does not extend to resource limits — audit/diagnostics are read-only observability primitives, resource limits are the enforcement mechanism itself"
  - "D-02: ledger addendum is a permanent divergence, explicitly distinguished from the tool-sandbox-surface cluster's DEFERRED->v3.7 treatment (no future phase this hands off to)"
  - "CORE-02 marked Complete now that both split halves (111-02 help-text, 111-03 ADR+ledger) have landed, following the Phase 110 PROF-03 split-requirement precedent"

patterns-established:
  - "Pattern: a 'closed' ledger document can carry one deliberate, explicitly-labeled locked-exception addendum without reopening the document to general edits — the closing sentence itself is amended to name the sole exception."

requirements-completed: [CORE-02]

# Metrics
duration: 20min
completed: 2026-08-04
---

# Phase 111 Plan 03: ADR-111 + Ledger Standing-Divergence Addendum Summary

**Wrote proj/ADR-111-resource-limits-boundary.md settling the ADAPT-not-adopt disposition of upstream's core resource module (#1269/#1403), and appended a locked standing-divergence addendum to the closed 108-DIVERGENCE-LEDGER.md — closing out CORE-02.**

## Performance

- **Duration:** ~20 min
- **Completed:** 2026-08-04T23:31:25Z
- **Tasks:** 2 completed
- **Files modified:** 3 (1 created, 2 modified — including the requirements traceability update)

## Accomplishments
- Authored `proj/ADR-111-resource-limits-boundary.md` mirroring `ADR-108`'s shape (Status/Phase/Date/Authors header, Context, Decision, a dedicated section answering the ADR-86 carve-out question, Consequences, References), citing both upstream SHAs verbatim and explicitly stating `crates/nono/src/lib.rs` will not gain `pub mod resource;`.
- Appended a "Phase 111 Standing Divergence Addendum" to `108-DIVERGENCE-LEDGER.md`, reconciling with the file's own "Ledger closed" declaration by amending the closing sentence to name this as the sole locked exception, per D-02.
- Marked `CORE-02` Complete in `.planning/REQUIREMENTS.md` (checkbox + traceability table), now that both split halves — 111-02's help-text correction and this plan's ADR/ledger artifacts — have landed, following the Phase 110 PROF-03 split-requirement precedent explicitly handed off by 111-02.

## Task Commits

Each task was committed atomically:

1. **Task 1: Write proj/ADR-111-resource-limits-boundary.md** - `10194152` (docs)
2. **Task 2: Record the standing divergence in 108-DIVERGENCE-LEDGER.md** - `6bf65687` (docs)

**Plan metadata:** committed alongside this SUMMARY (final metadata commit, see below)

## Files Created/Modified
- `proj/ADR-111-resource-limits-boundary.md` - New standalone ADR: Context (upstream `e6d26871`/#1269 + `34c2c975`/#1403 shape, Windows-relevance finding from the Phase 108 audit), Decision (ADAPT, D-04 flag-freeze), a dedicated section explaining why ADR-86's audit/diagnostics carve-out does not extend to resource-limit enforcement, Consequences (standing divergence, future-sync re-affirmation duty, help-text-only user impact), References.
- `.planning/phases/108-upst12-divergence-audit/108-DIVERGENCE-LEDGER.md` - Appended "Phase 111 Standing Divergence Addendum" section immediately after the "Ledger closed" sentence (which was itself amended to name this exception); records both SHAs, cites the ADR by path, and explicitly distinguishes "permanent divergence" from the tool-sandbox-surface cluster's `DEFERRED->v3.7` treatment.
- `.planning/REQUIREMENTS.md` - `CORE-02` checkbox flipped to `[x]` with a note on the two-plan split; traceability table row `CORE-02 | Phase 111 | Complete`.

## Decisions Made
- **Note (part of D-01, confirmed not re-derived):** ADR-111's "why the carve-out doesn't extend" section grounds the distinction in *function* (does the code decide/enforce, or only observe/report?) rather than *location*, matching ADR-86's own stated boundary invariant ("the library still applies ONLY what clients explicitly add to `CapabilitySet`").
- **Ledger reconciliation approach:** rather than silently editing away the "Ledger closed" declaration, the closing sentence itself was amended in-place to point at the one named exception — preserving the historical record's integrity while making the addendum's legitimacy self-evident to a future reader who lands on that sentence first.
- **CORE-02 completion trigger:** per the 111-02 hand-off note, CORE-02 was left `[ ]` after 111-02 specifically so that landing 111-03 would be the single point of truth for flipping it — avoiding a premature "complete" claim that misrepresented phase progress, matching the Phase 110 PROF-03 precedent across its 4 sub-plans.

## Deviations from Plan

None — plan executed exactly as written. Both tasks' automated verification greps (both SHAs, `pub mod resource`, the addendum heading, the ADR cross-reference) all pass by construction, and the manual/editorial acceptance criteria (does the ADR *answer* rather than *assert* the ADR-86 question; does the addendum *distinguish* rather than *conflate* permanent-vs-deferred) were satisfied in the authored prose per this plan's own acceptance criteria.

## Issues Encountered

**`proj/` directory is gitignored but individually tracked files require `git add -f`.** Discovered when `git status --short` showed no changes after writing the new ADR file. Confirmed via `git check-ignore -v` (`.gitignore:16:proj/`) and cross-checked against the existing `ADR-108` commit history, which also used a force-add. This matches the durable lesson already recorded in project memory for `docs/cli/development/` — both directories are gitignored wholesale but carry individually force-added tracked files. No plan or process change needed; `git add -f` was used for Task 1's commit.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- `CORE-02` is now fully satisfied (both 111-02 and 111-03 landed) — Wave 1 of Phase 111 is complete pending confirmation the other Wave 1 plans (111-01, 111-02) are also committed.
- The next upstream sync auditor encountering `e6d26871`/`34c2c975` (or any future upstream commit touching the `resource` module) will find `proj/ADR-111-resource-limits-boundary.md` and the `108-DIVERGENCE-LEDGER.md` addendum as the authoritative, pre-settled disposition — no re-litigation needed.
- Wave 2 (`111-04`, VERIFY-01 combined 108-111 fork-invariant pass) can proceed; this plan touched no source code, so it introduces no new cross-target clippy surface for that pass to cover.

---
*Phase: 111-core-carry-resource-cli-verify-release-leapfrog*
*Completed: 2026-08-04*

## Self-Check: PASSED

- FOUND: `proj/ADR-111-resource-limits-boundary.md`
- FOUND: `.planning/phases/111-core-carry-resource-cli-verify-release-leapfrog/111-03-SUMMARY.md`
- FOUND: commit `10194152` (Task 1)
- FOUND: commit `6bf65687` (Task 2)
- FOUND: commit `a56fbfb4` (SUMMARY + REQUIREMENTS metadata commit)
