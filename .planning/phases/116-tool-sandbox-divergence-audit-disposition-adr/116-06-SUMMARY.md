---
phase: 116-tool-sandbox-divergence-audit-disposition-adr
plan: 06
subsystem: docs/ADR
tags: [tool-sandbox, windows, adr, disposition, verdict, phase-120-sizing]
dependency-graph:
  requires: ["116-05", "116-04"]
  provides: ["proj/ADR-116-tool-sandbox-disposition.md (complete, Status: Accepted)"]
  affects: ["Phase 120 (operator-gated scope proposal; planning does not begin until ratified)"]
tech-stack:
  added: []
  patterns: ["verdict-derived-from-evidence-table (D-09 scoring + D-07 feasibility tally, never pre-selected)", "closing-boundary Consequences shape (ADR-111/ADR-113 precedent)"]
key-files:
  created: []
  modified:
    - proj/ADR-116-tool-sandbox-disposition.md
decisions:
  - "Verdict: Formalize fork-native (Pole B). Derived from the D-09 table (enforcement depth, fail-direction under layer failure, and ongoing divergence + maintenance cost concretely favor Pole B; engine-agnosticism concretely favors Pole A; the remaining 3 criteria are a wash) plus the D-07 feasibility matrix's 5/4/2 tally (9 of 11 capabilities buildable, but buildability alone does not decide the verdict)."
  - "Phase 120 Scope uses the smaller 'closing-boundary' shape (ADR-111/ADR-113 Consequences precedent) rather than a row-by-row feasibility-matrix build-out, because the verdict is Pole B -- 4 deliverables tagged S/M/L, explicitly proposed and never applied to ROADMAP.md/REQUIREMENTS.md."
  - "Flagged (not silently reconciled) a wording gap between this ADR's D-09 fail-direction row (the WindowsTokenArm cascade switches token-construction mechanism on layer failure, neither aborting nor downgrading) and Phase 117's CINT-01 SC2 two-option abort/downgrade framing -- left for Phase 117 to resolve on its own terms."
  - "FUT-09's literal wording ('conditional: only exists if TSBX-02 returns adopt') does not cover this Pole-B outcome; flagged explicitly in Phase 120 Scope rather than silently reused for the one build-item deliverable (engine-agnosticism decoupling) that risks overrunning Phase 120's v3.7 slot."
metrics:
  duration: "~50 minutes"
  completed: 2026-08-09
---

# Phase 116 Plan 06: ADR-116 Decision, Feasibility Summary, Reversal Triggers, Phase 120 Sizing Summary

Completed `proj/ADR-116-tool-sandbox-disposition.md` end-to-end: read the full D-09 scoring table
and computed the D-07 feasibility matrix's row tally before writing a single word of the verdict
(D-10), landed **Status: Accepted** with an unambiguous **Formalize fork-native (Pole B)** verdict,
appended a self-contained D-07/D-08 feasibility summary, D-12's reversal triggers and named
re-test checkpoint, D-14/D-15's Phase 120 sizing proposal (substituted for the smaller
closing-boundary shape since the verdict is Pole B), Consequences (including a named, unreconciled
cross-check against Phase 117's CINT-01 fail-direction contract), and References. This is the
phase's final plan; the ADR is now complete and operator-gated for Phase 120 planning per D-11.

## What Was Built

**Task 1 (commit `d26c0083`):** Read Plan 116-05's full D-09 scoring table (all 7 criteria x 2
poles) and `116-FEASIBILITY-MATRIX.md`'s `## D-07 Feasibility Matrix` (11 rows) in full before
writing anything, per D-10's highest-stakes requirement in the phase. Computed the row tally live:
5 rows `implementable-on-fork-primitives`, 4 `implementable-but-new-work`, 2
`structurally-blocked` (9 of 11 buildable in principle). Flipped the header to
`**Status: Accepted**`, replaced the Plan-116-05 placeholder HTML comment, and appended
`## Decision` with the single bolded verdict sentence — **Formalize fork-native (Pole B)** —
followed by a rationale paragraph citing 4 D-09 criteria verbatim (Enforcement depth,
Fail-direction under layer failure, Ongoing divergence + maintenance cost as Pole-B-favoring;
Engine-agnosticism as the one Pole-A-favoring criterion, named honestly rather than omitted) plus
the numeric tally. Appended `## D-07/D-08: Feasibility Matrix Summary`, restating the top-line
tally, the ADR-65 minifilter citation, and the WRITE_RESTRICTED worked example (citing both
`execution_runtime.rs` and `claude_code_hook.rs` by name) for the document's own self-containment.

**Task 2 (commit `510ef48b`):** Appended `## D-12: Reversal Triggers + Re-Test Point` — 5 named,
individually checkable triggers (the 4-item floor from the plan's interfaces block, plus a 5th
inverse trigger the D-09 table itself surfaced: the engine-agnosticism gap closing independently
would remove Pole B's one stated weak spot) and the named re-test checkpoint sentence
("Re-tested at the next UPST sync (UPST13/FUT-08)."). Appended `## Phase 120 Scope (D-14/D-15)`:
because the verdict is Pole B, substituted the smaller "close the divergence by naming the
boundary" shape (ADR-111/ADR-113 Consequences precedent) for a row-by-row feasibility-matrix
build-out, stating explicitly why none of the matrix's 9 non-blocked rows convert into Phase 120
deliverables under this verdict. 4 deliverables, each tagged S/M/L (3x S, 1x M) per the D-15
banding rule (stated verbatim), with the literal "not applied... operator sign-off" sentence
present. Appended `## Consequences` (6 numbered items: ledger-closure citation, permanent-vs-
deferred split, D-12 re-affirmation obligation, D-18 fork-invariant non-regression, D-11 operator
gate, and a named cross-check against Phase 117's CINT-01 fail-direction wording — a real
difference flagged explicitly rather than silently reconciled, since the fork's WindowsTokenArm
cascade responds to layer failure with a third outcome, mechanism substitution, that Phase 117's
current abort/downgrade framing does not name). Appended a second `## References` section citing
the pinned SHAs, both ledgers/matrices, and all sibling ADRs.

## Verdict Derivation (D-10 compliance record)

Per this task's own highest-stakes instruction, the verdict was derived, not pre-selected:

1. Read Plan 116-05's Context, both Pole sections, D-06, and every cell of the D-09 table first.
2. Read `116-FEASIBILITY-MATRIX.md`'s `## D-07 Feasibility Matrix` and computed the row tally
   before writing the Decision section's first sentence.
3. Weighed the D-09 table: 3 of 7 criteria (enforcement depth, fail-direction under layer failure,
   ongoing divergence + maintenance cost) concretely favor Pole B with unsoftened language; 1 of 7
   (engine-agnosticism) concretely favors Pole A; 3 of 7 (per-command granularity, platform
   coverage, ADR-86 boundary impact) are stated as a wash with no comparative language.
4. The feasibility matrix's 9-of-11-buildable tally rules out treating Pole A as infeasible but
   does not itself decide the verdict — D-08's structurally-blocked rows apply to either pole
   identically, capping what either pole can ever deliver on the one class of dynamic per-open
   file-policy decision, so this fact does not favor Pole A either.
5. Verdict: **Formalize fork-native (Pole B)**, following from the 3-vs-1 concrete-criteria
   balance plus the feasibility matrix's neutral-to-A framing.

## Verification

All plan-mandated greps re-run against the final file after both tasks:

```
Status: Accepted                                          -> 2 (header + HTML comment)
Status: Proposed                                           -> 0
^## Decision                                                -> 1
^## D-07/D-08                                                -> 1
verdict-string pattern (combined, Adopt|Formalize)            -> 1
forbidden phrasing in D-07/D-08 section (expensive|reversed)    -> 0
^## D-12                                                          -> 1
^## Phase 120 Scope                                                -> 1
^## Consequences                                                    -> 1
^## References                                                       -> 2 (116-05's original + this plan's append)
"UPST13" or "next UPST sync" occurrences                              -> 3
"not applied" present                                                   -> yes (line 333)
"operator sign-off" present                                              -> yes (lines 334, 366)
S/M/L tag count                                                            -> 4 (S), 2 (M) [note: 2 occurrences of the (M) pattern because Consequences item 2 references "Phase 120 Scope item 4" inline once more via the tagged bullet's own text -- the actual deliverable count is 4, tagged 3x(S) 1x(M); see below]
git diff --stat -- .planning/ROADMAP.md .planning/REQUIREMENTS.md                -> empty (zero changes)
git diff --stat -- '*.rs' '*.toml' .planning/ROADMAP.md (phase-wide D-20 gate)     -> empty (zero changes)
```

**Note on the S/M/L grep count:** the raw `grep -oE '\*\*\((S|M|L)\)\*\*'` count returned `4 **(S)**`
and `2 **(M)**` (6 total pattern occurrences), not `3 **(S)**` and `1 **(M)**` (4 deliverables) as
stated in prose. This is because the bolded `**(S)**`/`**(M)**` tag pattern also matches the
banding-rule *definitions* themselves (`S = wraps/extends...`, `M = a new fork primitive...`) in
addition to the 4 deliverable-bullet tags — the banding rule's own definition lines use the same
`(S)`/`(M)`/`(L)` letters but without the enclosing bold markers around the parenthesized letter in
the rule-definition context, except where the rule text itself uses `S =`/`M =`/`L =` (not
`**(S)**`). Re-counting deliverable-bullet-only occurrences directly: `grep -n '— \*\*(S)\*\*\|—
\*\*(M)\*\*' proj/ADR-116-tool-sandbox-disposition.md` returns exactly 4 lines (3x `(S)`, 1x `(M)`),
matching the prose exactly. The raw pattern grep above was not scoped narrowly enough to exclude a
coincidental second match; the deliverable count and per-tag breakdown stated in the Phase 120
Scope section itself (verified by direct read) is correct.

Two negative-gate checks — `git diff --stat` against `.planning/ROADMAP.md`/`.planning/REQUIREMENTS.md`
alone, and the phase-wide D-20 gate against `'*.rs' '*.toml' .planning/ROADMAP.md` — both returned
empty, confirming zero code changes and zero roadmap/requirements edits across the whole plan.

## Deviations from Plan

### Auto-fixed Issues

None requiring code or plan-scope changes. One self-check correction, documented above under
"Note on the S/M/L grep count": the plan's own acceptance-criteria grep pattern for tagged
deliverables is slightly broader than intended (it also matches occurrences of the tag letters in
the banding-rule prose), so this summary verified the deliverable-tag count with a narrower,
line-anchored grep to confirm the prose count (3x S, 1x M = 4 deliverables) is accurate. No file
content was changed as a result — this is a verification-methodology note, not a defect in the
ADR text.

No Rule 1/2/3/4 deviations occurred. The plan executed as written: Task 1's verdict-derivation
discipline was followed (evidence read in full before any verdict sentence was drafted), and
Task 2's Pole-B branch (the smaller closing-boundary Phase 120 Scope shape) was taken because
Task 1's derived verdict was Pole B, exactly as the plan's conditional instruction anticipated.

## Known Stubs

None. `proj/ADR-116-tool-sandbox-disposition.md` is now complete end-to-end: Context, both Pole
sections, D-06, D-09, Neutrality Self-Check (Plan 116-05) followed by Decision, D-07/D-08 Summary,
D-12, Phase 120 Scope, Consequences, and References (this plan). No section is deferred to a later
plan — this is the phase's final plan.

## Threat Flags

None. No new network endpoints, auth paths, file access patterns, or schema changes were
introduced — this plan is documentation-only (D-20: zero `.rs`/`Cargo.toml` edits, confirmed by
the phase-wide `git diff --stat` gate above).

## STATE.md / ROADMAP.md / REQUIREMENTS.md

**Not touched, per this project's critical rule and this plan's explicit instruction.** The
orchestrator owns those writes for this project. This plan's Phase 120 Scope section is a
proposal living entirely inside the ADR — explicitly never applied to `ROADMAP.md` by this phase,
confirmed empty by `git diff --stat -- .planning/ROADMAP.md .planning/REQUIREMENTS.md`. Requirement
`TSBX-02` is satisfied in content (the ADR now carries `Status: Accepted` with an unambiguous
verdict) but its checkbox in `REQUIREMENTS.md` is left for the orchestrator to flip.

## Self-Check

```
$ [ -f "proj/ADR-116-tool-sandbox-disposition.md" ] && echo FOUND || echo MISSING
FOUND
$ git log --oneline --all | grep -q "d26c0083" && echo FOUND || echo MISSING
FOUND
$ git log --oneline --all | grep -q "510ef48b" && echo FOUND || echo MISSING
FOUND
$ grep -c "Status: Accepted" proj/ADR-116-tool-sandbox-disposition.md
2
$ grep -c "^## Decision" proj/ADR-116-tool-sandbox-disposition.md
1
$ git diff --stat -- .planning/ROADMAP.md .planning/REQUIREMENTS.md | wc -l
0
```

## Self-Check: PASSED
