---
phase: 116
slug: tool-sandbox-divergence-audit-disposition-adr
status: draft
nyquist_compliant: false
wave_0_complete: true
created: 2026-08-09
---

# Phase 116 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
>
> **This phase produces documentation, not code (D-20: zero `.rs`, zero `Cargo.toml`,
> `ROADMAP.md` untouched, `make ci` explicitly not a gate).** "Validation" here is
> *evidential reproducibility*: can a reviewer independently re-run every command the two
> deliverables cite and get the numbers those deliverables state? That question is fully
> checkable — every command is deterministic given the pinned SHAs.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | None — deliverables are two markdown documents |
| **Config file** | N/A |
| **Quick run command** | Re-run a single Reproduction-block command from `116-RESEARCH.md` §2 and diff its output against the figure recorded in the ledger cell that cites it |
| **Full suite command** | Re-run the entire Reproduction block (`116-RESEARCH.md` §2a–2j) end-to-end; diff every count against the ledger's stated figures |
| **Estimated runtime** | ~2–4 minutes (git-local; no network beyond `git ls-remote` for tag re-resolution) |

**Pre-execution guard (the "Wave 0" equivalent):** confirm the pinned SHAs still resolve
(`v0.64.1` → `0551eba2`, `v0.71.0` → `0055bf3c`) and the working tree is clean. Both were
true at research time. Any other SHA means upstream has moved and the audit restarts.

---

## Sampling Rate

- **After every task commit:** re-run the specific git/grep command(s) that task's evidence
  cells cite; confirm each hit count matches what was written into the deliverable *before*
  the commit lands.
- **After every plan wave:** re-run the FULL Reproduction block. `upstream` is a live remote;
  a moved tag between task execution and wave close invalidates the pinned-SHA contract.
- **Before `/gsd:verify-work 116`:** every command appearing literally in either deliverable
  is re-run — in full, not sampled. The command population is dozens, not hundreds.
- **Max feedback latency:** ~30 seconds per task-level check.

---

## Per-Task Verification Map

Task IDs are assigned by the planner. This map is keyed by **success criterion** and
**requirement**, and the planner must attach each row to the task(s) that satisfy it.

| SC / Req | Behavior to prove | Threat Ref | Test Type | Automated Command | Status |
|----------|-------------------|------------|-----------|-------------------|--------|
| SC1 / TSBX-01 | Ledger enumerates the full `v0.64.1..v0.71.0` surface with per-commit `windows-touch` + disposition; PR #1105 and all 7 fenced PRs present | T-116-01 | git-archaeology re-run | `git log --no-merges --oneline v0.64.1..v0.71.0 -- <3-path-union>` + per-commit `git show --name-only --format='' <sha>` | ⬜ pending |
| SC1 / TSBX-01 | Pinned-SHA Reproduction block matches live upstream | T-116-01 | tag re-resolution | `git ls-remote --tags upstream v0.64.1 v0.71.0` | ⬜ pending |
| SC1 / TSBX-01 | D-01's hypothesis table is re-measured at audit time, and any disagreement is reconciled **in place** with the superseded figure retained | T-116-02 | measurement diff | re-run §2c/§2g counts; ledger states measured-vs-hypothesis delta explicitly (or "no delta") | ⬜ pending |
| SC1 / D-04 | Every path in every `split` commit lands in exactly one bucket (`absorb`/`defer`/`noise`) | T-116-03 | bucket-count reconciliation | ledger's own arithmetic-check row: Σ(bucketed paths) == Σ(paths in split commits) | ⬜ pending |
| SC1 / D-19 | Carve-out re-touch check run per accumulated carve-out; **zero-hit results recorded explicitly** as "clean — no re-touch in window" | T-116-04 | per-path `git log` | `git log --oneline v0.64.1..v0.71.0 -- '<exact literal path>'` (no `*` glob — see Pitfall 1) | ⬜ pending |
| SC2 / TSBX-02 | Every confidence rating carries the literal command, its hit count, and the date measured | T-116-05 | citation re-run | each cited `grep -nE ...` re-run against the cited file at the cited ref; hit count must match | ⬜ pending |
| SC2 / TSBX-02 | No rating rests on file presence; greps **discover** their target rather than confirming a pre-named path | T-116-05 | anti-pattern scan | no evidence cell whose only proof is `test -f` / `ls`; symbol greps are tree-wide, not path-pinned | ⬜ pending |
| SC3 / TSBX-02 | ADR states one unambiguous verdict and carries `Status: Accepted` | T-116-06 | document assertion | `proj/ADR-116-tool-sandbox-disposition.md` contains `Status: Accepted` and exactly one verdict statement | ⬜ pending |
| SC3 / D-12 | Losing-option reversal conditions are **named falsifiable triggers** plus a **named re-test point** | T-116-06 | document assertion | ADR lists ≥3 checkable trigger conditions and names the next UPST sync as the re-test checkpoint | ⬜ pending |
| SC4 / TSBX-02 | Both poles scored against the *same* D-09 criteria set; PR #4's path scored on merits | T-116-07 | symmetry check | every D-09 criterion appears with a Pole A **and** a Pole B cell; no criterion scored for one pole only | ⬜ pending |
| SC4 | The `.NET`/PowerShell-CLR-under-`WRITE_RESTRICTED` finding appears as a worked `structurally-blocked` example, not an anecdote | T-116-07 | document assertion + symbol grep | ADR cites it; cited fork symbol (`execution_runtime.rs` token-arm split) verified greppable | ⬜ pending |
| SC4 / D-08 | Minifilter-dependent rows marked `structurally-blocked` citing ADR-65 — not "expensive", not "available-if-reversed" | T-116-07 | anti-pattern scan | no matrix row frames a minifilter capability as conditional on reversing ADR-65 | ⬜ pending |
| SC4 / D-06 | The two intermediate shapes are named as considered-and-rejected, one paragraph each | T-116-07 | document assertion | ADR contains both (i) adopt-Unix-only and (ii) adopt-policy-model-only as rejected shapes | ⬜ pending |
| SC5 / D-15 | Phase 120 scope sized row-by-row from the feasibility matrix with S/M/L bands; in-v3.7 vs FUT-09 split stated | T-116-08 | document assertion | every non-blocked matrix row maps to a named deliverable with a band; commit counts are **not** the sizing basis | ⬜ pending |
| SC5 / D-14 | ROADMAP amendment written as an operator-gated **proposal inside the ADR**, not applied | T-116-08 | negative assertion | `git diff --stat` for the phase shows **zero** changes to `.planning/ROADMAP.md` | ⬜ pending |
| D-20 | No code changed | T-116-09 | negative assertion | `git diff --stat <phase-base>..HEAD -- '*.rs' '*.toml'` returns empty | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

None — there is no test framework to stand up.

- [x] Pinned SHAs resolve (`v0.64.1` = `0551eba2`, `v0.71.0` = `0055bf3c`) — verified 2026-08-09
- [x] `upstream` remote present and fetched (`nolabs-ai/nono`) — verified 2026-08-09
- [x] Working tree clean before task execution begins — verified 2026-08-09
- [x] Prior-ledger and ADR precedent files present on disk — verified 2026-08-09

*Existing infrastructure covers all phase requirements.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| The verdict is *sound*, not merely *stated* | TSBX-02 / SC3 | Judgment cannot be asserted by a command. The evidence under it is fully re-runnable; the inference from evidence to verdict is not. | Operator reads the D-09 scoring table, checks each cell's cited evidence re-runs, and ratifies or rejects (D-11 gate). |
| The verdict was reached without a pre-seeded lean | D-10 / SC4 | Absence of bias is not greppable. | Operator checks that neither plan task descriptions nor the ADR's framing presuppose a pole; the matrix and scoring must precede the verdict section in document order. |
| Phase 120 planning may begin | D-11 | Operator ratification gate by locked decision. | Operator explicitly ratifies the ADR before `/gsd:plan-phase 120` is invoked. |

---

## Known Method Hazards (from `116-RESEARCH.md`)

These are validation failure modes with live precedent in this repo — the checks above are
written to catch them:

1. **Substring-anywhere pathspec.** `git log -- '*tool-sandbox*'` matches
   `docs/cli/features/tool-sandbox.mdx` and inflates the surface (35 → 37 in this window;
   commits `5a7447d3`, `ebd51cbb`). Use exact literal paths.
2. **Narrow visibility grep under-reports.** `exec_strategy_windows/supervisor.rs` (241 KB,
   load-bearing) returns **zero** hits on a `pub|pub(crate)` pattern because its API is
   `pub(super)`. A zero-hit result is suspicious, not conclusive.
3. **File presence ≠ symbol presence.** Phase 112's disposition table was wrong in 3 of ~6
   re-checked rows for exactly this reason (D-16).
4. **Tests/greps that name their target are blind by construction.** Phase 115 V-01. Evidence
   greps must search the tree for the symbol, not confirm a path someone already chose.

---

## Validation Sign-Off

- [ ] Every SC row above is attached to at least one plan task
- [ ] Every evidence cell in both deliverables carries command + hit count + date (D-16)
- [ ] Full Reproduction block re-run green at wave close
- [ ] Bucket-count reconciliation arithmetic balances (D-04)
- [ ] Carve-out zero-hits recorded explicitly, not omitted (D-19)
- [ ] `git diff --stat` confirms zero `.rs` / `.toml` / `ROADMAP.md` changes (D-20 / D-14)
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
