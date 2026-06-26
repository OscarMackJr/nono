# Phase 95: Upstream Absorb + Fork-Invariant Verify - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-26
**Phase:** 95-Upstream Absorb + Fork-Invariant Verify
**Areas discussed:** Cluster B split depth, Cluster C / Phase 89 conflict, Cross-target sequencing, Post-sync test gate

---

## Cluster B — tool-sandbox split depth

| Option | Description | Selected |
|--------|-------------|----------|
| Shared-surface only (Rec) | Extract additive audit.rs events + cleanly-applying proxy hunks; skip tool-sandbox dir + tls_intercept/ | ✓ |
| Audit-events only (minimal) | Take only additive audit.rs event types; defer all proxy/sandbox hunks | |
| Defer Cluster B entirely | Do only A + C this phase; route all of B to a later phase | |

**User's choice:** Shared-surface only
**Notes:** Matches the ledger's recommended split. Taking the whole tool-sandbox feature would be a new capability (own phase); audit.rs hit must stay additive-only vs CR-02.

---

## Cluster C — Phase 89 fail-secure divergence conflict

| Option | Description | Selected |
|--------|-------------|----------|
| Preserve fork + fix only (Rec) | Keep Phase 89 divergence + guard test; apply only the credentials_intent fix | ✓ |
| Adopt upstream behavior | Abandon Phase 89 divergence, take upstream refactor wholesale | |
| Hybrid | Take upstream activation but keep guard test as sentinel | |

**User's choice:** Preserve fork + fix only
**Notes:** Upstream 9b37dc52 reverses a deliberate fork security decision; fork-preserve wins. Keep `proxy_activates_with_custom_credentials_only` as regression sentinel.

---

## Cross-target clippy sequencing

| Option | Description | Selected |
|--------|-------------|----------|
| Land in 95, clippy → 96 (Rec) | Cherry-pick A & B now, native Windows gate; cross-target clippy PARTIAL→96 | ✓ |
| Block 95 on Phase 96 | Stand up toolchain before absorbing (inverts roadmap order) | |
| Native-only, no deferral | Windows clippy only; cross-target out of scope (rely on CI) | |

**User's choice:** Land in 95, clippy → 96
**Notes:** Matches established fork pattern; preserves 95→96 roadmap order. Ledger cross-target notes carried forward into Phase 96 checklist.

---

## Post-sync test gate strictness

| Option | Description | Selected |
|--------|-------------|----------|
| No-new-failures vs baseline (Rec) | Pass if cherry-picks add zero new failures vs documented ~5-red Windows baseline | ✓ |
| Strict all-green | Fix pre-existing baseline reds too | |

**User's choice:** No-new-failures vs baseline
**Notes:** Matches how phases 57-59 handled the known nono-cli + try_set_mandatory_label baseline reds. Planner captures baseline at phase-base commit before any cherry-pick so "new" is provable.

---

## Claude's Discretion

- Cluster D `sigstore-verify` dep-bump evaluation (ledger routed to "Phase 95 DEPS"): planner decides absorb-now vs fold-into-97.
- Cherry-pick mechanics (`-x` vs manual replay), commit ordering, plan/wave decomposition.

## Deferred Ideas

- Cluster D release metadata + leapfrog floor ≥ 0.65.0 → Phase 97.
- Full tool-sandbox subsystem absorb (skipped #1105 hunks) → future UPST cycle / own phase.
- Reviewed-not-folded todos: MSI VC++ prereq + POC-cert broker clean-host (FUT-03 distribution).
