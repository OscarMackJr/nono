---
phase: 47-upst6-audit-v0-41-v0-43-drift-ingestion
plan: 01
ledger_type: upst6-audit
range: v0.54.0..v0.57.0
upstream_head_at_audit: 807fca38efc768c4e9856a0cb5c47d961b9287e5
drift_tool_sh_sha: 0834aa664fbaf4c5e41af5debece292992211559
drift_tool_ps1_sha: 0834aa664fbaf4c5e41af5debece292992211559
drift_tool_invocation: 'make check-upstream-drift ARGS="--from v0.54.0 --to v0.57.0 --format json"'
fork_baseline: v0.54.0 (Phase 43 + 45 UPST5 sync point — Cluster 5 0748cced/5d821c12 + Cluster 2 8b888a1c source migration absorbed 2026-05-18..2026-05-20)
total_unique_commits: 42
date: 2026-05-24
---

# Phase 47 UPST6 Audit — Upstream v0.54.0..v0.57.0 Divergence Ledger

## Headline

TBD at audit-walk close — auditor fills with cluster count, total commit count, disposition breakdown, windows-touch:yes count, and ADR-review outcome verdict.

## Reproduction

This audit is regenerable from the values in the YAML frontmatter above (D-47-A2 / D-47-E1):

```bash
git fetch upstream --tags
# Drift-tool script pinned at commit sha 0834aa664fbaf4c5e41af5debece292992211559
# (Phase 24 ship commit; unchanged through Phase 33 + 39 + 42 + 47):
make check-upstream-drift ARGS="--from v0.54.0 --to v0.57.0 --format json"
# (On Windows hosts where `make` is not on PATH, the Makefile target dispatches to
#  bash scripts/check-upstream-drift.sh ... — same shell command, same JSON output.)
```

**Raw JSON output path:** `ci-logs-local/drift/<UTC-timestamp>-v054-v057.json` (NOT committed per D-47-E1 / D-33-A2 inherited; `ci-logs-local/` is in `.gitignore`). The ledger below is the canonical artifact.

**Auditor-rerun:** A fresh auditor reproduces the input set by running the locked
invocation against the same `range` (`v0.54.0..v0.57.0`) + `upstream_head_at_audit`
(`807fca38efc768c4e9856a0cb5c47d961b9287e5`) + `drift_tool_sh_sha`
(`0834aa664fbaf4c5e41af5debece292992211559`). Output is deterministic against the same
git ref state.

Per D-11 (see `.planning/phases/24-parity-drift-prevention/24-CONTEXT.md` D-11), `*_windows.rs` and
`crates/nono-cli/src/exec_strategy_windows/` are EXCLUDED from drift-tool output. The `windows-touch`
column on commit rows (D-47-A5 inherited from D-42-C1) flags upstream commits adding NEW Windows
code OUTSIDE the D-11-excluded paths.

## Cluster Summary

| cluster_id | theme | commits | disposition | windows-touch | rationale |
|------------|-------|---------|-------------|---------------|-----------|
| <!-- auditor fills in Task 4 --> | | | | | |

<!-- ### Cluster 1: ... (auditor fills in Task 4) -->

## ADR review

<!-- D-47-E8 placeholder — body filled in Task 6 -->

## Empirical cross-check

<!-- D-47-D1 placeholder — body filled in Task 7 -->

## Cross-cluster re-export deps detected

<!-- D-47-D3 placeholder — body filled in Task 7 (consolidates Task 5 findings) -->
