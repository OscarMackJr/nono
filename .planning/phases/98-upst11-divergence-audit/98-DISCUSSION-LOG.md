# Phase 98: UPST11 Divergence Audit - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-29
**Phase:** 98-upst11-divergence-audit
**Areas discussed:** #1225 stance, Carve-out scope, ADR output, Inspection depth

---

## #1225 NetworkIntent disposition

| Option | Description | Selected |
|--------|-------------|----------|
| Open ADR, diff-grounded rec | Audit reads the actual #1225 diff + maps every fork ProxyOnly/endpoint_policy touchpoint, then the ADR presents adopt-vs-diverge with a recommendation — no pre-commitment. | ✓ |
| Lean adopt (Phase 86 precedent) | Default toward full-sync-adopt (convergence) as v3.1 Phase 86 did; ADR documents carve-outs. Burden of proof on diverging. | |
| Lean fork-preserve | Default toward a fork-divergence carve-out keeping NetworkMode::ProxyOnly; burden of proof on adopting. | |

**User's choice:** Open ADR, diff-grounded recommendation.
**Notes:** The call must be evidence-driven — the audit fetches and `git show`s the
real #1225 commit(s) and maps every fork touchpoint before the ADR renders a
recommendation. → CONTEXT.md D-04/D-05.

---

## Carve-out re-touch check scope

| Option | Description | Selected |
|--------|-------------|----------|
| Expanded | Original 3 (CR-02 / CR-01 / Cluster F) PLUS the v3.3 Phase 95 carve-outs (endpoint_policy wiring + 4 restored invariants) and v3.2 override surface. | ✓ |
| Original 3 only | Just CR-02 / CR-01 / Cluster F as in Phase 94. | |

**User's choice:** Expanded.
**Notes:** #1225 hits the exact network/endpoint_policy surface Phase 95 touched, so
the newer carve-outs are the most exposed this window. → CONTEXT.md D-07/D-08.

---

## #1225 ADR output location

| Option | Description | Selected |
|--------|-------------|----------|
| Standalone proj/ADR-NN | A citable proj/ADR-98-network-intent-disposition.md (mirrors ADR-86/87). | ✓ |
| Inline ledger section | A '#1225 Disposition' section inside the DIVERGENCE-LEDGER. | |

**User's choice:** Standalone `proj/ADR-98-network-intent-disposition.md`.
**Notes:** Independently citable by Phase 99 cherry-pick guidance and future
auditors; survives ledger archival. → CONTEXT.md D-06.

---

## Inspection depth

| Option | Description | Selected |
|--------|-------------|----------|
| Uniform actual-diff (D-09) | `git show` every substantive commit, re-confirming each quick-ledger disposition at commit granularity. | ✓ |
| Risk-tiered (Phase 85 style) | Full-diff only HIGH-CONFLICT/ADAPT/security/cfg-gated rows; trust VERIFY/docs/dep rows with a lighter scan. | |

**User's choice:** Uniform actual-diff.
**Notes:** Window (~19 PRs) is small enough to afford it; uniform inspection kills
the additive-hiding risk. The 260629-toe per-PR dispositions are a hypothesis to
re-confirm per-commit, not ground truth. → CONTEXT.md D-09.

---

## Claude's Discretion

- Cluster naming/theme labels, the exact empirical spot-check file set (beyond the
  mandatory D-07 carve-out paths), and the ledger's internal section ordering —
  follow the Phase 94/85 ledger shape.

## Deferred Ideas

- Actual cherry-pick / absorption + fork-invariant verify → Phase 99.
- #1225 *implementation* of whichever disposition the ADR picks → Phase 99.
- Crate leapfrog to 0.66.1 + pipeline reconcile + PyPI RouteConfig blocker + runbook → Phase 100.
- #1245 / #1251 CI reconciliation against the prepare-only pipeline → classified here, applied 99/100.
