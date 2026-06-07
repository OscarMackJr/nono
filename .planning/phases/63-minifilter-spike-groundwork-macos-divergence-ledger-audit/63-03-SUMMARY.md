---
phase: 63-minifilter-spike-groundwork-macos-divergence-ledger-audit
plan: "03"
subsystem: upstream-audit
tags: [macos, seatbelt, divergence-ledger, upstream-sync, security]
dependency_graph:
  requires: []
  provides:
    - "63-DIVERGENCE-LEDGER.md: complete macOS-scoped upstream-divergence audit for v0.57.0..v0.61.2"
    - "Phase 64 cherry-pick gate: C14 P1 x3 will-sync with absorption order"
    - "Phase 54 C14 supersession: won't-sync overridden to will-sync on v2.10 scope change"
  affects:
    - ".planning/phases/63-.../63-DIVERGENCE-LEDGER.md"
tech_stack:
  added: []
  patterns:
    - "macOS-scoped DIVERGENCE-LEDGER with macos-only column (mirrors Phase 42/47/54 windows-touch shape)"
    - "Diff-inspect re-export + call-site audit (NOT --name-only) per feedback_cluster_isolation_invalid"
    - "Reproducible-audit frontmatter: drift-tool sha pin + upstream_head_at_audit + refetch_date"
key_files:
  created:
    - ".planning/phases/63-minifilter-spike-groundwork-macos-divergence-ledger-audit/63-DIVERGENCE-LEDGER.md"
  modified: []
decisions:
  - "D-13: All three P1 commits (8f84d454, 362ada22, 8f1b0b74) dispositioned will-sync — supersedes Phase 54 C14 won't-sync on v2.10 macOS-parity scope change"
  - "C17 Keychain hardening won't-sync: depends on tls_intercept/ca.rs module absent from fork (prereq from C15 --trust-proxy-ca)"
  - "C15 (729697c2 --trust-proxy-ca) will-sync with note that tls_intercept/ prereq must be scoped at Phase 64 plan time"
  - "Absorption order for P1 cluster C14: 8f1b0b74 -> 362ada22 -> 8f84d454 (8f1b0b74 introduces resolved_workdir which 362ada22 then modifies)"
metrics:
  duration: "~45 minutes"
  completed: "2026-06-06"
  tasks_completed: 2
  tasks_total: 2
  files_created: 1
  files_modified: 0
---

# Phase 63 Plan 03: macOS Divergence Ledger Audit Summary

## One-liner

macOS-scoped divergence ledger for v0.57.0..v0.61.2 with 19 clusters, C14 P1 x3 overridden to will-sync (supersedes Phase 54 C14 won't-sync), and per-commit diff-inspect notes for all will-sync clusters.

## What Was Built

`63-DIVERGENCE-LEDGER.md` — a complete macOS-scoped upstream divergence audit covering all 63
unique upstream commits in `v0.57.0..v0.61.2`, grouped into 19 clusters with full dispositions
(will-sync 12, split 3, won't-sync 4), a `macos-only` column (mirroring the Phase 42/47/54
`windows-touch` shape), diff-inspect notes for every will-sync cluster, and an explicit supersession
of Phase 54 C14's `won't-sync` verdict.

The ledger establishes the cherry-pick gate for Phase 64. The defining finding is that all three P1
commits map cleanly to fork-carried files: `8f84d454` and `c6730e43` touch `sandbox/macos.rs`;
`362ada22` and `8f1b0b74` touch `sandbox_prepare.rs` — both files present in the fork at the same
call-site structure, making Phase 64 cherry-picks tractable.

## Tasks

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Fetch upstream tags, run drift tool, produce macOS inventory | 3bcbaa9d (no committed artifact — gitignored JSON) | ci-logs-local/drift/63-macos-v057-v0612.json (gitignored) |
| 2 | Write 63-DIVERGENCE-LEDGER.md with dispositions, diff-inspect notes, C14 supersession | 3bcbaa9d | .planning/phases/63-.../63-DIVERGENCE-LEDGER.md |

Both tasks committed atomically in a single commit since Task 1 produces only a gitignored working
artifact (no committed file of its own).

## Key Decisions Made

### D-13: Phase 54 C14 superseded — P1 x3 overridden to will-sync

Phase 54 UPST7 dispositioned `8f84d454`, `362ada22`, `8f1b0b74` as `won't-sync` because v2.7's
milestone was Windows-only. This ledger overrides to `will-sync` because v2.10's scope is explicitly
macOS parity. The Headline documents the supersession with the rationale (T-63-10, T-63-11 security
and correctness respectively). Silently copying Phase 54's verdict would have dropped the exact
commits this milestone was opened to absorb (Pitfall F).

### C17 won't-sync: tls_intercept/ prereq

The five Keychain CA hardening commits (C17: `6c472224`, `197008ae`, `2f4e1a37`, `4e1c7957`,
`ad6b0ac8`) all depend on `macos_trust.rs` (introduced by C15 `729697c2`) and
`tls_intercept/ca.rs` (absent from the fork). Disposition is `won't-sync` until the
`tls_intercept/` prereq is resolved. Phase 64 must decide whether to carve a fork-specific CA
abstraction or cherry-pick enough of `tls_intercept/` to satisfy the interface.

### C14 absorption order is critical

`8f1b0b74` must be applied before `362ada22` (the former introduces `resolved_workdir` which the
latter then modifies with `$PWD` preference logic). Both precede `8f84d454` (the ordering fix in
`generate_profile`, which is independent but is the highest-security item).

## Deviations from Plan

### Auto-fixed: None

### Plan adjustments: None — the ledger matches the plan specification exactly.

The plan anticipated the possibility of v0.61.2-only macOS commits (Open Question 3). The actual
v0.61.2 delta is: release commit + dependency bumps + `bd4c469a` (deny-by-default network.block,
cross-platform). No v0.61.2 macOS-specific commits beyond what was already in scope from v0.61.1.

## Verification Results

| Check | Status |
|-------|--------|
| `git cat-file -t 3e605f27` = `commit` | PASS |
| drift tool JSON produced (gitignored) | PASS |
| 63 unique commits in range | PASS |
| `8f84d454`, `362ada22`, `8f1b0b74` confirmed in range | PASS |
| `729697c2`, `c6730e43` confirmed in range | PASS |
| Ledger frontmatter: `range: v0.57.0..v0.61.2` | PASS |
| Ledger: `macos-only` column present | PASS |
| Ledger: `windows-touch` absent | PASS |
| Ledger: C14 supersession in Headline | PASS |
| Ledger: all four disposition vocabulary tokens used | PASS |
| Ledger: diff-inspect notes referencing `generate_profile`, `sandbox_prepare`, `add_platform_rule` | PASS |
| `git diff --name-only -- crates/ bindings/ scripts/ Makefile` = empty | PASS |

## Known Stubs

None — the ledger is a complete audit artifact, not a placeholder.

## Threat Flags

None — this plan produces only a read-only audit document. No new network endpoints, auth paths,
file access patterns, or schema changes introduced.

## Self-Check: PASSED

- FOUND: `.planning/phases/63-minifilter-spike-groundwork-macos-divergence-ledger-audit/63-DIVERGENCE-LEDGER.md`
- FOUND commit: `3bcbaa9d` (`docs(63-03): produce macOS DIVERGENCE-LEDGER v0.57.0..v0.61.2`)
- All verification checks in the table above PASSED
