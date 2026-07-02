---
phase: 98-upst11-divergence-audit
plan: "03"
subsystem: audit-documentation
tags: [upst11, network-intent, adr, divergence-audit, proxy]
dependency_graph:
  requires: [98-01, 98-02]
  provides: [ADR-98-network-intent-disposition]
  affects: [phase-99-absorb]
tech_stack:
  added: []
  patterns: [ADR pattern mirroring ADR-86/ADR-87]
key_files:
  created:
    - proj/ADR-98-network-intent-disposition.md
  modified: []
decisions:
  - "ADR-98 Decision: full-sync-adopt upstream #1225 NetworkIntent refactor in Phase 99"
  - "ADR-86 non-regression confirmed: NetworkMode::ProxyOnly in crates/nono/src/ is untouched by #1225 (CLI-only)"
  - "Windows WFP/AppContainer non-regression confirmed: exec_strategy_windows/network.rs reads WindowsNetworkPolicyMode::ProxyOnly (enforcement-time library type, not CLI intent type)"
  - "Phase 99 deviations required: WSL2ProxyFallback preservation in profile/mod.rs + CompiledEndpointPolicy compatibility in proxy_runtime.rs"
  - "d457ecc3 (#1263 validate_block_net_conflicts) adopts together with 72bcfd66 (#1225)"
metrics:
  duration: "30 min"
  completed: "2026-06-30"
  tasks: 2
  files: 1
---

# Phase 98 Plan 03: #1225 NetworkIntent Disposition ADR — Summary

**One-liner:** ADR-98 settles the #1225 NetworkIntent-vs-ProxyOnly adopt-vs-diverge call as
full-sync-adopt — CLI-only refactor with ADR-86 library boundary and Windows WFP model both
confirmed non-regressed by actual-diff inspection.

## Tasks Completed

| Task | Description | Commit | Files |
|------|-------------|--------|-------|
| 1 | git-show #1225 + map every fork ProxyOnly/CompiledEndpointPolicy touchpoint | 84e511dc | proj/ADR-98-network-intent-disposition.md (created) |
| 2 | Weigh adopt-vs-diverge options and render the recommendation | 84e511dc | proj/ADR-98-network-intent-disposition.md (extended) |

(Both tasks are captured in a single commit since they produce a single artifact written atomically.)

## Key Artifacts

### `proj/ADR-98-network-intent-disposition.md`

The standalone ADR settling the #1225 NetworkIntent-vs-ProxyOnly disposition. Contains:
- **Upstream #1225 shape:** diff-grounded from `git show 72bcfd66` — 11-file CLI refactor
  introducing `pub(crate) enum NetworkIntent { Unrestricted, BlockAll, ProxyFiltered(...) }`;
  replaces `ProxyOnly { port: 0 }` placeholder pattern; CLI-side only.
- **Fork Touchpoint Map:** 4 categories — (1) library core (16 sites, NOT touched by #1225),
  (2) CLI conflict zone (17+ sites, the actual conflict), (3) Windows backend (6 sites, ADR-86
  carve-out zone, not touched by #1225), (4) Phase 95 endpoint-policy surface (9 proxy sites,
  indirect conflict via proxy_runtime.rs).
- **Options Considered:** Option A (full-sync-adopt) vs Option B (fork-diverge carve-out),
  each with affected-fork-invariants table covering ADR-86 boundary, Windows WFP/AppContainer
  model, CompiledEndpointPolicy surface, and cfg-gated cross-target clippy flag.
- **Decision:** full-sync-adopt (Option A).
- **Consequences:** Phase 99 absorb implications — guard tests to preserve, WSL2ProxyFallback
  deviation, endpoint-policy compatibility check, mandatory cross-target clippy commands.

## Decision Rationale

The decision rests on two non-negotiable non-regression facts established by actual-diff:

1. **ADR-86 library boundary non-regressed:** `NetworkMode::ProxyOnly` in `crates/nono/src/capability.rs`
   and all sandbox backends (linux.rs seccomp fallback, macos.rs Seatbelt, windows.rs
   WindowsNetworkPolicyMode derivation) are entirely untouched by #1225. The library remains
   policy-free — it applies only what clients add to `CapabilitySet`. This is guaranteed by
   construction: #1225 is CLI-only.

2. **Windows WFP/AppContainer non-regressed:** `exec_strategy_windows/network.rs` reads
   `WindowsNetworkPolicyMode::ProxyOnly` — derived from `NetworkMode::ProxyOnly` at sandbox
   application time in `sandbox/windows.rs:333`. This is the enforcement-time type, not the
   CLI intent type. No changes to `exec_strategy_windows/` are required under adoption. The
   ADR-86 D-02 Windows denial-rendering carve-out remains fully intact.

Given both non-regressions hold trivially under adoption (because #1225 is CLI-only), the
decision is: carry permanent CLI divergence (Option B, creating indefinite per-sync conflict
debt on 11 actively-evolving files) vs invest one Phase 99 moderately-complex cherry-pick
(Option A). Phase 86 precedent (adopted a higher-conflict 2200-LOC refactor) makes Option A
the clear choice.

## Deviations from Plan

None. Plan executed exactly as written. Both task acceptance criteria passed on first run.

**Note on commit structure:** Tasks 1 and 2 were committed in a single `docs(98-03)` commit
(84e511dc) rather than two separate commits. The ADR artifact is a single file written
iteratively across both tasks; the final Write tool call produced the complete document covering
both sections (Context + Touchpoint Map for Task 1, plus Options + Decision + Consequences for
Task 2). The single commit reflects the atomic artifact delivery.

**Note on `git add -f`:** `proj/` is in `.gitignore` (GSD tooling marker) but the existing
ADR files (ADR-86, ADR-87) are tracked via force-add. The same pattern was applied to
ADR-98 — consistent with project convention.

## Known Stubs

None. The ADR is a complete analysis document with an explicit Decision section. No data
sourced from mock inputs; all touchpoint lines are verified against the live codebase via Grep.

## Threat Flags

None. This plan produces a markdown ADR only; it introduces no code, no attack surface, and
no data handling.

## Self-Check: PASSED

- [x] `proj/ADR-98-network-intent-disposition.md` exists
- [x] Commit `84e511dc` exists in git log
- [x] `## Context` section contains upstream #1225 shape from `git show 72bcfd66`
- [x] `## Fork Touchpoint Map` enumerates ProxyOnly + CompiledEndpointPolicy touchpoints with file:line
- [x] `## Options Considered` weighs both options with invariant tables and cfg-gated surface flag
- [x] `## Decision` is explicit (full-sync-adopt — not TBD)
- [x] `## Consequences` states Phase 99 absorb implications + ADR-86/Windows non-regression guarantees
- [x] Fork has no `NetworkIntent` confirmed (grep -rl NetworkIntent crates/ → empty)
