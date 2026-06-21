---
phase: 89-proxy-hardening-sync
plan: 04
wave: 2
status: complete
requirements: [PROXY-01, PROXY-02]
tags: [divergence-ledger, cluster-f, reconciliation, D-11, docs-only]
---

# Phase 89 Plan 04: Cluster F Reconciliation Ledger Addendum Summary

**One-liner:** Appended the `## Phase 89 Cluster F Reconciliation Addendum` to the Phase-85
divergence ledger (D-11), recording the four equivalence findings, two won't-sync findings, and the
one deliberate fork-divergence (D-07) with their guard-test fn names — so future upstream syncs
expect the Cluster F divergence and never blind-cherry-pick the `tls_intercept/` / `RouteSelection`
/ `TlsInterceptIntent` hunks.

## What was built

A new append-only section in
`.planning/phases/85-upst9-divergence-audit/85-DIVERGENCE-LEDGER.md`, mirroring the Phase 87 CR-02
and Phase 88 CR-01 addendum format, with four sub-sections:

1. **Equivalence findings** (no fork change — guarded by a test):
   - `a5d623fd` (#1077, D-09) → `denied_endpoint_returns_403_and_audit`
   - `b5f8db5c` (#1048/#1091, D-01) → `build_proxy_config_maps_upstream_proxy_to_external_proxy`
   - `7c9abd3b` (#1151, D-02) → `connect_keeps_open_on_missing_proxy_auth`
   - `b0b2c743` (#1132, D-10) → `allow_domain_endpoint_route_does_not_shadow_credential_route`
2. **Won't-sync findings** (architecture divergence):
   - `76b7b695` (#1192, D-05) → won't-apply (`forward_inner_request` in absent `tls_intercept/`)
   - `bd4b6b7f` (#1199, D-04) → won't-sync (intent/activation refactor + `TlsInterceptIntent`)
3. **Deliberate fork-divergence landed** (D-07): `crates/nono-cli/src/proxy_runtime.rs` activation
   predicate now includes `!prepared.custom_credentials.is_empty()`; upstream reference `724bb207`
   (#1197); fix commit `0c08e5d2`.
4. **Future sync note** naming all six guard tests as reversion guards and reiterating: preserve the
   fork's exact-prefix `RouteStore` + `EffectiveProxySettings`; do NOT import `RouteSelection` or
   `TlsInterceptIntent`.

No Cluster F test reproduced a real gap — every equivalence holds and the #1132 shadow class is
structurally absent on the fork's exact-prefix model, so no row moved from Equivalence to
fork-divergence.

## Tasks

| # | Task | Commit | Files |
|---|------|--------|-------|
| 1 | Append Phase 89 Cluster F Reconciliation Addendum | (see commit below) | `.planning/phases/85-upst9-divergence-audit/85-DIVERGENCE-LEDGER.md` |

## Verification

- `grep -c "Phase 89 Cluster F Reconciliation Addendum"` → 1
- Prior addenda intact: "Phase 87 CR-02 Addendum" (1), "Phase 88 CR-01 Addendum" (1)
- All seven Cluster F commit SHAs present: `a5d623fd`, `b5f8db5c`, `7c9abd3b`, `b0b2c743`,
  `76b7b695`, `bd4b6b7f`, `724bb207`; plus D-07 fix `0c08e5d2`
- All five+ guard-test fn names present
- `**Future sync note:**` paragraph present with `RouteSelection` + `TlsInterceptIntent` do-not-import context
- DCO sign-off present on the commit

## Notes / deviations

- **Executed inline by the orchestrator** rather than via a background worktree executor: two
  consecutive background worktree agents (a01dba32, a7b6e15c) were denied Bash access (no
  interactive approver exists in background mode) and could not run the mandatory git operations.
  The work is a single docs-only append, so the orchestrator performed it directly on the main tree
  with the same append-only + DCO contract. No partial state was left by the failed agents (verified:
  no SUMMARY, no commits, no residual worktree/branch, ledger untouched, HEAD unchanged).
- Docs-only change — no code build/test required; the guard tests are verified by 89-01/02/03.

## Self-Check: PASSED
