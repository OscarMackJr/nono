---
phase: 116-tool-sandbox-divergence-audit-disposition-adr
verified: 2026-08-09T19:25:50Z
status: passed
score: 18/18 must-haves verified
overrides_applied: 0
---

# Phase 116: Tool-Sandbox Divergence Audit + Disposition ADR Verification Report

**Phase Goal:** Produce a reproducible per-commit divergence audit of upstream's tool-sandbox
subsystem across the v0.64.1..v0.71.0 window, plus a neutral feasibility matrix, and land an
evidence-derived disposition ADR (`proj/ADR-116-tool-sandbox-disposition.md`).

**Verified:** 2026-08-09T19:25:50Z
**Status:** passed
**Re-verification:** No — initial verification

**Adversarial stance note:** This is a documentation-only (D-20) phase, but the artifacts make
dozens of factual claims (commit SHAs, hit counts, symbol names, line numbers) that are exactly
the kind of claim this repo's history shows audits get wrong at the symbol level (Phase 112: 3 of
~6 re-checked disposition rows wrong from "file exists" ≠ "symbol exists"). This verification did
not accept any SUMMARY.md narrative at face value — every numeric/symbol claim checked below was
independently re-run against live `git log`/`git show`/`grep` from a clean working tree, not read
off the SUMMARY files.

## Goal Achievement

### Observable Truths (must_haves across all 6 plans + roadmap contract)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Reproduction block's SHAs/count are independently re-runnable | ✓ VERIFIED | `git ls-remote --tags upstream v0.64.1 v0.71.0` live-reproduced `0551eba2`/`0055bf3c` verbatim; `git log --no-merges --oneline <range> -- <3-path-union>` live-reproduced **38** exactly |
| 2 | Module set re-derived from live grep, not inherited unexamined; 4 named candidates get explicit in-or-out calls | ✓ VERIFIED | Re-ran `main.rs` mod/use sweep: 91 raw lines / 84 `mod` lines / 82 after excluding 2 test-only — matches ledger's "82-module sweep" exactly. Re-ran the 4-candidate commit-history grep: **1** hit (`4cc0af2c`, `open_url_runtime.rs`), matching ledger's stated finding and its "moot for this ledger" conclusion |
| 3 | Every disputed number (D-01 hypothesis vs. measurement) reconciled in place, incl. no-delta case | ✓ VERIFIED | Independently re-ran full-window (38), pre-fence (7), post-fence (11), fenced (20), pathspec-hazard (37 vs 35) — all match the ledger's "no delta" claims exactly |
| 4 | Every fork Windows-confinement symbol in the matrix is independently re-greppable, incl. `pub(super)`-only surfaces | ✓ VERIFIED | Re-ran the broadened grep against `supervisor.rs` (23 hits, all `pub(super)`) and `dacl_guard.rs` (7 hits) — both match exactly; naive `pub\|pub(crate)` pattern independently confirmed to return 0 on `supervisor.rs`, reproducing the Pitfall-2 finding live |
| 5 | Every D-07 capability row rated with a citation to a re-greppable fork symbol or ADR-65's verdict, never file presence alone | ✓ VERIFIED | Spot-checked 4 cited symbols (`dynamic_tokens.rs` doc comment, `capability_ext.rs` cfg-gate split, `windows_wfp_contract.rs` `FWP_MATCH_RANGE`/`localhost_port_ranges`, `keystore.rs::load_secret_by_ref`/`load_secrets`) — all exist exactly as cited, including line-adjacent content |
| 6 | No matrix row states/implies a pole preference | ✓ VERIFIED | `grep -icE "expensive\|if ADR-65 (were\|is) reversed"` against matrix = 0; Neutrality Self-Check section present and consistent with a full read |
| 7 | Document states evidence date once, document-wide | ✓ VERIFIED | Single `**Date measured:** 2026-08-09` header, no second date field introduced |
| 8 | Every split-commit path bucketed exactly once (absorb/defer/noise), zero unbucketed | ✓ VERIFIED | Bucket-Count Reconciliation section shows Σ_bucketed=161=Σ_touched, re-verified against a fresh `git show --name-only \| wc -l` re-run recorded inline per commit |
| 9 | Fenced-window absorb claims verified by live grep against fork HEAD, not routing-note prose | ✓ VERIFIED | Spot-checked `8a4237f2`'s `not-landed` verdict against `crates/nono/src/sandbox/linux.rs` — fork's own pre-existing Phase 112 comment independently names this exact SHA as never absorbed, corroborating the grep-based verdict |
| 10 | Post-fence residue mapping to no phase named as an operator-gated proposal, never applied to ROADMAP/REQUIREMENTS | ✓ VERIFIED | `## Post-Fence Residue Finding` section present with literal "is not applied to... requires operator approval" sentence; `git diff --name-only` for the whole phase touches neither file |
| 11 | Every accumulated carve-out surface (Phase 108's 7 + ADR-108/111/113/114-named) gets an explicit clean/HIT result against the full window | ✓ VERIFIED | Re-ran 3 of 10 carve-out commands live (`exec_strategy_windows/` = 0 hits/clean; `audit.rs` = 3 hits matching cited SHAs; `resource/`+`resource.rs` = 2 hits matching cited SHAs) — all reproduce exactly |
| 12 | Bucket-count reconciliation shows both sums and equality explicitly | ✓ VERIFIED | `## Bucket-Count Reconciliation (D-16)` shows per-commit arithmetic and 161==161 |
| 13 | Finalized Headline restates D-01 reconciliation incl. no-delta case | ✓ VERIFIED | Headline (lines 13-86) contains "measured 38, hypothesis 38, no delta", disposition/windows-touch tallies, and pointers to all 3 sibling docs, closing "Ledger closed except for named exceptions" |
| 14 | Both poles equal structural depth; 74%-Unix-drivers framing neutral | ✓ VERIFIED | Context states the framing once, immediately followed by "This framing applies equally to evaluating both poles below; it is not evidence for either"; both Pole sections are one paragraph each, symbol-cited, no comparative language |
| 15 | All 7 D-09 criteria scored for both poles, symmetric, no weighting | ✓ VERIFIED | 7-row table, 14 non-empty cells, no score/weight/total column, zero comparative-language hits document-wide |
| 16 | Two named intermediates recorded as considered-and-rejected only | ✓ VERIFIED | D-06 has exactly 2 subsections; `command_policies` docs-only claim re-verified live in this session too (0 hits in `src/`, 6 in `profile-authoring-guide.md`) |
| 17 | ADR states exactly one unambiguous verdict with `Status: Accepted`, derived from reading the full evidence, not pre-selected | ✓ VERIFIED | `Status: Accepted` present, `Status: Proposed` absent; single `## Decision` with one bolded verdict; commit timestamps independently confirm 116-05's commits (18:27-18:28) predate 116-06's verdict commit (18:35) — evidence necessarily existed before the verdict was written |
| 18 | Reversal governed by ≥3 named triggers + 1 re-test point; Phase 120 sizing proposed, never applied to ROADMAP | ✓ VERIFIED | D-12 has 5 named triggers + "next UPST sync (UPST13/FUT-08)"; Phase 120 Scope section present, explicitly "not applied... operator sign-off"; `git diff --stat -- .planning/ROADMAP.md .planning/REQUIREMENTS.md` for the whole phase is empty |

**Score:** 18/18 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `116-DIVERGENCE-LEDGER.md` | Full per-commit ledger, `v0.64.1..v0.71.0`, windows-touch + disposition + residue + carve-out + closure | ✓ VERIFIED | 1286 lines; all 15 required `## ` sections present; every spot-checked figure (SHA lists, counts, carve-out results) independently reproduces |
| `116-FEASIBILITY-MATRIX.md` | Fork primitive inventory + D-07 capability rating, neutral | ✓ VERIFIED | 447 lines; 151-symbol inventory (spot-checked `supervisor.rs`=23, `dacl_guard.rs`=7 both exact); 11-row D-07 table (5/4/2 split verified by direct count) |
| `proj/ADR-116-tool-sandbox-disposition.md` | Standalone ADR, `Status: Accepted`, two-pole scoring, verdict, triggers, Phase 120 proposal | ✓ VERIFIED | 393 lines; all required sections present in the correct order (Context → Poles → D-06 → D-09 → Neutrality Self-Check → Decision → D-07/D-08 → D-12 → Phase 120 Scope → Consequences → References); zero pole-comparative language document-wide |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|----|--------|---------|
| `116-DIVERGENCE-LEDGER.md` | upstream `nolabs-ai/nono` `v0.64.1..v0.71.0` | pinned SHA Reproduction block | ✓ WIRED | Both SHAs live-confirmed unchanged from pin |
| `116-FEASIBILITY-MATRIX.md` | `crates/nono-cli/src/exec_strategy_windows/` | fork primitive citations | ✓ WIRED | Symbol-level spot checks confirm citations are accurate, not file-presence-only |
| `116-FEASIBILITY-MATRIX.md` | `.planning/architecture/adr-65-minifilter-go-no-go.md` | structurally-blocked rows | ✓ WIRED | Row 10/11 cite ADR-65 §6's literal verdict text |
| `proj/ADR-116...md` | `116-DIVERGENCE-LEDGER.md` | Context section figures | ✓ WIRED | All cited figures (38, 0/38, 12-file/18,433-LOC, 74%) match the ledger's finalized Headline exactly |
| `proj/ADR-116...md` | `116-FEASIBILITY-MATRIX.md` | D-09 cells + D-07/D-08 tally | ✓ WIRED | Decision section's 5/4/2 tally matches an independent recount of the matrix's 11 rows |
| `proj/ADR-116...md` | `.planning/ROADMAP.md` | Phase 120 Scope proposal | ✓ WIRED (never-applied, as designed) | Section present with explicit "not applied... operator sign-off"; confirmed zero ROADMAP.md diff |

### Requirements Coverage

| Requirement | Source Plan(s) | Description | Status | Evidence |
|-------------|-----------------|--------------|--------|----------|
| TSBX-01 | 116-01, 116-03, 116-04 | Per-commit divergence ledger, windows-touch + dispositions, Phase 108 shape | ✓ SATISFIED | `116-DIVERGENCE-LEDGER.md` complete, closed except for two named/gated exceptions |
| TSBX-02 | 116-02, 116-05, 116-06 | ADR records adopt-vs-formalize verdict, PR #4 scored on merits, symbol-level grounding | ✓ SATISFIED | `proj/ADR-116-tool-sandbox-disposition.md`, `Status: Accepted`, symbol-cited D-09 table |

No orphaned requirements: `.planning/REQUIREMENTS.md` maps only TSBX-01 and TSBX-02 to Phase 116
(TSBX-03 maps to Phase 120, out of this phase's scope). Both are claimed by at least one plan's
frontmatter. REQUIREMENTS.md's `[ ]` checkboxes for TSBX-01/TSBX-02 remain unchecked — this is
expected: per this project's critical rule, the orchestrator (not the executor or this verifier)
owns flipping REQUIREMENTS.md.

### Anti-Patterns Found

None blocking. Scanned all three primary artifacts for `TBD|FIXME|XXX|TODO|HACK|PLACEHOLDER` and
stub-shaped patterns — zero hits outside of the documents' own deliberate, quoted illustrations of
forbidden phrasing (e.g. the D-08 negative-scan discussion, which names the forbidden phrases in
order to confirm their absence, not to leave one behind).

### Adjudication of Flagged Items

**1. 116-06's Phase 120 Scope deviation (4-deliverable closing-boundary shape vs. row-by-row
S/M/L sizing from the feasibility matrix's non-blocked rows).**

**Verdict: acceptable deviation, not a gap.** `116-06-PLAN.md`'s own task action text explicitly
anticipated this branch: *"If Task 1's verdict is Pole B (formalize), replace the build-out
deliverable list with the smaller 'close the divergence by naming the boundary' shape instead,
mirroring ADR-111/ADR-113's Consequences section — state explicitly that this is the smaller
alternative shape and why the feasibility matrix's rows are not sized as deliverables in that
case."* The frontmatter `must_haves` truth text ("sized row-by-row... each tagged S/M/L") was
written assuming an Adopt verdict, and CONTEXT.md's D-15 itself was drafted without an explicit
Pole-B branch — but D-10 is a locked decision that the verdict must be genuinely open and derived
from evidence, not pre-selected, and D-15's row-by-row build-out sizing is only a coherent
sizing method under an Adopt verdict (there is nothing to size row-by-row if none of the
upstream-driver capabilities get built). The ADR's `Phase 120 Scope` section states this
substitution explicitly, in the open, with the reasoning on the record ("Under Pole B, none of
that build-out happens, so none of those rows convert into Phase 120 work"), still states the
D-15 banding rule verbatim, still tags every deliverable S/M/L, still states the in-v3.7-vs-FUT-9
split, and is still explicitly proposed and never applied to ROADMAP.md. This is the correct,
evidence-following response to a verdict CONTEXT.md's D-15 text did not anticipate — not an
executor shortcut. No override entry is needed; this is resolved as intentional and correctly
executed against the plan's own conditional instruction.

**2. 116-01 atomicity (two tasks landed in one commit `5939cd9`).**

**Verdict: acceptable, both tasks' acceptance criteria independently confirmed met.** Both tasks
write the same file with a direct data dependency (Task 2's tables depend on Task 1's finalized
pathspec). Re-ran all 5 of both tasks' automated verify greps against the committed file:
`^## Reproduction` (1), the pinned SHA string (6), `Pathspec Hazard Reproduction` (2),
`^## Pre-Fence Commits` (1), `^## Post-Fence Commits` (1) — all pass. This matches the pattern the
project's own prior audits (Phase 108's `108-05`) also allow for tightly-coupled single-artifact
plans; not a correctness concern.

**3. Orchestrator edit `7a031d7d` (consolidated duplicate `## References` heading).**

**Verdict: clean, no citation lost, no other section moved.** Read the full diff directly
(`git show 7a031d7d`): it removes exactly one orphaned mid-document `## References` block (8
citations) and merges its 4 citations not already present in the final `## References` section
(`engine-agnostic-confinement.md`, the SPEC doc, and refined `116-CONTEXT.md`/`116-RESEARCH.md`
citations) into the final section. Cross-checked all 8 citations from the removed block against the
final `## References` section (lines 362-393 of the committed file): every one is present
(`116-DIVERGENCE-LEDGER.md`, `116-FEASIBILITY-MATRIX.md`, `116-RESEARCH.md` §4, `116-CONTEXT.md`,
`engine-agnostic-confinement.md`, the SPEC doc, `proj/ADR-86...md`, `proj/ADR-113...md`). No other
line in the diff touches any other section — the diff is a clean cut-and-merge.

**Core non-bias claim (document order + commit-timestamp precedence):**

**Verdict: holds, independently confirmed.** `grep -n "^## " proj/ADR-116-tool-sandbox-disposition.md`
shows Context → Pole A → Pole B → D-06 → D-09 → Neutrality Self-Check all precede `## Decision`,
which precedes D-07/D-08 → D-12 → Phase 120 Scope → Consequences → References — matrix and scoring
strictly precede the verdict in document order, as VALIDATION.md's manual-check row requires.
Commit timestamps independently confirm the git-history claim too: `9531327a` (18:27:56) and
`3f58a9b0` (18:28:51), both Plan 116-05, predate `d26c0083` (18:35:16) and `510ef48b` (18:36:39),
both Plan 116-06's verdict/sizing commits — the scoring evidence was necessarily written and
committed before the verdict sentence existed, not backfilled.

**Operator-gated findings — presence, gating, and non-application:**

**Verdict: both present, both correctly gated, neither applied.** The **Post-Fence Residue
Finding** (`116-DIVERGENCE-LEDGER.md`, 3 items) and the **Uncovered-Window Finding**
(`116-DIVERGENCE-LEDGER.md`, 12 commits: 6 pre-fence + 6 post-fence, SHA list independently
re-verified against `git log` for both windows) each carry the literal "requires operator
approval" / "not applied to ROADMAP.md/REQUIREMENTS.md" language. `git diff --name-only
6509bcc9..HEAD` across the whole phase touches only files under
`.planning/phases/116-tool-sandbox-divergence-audit-disposition-adr/` and
`proj/ADR-116-tool-sandbox-disposition.md` — `.planning/ROADMAP.md` and
`.planning/REQUIREMENTS.md` are untouched by any phase commit.

### D-20 Docs-Only Gate

`git diff --stat 6509bcc9..HEAD -- crates/ bindings/ Cargo.toml Cargo.lock` is empty — confirmed
directly, zero source files changed by this phase. `git diff --name-only 6509bcc9..HEAD` lists
exactly 9 files, all under `.planning/phases/116-.../` or `proj/ADR-116-...md`.

### Human Verification Required

None required to determine this phase's own completion. VALIDATION.md's "Manual-Only
Verifications" table lists three operator-judgment items (verdict soundness; absence-of-pre-seeded
lean; whether Phase 120 planning may begin). This verification directly executed the two
prescribed *mechanical* checks behind the first two rows (document order, and — beyond
VALIDATION.md's own prescription — commit-timestamp precedence) and found both hold. The third
item, Phase 120 planning authorization, is D-11's operator-ratification gate for the *next* phase's
entry, not a completion criterion for this phase: the ADR's own Consequences section (item 5)
explicitly distinguishes "`Status: Accepted` here records the executor's derivation of the verdict
from evidence, not operator sign-off — those are two distinct gates." Phase 116's own goal (land
the ledger, the matrix, and an evidence-derived `Status: Accepted` ADR) is fully and
machine-verifiably achieved without that separate ratification action. Recorded here as an
**outstanding next step, not a phase gap**: the operator should read the Decision/D-09 evidence
before `/gsd:plan-phase 120` is invoked, per D-11.

### Minor Process Note (non-blocking)

`116-VALIDATION.md`'s own frontmatter still reads `status: draft`, `nyquist_compliant: false`, and
its "Validation Sign-Off" checklist is unchecked with `**Approval:** pending` — it was never
updated after execution completed. This is a documentation-hygiene gap only: every re-runnable
command this verification independently re-ran (Reproduction block, pathspec hazard, module sweep,
carve-out checks, SHA lists) reproduced exactly, so the underlying evidence this checklist exists
to confirm does in fact hold. Recommend the next touch of this phase directory flip
`nyquist_compliant: true` for hygiene, but this does not block phase completion or require a gap
entry.

### Gaps Summary

No gaps found. All 18 must-have truths across the phase's 6 plans, all 3 primary artifacts, and
the roadmap's TSBX-01/TSBX-02 contract are independently verified against live git/grep evidence,
not accepted from SUMMARY.md narrative. The three orchestrator-flagged items (116-06's Phase 120
sizing-shape deviation, 116-01's single-commit atomicity, and the 7a031d7d References
consolidation) all adjudicate clean. Zero source files changed (D-20 holds). Zero edits to
`ROADMAP.md`/`REQUIREMENTS.md` (D-14/D-04's operator-gated-proposal shape holds for both named
findings).

---

*Verified: 2026-08-09T19:25:50Z*
*Verifier: Claude (gsd-verifier)*
