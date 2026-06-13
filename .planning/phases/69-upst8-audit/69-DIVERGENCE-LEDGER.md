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

**Audit summary:** UPST8 audits **9 unique upstream commits** (drift-tool count) across
`v0.60.0..v0.62.0`, grouped into **4 clusters**. Disposition breakdown: **will-sync 3**
(C2 network-policy security, C3 profile/diagnostic features, C4 nono-pull recovery), **won't-sync 1**
(C1 release bumps). **windows-touch:yes** clusters: **0** — no commit in this range touches
`exec_strategy_windows/`, registry, WFP, or any Windows-specific surface. Every overlap-range
commit (7 of 9) carries a Phase 63 pointer. The 2 tail commits (`db073750`, `52809dda`) carry no
Phase 63 pointer and no macOS-relevant code — the "macOS un-audited" flag is vacuously satisfied.
ADR review outcome: **(a) confirm Option A 'continue'** (pending Task 6 fill-in).

- Clusters: 4
- will-sync: 3 | fork-preserve: 0 | won't-sync: 1 | split: 0 (Task 5 re-export scan may flip C4 to split)
- windows-touch:yes: 0
- ADR review outcome: (a) — see ## ADR review

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
| C1 | Release bumps (Cargo.toml only) | 4 | won't-sync | no | Version bumps touch only crates/nono/Cargo.toml; fork manages its own versioning per fork-release-tag rule |
| C2 | Network-policy security hardening (cross-platform) | 2 | will-sync | no | Cross-platform security improvements (remove implicit credential routes; deny-by-default); security-relevant |
| C3 | Profile/diagnostic feature additions (cross-platform) | 2 | will-sync | no | Cross-platform profile/diagnostic features not yet in fork |
| C4 | nono pull --force recovery (TAIL) | 1 | will-sync | no | Cross-platform package-recovery feature; tail range (Phase 63 never saw it) |

---

### Cluster C1: Release bumps (Cargo.toml only)

**Commits:** 4 — chore: release v0.61.0, chore: release v0.61.1, chore: release v0.61.2, chore: release v0.62.0
**Disposition:** won't-sync
**Windows-touch:** no
**Rationale:** All four commits touch only `crates/nono/Cargo.toml` version bumps. The fork
manages its own versioning (MSI version flows from the git tag; crate version stays fork-controlled;
the fork leapfrogged to v0.62.x per the fork-release-tag rule to clear upstream's tag line).
Same pattern as Phase 54 C1 and Phase 63 C1. CHANGELOG entries may be referenced but the
Cargo.toml/Cargo.lock version bumps are dropped.
**Phase 63 pointer:** C1 overlap — commits `658e40f8` (v0.61.0) and `3e605f27` (v0.61.2) also
appear in Phase 63 Cluster C1 (won't-sync, same rationale). `b37198c0` (v0.61.1) is in the
overlap range (v0.60.0..v0.61.2) but does not appear in Phase 63 Cluster C1; audited fresh here
and given the same won't-sync disposition (pure Cargo.toml bump). Tail commit `52809dda` (v0.62.0
release bump) carries NO Phase 63 pointer — Phase 63 never saw the v0.61.2..v0.62.0 tail.

| sha | subject | upstream-tag | categories | files-changed | windows-touch |
|-----|---------|--------------|------------|---------------|---------------|
| 658e40f8 | chore: release v0.61.0 | v0.61.0 | other | 1 | no |
| b37198c0 | chore: release v0.61.1 | v0.61.1 | other | 1 | no |
| 3e605f27 | chore: release v0.61.2 | v0.61.2 | other | 1 | no |
| 52809dda | chore: release v0.62.0 | v0.62.0 | other | 1 | no |

---

### Cluster C2: Network-policy security hardening (cross-platform)

**Commits:** 2 — refactor(network-policy): do not enable credentials by default in profiles;
fix(proxy): deny-by-default when network.block is set (#1082)
**Disposition:** will-sync
**Windows-touch:** no
**Rationale:** Cross-platform security improvements not yet in fork main. `0fb59375` removes
implicit credential routes from embedded profiles — a security improvement that prevents
credentials from being injected on connections where they were not explicitly requested.
`bd4c469a` adds `strict_filter: bool` to `ProxyConfig` and threads deny-by-default behavior
through `PreparedSandbox` when `network.block` is set — a security hardening of the proxy's
network blocking enforcement. Both are cross-platform (no macOS-gated code, no Windows-specific
code). Security-relevant; absorbing keeps the fork current on cross-platform security hardening.
**Phase 63 pointer:** Both commits appear in Phase 63 Cluster C18 (network-policy security +
deny-by-default, cross-platform UPST8, disposition: will-sync). Phase 63 already diff-inspected
`bd4c469a` re-export surface: `strict_filter: bool` field addition is intra-cluster; clean.
**Cross-cluster re-export check:** Placeholder — to be filled in Task 5.

| sha | subject | upstream-tag | categories | files-changed | windows-touch |
|-----|---------|--------------|------------|---------------|---------------|
| 0fb59375 | refactor(network-policy): do not enable credentials by default in profiles | v0.61.0 | other | 2 | no |
| bd4c469a | fix(proxy): deny-by-default when network.block is set (#1082) | v0.61.2 | other,proxy | 8 | no |

---

### Cluster C3: Profile/diagnostic feature additions (cross-platform)

**Commits:** 2 — feat(diagnostic): add profile option to suppress system service diagnostics (#1059);
feat(profile): allow registry refs in profile extends (#1061)
**Disposition:** will-sync
**Windows-touch:** no
**Rationale:** Cross-platform profile/diagnostic features not yet in fork. `cc21229f` adds a
profile option to suppress system service diagnostics — touches `exec_strategy.rs` (the UNIX
version, NOT `exec_strategy_windows/`) plus `diagnostic.rs`, `policy.rs`, `profile/mod.rs`,
`sandbox_prepare.rs`, and related runtime files; all are cross-platform surfaces the fork carries.
`20cc5df9` allows registry refs in profile `extends` fields — touches `profile_save_runtime.rs`
and `sandbox_state.rs`; purely cross-platform profile resolution logic. Neither commit touches
macOS-specific code (no `sandbox/macos.rs`, no `#[cfg(target_os = "macos")]` blocks added).
**Phase 63 pointer:** Both commits appear in Phase 63 Cluster C19 (UPST8 new profile/diagnostic
features, cross-platform, disposition: will-sync).
**Cross-cluster re-export check:** Placeholder — to be filled in Task 5.

| sha | subject | upstream-tag | categories | files-changed | windows-touch |
|-----|---------|--------------|------------|---------------|---------------|
| cc21229f | feat(diagnostic): add profile option to suppress system service diagnostics (#1059) | v0.61.0 | other,policy,profile | 11 | no |
| 20cc5df9 | feat(profile): allow registry refs in profile extends (#1061) | v0.61.0 | other | 2 | no |

---

### Cluster C4: nono pull --force recovery (TAIL)

**Commits:** 1 — Merge commit from fork (adds --force to nono pull for metadata recovery;
touches package_cmd.rs, profile_runtime.rs, wiring.rs; 294 insertions, 67 deletions)
**Disposition:** will-sync
**Windows-touch:** no
**Rationale:** Cross-platform package-recovery feature. `db073750` is a tail-range commit
(v0.61.2..v0.62.0) that Phase 63 never saw — it carries NO Phase 63 pointer. The commit
adds `--force` recovery behavior to the `nono pull` command, touching `package_cmd.rs`,
`profile_runtime.rs`, and `wiring.rs` — all cross-platform surfaces. Confirmed no macOS-relevant
code: `git show db073750 | grep -i macos` returns nothing. Therefore the "macOS un-audited —
needs a future macOS top-up" flag is vacuously satisfied (zero macOS-relevant lines in this commit).
The Task 5 re-export diff-inspect scan may flip this to `split` if cross-cluster deps are detected.
**Cross-cluster re-export check:** Placeholder — to be filled in Task 5.

| sha | subject | upstream-tag | categories | files-changed | windows-touch |
|-----|---------|--------------|------------|---------------|---------------|
| db073750 | Merge commit from fork (nono pull --force recovery) | v0.62.0 | other,package | 3 | no |

---

## ADR review

<!-- To be populated in Task 6 (human ADR judgment checkpoint) -->

## Empirical cross-check

<!-- To be populated in Task 5 (cross-cluster re-export diff-inspect checkpoint) -->

## Cross-cluster re-export deps detected

<!-- To be populated in Task 5 (cross-cluster re-export diff-inspect checkpoint) -->
