---
phase: 54
slug: upst7-audit
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-06-01
---

# Phase 54 — Validation Strategy

> Per-phase validation contract. Phase 54 is a **doc-producing audit** (output:
> `DIVERGENCE-LEDGER.md`, zero code edits). "Validation" here = grep-falsifiable structural
> gates on the ledger + diff-inspect evidence, not unit tests. There is no test framework to run.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | none — audit produces a markdown ledger; validation is grep/structural assertions + diff-inspect evidence |
| **Config file** | none |
| **Quick run command** | `grep -E '^\| ' .planning/phases/54-upst7-audit/54-DIVERGENCE-LEDGER.md` (cluster rows present) |
| **Full suite command** | the SC-gate grep set below (all four SC checks) |
| **Estimated runtime** | ~5 seconds |

---

## Sampling Rate

- **After every task commit:** re-run the SC-gate grep for the section that task produced
- **After the plan completes:** run the full SC-gate grep set; all four must pass
- **Before `/gsd:verify-work`:** all SC gates green + the upstream re-fetch SHA recorded in frontmatter
- **Max feedback latency:** ~5 seconds (grep)

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 54-01-* (re-fetch) | 01 | 1 | REQ-UPST7-01 | — | upstream re-fetched; HEAD SHA + refetch_date in ledger frontmatter (SC3) | structural | `grep -E 'upstream_head_at_audit\|refetch_date' 54-DIVERGENCE-LEDGER.md` | ❌ W-task | ⬜ pending |
| 54-01-* (clusters) | 01 | 1 | REQ-UPST7-01 | — | every cluster row has a disposition ∈ {will-sync,fork-preserve,won't-sync,split} + a windows-touch column (SC1) | structural | `grep -E 'will-sync\|fork-preserve\|won.t-sync\|split' 54-DIVERGENCE-LEDGER.md` + `grep -i 'windows-touch' …` | ❌ W-task | ⬜ pending |
| 54-01-* (ADR review) | 01 | 1 | REQ-UPST7-01 | — | `## ADR review` confirms or revises Phase 33 Option A `continue` (SC1) | structural | `grep -A20 '## ADR review' 54-DIVERGENCE-LEDGER.md \| grep -iE 'Option A\|continue\|revise'` | ❌ W-task | ⬜ pending |
| 54-01-* (cross-check) | 01 | 1 | REQ-UPST7-01 | — | `## Empirical cross-check` verifies re-export isolation via diff-inspect (not `--name-only`) on fork-shared files (SC2) | structural | `[ "$(grep -c '^### File: ' 54-DIVERGENCE-LEDGER.md)" -ge 4 ]` (matches plan 54-01 Task 7's binding gate; ≥4 diff-inspected files) | ❌ W-task | ⬜ pending |
| 54-01-* (TLS C11) | 01 | 1 | REQ-UPST7-01 | — | TLS-interception (Phase 34 C11 fork-preserve) addressed with a diff-inspect clean-apply-vs-manual-replay note (SC4) | structural | `grep -iE 'tls.intercept\|C11' 54-DIVERGENCE-LEDGER.md` | ❌ W-task | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky · "W-task" = produced by an earlier task in the same plan*

---

## Wave 0 Requirements

- Existing tooling covers the audit: the divergence/drift tool (pinned per RESEARCH.md) + git. No new
  test framework. The mandatory **upstream re-fetch** (`git fetch upstream --tags`, SC3) is the audit's
  effective "Wave 0" — it must run first or the drift output is wrong/empty (local `upstream/main` is
  stale at the v0.57.0 era).

*No unit-test stubs apply — this phase writes a doc, not code.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Cluster dispositions are CORRECT (not just present) | REQ-UPST7-01 | Disposition correctness is an analytical judgment, not grep-checkable | Human reviews each cluster's will-sync/fork-preserve/won't-sync/split call against the diff-inspect evidence |
| SC4 TLS clean-apply vs manual-replay verdict is accurate | REQ-UPST7-01 | Requires reading the actual v0.59 TLS-intercept diff against the fork's `nono-proxy` surface | Human confirms the diff-inspect note's clean/replay conclusion |
| Scope-boundary decision (include v0.60.0 or defer to UPST8) | REQ-UPST7-01 | Product/scope judgment (v0.60.0 was cut after the gap analysis) | Human decides; ledger records the decision + rationale |

---

## Validation Sign-Off

- [ ] All SC gates (1–4) have a grep-falsifiable structural check above
- [ ] Sampling continuity: each ledger section has a verify gate
- [ ] Wave 0 (upstream re-fetch) recorded with HEAD SHA + date in ledger frontmatter
- [ ] No watch-mode flags (N/A — no test runner)
- [ ] Feedback latency < 5s (grep)
- [ ] `nyquist_compliant: true` set in frontmatter once gates wired into the plan

**Approval:** pending
