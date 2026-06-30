---
quick_id: 260629-toe
slug: v066-parity
status: complete
date: 2026-06-29
---

# Summary — UPST11 v0.66.0 Parity Phase Definition

Reviewed upstream `nolabs-ai/nono` **PR #1293** (`chore: release v0.66.0`, MERGED) and authored a
ready-to-promote **phase definition** for bringing the fork to parity with v0.66.0.

## Key findings

- **#1293 is a release-cut PR only** (CHANGELOG + version bumps `0.65.1→0.66.0`). The real work is
  the ~19 PRs it references. **Parity gap = `v0.65.1 → v0.66.0`** (fork's high-water mark is
  v0.65.1, absorbed in v3.3/UPST10).
- **Highest-conflict item: #1225** (introduce `NetworkIntent`, remove `ProxyOnly`) — touches the
  fork's core `NetworkMode::ProxyOnly` across `capability.rs`, `manifest_convert.rs`,
  `sandbox/linux.rs`; fork has no `NetworkIntent`. Needs a deliberate adopt-vs-fork-divergence call.
- **Version collision:** fork is already at `0.66.0` (leapfrogged in v3.3); upstream now ships
  `0.66.0` too → next fork release must leapfrog **≥0.67.0**. Do not publish `0.66.0`.
- Quick-scan dispositions set for all 19 PRs (ADOPT/ADAPT/VERIFY/HIGH-CONFLICT) with per-PR absorb
  notes; #1235 org-rename and #1127 `--allow-endpoint` likely already partly present in fork.

## Deliverable

`.planning/quick/20260629-v066-parity/PLAN.md` — divergence ledger, fork-invariant gate list,
recommended 3-wave phase structure (Audit → Absorb+Verify → Release-reconcile), and promotion path.

## Next step

Promote via `/gsd-new-milestone` (UPST11 — Upstream Sync to v0.66.0) or `/gsd-phase add`. Execution
is **not** done here — the merge follows the audited audit→absorb→verify path (no blind cherry-pick).
