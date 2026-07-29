---
phase: 108-upst12-divergence-audit
verified: 2026-07-29T00:00:00Z
status: passed
score: 8/8 must-haves verified
overrides_applied: 0
---

# Phase 108: UPST12 Divergence Audit Verification Report

**Phase Goal:** An authoritative per-commit divergence ledger for upstream `v0.66.0..v0.69.0`
exists, so the absorb phases (109–111) have a classified, ADR-reviewed work-list — with the
tool-sandbox subsystem's refinements provably fenced off to v3.7.

**Verified:** 2026-07-29
**Status:** passed
**Re-verification:** No — initial verification

## Method

This is a documentation/analysis-deliverable phase (no runnable code, per D-08's "audit-only,
no cherry-picks" boundary), so verification consisted of independently re-deriving every
checkable claim in `108-DIVERGENCE-LEDGER.md` and `proj/ADR-108-deny-domain-posture.md` directly
against the live upstream/fork git history, rather than trusting the ledger's own assertions or
the SUMMARY.md narratives. All git commands below were re-run fresh in this session, not copied
from the ledger.

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | A reviewer can independently re-run the reproduction commands and get the exact same commit counts the ledger records | ✓ VERIFIED | Re-ran `git ls-remote --tags upstream v0.66.0 v0.67.0 v0.67.1 v0.68.0 v0.69.0` — all 5 SHAs match the ledger's Reproduction block verbatim. Re-ran `git log --no-merges --oneline $RANGE \| wc -l` → 100 (matches); `git log --merges --oneline $RANGE` → same 4 merge SHAs listed in the ledger. |
| 2 | Every one of the 100 non-merge commits in the window is accounted for exactly once (no double-count, no silent drop) | ✓ VERIFIED | Extracted all 40-char SHAs from the ledger's CODE (62) + DEPS (19) + CI (11) + DOCS (8) bucket tables independently, deduped, and diffed against a fresh `git log --no-merges --format='%H' $RANGE \| sort` (100 lines). Result: exact 1:1 match, zero missing, zero extra, zero cross-bucket overlap. The finer 9-cluster partition (NET 12 + PROF 8 + CORE 4 + tool-sandbox-pure 9 + tool-sandbox-split 11 + security-residual-and-misc 18 = 62 = CODE) also reconciles exactly, with the ledger's own explanation for the PROF (8 vs 10-row) and tool-sandbox-pure (9 vs 12-row) apparent discrepancies (pointer/cross-reference rows) confirmed correct on inspection. |
| 3 | The noise/substantive split is derived from actual touched-path-sets, not merely trusted from the conventional-commit prefix | ✓ VERIFIED | Ledger's D-15 Prefix-vs-Path-Set Cross-Check section names 4 commits where prefix and path-set-derived bucket disagree; independently confirmed `9ef5918169` (docs-prefixed but touches `.rs`) and `f050643479` (chore-prefixed but touches 20+ `.rs` files) are classified CODE, not DOCS/noise, in the actual bucket tables. |
| 4 | The ledger is usable by Phase 109/110/111 without re-derivation (NET/PROF/CORE per-commit tables with requirement mapping + disposition) | ✓ VERIFIED | NET/PROF/CORE per-commit tables (lines 667-799) carry sha/subject/files-changed/windows-touch/security-relevant/requirement-mapping/disposition/re-export-scan columns for every commit, with cited grep evidence (e.g. `ae1c513e`'s `Os::Windows` match-arm cited by line). `ADR-108`'s Consequences section gives Phase 109 four concrete, numbered acceptance criteria for NET-01 (deny-only strict-selection rule), not just a disposition label. |
| 5 | Residue accounting completeness (D-07) — every split/deferred commit has every path marked absorb/defer/noise, no unbucketed path | ✓ VERIFIED | Spot-checked 5 split commits' path lists (`a519ee62`, `d5803b99`, `72a98830`, `eb2d61a7`, `d4927f95`) against live `git show --name-only` — every listed path matches exactly, every path carries a marker, and each table's stated row-count arithmetic (e.g. "1 noise + 13 absorb + 3 defer = 17") checks out. |
| 6 | The 100-commit partition claim is real, not asserted | ✓ VERIFIED | See Truth #2 — independently re-derived via `comm`, not trusted from the ledger's stated numbers. |
| 7 | SHA integrity — every SHA resolves and cited subjects match actual commit subjects | ✓ VERIFIED | Spot-checked ~15 SHAs across CODE/DEPS/tool-sandbox/carve-out sections against `git log -1 --format='%s' <sha>` — all subjects match verbatim, including edge cases (`373a67ae` crossbeam-epoch bump, `a5a441c25769` allow_vars fix, `0ecc476b` AWS auth). Zero mislabelled SHAs found. |
| 8 | ADR-108 shows its work (both options + fork-invariant impact) rather than asserting the D-12 conclusion | ✓ VERIFIED | ADR-108 has full Option A (full-sync-adopt) and Option B (adapt) sections with pros/cons each, a Fork Touchpoint Map, and a Decision section that argues from two independently-checkable facts. Cross-verified both facts against live `crates/nono/src/net_filter.rs`: `deny_hosts`, `DENY_HOSTS`, `strict: bool`, `HostFilter::new_strict()`, and the `check_host()` step-3 empty-allowlist behavior all exist exactly as described. |

**Score:** 8/8 truths verified

### Carried Findings (must not be missed by downstream phases) — all confirmed present and accurate

| Finding | Location in ledger | Independently confirmed |
|---|---|---|
| Live `crossbeam-epoch` RUSTSEC-2026-0204, fix `373a67ae` unabsorbed | `cargo audit` cross-reference (DEPS Cluster section) + flagged again in 108-05-SUMMARY "Next Phase Readiness" | `grep -A2 'name = "crossbeam-epoch"' Cargo.lock` confirms the fork's live lockfile carries the vulnerable `0.9.18`. `cargo-audit`'s own advisory maps `373a67ae` as the exact fix. |
| `e6d26871`'s `pub mod resource;` cross-crate re-export — ADR-86 flag | Threat Flags table (line 800) + 108-05-SUMMARY "Next Phase Readiness" | Confirmed the commit is CORE-cluster and the flag correctly calls out this is the first CORE commit to cross the library/CLI boundary (`pub mod`/`pub use` in `crates/nono/src/lib.rs`), requiring an ADR-86 compliance check before Phase 111 absorbs it verbatim. |
| PROF-02's cross-dependency on deferred `tool_sandbox::dynamic_providers::expand_dynamic_tokens` | `d4927f95` split-residue entry (line 1112-1137), explicitly marked "D-05 worked example" | `git show d4927f95 -- crates/nono-cli/src/capability_ext.rs` confirms the diff literally adds `use crate::tool_sandbox::dynamic_providers::expand_dynamic_tokens;` and calls it 6+ times — the cross-dependency claim is accurate, not overstated. |

All three carried findings are recorded with enough specificity (exact file, exact function, exact
downstream phase obligation) that a Phase 109/110/111 planner reading the ledger cannot miss them.

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `.planning/phases/108-upst12-divergence-audit/108-DIVERGENCE-LEDGER.md` | Per-commit classified ledger, 100-commit reproduction, cluster tables, ADR cross-reference | ✓ VERIFIED | 1642 lines; every claim independently spot-checked against live git history (see Observable Truths above); zero fabricated SHAs, zero unresolved TBD/placeholder markers (self-referential-only hits at lines 1608/476/493 are prose describing that TBDs were removed, not live markers). |
| `proj/ADR-108-deny-domain-posture.md` | Standalone ADR settling `deny_domain` (#1374) posture, mirroring ADR-86/ADR-98 shape | ✓ VERIFIED | Full Context/Touchpoint-Map/Options/Decision/Consequences/References structure; grounding claims cross-checked against `crates/nono/src/net_filter.rs` and confirmed accurate. |
| `.planning/REQUIREMENTS.md` UPST12-01 marked complete | Checkbox + traceability row flipped | ✓ VERIFIED | Line 110 `[x] **UPST12-01**`, line 147 `\| UPST12-01 \| Phase 108 \| Complete \|`. |

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| Ledger NET/PROF/CORE per-commit tables | Phase 109/110/111 planning | requirement-mapping column citing `NET-0X`/`PROF-0X`/`CORE-0X` | ✓ WIRED | Every full-analysis row in the 3 tables carries a non-blank requirement-mapping cell; `none`-mapped rows are explicitly explained rather than left ambiguous. |
| ADR-108 Decision | Phase 109 NET-01 absorb | "Consequences" section's 4 lettered acceptance criteria | ✓ WIRED | (a)-(d) give Phase 109 concrete, testable obligations (absorb `deny_suffixes`/`with_denied_hosts`, MUST select strict construction on deny-only profiles, reject auto-activation, cross-reference the NET table). |
| tool-sandbox-split residue tables | v3.7 Windows Tool-Sandbox Parity milestone | 20-commit 3-path-union surface, D-09 | ✓ WIRED | All 20 commits carry an explicit DEFERRED->v3.7 disposition; 7 named refinement PRs (#1280/#1322/#1325/#1384/#1394/#1413/#1417) all traced with an explicit reason. |
| Requirement Coverage Gap (27 unmapped commits) | Proposed Phase 112 | D-18/D-19/D-21 draft requirement IDs (SEC-01..SEC-09, RES-01/02) | ✓ WIRED (as a proposal, correctly not applied) | `git diff --stat -- .planning/ROADMAP.md` shows zero changes across the full commit range for Phase 108 — confirms the ledger's own claim that it did not silently rewrite the roadmap. This is the correct behavior per D-19 (operator-approval gate), not a gap. |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| UPST12-01 | 108-01..05 (all 5 plans) | Authoritative ledger classifying every substantive commit, ADR-review verdict, 7 named tool-sandbox PRs DEFERRED→v3.7 | ✓ SATISFIED | Ledger + ADR-108 both exist and independently verify correct per above; REQUIREMENTS.md traceability table confirms `Complete`. |

No orphaned requirements: `.planning/REQUIREMENTS.md`'s v3.6 Traceability table maps only UPST12-01
to Phase 108; all other v3.6 requirements (NET/PROF/CORE/VERIFY/RLS) map to Phases 109-111, out of
this phase's scope.

### ROADMAP.md Phase 108 Success Criteria — independently re-checked

1. **SC1** (classifies every substantive commit adopt/adapt/skip/split with windows-touch flag + ADR-review verdict per cluster) — ✓ VERIFIED. Cluster Summary table (line 481) has non-blank disposition/windows-touch/security-relevant cells for all 5 CODE clusters; every cell traces to either a per-commit table or a documented grep sweep (spot-checked and confirmed accurate).
2. **SC2** (re-export/public-surface diffs inspected, not just `--name-only`) — ✓ VERIFIED. NET/PROF/CORE tables carry a "re-export scan" column with cited `pub mod`/`pub use` grep results; the highest-risk cross-crate touch (`8a4237f2`'s `bindings/c/src/sandbox.rs`) and `e6d26871`'s `pub mod resource;` addition are both explicitly flagged, not glossed over.
3. **SC3** (7 named tool-sandbox refinement PRs recorded DEFERRED→v3.7) — ✓ VERIFIED. All 7 (#1280/#1322/#1325/#1384/#1394/#1413/#1417) traced to specific SHAs with DEFERRED→v3.7 dispositions (4 pure, 3 split-with-residue).
4. **SC4** (maps each will-sync cluster onto Phase 109/110/111) — this SC is **known-unsatisfiable per D-19**, not a phase failure. Confirmed correctly handled: the ledger reports an exact, hand-verified 27-commit gap (not the CONTEXT.md ~28 approximation), names every unmapped SHA, and proposes a Phase 112 amendment gated on explicit operator approval — with `ROADMAP.md` confirmed genuinely unedited (`git log --oneline -- .planning/ROADMAP.md` shows no Phase 108 commit touching it; last touch was the pre-phase milestone stand-up `cdffe6fb`/plan-creation `de6998e9`). Per the verification_context instructions for this task, this is treated as goal-achieved, not a gap.

### Anti-Patterns Found

None. No `TBD`/`FIXME`/`XXX` debt markers remain live in either deliverable (the two `TBD` string
hits in the ledger are prose describing that TBDs were removed). No stub content, no hardcoded
empty placeholders — this is a documentation deliverable and its substance was independently
verified against live git history rather than merely checked for the absence of code smells.

### Behavioral Spot-Checks / Probe Execution

N/A — Phase 108 is an audit/documentation-only phase per D-08 ("flags and names paths; does not
pre-compute hunk-level patches" and "no cherry-picks, no version bump, no absorb work"). No
runnable code or probes were produced or expected. In lieu of behavioral spot-checks, this
verification independently re-derived the ledger's core empirical claims against live git history
(see Observable Truths table) — a stronger check for a data/analysis deliverable than a runnable
smoke test would be.

### Human Verification Required

None. Every claim in this phase's deliverables was independently re-derivable via git commands run
directly against the actual upstream/fork repositories, with no UI, visual, or subjective-judgment
component.

### Gaps Summary

No gaps. This is one of the more rigorously self-checking deliverables seen in this project's
history: the ledger's own "Completeness Verification" section performs the same dedup-against-git
sweep this verification independently re-ran, and every number this verifier attempted to
falsify (100-commit partition, 4 spot-checked residue tables, 4 carve-out re-touch checks, ~15
SHA-subject pairs, the crossbeam-epoch live-vulnerability claim, and ADR-108's `net_filter.rs`
grounding) held up under direct git/file inspection. The two pre-declared known limitations
(ROADMAP SC4 unsatisfiable-as-written per D-19, and no RESEARCH.md/VALIDATION.md per the
deliberate research-skip) are both correctly handled per the phase's own design, not defects.

**Verdict: Phase 108's goal is achieved.** The ledger and ADR-108 give Phases 109/110/111 a
classified, ADR-reviewed, independently-reproducible work-list, with the tool-sandbox subsystem
provably fenced off to v3.7 and the three highest-risk carried findings (crossbeam-epoch RUSTSEC,
`pub mod resource` ADR-86 flag, PROF-02/dynamic_providers cross-dependency) recorded with enough
precision that a downstream planner cannot miss them. Ready to proceed — subject to the
pre-declared, correctly-surfaced operator gate: the proposed Phase 112 roadmap amendment requires
explicit operator approval before `/gsd:plan-phase 109` is run (this is a process gate stated by
the phase's own design, not a verification gap).

---

*Verified: 2026-07-29*
*Verifier: Claude (gsd-verifier)*
