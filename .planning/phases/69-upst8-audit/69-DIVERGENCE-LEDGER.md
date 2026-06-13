---
phase: 69-upst8-audit
plan: 01
ledger_type: upst8-audit
range: 9a05a4ff..52809dda (v0.60.0..v0.62.0, D-01 corrected)
range_note: "ROADMAP/REQUIREMENTS SC says v0.60.0..v0.61.2; D-01 extends to v0.62.0 (+3 tail commits — v0.62.0 release + deny-by-default proxy fix + package_cmd merge commit); SC flagged for +3 update — do NOT silently edit REQUIREMENTS.md"
upstream_head_at_audit: 849cda42c0541f18915708cd3ff31d61c12d136d
refetch_date: 2026-06-13
drift_tool_sh_sha: 0834aa664fbaf4c5e41af5debece292992211559
drift_tool_ps1_sha: 0834aa664fbaf4c5e41af5debece292992211559
drift_tool_invocation: 'make check-upstream-drift ARGS="--from v0.60.0 --to 52809dda --format json"'
fork_baseline: v0.60.0 (Phase 55 UPST7 sync point — see Phase 54/55 ledgers)
total_unique_commits: 9
date: 2026-06-13
---

# Phase 69 UPST8 Audit — Upstream v0.60.0..v0.62.0 Divergence Ledger

## Headline

**Range-extension note:** The ROADMAP.md and REQUIREMENTS.md UPST8-01 success criteria specify
the range `v0.60.0..v0.61.2`. Per D-01, this audit extends to upstream v0.62.0 (SHA `52809dda`),
covering **+3 additional commits** beyond `v0.61.2`: the v0.62.0 release bump, a deny-by-default
proxy fix (`bd4c469a`), and the "Merge commit from fork" package_cmd change (`db073750`). The
ROADMAP/REQUIREMENTS SC is NOT silently updated here — this ledger records the divergence and
flags it. After this audit closes, REQUIREMENTS.md UPST8-01 acceptance language should be updated
to reflect `v0.62.0` as the upper bound.

**macOS-overlap note (D-04):** The overlap range `v0.60.0..v0.61.2` was already covered by
Phase 63's macOS audit (`63-DIVERGENCE-LEDGER.md`, range `v0.57.0..v0.61.2`). Pure macOS-only
commits in that overlap range carry a Phase 63 pointer here and are `won't-sync`; the non-macOS
delta of shared commits is audited fresh. The tail range `v0.61.2..v0.62.0` was NOT seen by
Phase 63 — any macOS-relevant tail commits are flagged "macOS un-audited — needs a future
macOS top-up."

_[Placeholder: cluster count, disposition breakdown, windows-touch:yes count, and ADR verdict
to be filled by the auditor after Tasks 4-6.]_

- Clusters: TBD
- will-sync: TBD | fork-preserve: TBD | won't-sync: TBD | split: TBD
- windows-touch:yes: TBD
- ADR review outcome: TBD

## Reproduction

- **Precondition:** `git fetch upstream` (note: `git fetch upstream --tags` will be REJECTED
  due to local fork tag collision on `v0.62.0`; use branches-only fetch and verify SHAs via
  `git ls-remote upstream refs/tags/v0.62.0`). Assert that `git cat-file -t 52809dda` prints
  `commit` (upstream v0.62.0 must exist locally before the drift run).
- **Invocation (verbatim):**
  `make check-upstream-drift ARGS="--from v0.60.0 --to 52809dda --format json"`
  (Windows-host fallback used: `bash scripts/check-upstream-drift.sh --from v0.60.0 --to 52809dda --format json`)
- **CRITICAL:** `--to` MUST be the literal SHA `52809dda`, NOT the tag `v0.62.0`. The local
  fork tag `v0.62.0` resolves to `3c5e9025` (divergent history) and produces garbage output
  (~1889 commits due to tag collision).
- **JSON output:** `ci-logs-local/drift/20260613T004146Z-v060-v062-upst8.json` (gitignored, NOT committed)
- **upstream_head_at_audit:** `849cda42c0541f18915708cd3ff31d61c12d136d`
- **refetch_date:** `2026-06-13`
- **drift_tool sha pin:** `0834aa664fbaf4c5e41af5debece292992211559` (asserted == before run)
- **total_unique_commits:** 9 (source of truth; merges excluded by the tool)
- **SHA collision guard:** local tag `v0.62.0` = `3c5e9025` (fork release on divergent history);
  upstream `v0.62.0` = `52809dda`; `--to` MUST be the SHA, never the tag.
- **upstream_newer_than_v0.62.0:** none — upstream v0.7.0 is from the old numbering series
  (pre-dates v0.60.x); D-03 UPST9 deferral gate does NOT fire.
- **auditor-rerun instructions:** Fetch upstream branches (`git fetch upstream`), verify
  `git ls-remote upstream refs/tags/v0.62.0` returns `52809dda...`, assert the drift_tool
  sha pin via `git log -1 --format=%H -- scripts/check-upstream-drift.sh`, then run the
  invocation above. The same range + HEAD reproduces the 9-commit set.

## Cluster Summary

| cluster_id | theme | commits | disposition | windows-touch | rationale |
|------------|-------|---------|-------------|---------------|-----------|
| _TBD_ | _To be populated in Task 4_ | | | | |

<!-- Cluster sections to be populated in Task 4 (audit-walk checkpoint) -->

## ADR review

<!-- To be populated in Task 6 (human ADR judgment checkpoint) -->

## Empirical cross-check

<!-- To be populated in Task 5 (cross-cluster re-export diff-inspect checkpoint) -->

## Cross-cluster re-export deps detected

<!-- To be populated in Task 5 (cross-cluster re-export diff-inspect checkpoint) -->
