# Phase 36: UPST3 deep closure - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-12
**Phase:** 36-UPST3 deep closure
**Areas discussed:** Initial gray-area selection, REQ-02 deprecated_schema port shape, REQ-04 wiring.rs + yaml_merge strategy, REQ-05 b5f0a3ab + bbdf7b85 strategy, overall plan sequencing & wave structure

---

## Initial Gray-Area Selection

| Option | Description | Selected |
|--------|-------------|----------|
| REQ-02 deprecated_schema port shape | Verbatim / phased / adapted port options | ✓ |
| REQ-04 wiring.rs base port strategy | Full braiding / cherry-pick only / stripped-down | (added later in session) |
| REQ-05 ExecConfig refactor strategy | Adopt upstream / surgical port / hybrid | (added later in session) |
| Plan slicing, sequencing & 36.5 fold-in | Parallel / sequential / fold-in options | (added later in session) |

**User's choice:** REQ-02 only initially; later extended to discuss REQ-04, REQ-05, and wave structure.
**Notes:** Multi-select question; user originally selected only REQ-02. After REQ-02 completed (4 questions), Claude asked whether to extend; user chose "Discuss them now (extend this session)" which added REQ-04 + REQ-05 + plan slicing to the discussion.

---

## REQ-02 Port Shape

| Option | Description | Selected |
|--------|-------------|----------|
| Full verbatim port (~1-2 weeks, high churn) | All 824 LOC + 210-callsite rename + canonical sections + JSON schema restructure + data + docs migration. Maximum byte-for-byte parity with upstream. | ✓ |
| Phased (LegacyPolicyPatch + DeprecationCounter first; rename later) | 2-3 plans across machinery → rename → data/docs. Lets heavy machinery land independently. | |
| Adapted (skip 210-callsite rename; keep Option C base) | Add machinery but don't rename internally. ~25-30% less code churn. | |
| Minimal (only --strict mode + DeprecationCounter) | Fastest closure but leaves REQ-02 acceptance #1, 4, 5, 6 unsatisfied. | |

**User's choice:** Full verbatim port.
**Notes:** Future P34-DEFER absorptions pick up canonical surface for free; fork stops accumulating divergence on profile-schema naming.

---

## REQ-02 Plan Slicing

| Option | Description | Selected |
|--------|-------------|----------|
| Four sub-plans per deferred-items.md split (Recommended) | 36-01a (module) → 36-01b (canonical sections) → 36-01c (rename) → 36-01d (data + docs + tooling). Per-plan rollback independence. | ✓ |
| Two sub-plans: machinery first, surface second | Machinery + struct shape; then callsite + content. | |
| Single mega-plan | Everything atomic; 14+ file / 824 LOC + 210 callsites diff. | |
| Three sub-plans (collapse 2c+2d) | Module foundation; canonical sections; rename + data + docs. | |

**User's choice:** Four sub-plans per deferred-items.md split.
**Notes:** Each plan owns a single change-class; per-plan rollback independence valued over atomic landing.

---

## REQ-02 Legacy Key Migration Window

| Option | Description | Selected |
|--------|-------------|----------|
| Indefinite serde alias acceptance + one-shot warn (Recommended) | Both keys accepted in JSON forever; non-strict warns once per key per process; strict fails closed. Mirrors upstream exactly. | ✓ |
| Hard-deprecate at v2.5 | Phase 36 lands warn + strict; v2.5 removes serde aliases. Forces clean canonical-only state. | |
| Strict-default-on for new installs, off for upgrades | Mid-ground; new installs default strict; upgrades preserve current. | |
| Defer policy decision to Phase 36 close | Land both modes, decide cutover later via ADR. | |

**User's choice:** Indefinite serde alias acceptance + one-shot warn.
**Notes:** No forced migration date in v2.4; future hard-deprecation tracked as v2.5-FU-5 in CONTEXT.md deferred section.

---

## REQ-02 Rename Approach (Plan 36-01c)

| Option | Description | Selected |
|--------|-------------|----------|
| Atomic single-commit (Recommended) | One commit, mechanical rename across 14+ files. Reviewer sees one clean diff. | ✓ |
| Staged file-by-file mini-commits with type-alias bridges | ~14 commits + throwaway bridge scaffolding in git history. | |
| Two-step: types renamed first, then callsites | 3 small commits + temporary type alias for one commit window. | |

**User's choice:** Atomic single-commit.
**Notes:** Matches Phase 33 + Phase 34 atomic-cherry-pick discipline; one revert rolls back if regression surfaces post-merge.

---

## Bridge — Extend Session to REQ-04 + REQ-05 + Plan Slicing?

| Option | Description | Selected |
|--------|-------------|----------|
| Discuss them now (extend this session) | Lock decisions for REQ-04 + REQ-05 + plan slicing. ~6-8 more questions. | ✓ |
| Default-flag for planner with recommended posture | CONTEXT.md captures REQ-02 + flags others with recommended defaults. | |
| Mark REQ-04/REQ-05 as "needs follow-up discussion before planning" | Forces explicit decision; slowest but most explicit. | |

**User's choice:** Extend this session.
**Notes:** User wanted unambiguous planner direction on all three REQs.

---

## REQ-04 Wiring.rs Strategy

| Option | Description | Selected |
|--------|-------------|----------|
| Stripped-down port: yaml_merge directive only (Recommended) | ~300-400 LOC; yaml_merge + serde_yaml_ng pin + reversal test. Skip WriteFile/JsonMerge/JsonArrayAppend. Acceptance #1 scope-trimmed. | ✓ |
| Full braiding: port wiring.rs + braid with fork's package system (~2-3 wks) | ~1761 LOC + full braiding with hooks.rs + validate_path_within. Full REQ-04 acceptance. HIGH risk. | |
| Cherry-pick yaml_merge only, full defer wiring.rs base | Thin fork-side wiring.rs with just yaml_merge; defer base abstraction to v2.5. | |
| Stripped-down + future-proof scaffold | yaml_merge today + empty trait/module shell for v2.5 follow-up. | |

**User's choice:** Stripped-down port (yaml_merge only).
**Notes:** Fork's package/hooks system preservation (D-34-B1) outweighs the HIGH-VALUE SHA-256 install records; deferred to v2.5-FU-3 with dedicated 2-3 week D-20 plan.

---

## REQ-04 Commit Shape

| Option | Description | Selected |
|--------|-------------|----------|
| D-20 manual-replay shape — single combined commit (Recommended) | One commit citing all 3 upstream commits as design-source; no D-19 trailer. | ✓ |
| D-19 cherry-pick trailer — three sequential commits | Three commits, each with full 6-line trailer block. Reviewers must read each body to understand the deviation. | |
| Hybrid — D-19 trailers on content commits, D-20 on skeleton | Skeleton (D-20) + 3 sequential cherry-picks (D-19). | |

**User's choice:** D-20 single combined commit.
**Notes:** Cleanest provenance shape given the structural infeasibility of clean cherry-pick into upstream-only wiring.rs.

---

## REQ-05 Strategy

| Option | Description | Selected |
|--------|-------------|----------|
| Keep fork's ExecConfig shape; surgically port helpers (Recommended) | Do NOT refactor ExecConfig struct. Surgically port helpers + bbdf7b85 tail. All 8+ fork fields stay intact. | ✓ |
| Adopt upstream's ExecConfig shape; explicitly add fork-side fields | Refactor to upstream's shape, add fork fields as extension. Full b5f0a3ab parity. HIGH risk to Phase 18/26/27/31/34-08a surfaces. | |
| Hybrid: adopt function-level helpers, keep struct layout | Keep struct verbatim; restructure callers to upstream's idiom. | |

**User's choice:** Keep fork's ExecConfig shape; surgical helper port.
**Notes:** Fork's 8+ extra ExecConfig fields are load-bearing for multiple defense surfaces; surgical port minimizes risk; user-visible improvements absorbable via function-level helpers without restructuring.

---

## REQ-05 Plan Slicing

| Option | Description | Selected |
|--------|-------------|----------|
| Single plan 36-03, sequenced commits (Recommended) | One plan, 3 sequenced commits: b5f0a3ab diagnostic.rs restoration (D-20), b5f0a3ab exec_strategy + execution_runtime + cli.rs (D-20), bbdf7b85 body rewrite (D-19 cherry-pick). | ✓ |
| Two plans: 36-03a surgical port, 36-03b escape-quote tail | Wave-sequential split. | |
| Three plans split by domain | 36-03a diagnostic.rs; 36-03b exec_strategy/execution_runtime; 36-03c bbdf7b85. | |

**User's choice:** Single plan 36-03 with sequenced commits.
**Notes:** Bundling b5f0a3ab + bbdf7b85 keeps the dependency chain intact (bbdf7b85 depends on b5f0a3ab's analyze_error_output wiring) without inter-plan ordering ceremony.

---

## Wave Shape

| Option | Description | Selected |
|--------|-------------|----------|
| Mostly parallel: REQ-02/04/05 disjoint, REQ-02 sub-plans sequential (Recommended) | Wave 1 (parallel): 36-01a + 36-02 + 36-03. Wave 2 (sequential): 36-01b → 36-01c → 36-01d. ~4 weeks. | ✓ |
| Strict sequential: REQ-02 first (foundation), REQ-04 second, REQ-05 last | All 6 plans sequential. Lowest risk; ~5-6 weeks. | |
| Pure wave-parallel: all 6 plans in one wave | All plans parallel from day 1; risk of REQ-02 sub-plan collisions on Profile struct. | |
| Mostly parallel + fold Phase 36.5 in as 7th plan | Option 1 + Phase 36.5 (profile drafts) folded as Plan 36-04. ~5-7 weeks. | |

**User's choice:** Mostly parallel; REQ-02 sub-plans sequential; Phase 36.5 stays separate.
**Notes:** Balances reviewer attention with wall-clock efficiency. Phase 36.5 stays in its own phase per ROADMAP planner-discretion default.

---

## Claude's Discretion (deferred to planner)

- Exact plan-letter suffix conventions for 36-01a/b/c/d (theme-readable suffix recommended).
- Wave-1-to-Wave-2 transition shape (overlap permitted if 36-01a closes before 36-02/36-03 finish).
- Whether to merge 36-01b + 36-01c if file scope overlaps heavily (recommendation: keep separate).
- Specific test naming inside Plan 36-03 beyond the 3 locked upstream tests.
- PR title conventions, draft vs ready-for-review state at open, reviewer assignment.
- Phase 36 SUMMARY closure section append (last plan to close, likely 36-01d, owns the consolidated append).
- Whether to add a regression test for LegacyPolicyPatch rewriter round-trip (recommendation: yes, as a property-level invariant).
- PROJECT.md milestone summary line update — handled by `/gsd-progress` at Phase 36 close.

## Deferred Ideas

- **v2.5-FU-3:** Full wiring.rs base abstraction port (~1761 LOC; WriteFile + JsonMerge + JsonArrayAppend + SHA-256 install records + lockfile v3+v4 + idempotent reversal + `--force` on `nono remove`). Closes REQ-PORT-CLOSURE-04 acceptance #1 fully. 2-3 weeks D-20 plan.
- **v2.5-FU-4:** Upstream-shape ExecConfig adoption with per-field migration audit.
- **v2.5-FU-5:** Hard-deprecation ADR for legacy `override_deny` key + migration tool.
- **v2.5-FU-6:** Parametric regression test surface for PTY-quiet-period (50ms / 100ms / 150ms / 250ms arms).
- **`run_nono` integration tests for Phase 36 surface** — host-blocked; defer to Phase 37/38.
- **Phase 36.5 — REQ-PORT-CLOSURE-03 profile drafts** — separate phase per ROADMAP planner-discretion default.
- **PTY-quiet-period parametric proptest** — adds proptest setup cost; reconsider if PTY interactions reveal complexity post-Phase 36.
- **Audit-event emission for profile-validate `--strict` rejections** — D-34-B2 surgical-retrofit posture defers; reconsider in v2.5+.
