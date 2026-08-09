---
phase: 116-tool-sandbox-divergence-audit-disposition-adr
plan: 03
subsystem: git-archaeology / upstream-fork divergence audit
tags: [upstream-sync, tool-sandbox, divergence-ledger, D-04, residue-accounting, symbol-level-verification]

# Dependency graph
requires:
  - phase: 116-01
    provides: "116-DIVERGENCE-LEDGER.md with the Reproduction block, D-03 module-set re-derivation,
      and pre-fence/post-fence per-commit disposition tables (7 pre-fence, 11 post-fence commits)
      this plan appends full D-04 residue accounting to"
  - phase: 108-upst12-divergence-audit
    provides: "the 11-commit tool-sandbox-split fenced-window residue tables (with routing-note
      absorb claims) this plan grep-verifies against fork HEAD"
provides:
  - "116-DIVERGENCE-LEDGER.md gains a `## Split-Commit Residue Accounting (Pre-Fence + Post-Fence)`
    section: full absorb/defer/noise bucketing for all 7 split commits Plan 116-01 flagged (161
    touched-path rows total, zero unbucketed, every row spot-checked against a live
    `git show --name-only` count)"
  - "a `## Fenced-Window Residue Reconciliation (grep-verified against fork HEAD)` section:
    Phase 108's 31 fenced-window absorb-marked paths (from the 4 D-05-named worked-example split
    commits) each verified by a live grep against fork HEAD rather than accepted from 108's
    routing-note prose — 14 landed, 17 not-landed"
  - "a `## Post-Fence Residue Finding` section disposing the 3 unresolved post-fence feature items
    as module-scoped extensions of the still-deferred tool-sandbox subsystem, proposed (not
    applied) per D-04's operator-gated shape"
affects: [116-04, 116-05, 116-06, proj/ADR-116-tool-sandbox-disposition.md]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "D-16 grep-not-prose reconciliation: a routing note's phase-number citation is treated as an
      intent to verify, not proof of an outcome — every absorb claim gets a fresh live grep against
      fork HEAD, with the exact command and hit count recorded"
    - "Symbol-name-collision detection: a grep hit on the claimed symbol name is not sufficient —
      the ea334d2b musl-fix finding shows a fork-native, unrelated feature (WSL2 9P-mount
      detection) can reuse identical function/const names for a different purpose, producing a
      false-positive 'landed' verdict if only presence (not the actual retyped signature) is
      checked"

key-files:
  created: []
  modified:
    - .planning/phases/116-tool-sandbox-divergence-audit-disposition-adr/116-DIVERGENCE-LEDGER.md

key-decisions:
  - "d5803b99 (#1398, PROF-03 port-range support) and d4927f95 (#1298, PROF-02 @git:* token
    expansion): all 14 absorb-marked paths verdict `landed` — both features exist in the fork today,
    matching 108's routing-note claims symbol-for-symbol (one benign field-rename exception,
    `localhost_range` vs. `localhost_port_ranges`, noted explicitly)."
  - "ea334d2b (#1332, musl build fix): verdict `not-landed` — a GENUINE false-positive grep trap.
    The fork's `V9FS_MAGIC`/`fs_type_unsupported` symbols exist but are a coincidentally-identically-
    named, independently-developed, unrelated fork-native feature (WSL2 9P-filesystem detection);
    the actual musl-specific `libc::c_long`→`u64` retype was never applied, and the fork has no
    musl cross-compilation target at all. A naive presence-only grep would have wrongly reported
    `landed`."
  - "8a4237f2 (#1283, SeccompPolicy struct refactor, 16 absorb paths): verdict `not-landed` across
    all 16 paths — independently corroborated by the fork's own pre-existing Phase 112 test comment
    (`crates/nono/src/sandbox/linux.rs:4533-4538`), which explicitly names this exact SHA as never
    absorbed. This is the strongest-evidence finding in the reconciliation: a live grep AND the
    fork's own prior-recorded documentation agree."
  - "Post-Fence Residue Finding: the 3 remaining unresolved post-fence feature items (OIDC
    vault-login mediation, intercept-match-predicate refinement, JWT-phantom capture nonces) are
    all module-scoped refinements of the tool-sandbox subsystem itself, with no independent value
    absent the base subsystem landing first — same shape Task 1's residue tables already
    established for most non-absorb split-commit residue. Disposed as 'no new successor phase
    needed; natural home is whatever future work executes this ADR's verdict or a future
    UPST13/FUT-08 sync' — a proposal only, zero ROADMAP.md/REQUIREMENTS.md edits."
  - "Reconciled a genuine hypothesis-vs-measurement discrepancy (D-17): 116-01's Pre-Fence table
    prose said c808f000 touches '29 paths' and 11fd10e0 touches '90 paths'; live re-measurement
    via `git show --name-only --format='' <sha> | wc -l` found 31 and 91 respectively. Both deltas
    trace to the prose summary's comma-list omitting a few noise-bucket paths (e.g. `cliff.toml`,
    `Cargo.lock`), not a disagreement about which commits are split or a miscounted `git log`."

patterns-established:
  - "Residue-table row-count sum-check recorded per commit (touched-path count from a fresh
    `git show --name-only` run, stated immediately above each table, compared against the table's
    own row count) — makes silent path loss structurally detectable per T-116-03's threat-model
    mitigation."

requirements-completed: [TSBX-01]

# Metrics
duration: ~55min
completed: 2026-08-09
---

# Phase 116 Plan 03: Split-Commit Residue Accounting + Fenced-Window Reconciliation Summary

**Appended full D-04 residue accounting for all 7 pre-fence/post-fence `split` commits (161
touched-path rows, zero unbucketed) plus a grep-verified fenced-window reconciliation of Phase
108's 31 absorb-marked paths against fork HEAD — 14 landed, 17 not-landed, including a genuine
false-positive symbol-name-collision catch (ea334d2b's musl fix) and an independently-corroborated
non-absorption finding (8a4237f2's SeccompPolicy refactor, confirmed by the fork's own Phase 112
code comments) — to `116-DIVERGENCE-LEDGER.md`.**

## Performance

- **Duration:** ~55 min
- **Completed:** 2026-08-09
- **Tasks:** 2 (Task 1: pre-fence/post-fence split-commit residue accounting; Task 2: fenced-window
  grep-verified reconciliation + Post-Fence Residue Finding)
- **Files modified:** 1 (`116-DIVERGENCE-LEDGER.md`, append-only — no existing content removed or
  renumbered)

## Accomplishments

- Built 7 full per-commit residue tables (2 pre-fence: `c808f000` 31 paths, `11fd10e0` 91 paths;
  5 post-fence: `ce3e5101` 12, `65163c6a` 8, `35af3417` 8, `6a63b424` 9, `f76733f6` 2) — 161 total
  rows, every row bucketed exactly one of `absorb`/`defer`/`noise`, every commit's row count
  spot-checked equal to a live `git show --name-only --format='' <sha> | wc -l` re-run.
- Recorded and reconciled a genuine hypothesis-vs-measurement discrepancy: 116-01's prose said
  "29 paths"/"90 paths" for the two pre-fence split commits; live re-measurement found 31/91 —
  explained (noise-bucket paths the prose summary omitted from its comma-list), not silently
  corrected.
- Grep-verified all 31 of Phase 108's fenced-window `absorb`-marked paths (across the 4 D-05-named
  worked-example split commits — `d5803b99`, `ea334d2b`, `d4927f95`, `8a4237f2`; the other 7 split
  commits carry zero absorb rows per 108's own finding, reused not re-derived) against fork HEAD:
  14 `landed` (both PROF-02 and PROF-03 fully landed), 17 `not-landed`.
- Caught a genuine false-positive grep trap: `ea334d2b`'s claimed musl build fix
  (`V9FS_MAGIC`/`fs_type_unsupported` `libc::c_long`→`u64` retype) shows a grep hit on the same
  symbol names in the fork today, but reading the code shows it is an unrelated, independently
  developed WSL2 9P-mount-detection feature (confirmed via `git log -S`/commit-message search) —
  the actual musl-compat fix never landed, and the fork has no musl cross-compilation target at
  all (`Cross.toml`/CI workflows checked directly).
- Found independent corroboration for `8a4237f2`'s (`SeccompPolicy` refactor) non-absorption: a
  pre-existing Phase 112 test comment in `crates/nono/src/sandbox/linux.rs` already explicitly
  names this exact SHA (`fa21a004/8a4237f2, #1283`) as never absorbed — a live grep and the fork's
  own prior documentation agree, the strongest-evidence finding in this plan.
- Built the Post-Fence Residue Finding: of the 4 feature-bearing post-fence `absorb`-marked paths,
  1 (`ce3e5101`'s non-UTF-8-arg handling) has independent fork coverage; the other 3 are
  module-scoped tool-sandbox refinements with no value absent the base subsystem — disposed as "no
  new successor phase needed," a proposal only, with the required literal "requires operator
  approval" sentence and zero `ROADMAP.md`/`REQUIREMENTS.md` edits (verified via
  `git diff --stat`).

## Task Commits

Both tasks build the same file incrementally (Task 2's fenced-window reconciliation table
structurally depends on Task 1's residue tables having already established which split commits
carry `absorb`-marked rows to reconcile), and were authored end-to-end in one continuous
evidence-gathering pass, matching the exact shape Plan 116-01 already used and documented as a
deviation for the same reason:

1. **Task 1 + Task 2: Append Split-Commit Residue Accounting, Fenced-Window Residue
   Reconciliation, and Post-Fence Residue Finding to `116-DIVERGENCE-LEDGER.md`** - `008e1fd9`
   (docs)

**Plan metadata:** this SUMMARY.md's own commit (below)

_Note: unlike typical per-task commits, this plan's two tasks incrementally extend one shared
ledger file with tightly coupled evidence (Task 2 re-uses Task 1's residue tables to identify
which split commits have absorb rows to verify); splitting into two partial commits would have
required reconstructing an artificial intermediate state with no independent review value. This
mirrors Plan 116-01's own documented deviation. Both tasks' acceptance-criteria greps pass against
the single resulting commit (verified below)._

## Files Created/Modified

- `.planning/phases/116-tool-sandbox-divergence-audit-disposition-adr/116-DIVERGENCE-LEDGER.md` -
  Appended (all of Plan 116-01's existing sections preserved intact, nothing renumbered or
  removed): `## Split-Commit Residue Accounting (Pre-Fence + Post-Fence)` (7 per-commit tables,
  161 rows), `## Fenced-Window Residue Reconciliation (grep-verified against fork HEAD)` (4
  per-commit reconciliation tables + summary, 31 rows), `## Post-Fence Residue Finding` (disposal
  table + proposal-only closing statement), and an updated closing `Ledger status` line.

## Decisions Made

See `key-decisions` in frontmatter for the 5 load-bearing findings (PROF-02/PROF-03 landed,
ea334d2b's false-positive symbol collision, 8a4237f2's independently-corroborated non-absorption,
the Post-Fence Residue disposition, and the pre-fence row-count reconciliation).

## Deviations from Plan

None affecting correctness or scope. One process note (matching Plan 116-01's own precedent):

**Single commit covering both Task 1 and Task 2.** Documented above under Task Commits — both
tasks incrementally build the same ledger file with a direct data dependency (Task 2 reuses which
split commits Task 1's tables show carry absorb rows), so they were authored and committed as one
unit. All acceptance-criteria automated-verify greps for both tasks pass against the single
resulting file (verified below).

## Issues Encountered

None blocking. Two findings worth flagging as genuinely surprising (not blockers, but the exact
kind of result this plan's Task 2 design anticipates):

1. `ea334d2b`'s claimed musl-fix symbol names exist in the fork today for an entirely unrelated
   reason (WSL2 detection) — a naive "does the symbol exist" check would have wrongly reported
   `landed`. Caught by reading the actual type signature (`libc::c_long`, not the claimed `u64`
   retype) and cross-checking `git log -S`/commit messages for the symbol's actual origin.
2. `8a4237f2`'s non-absorption was independently confirmed by a pre-existing Phase 112 code comment
   that names the exact SHA — found while reading `crates/nono/src/sandbox/linux.rs` for the
   `apply_external()`/`apply_landlock` grep, not searched for deliberately; a useful reminder that
   fork code comments can themselves be D-16-grade evidence, sometimes stronger than a fresh grep.

## User Setup Required

None - no external service configuration required. This is a documentation-only plan (D-20: zero
`.rs`, zero `Cargo.toml`, `ROADMAP.md`/`STATE.md`/`REQUIREMENTS.md` untouched by this executor —
`git diff --stat -- .planning/ROADMAP.md .planning/REQUIREMENTS.md` confirmed empty; per this
project's CLAUDE.md/executor-prompt constraint, `.planning/STATE.md` and `.planning/ROADMAP.md`
were NOT touched by this executor, those are the orchestrator's to update after this plan
completes).

## Next Phase Readiness

- `116-DIVERGENCE-LEDGER.md` now has complete D-04 residue accounting for every pre-fence/
  post-fence split commit and a grep-verified (not routing-note-prose-accepted) fenced-window
  reconciliation — the two must-have truths this plan's frontmatter states are both satisfied,
  with zero unbucketed paths and every fenced-window verdict backed by a recorded command/hit
  count.
- **Not yet done (explicitly out of scope for this plan, per its own objective statement and
  116-01-PLAN.md's original scope split):** any further D-03-adjacent module-set candidate
  re-touch check or completeness sweep is Plan 116-04's job, if named in that plan.
- No blockers. `upstream` did not move during this plan's execution (all grep/verification work
  ran against fork HEAD, which is stable and not a moving target for this plan's purposes).
- The Post-Fence Residue Finding's disposition ("no new successor phase needed") is itself a
  proposal, not a decision — it requires no operator action unless the operator disagrees with the
  "these 3 items are module-scoped tool-sandbox refinements, not standalone features" reasoning.

---
*Phase: 116-tool-sandbox-divergence-audit-disposition-adr*
*Completed: 2026-08-09*
