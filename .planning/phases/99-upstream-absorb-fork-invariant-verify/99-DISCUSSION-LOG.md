# Phase 99: Upstream Absorb + Fork-Invariant Verify - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-30
**Phase:** 99-upstream-absorb-fork-invariant-verify
**Areas discussed:** Scope authority + SC reconcile, Cluster C split strategy, Absorption mechanics, Verify gate + Cluster A deviations

---

## Scope authority + SC/REQUIREMENTS reconciliation

Discrepancy surfaced during analysis: the ROADMAP Phase 99 SC / REQUIREMENTS UPST11-02/03 enumerate
a large preliminary PR list (from the `260629-toe` guess off PR #1293) that disagrees with the Phase 98
ledger — notably the SC says "absorb tool-sandbox #1268…" but the ledger classifies tool-sandbox as
won't-sync, and several SC-listed PRs are not will-sync clusters in the actual `v0.65.1..v0.66.0` window.

| Option | Description | Selected |
|--------|-------------|----------|
| Ledger binding + reconcile SC | Ledger is the binding scope AND edit ROADMAP SC #1/#2 + REQUIREMENTS UPST11-02/03 to match reality (tracked Phase 99 task) | ✓ |
| Ledger binding, SC left as-is | Use ledger as scope, don't edit tracking; record mapping in SUMMARY (risk: verifier flags 'missing' PRs) | |
| Re-investigate gap PRs first | Spend a plan confirming the SC PRs are out-of-window/already-synced before accepting the ledger | |

**User's choice:** Ledger binding + reconcile SC
**Notes:** Captured caveat (D-02) — before deleting any SC-listed PR, the planner must confirm against
the ledger window that it is genuinely out-of-window / already-synced, not folded under a different SHA.

---

## Cluster C (split) absorption strategy

| Option | Description | Selected |
|--------|-------------|----------|
| Hand-replay applicable hunks | Manually replay pool.rs + shared surface (route/server/reverse) + endpoint wiring; never stage tls_intercept/ or h2_forward/h2_probe; cite upstream SHA; verify CompiledEndpointPolicy compat | ✓ |
| cherry-pick -x then drop hunks | cherry-pick with -x, then git rm tls_intercept/ + revert conflicting hunks | |
| Defer Cluster C to follow-up | Absorb only A + D/E/F/G now; split proxy work later | |

**User's choice:** Hand-replay applicable hunks
**Notes:** Cleanest fork-invariant control; the fork has no TLS-interception surface so the dropped
hunks must never be staged (Cluster F carve-out).

---

## Absorption mechanics (clusters A, D, E, F, G)

| Option | Description | Selected |
|--------|-------------|----------|
| cherry-pick -x, fallback manual | cherry-pick -x where clean, manual replay on conflict; one atomic DCO-signed commit per upstream commit; Cluster A first then D/E/F/G; Cargo.lock regen + sigstore cascade folded into Cluster F | ✓ |
| Manual replay everything | Uniform hand-replay, per-cluster commits (loses -x provenance) | |
| cherry-pick -x everything | cherry-pick all in-scope incl. Cluster C, resolve conflicts inline | |

**User's choice:** cherry-pick -x, fallback manual

---

## Verification depth + Cluster A deviations

| Option | Description | Selected |
|--------|-------------|----------|
| New targeted tests + existing gates | Author focused regression tests for WSL2ProxyFallback + CompiledEndpointPolicy, PLUS run Phase 89 proxy guards + linux.rs seccomp tests + both-gate clippy + make ci + code-review + verifier carve-out checklist | ✓ |
| Existing gates only | Run existing suites + gates; no new test authorship | |
| Full new suite per cluster | New tests for every absorbed cluster (heaviest) | |

**User's choice:** New targeted tests + existing gates

---

## Claude's Discretion

- Wave/plan breakdown (planner's call; suggested 4-stage shape recorded in CONTEXT.md, subject to
  D-07 dependency order + D-09 gates).
- Exact test names/locations for the D-08 deviation regression tests (executor's call).

## Deferred Ideas

- Cluster B tool-sandbox carry-forward (won't-sync; future phase if tool-sandbox adopted).
- Cluster H release metadata / 0.66.1 leapfrog (Phase 100).
- TLS-interception capability (dropped `tls_intercept/` hunks — deliberate fork divergence).
- Reviewed-not-folded todos: `20260611-msi-vcredist-prereq.md`, `20260611-poc-cert-broker-clean-host.md`
  (clean-host distribution / host-gated; not upstream-code absorb).
