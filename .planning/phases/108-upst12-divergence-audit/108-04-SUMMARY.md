---
phase: 108-upst12-divergence-audit
plan: 04
subsystem: infra
tags: [upstream-sync, divergence-audit, tool-sandbox, residue-accounting, deps-audit, ci-audit, docs-audit]

requires:
  - phase: 108-01
    provides: "100-commit accounting, DEPS/CI/DOCS bucket SHA lists (19/11/8), tool-sandbox 20-commit surface, D-06 directory-vs-union re-measurement"
  - phase: 108-03
    provides: "NET/PROF/CORE per-commit tables; PROF-02/PROF-03 pointer rows for d4927f95/d5803b99"
provides:
  - "tool-sandbox-pure table (9 commits) and tool-sandbox-split residue tables (11 commits, 95 total path rows, zero unbucketed) — full D-07 accounting for the 20-commit tool-sandbox surface"
  - "Independent pure/split reconciliation: 9 pure / 11 split, confirming the plan's own reconnaissance and superseding CONTEXT.md D-05's unenumerated '12 entangled' estimate"
  - "All 7 named tool-sandbox refinement PRs explicitly marked DEFERRED->v3.7"
  - "DEPS Cluster table (19 rows) with cargo-audit cross-reference — flags crossbeam-epoch RUSTSEC-2026-0204 as unabsorbed"
  - "CI Cluster table (11 rows) and DOCS Cluster table (8 rows), each individually reviewed against the fork's actual current files"
affects: [110-prof-absorb, 111-core-absorb-and-release, v3.7-tool-sandbox-parity, proposed-phase-112]

tech-stack:
  added: []
  patterns: ["D-07 per-path residue accounting (absorb/defer/noise) with row-count-equals-touched-path-count verification", "cross-checking cluster commits against the fork's actual current files (ls/grep), not just commit subject text"]

key-files:
  created:
    - .planning/phases/108-upst12-divergence-audit/108-04-SUMMARY.md
  modified:
    - .planning/phases/108-upst12-divergence-audit/108-DIVERGENCE-LEDGER.md

key-decisions:
  - "Independently re-derived pure/split classification via git show --name-only for all 20 SHAs rather than trusting either CONTEXT.md's '12 entangled' aggregate or the plan's own reconnaissance blindly; the result (9 pure / 11 split) matched the plan's reconnaissance exactly and is recorded as the authoritative, reproducible figure, with e2c87fd5 named as the most plausible source of the 12-vs-11 gap (touches 2 non-module paths that a coarse heuristic would likely flag as entangled, but both are non-production per this task's own rule)."
  - "3 of the 7 named refinement PRs (#1322/#1384/#1417) are split commits, not pure, contradicting a literal reading of the plan's acceptance criteria ('all 7 named PRs appear in the tool-sandbox-pure table'). Resolved by marking all 7 DEFERRED->v3.7 — 4 as full pure-table rows, 3 via an explicit supplementary table noting their module-scoped portions defer while their non-module paths get separate split-residue accounting. Blanket-listing all 7 as pure would have silently misrepresented 3 commits as fully deferred when they in fact also carry absorbable-or-wiring content — the exact 'blanket defer' anti-pattern D-05 forbids."
  - "7 of the 11 split commits (a519ee62, 72a98830, 1f54f4ae, 7c20dc75, eb2d61a7, 5a7447d3, ebd51cbb) were found to have ZERO absorb-worthy content despite being structurally split — every non-module path in them is small wiring (module registration, field threading, or a helper whose only caller lives inside the deferred subsystem), verified by reading each diff rather than assuming production-path-outside-module-set implies independent value. Recorded explicitly as a finding rather than silently marking them absorb by the letter of the rule."
  - "d4927f95's PROF-02 absorb target (capability_ext.rs) is flagged with a load-bearing cross-dependency: it calls tool_sandbox::dynamic_providers::expand_dynamic_tokens, a function defined in the deferred module set. Phase 110 cannot cherry-pick PROF-02 as a clean standalone absorb without either porting a minimal expand_dynamic_tokens shim or explicitly scoping PROF-02 down."
  - "cargo-audit cross-reference found crossbeam-epoch 0.9.18 (RUSTSEC-2026-0204, vulnerability not just warning) live in the fork's current Cargo.lock, with the exact fix already sitting unabsorbed in the DEPS bucket (373a67ae, #1369). Flagged as priority-absorb rather than routine noise."
  - "CI/DOCS cluster review used direct inspection of the fork's actual current .github/workflows/ and docs/ files (ls + grep), not commit-subject inference — this caught that the fork's sign-instruction-files.yml pins a differently-named/differently-versioned action (always-further/agent-sign, not nolabs-ai/agent-sign) than the one upstream bumps, and that the fork's SHA256SUMS.txt generation already avoids the ./-prefix bug 002fe498 fixes via a structurally different script."

patterns-established:
  - "Per-split-commit residue tables cite absorb targets by verified diff content (not touched-path-name inference alone) — e.g. capability_ext.rs's PROF-03 absorb was confirmed by reading the actual mechanism (Landlock NetPort/Seatbelt port-range emission), not assumed from the file being outside the module set."
  - "CI/DOCS adapt-reasoning and reconcile-note cells are grounded in the fork's live filesystem state (ls .github/workflows/, grep for action references) rather than the commit's own diff content alone, since 'does the fork even have this file/mechanism' is the deciding factor for adapt-vs-conflict."

requirements-completed: [UPST12-01]

duration: ~2h
completed: 2026-07-29
---

# Phase 108 Plan 04: Tool-Sandbox Residue Accounting + DEPS/CI/DOCS Clusters Summary

**Full D-07 per-path residue accounting for the 20-commit tool-sandbox surface (9 pure / 11 split, 95 residue rows, zero unbucketed paths), all 7 named refinement PRs marked DEFERRED->v3.7, plus individually-reviewed DEPS/CI/DOCS cluster tables that surface a live unabsorbed RUSTSEC fix (crossbeam-epoch) in the fork's current Cargo.lock.**

## Performance

- **Duration:** ~2h
- **Tasks:** 2
- **Files modified:** 1 (`108-DIVERGENCE-LEDGER.md`, two Edit appends totaling ~560 lines, plus a follow-up heading-format fix)

## Accomplishments

- Independently re-derived the tool-sandbox pure/split classification for all 20 SHAs via
  `git show --name-only --format=''`, landing on **9 pure / 11 split** — matching the plan's own
  "Planner-found" reconnaissance exactly and superseding CONTEXT.md D-05's unenumerated "12
  entangled" aggregate estimate. Named `e2c87fd5` as the most plausible source of the 12-vs-11 gap.
- Wrote a `## tool-sandbox-pure` table (9 rows, 4 of the 7 named refinement PRs) and a full D-07
  residue table for each of the 11 split commits — **95 total touched-path rows, zero unbucketed**,
  every commit's row count spot-checked against `git show --name-only --format='' <sha> | wc -l`.
- Read the actual diffs (not just file paths) for every non-module path in every split commit to
  determine genuine `absorb` vs. wiring-only `defer` — found that 7 of 11 split commits carry
  **zero** absorb-worthy content (pure wiring for the deferred subsystem), while the 4 D-05-named
  worked examples (`d5803b99`, `ea334d2b`, `d4927f95`, `8a4237f2`) each carry substantial,
  independently-valuable absorb content (PROF-03, a portable musl fix, PROF-02, and a general
  seccomp-policy-selection mechanism respectively).
- Ran the mandatory re-export scan for `8a4237f2`'s `bindings/c/src/sandbox.rs` row: **Clean** —
  a 1-line call-site rename, zero `pub mod`/`pub use`/`extern crate` anywhere in the 18-file diff
  (only 4 intra-crate `pub(crate)` fields).
- Flagged a load-bearing cross-dependency: `d4927f95`'s PROF-02 absorb target
  (`capability_ext.rs`) calls `tool_sandbox::dynamic_providers::expand_dynamic_tokens`, itself
  part of the deferred module set — Phase 110 cannot treat PROF-02 as a clean standalone
  cherry-pick without resolving this.
- Ran `cargo audit` against the fork's live `Cargo.lock`: found `crossbeam-epoch 0.9.18`
  (RUSTSEC-2026-0204, a real vulnerability, not just an unmaintained warning) still present, with
  its exact fix (`373a67ae`, #1369, DEPS bucket) sitting unabsorbed in this window — flagged as
  priority-absorb.
- Individually reviewed all 11 CI-bucket commits against the fork's actual
  `.github/workflows/` inventory (not commit-subject text): 7 portable, 4 fork-specific-conflicting
  (differently-named/versioned signing action, unused GitHub attestation action, architecturally
  different homebrew mechanism, upstream-org-specific Projects v2 board automation).
- Individually reviewed all 8 DOCS-bucket commits against the fork's actual `docs/` tree and root
  files: 5 needs-doc-follow-up, 3 safe-to-ignore (fork carries no community-governance files at
  all, being solo-maintained).
- Confirmed `ea334d2b` (musl fix) is correctly excluded from the CI cluster table (it is a
  tool-sandbox-split CODE-bucket commit, already fully accounted for in Task 1).

## Task Commits

1. **Task 1: tool-sandbox-pure/split residue accounting** - `43a00d2e` (docs)
2. **Task 2: DEPS/CI/DOCS cluster tables + cargo-audit** - `57689099` (docs, includes a follow-up
   heading-format correction in the same commit's diff review pass — see Deviations)

**Plan metadata:** this commit (docs: complete plan, includes SUMMARY.md)

## Files Created/Modified

- `.planning/phases/108-upst12-divergence-audit/108-DIVERGENCE-LEDGER.md` - Appended
  `## tool-sandbox-pure` (9 rows), `## tool-sandbox-split` (11 per-commit residue tables, 95
  rows), `## DEPS Cluster` (19 rows + cargo-audit findings), `## CI Cluster` (11 rows),
  `## DOCS Cluster` (8 rows), and `## Bucket-Count Reconciliation`.

## Decisions Made

1. **Pure/entangled reconciliation resolved to 9/11, not CONTEXT's 8/12.** CONTEXT.md D-05
   provides no per-SHA enumeration for its "12" figure. Independent re-derivation matches the
   plan's own reconnaissance exactly; `e2c87fd5` is named as the most plausible source of the gap
   (2 non-module paths — a schema file and a test file — that a coarse heuristic would likely
   flag as entangled, but both are non-production per this task's rule).
2. **All 7 named refinement PRs marked DEFERRED->v3.7, but not all listed as "pure."** 3 of the 7
   (#1322/#1384/#1417) are split commits. Rather than force them into the pure table (which the
   plan's acceptance-criteria phrasing might literally suggest), they get a supplementary
   explicit-DEFERRED note plus full split residue tables — avoiding the exact "blanket defer"
   failure mode D-05 forbids.
3. **7 of 11 split commits found to have zero absorb-worthy content.** Verified by reading each
   non-module path's actual diff, not inferring from touched-path-outside-module-set alone. This
   is recorded as a finding distinguishing the "structurally split" commits from the 4 D-05 worked
   examples that have genuine independent value.
4. **Heading format corrected mid-task (Rule 1 — bug).** See Deviations below.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Section headings written as nested `### `## X`` instead of literal `## X` at line start**
- **Found during:** Task 2, immediately after writing the DEPS/CI/DOCS content
- **Issue:** The plan's `<verify>` block for both tasks uses anchored regexes
  (`grep -c "^## DEPS Cluster\|^## CI Cluster\|^## DOCS Cluster"` and
  `grep -c "DEFERRED->v3.7"`). I initially wrote the section headers as `### \`## tool-sandbox-pure\``
  / `### \`## DEPS Cluster\`` (a level-3 heading with the literal target text quoted in backticks,
  following the plan's own literal phrasing "write a `## DEPS Cluster` table") — this does not
  match `^## DEPS Cluster` because of the leading `### \`` characters.
- **Fix:** Changed all 6 headings (`tool-sandbox-pure`, `tool-sandbox-split`, `DEPS Cluster`,
  `CI Cluster`, `DOCS Cluster`, `Bucket-Count Reconciliation`) to literal `## ` level-2 headings
  matching the verify regex exactly.
- **Files modified:** `.planning/phases/108-upst12-divergence-audit/108-DIVERGENCE-LEDGER.md`
- **Verification:** `grep -c "^## DEPS Cluster\|^## CI Cluster\|^## DOCS Cluster"` returns 3;
  `grep -c "DEFERRED->v3.7"` returns 14. Both verify commands from the plan now pass.
- **Committed in:** `57689099` (Task 2 commit, fixed before commit — not a separate commit)

---

**Total deviations:** 1 auto-fixed (Rule 1 — a heading-format bug that would have failed both
tasks' automated `<verify>` checks).
**Impact on plan:** Necessary for correctness — no scope change, pure formatting fix caught and
corrected before committing.

## Issues Encountered

None beyond the heading-format fix documented above.

## User Setup Required

None - no external service configuration required. `cargo-audit` was already installed in this
environment (`cargo-audit-audit 0.22.1`).

## Next Phase Readiness

- **Phase 110 (PROF absorb):** has the full PROF-02 (`d4927f95`) and PROF-03 (`d5803b99`) residue
  tables with explicit absorb targets, plus the cross-dependency warning that PROF-02's
  `capability_ext.rs` calls into a deferred `tool_sandbox::dynamic_providers` function — must
  resolve before treating PROF-02 as a clean cherry-pick.
- **Phase 111 (CORE absorb):** inherits `8a4237f2`'s (#1283 seccomp refactor) 16-row absorb list as
  a CORE-cluster/Phase-111 residual item with a Clean re-export-scan finding for
  `bindings/c/src/sandbox.rs`, plus `ea334d2b`'s musl build fix (portable, no requirement ID).
- **v3.7 Windows Tool-Sandbox Parity milestone:** inherits the full 20-commit surface (9 pure + 11
  split, with the split commits' `defer`-marked module rows) rather than the 7-PR-level
  approximation, per D-09.
- **Whichever phase lands the DEPS cluster (currently unmapped per D-19):** should prioritize
  `373a67ae` (crossbeam-epoch bump) — it is the live fix for RUSTSEC-2026-0204, which the fork's
  `Cargo.lock` currently carries unpatched.
- **Phase 109 (NET absorb):** the DOCS cluster's `63c9589f` (SigV4 credential-injection doc) should
  land alongside NET absorb, not independently.
- No blockers for Plan 108-05 (roadmap amendment proposal for the security-residual-and-misc
  cluster and the proposed Phase 112).

---
*Phase: 108-upst12-divergence-audit*
*Completed: 2026-07-29*
