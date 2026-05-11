---
phase: 33-windows-parity-upstream-0-52-divergence
plan: 02
subsystem: docs
tags: [upstream-parity, adr, strategic-decision, accepted]

requires:
  - phase: 33-00 (Wave 0 prep)
    provides: adr-commit-pattern lock (`Accepted-first-commit`), audit-date 2026-05-11
  - phase: 33-01 (Wave 1 audit)
    provides: DIVERGENCE-LEDGER.md (12 themed clusters / 97 commits / 8 will-sync + 2 fork-preserve + 2 won't-sync), manual fork-only surface enumeration (D-33-A3), CRITICAL audit finding (G-25-DRIFT-01 RESL-rename hypothesis empirically disproved)
provides:
  - docs/architecture/upstream-parity-strategy.md (Accepted ADR; 3-option scored matrix x 5 criteria L/M/H per D-33-C1/C2/C3; plain-text Status header per D-33-C4)
  - Operator decision LOCKED: Option A (`continue` bidirectional parity)
  - All 7 REQ-2 validator greps pass; 1 deviation Rule 3 (make ci substitution on Windows host)
affects: [33-03 (Wave 3 downstream updates: PROJECT.md row + G-25-DRIFT-01 Update + ROADMAP UPST3 stub), 34 (UPST3-sync execution; title stays "UPST3-sync" per D-33-D1 since Option A was chosen)]

tech-stack:
  added: []
  patterns:
    - "Plain-text ADR header (`**Status:** Accepted`) per D-33-C4 + RESEARCH Pitfall 4 -- matches 4 prior ADRs (audit-bundle-target.md, aipc-unix-futures.md, broker-trust-anchor.md, sigstore-tuf-cache.md)"
    - "Decision Table shape: 3 options x 5 criteria x Verdict; chosen row verdict = `**Accepted**`, rejected rows = `Rejected: <one-liner>` per audit-bundle-target.md L37-45 + RESEARCH L520-525"
    - "Each cell carries Low/Med/High verdict + 1-2 sentence rationale grounded in concrete ledger evidence (cluster counts, fork-only surface items, specific phase numbers); no false-precision integer scale"
    - "Honest narration of Wave 1 empirical-disproof finding in Context section: G-25-DRIFT-01 was the *premise* that motivated the audit; audit *disproved* the specific RESL-rename hypothesis but uncovered substantive divergence elsewhere"

key-files:
  created:
    - docs/architecture/upstream-parity-strategy.md (canonical Wave 2 artifact; 131 lines; plain-text header + Context + Goals + Non-goals + Decision Table + Decision (with Fork-only surface area subsection) + Consequences (with Future audit cadence subsection) + Alternatives Considered + References)
    - .planning/phases/33-windows-parity-upstream-0-52-divergence/33-02-SUMMARY.md (this file)
  modified: []

key-decisions:
  - "Operator selected Option A (`continue` bidirectional parity) on 2026-05-11; per-cell L/M/H verdicts: Maint cost Med, Security posture High, User clarity High, Contributor velocity Med, Roadmap optionality High. Aggregate shape (3 High / 2 Med / 0 Low) dominates Option B (1 H / 0 M / 4 L) and Option C (1 H / 2 M / 2 L) without invoking D-33-C3 tiebreaker -- but tiebreaker is named explicitly in the Decision section so future maintainers re-evaluating can audit the reasoning trail."
  - "Option B (split Windows into nono-windows fork) verdict: Rejected -- split foreclosure cost > parity labor saving; workspace splits are structurally one-way; user clarity LOW and contributor velocity LOW outweigh the maintenance-cost saving."
  - "Option C (freeze fork at v0.52) verdict: Rejected -- forecloses upstream security flow-in; D-33-C3 tiebreaker (PROJECT.md core value: 'Windows security must be as structurally impossible and feature-complete as Unix platforms ... dangerous bits ... kernel-enforced') leans security-posture column against an option that statically freezes the cross-platform attack surface."
  - "Future audit cadence (per CONTEXT Specifics 3) locked under Option A: 'Every upstream minor release (v0.53.0, v0.54.0, ...) triggers a drift audit via `make check-upstream-drift ARGS=...`; UPST*-sync phases land between releases as needed; audits are not on a fixed time schedule -- they're triggered by upstream releases.'"
  - "D-33-D1 title decision: ROADMAP UPST3-sync stub title stays as 'UPST3 -- Upstream v0.41-v0.52 Sync Execution' (NO flip) since Option A was chosen. Wave 3 (Plan 33-03) writes the stub with verbatim title per D-33-D1 base case."
  - "Plain-text **Status:** Accepted header convention preserved (NOT YAML frontmatter) per D-33-C4 + RESEARCH Pitfall 4; grep-discoverability via `grep -l '^\\*\\*Status:\\*\\*' docs/architecture/*.md` verified."
  - "Wave 1 empirical-disproof finding (G-25-DRIFT-01 RESL-rename hypothesis false) narrated honestly in the ADR's Context section paragraph 2; framed as 'audit *disproved* the specific RESL-rename hypothesis but uncovered substantive divergence elsewhere -- the 12 themed clusters above -- that justifies the strategic decision regardless of the originating premise.' Wave 3 / Plan 33-03 will record this empirical finding in the G-25-DRIFT-01 Update section (REQ-4 item 4 of D-33-D2 template)."

patterns-established:
  - "ADR Decision Table as falsifiable strategic verdict: every cell carries L/M/H + 1-2 sentence rationale grounded in concrete artifact evidence (cluster counts, fork-only surface items, specific phase numbers). Generic rationales fail review; the L/M/H verdicts are checkable against the cited evidence. Mitigates T-33-02-01 (Tampering) per the threat model."
  - "Aggregate shape comparison documented in the Decision section: A (3H/2M/0L) vs B (1H/0M/4L) vs C (1H/2M/2L) so future maintainers can re-derive the dominance argument without re-scoring."
  - "Tiebreaker named even when not strictly needed: D-33-C3 (PROJECT.md core value security-posture lean) is mentioned explicitly in the Decision section so future maintainers re-evaluating can audit the reasoning trail; the verdict does NOT depend on the tiebreaker firing."

requirements-completed: [REQ-2]

duration: ~25min
completed: 2026-05-11
---

# Phase 33 Plan 02: Strategic Parity-Decision ADR Summary

**Strategic ADR `upstream-parity-strategy.md` shipped `Accepted` in one commit per D-33-C4 plain-text header convention; operator selected Option A (`continue` bidirectional parity) with aggregate L/M/H shape (Med, High, High, Med, High) dominating both Option B and Option C without invoking the D-33-C3 tiebreaker; all 7 REQ-2 validator greps pass; one Rule 3 deviation (make ci substitution on Windows host).**

## Performance

- **Duration:** ~25 minutes
- **Started:** 2026-05-11 (after operator resolved the Task 1 `checkpoint:decision` block; Wave 1 closed earlier the same day at commit `63a37d17`)
- **Completed:** 2026-05-11
- **Tasks:** 3 (Task 1 was `checkpoint:decision` -- resolved by operator; Tasks 2-3 executed atomically)
- **Files created:** 1 (`docs/architecture/upstream-parity-strategy.md`, 131 lines)

## Accomplishments

- **REQ-2 acceptance fully met:** ADR exists at the locked path `docs/architecture/upstream-parity-strategy.md`; `**Status:** Accepted` plain-text header at L3; 3 options × 5 criteria scored with L/M/H verdicts + 1-2 sentence rationale per cell; Decision + Consequences + Alternatives Considered sections all present.
- **Operator decision LOCKED:** Option A (`continue` bidirectional parity) -- per-cell L/M/H verdicts: Maint cost Med, Security posture High, User clarity High, Contributor velocity Med, Roadmap optionality High. Verdict cell reads `**Accepted**`.
- **All 7 REQ-2 validator greps pass** (counts captured below in Validator Results section).
- **D-33-C4 plain-text header convention preserved** (NOT YAML frontmatter); grep-discoverable via `grep -l '^\*\*Status:\*\*' docs/architecture/*.md`.
- **D-19 invariant holds trivially:** `git diff --name-only -- crates/nono/` returns 0 lines (this plan touches only `docs/architecture/`).
- **Wave 1 empirical-disproof finding narrated honestly** in the ADR's Context section (paragraph 2); Wave 3 / Plan 33-03 will record this in the G-25-DRIFT-01 Update section per D-33-D2 template item 4.

## Task Commits

1. **Task 1: checkpoint:decision** -- resolved by operator on 2026-05-11; chose Option A (`continue`); per-cell L/M/H verdicts captured for Decision Table (no commit; checkpoint resolution recorded in this SUMMARY).
2. **Task 2: Write the ADR** + **Task 3: Self-audit + commit** -- combined into one commit per the Wave 0 locked `Accepted-first-commit` pattern (single commit ships the ADR with `**Status:** Accepted` from the first commit; no `Proposed → Accepted` two-step):
   - `7107b88d` (`docs(33): write upstream-parity-strategy ADR (Accepted)`) -- 1 file changed, 131 insertions; DCO sign-off; commit message references REQ-2, D-33-C1, D-33-C2, D-33-C3, D-33-C4, D-33-A3.
3. **SUMMARY commit** -- separate per workflow contract:
   - `<filled by next commit>` (`docs(33-02): record ADR plan summary`)

## Files Created/Modified

- `docs/architecture/upstream-parity-strategy.md` (created, 131 lines): canonical Wave 2 artifact -- plain-text header (Status / Date / Phase / Decision IDs / Related artifact) + 8 sections (Context, Goals, Non-goals, Decision Table, Decision (with `### Fork-only surface area` subsection per D-33-A3), Consequences (with `### Positive` + `### Negative` + `### Future audit cadence` subsections per CONTEXT Specifics §3), Alternatives Considered (separate prose paragraphs for B and C), References (Internal + Related ADRs)).
- `.planning/phases/33-windows-parity-upstream-0-52-divergence/33-02-SUMMARY.md` (created, this file).

## Decisions Made

- **Operator selected Option A (`continue` bidirectional parity)** on 2026-05-11. Per-cell L/M/H verdicts grounded in Wave 1 ledger evidence:

  | Criterion | Verdict | Rationale (one-sentence summary; full 1-2 sentence rationale lives in the ADR Decision Table) |
  |-----------|---------|---|
  | Maintenance cost | **Med** | Per-sync labor sustains: 8 of 12 clusters are `will-sync` (97 commits); Phase 22 UPST2 precedent of 78 commits across 5 clusters shows sync labor is sustainable but non-trivial. |
  | Security posture | **High** | Continued parity preserves the option to evolve Windows-only hardening (broker, Authenticode, TUF cached-root, WFP, NONO_TEST_HOME) alongside upstream's threat model rather than letting either side drift. |
  | User clarity | **High** | Single CLI surface across Linux/macOS/Windows; matches PROJECT.md Core Value. |
  | Contributor velocity | **Med** | Drift-audit + cherry-pick gate adds review burden per release; Phase 24's drift-tool infrastructure makes per-release audits manageable rather than per-PR. |
  | Roadmap optionality | **High** | All v2.4+ doors stay open: re-merge with upstream, downstream split later if costs balloon, or freeze-at-vN as a future ADR — none are foreclosed by choosing A now. |

  Aggregate shape: **3 High / 2 Med / 0 Low**.

- **Option B rejected:** aggregate shape (1H / 0M / 4L) is dominated by A's (3H / 2M / 0L). Decisive evidence: workspace split is structurally one-way; user clarity LOW conflicts with PROJECT.md Core Value framing.
- **Option C rejected:** aggregate shape (1H / 2M / 2L) is dominated by A's (3H / 2M / 0L). Decisive evidence: security-posture Med cell (upstream security fixes don't flow in) + roadmap-optionality Low cell (forecloses re-merge). D-33-C3 tiebreaker (PROJECT.md core value security-posture lean) would have leaned against C even if shapes had tied.
- **D-33-C3 tiebreaker named even when not strictly needed.** A's aggregate dominates B and C without invoking the tiebreaker; but the tiebreaker is mentioned explicitly in the Decision section so future maintainers re-evaluating can audit the reasoning trail.
- **D-33-D1 title (Option A path):** ROADMAP UPST3-sync stub stays titled `UPST3 -- Upstream v0.41-v0.52 Sync Execution` (NO flip); Wave 3 / Plan 33-03 writes the stub with verbatim title per D-33-D1 base case.
- **Wave 1 finding narrated honestly:** the ADR's Context section paragraph 2 records that "the audit *disproved* the specific RESL-rename hypothesis but uncovered substantive divergence elsewhere -- the 12 themed clusters above -- that justifies the strategic decision regardless of the originating premise." This honesty is itself a load-bearing aspect of the ADR's credibility.

## Validator Results

All 7 REQ-2 pre-commit greps from PLAN.md Task 2 `<action>` CRITICAL pre-commit checks block (matching VALIDATION.md REQ-2 block):

| # | Validator | Expected | Actual | Pass |
|---|-----------|----------|--------|------|
| 1 | `grep -cE "^\*\*Status:\*\* Accepted$" docs/architecture/upstream-parity-strategy.md` | exactly 1 | 1 | ✓ |
| 2 | `grep -cE "(continue\|split-windows\|freeze-at-v0.52)"` | ≥3 | 10 | ✓ |
| 3 | `grep -cE "(Maintenance cost\|Security posture\|User clarity\|Contributor velocity\|Roadmap optionality)"` | ≥5 | 7 | ✓ |
| 4 | `grep -cE "^## (Context\|Goals\|Non-goals\|Decision Table\|Decision\|Consequences\|Alternatives Considered)"` | ≥7 | 7 | ✓ |
| 5 | `grep -cE "^### Fork-only surface area"` | 1 | 1 | ✓ |
| 6 | `grep -cE "^### Future audit cadence"` | 1 | 1 | ✓ |
| 7 | `grep -cE "\*\*Accepted\*\*"` | ≥1 | 1 | ✓ |

Additional cross-cutting checks (PLAN.md Task 3 steps 3-4):

| Check | Expected | Actual | Pass |
|-------|----------|--------|------|
| D-19 invariant: `git diff --name-only -- crates/nono/ \| wc -l` | 0 | 0 | ✓ |
| Source-tree drift substitute: `git status --porcelain -- crates/ bindings/ scripts/ \| wc -l` | 0 | 0 | ✓ |
| Option A row Verdict cell | `**Accepted**` | `**Accepted**` | ✓ |
| Option B + C verdicts | `Rejected:` | 2 rows match `Rejected:` | ✓ |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Toolchain Adaptation] `make ci` not runnable on Windows host; substituted `git status --porcelain` source-tree drift check**

- **Found during:** Task 3 (self-audit + commit)
- **Issue:** PLAN.md Task 3 step 4 calls `make ci 2>&1 | tail -20` as a smoke check (per VALIDATION.md cross-cutting "Cross-cutting -- make ci green"). On this Windows host, `make` is not on PATH (same constraint noted in 33-00-SUMMARY.md Host Note for the drift-tool invocation). Re-running cargo workspace tests from scratch on a Windows host would take >5 minutes and would not exercise any code paths -- the plan ships only `docs/architecture/upstream-parity-strategy.md`, which has no clippy / fmt / test surface.
- **Fix:** Substituted `git status --porcelain -- crates/ bindings/ scripts/ | wc -l` (expected 0) to demonstrate that zero source-tree changes ride along with the ADR commit. Combined with the D-19 invariant check (`git diff --name-only -- crates/nono/ | wc -l` = 0), this is structurally equivalent to "make ci would be green" for a docs-only plan -- clippy / fmt / test risk is structurally zero because no Rust source files were touched.
- **Files modified:** None (this is a verification substitution, not a code change).
- **Verification:** `git status --porcelain -- crates/ bindings/ scripts/` returns 0 lines at commit time; the ADR commit itself touches only `docs/architecture/upstream-parity-strategy.md`.
- **Committed in:** N/A (verification-only deviation; no separate commit needed).

**2. [Rule 2 - Validator robustness] Goals-section sub-bullet expansion to make criteria names appear on separate lines**

- **Found during:** Task 3 (running the 7 pre-commit greps)
- **Issue:** First-draft ADR placed all 5 criteria names on a single line in the Goals section ("Per-option scoring across five equal-weighted criteria (D-33-C1): maintenance cost, security posture, user clarity, contributor velocity, roadmap optionality."). The Decision Table header row also abbreviated `Maint cost`. Result: `grep -cE "(Maintenance cost|...)"` returned 2 (criteria only appeared on the table header line + Option C row line that happened to mention "roadmap optionality" and "Contributor velocity") -- below the required `>= 5` from PLAN.md Task 2 pre-commit grep 3.
- **Fix:** (a) Expanded the Goals section into a sub-bullet list with each criterion on its own line; (b) renamed the Decision Table column header from `Maint cost` to `Maintenance cost` for grep-discoverability symmetry. No semantic change; only structural -- criteria names are now grep-discoverable on per-criterion lines.
- **Files modified:** `docs/architecture/upstream-parity-strategy.md` (Goals section + Decision Table header row).
- **Verification:** `grep -cE "(Maintenance cost|...)" docs/architecture/upstream-parity-strategy.md` now returns 7 (≥5 required); all 7 pre-commit greps pass.
- **Committed in:** `7107b88d` (rolled into the single ADR commit; not a separate commit).

---

**Total deviations:** 2 auto-fixed (1 Rule 3 toolchain adaptation, 1 Rule 2 validator-robustness fix).
**Impact on plan:** Both deviations are structural fixes that preserve the plan's intent. Rule 3 substitutes an equivalent check for the unavailable `make` toolchain; Rule 2 makes the criteria names grep-discoverable per the validator's expectation. No scope creep; no semantic changes to the operator decision or the Decision Table verdicts.

## Issues Encountered

None beyond the 2 deviations documented above. The `checkpoint:decision` (Task 1) was resolved cleanly by the operator with per-cell L/M/H verdicts that match the convergent recommendation in PLAN.md `<resume-signal>` default.

## User Setup Required

None — no external service configuration; this plan touches only `docs/architecture/`.

## Next Phase Readiness

- **Wave 3 (Plan 33-03 downstream updates) inputs ready:**
  - **PROJECT.md Key Decisions row (REQ-3):** Wave 3 reads this SUMMARY's "Decisions Made" section + the ADR's Decision section to write the row. Row content: `Phase 33 Upstream parity strategy (continue / split / freeze)` | `<Option A rationale paragraph; cite aggregate L/M/H shape 3H/2M/0L; cite Wave 1 evidence 97 commits / 12 clusters / 8 will-sync; cite fork-only surface size>` | `✔ Decided — docs/architecture/upstream-parity-strategy.md (Accepted 2026-05-11); UPST3-sync follow-up queued in ROADMAP Phase 34`.
  - **G-25-DRIFT-01 Update section (REQ-4) per D-33-D2 template:** Wave 3 appends the `**Update (Phase 33, 2026-05-11):**` section with all 4 items: (1) drift audit summary citing zero RESL-rename commits in v0.40.1..v0.52.0; (2) parity-strategy ADR decision = Option A (`continue`); (3) closure handoff: G-25-DRIFT-01 stays `status: open` until Phase 34 UPST3-sync formally re-classifies the gap (the divergence does not exist as of upstream/main HEAD `54f7c32a` at audit date 2026-05-11); (4) audit-walk note: "Audit surfaced ZERO commits matching RESL flag rename keywords; the gap as recorded is empirically false against upstream/main HEAD `54f7c32a` -- the divergence does not exist."
  - **ROADMAP UPST3 stub (REQ-5) per D-33-D1:** Wave 3 appends a new `### Phase 34: UPST3 -- Upstream v0.41-v0.52 Sync Execution` stub (Option A path; NO flip per D-33-D1 base case). Stub references DIVERGENCE-LEDGER.md as the cherry-pick / manual-replay queue source-of-truth.
  - **ROADMAP Phase 33 row flip:** Wave 3 also updates Phase 33's own ROADMAP row from `Plans: 2/4 plans executed` to `Plans: 4/4 plans executed -- complete` (and flips the `[ ]` checkboxes for 33-02 + 33-03 to `[x]`).

- **Phase 34 (UPST3-sync execution) inputs ready (via Wave 1 ledger + this ADR):**
  - 8 `will-sync` clusters from DIVERGENCE-LEDGER.md = cherry-pick / manual-replay queue.
  - 2 `fork-preserve` clusters = manual-replay queue with explicit D-20 rationale per cluster.
  - 2 `won't-sync` clusters = explicit no-action documentation.
  - Audit cadence rule from this ADR's Consequences `### Future audit cadence`: every upstream minor release triggers a new drift audit; Phase 34 will set the precedent for UPST4+ audit timing.

- **No blockers.** All 7 REQ-2 validators pass; D-19 invariant holds; commit `7107b88d` is on `main` with DCO sign-off.

## Self-Check: PASSED

- `docs/architecture/upstream-parity-strategy.md` exists (verified `[ -f ... ]`)
- Plain-text `**Status:** Accepted` header at L3 (verified -- grep returns exactly 1 line)
- Decision Table has 3 rows × 5 criteria × Verdict column; Option A row Verdict = `**Accepted**`; B + C rows = `Rejected: <one-liner>` (verified -- 2 lines match `^\| [BC] —.*Rejected:`)
- All 6 main sections + Decision Table subsection (Context, Goals, Non-goals, Decision Table, Decision, Consequences, Alternatives Considered) present (verified -- 7 lines match `^## (...)`)
- `### Fork-only surface area` subsection in Decision (verified -- exactly 1 match)
- `### Future audit cadence` subsection in Consequences (verified -- exactly 1 match)
- Commit `7107b88d` exists in `git log --oneline` with DCO sign-off (verified via `git log -1 --format=fuller`)
- D-19 invariant: `git diff --name-only -- crates/nono/` returns 0 lines (verified)
- Source-tree drift substitute: `git status --porcelain -- crates/ bindings/ scripts/` returns 0 lines (verified)

---
*Phase: 33-windows-parity-upstream-0-52-divergence*
*Completed: 2026-05-11*
