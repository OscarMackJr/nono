---
phase: 47-upst6-audit-v0-41-v0-43-drift-ingestion
plan: 02
ledger_type: drift-ingest-backfill
range: v0.41.0..v0.43.0
upstream_head_at_audit: 807fca38efc768c4e9856a0cb5c47d961b9287e5
drift_tool_sh_sha: 0834aa664fbaf4c5e41af5debece292992211559
drift_tool_ps1_sha: 0834aa664fbaf4c5e41af5debece292992211559
drift_tool_invocation: 'make check-upstream-drift ARGS="--from v0.41.0 --to v0.43.0 --format json"'
framing: 'backfill-cleanup, not parity-sync (per REQ-DRIFT-INGEST-01)'
deferral_origin: 'v2.3 scope-lock 2026-04-29'
historical_absorption_phases: [22, 34]
pre_d19_trailer_era: false
total_unique_commits: 11
date: 2026-05-24
---

# Phase 47 v0.41–v0.43 Backfill Drift Ingestion Ledger

## Headline

**11 non-merge cross-platform commits across 2 release tags (v0.42.0 + v0.43.0) and one between-tag spread; 4 themed clusters with disposition breakdown 3 will-sync (retroactive paper-trail) / 0 fork-preserve / 1 won't-sync / 0 split (D-47-C3 standard 4-disposition vocab; split codified at v2.5 close).** Drift-tool category distribution: `profile=2, policy=0, package=0, proxy=1, audit=0, other=11` (overlap on multi-category commits — `other` is catch-all per Phase 24 D-05 multi-label semantics). CONTEXT § Drift signal preview estimated ~19 commits; actual post-D-11 fork-shared filter count is 11.

**Backfill framing: this is retroactive paper-trail on a long-deferred range, NOT parity-sync.** REQ-DRIFT-INGEST-01 was deferred at v2.3 scope-lock 2026-04-29 (when DRIFT-01/02 tooling shipped) and re-deferred at v2.4 + v2.5 scope-locks; Plan 47-02 closes the deferral. The range v0.41.0..v0.43.0 was already the explicit absorption scope for Phase 34 UPST3 (v2.3 milestone; Plans 34-00..34-10; closed 2026-05-12 per commit `01abbdf4`); the backfill ledger reconstructs the per-commit `absorbed-via:` attribution against the historical record per D-47-C2 subject-line + D-19 trailer match against fork main, with the historical Phase 34 `34-PHASE-OUTCOMES.md` artifact (now superseded by Phase 34 close artifacts but verifiable via `git show 01abbdf4`) providing the canonical disposition reference.

**absorbed-via distribution: 7 phase-34-plan-XX-commit-XXXXXXXX (unambiguous D-19-trailer match against fork main) + 4 intentionally-skipped (1 Cargo.toml release v0.42.0 per Phase 34 D-34-B2 version-bump rejection; 3 Unix-socket capability commits per Phase 34 D-34-A3 + C3 won't-sync) + 0 unmatched + 0 fork-divergence + 0 ambiguous-see-cluster-rationale.** CONTEXT § Drift signal preview "only 11 unique `Upstream-commit:` trailers exist in fork main" framing is dated — fork main now carries D-19 trailers on every Phase 34 + 40 + 43 cherry-pick, so trailer match yields unambiguous attribution for all 7 absorbed commits.

**windows-touch:yes count: 0.** Zero fires this backfill range — no upstream commit in v0.41.0..v0.43.0 adds Windows-conditional code outside the D-11-excluded `*_windows.rs` / `exec_strategy_windows/` paths. The D-47-A5 mechanical heuristic returns zero matches; auditor judgment-override confirms. Consistent with the Phase 34 era predating fork's Windows-platform-detection work (Phase 33 + 43 + 45) — upstream's v0.41–v0.43 cluster shape (CLI consolidation + proxy/network + keyring + Unix-socket) is structurally cross-platform-or-Linux-only.

**Phase 48 hand-off: 0 unmatched candidates.** ROADMAP § Phase 47 SC#4 most-likely-outcome materialized: "resolves the deferral by confirming no fork-side action needed". Phase 48 plan-phase has NO backfill cherry-pick candidates to absorb alongside UPST6 work. Phase 34's coverage of the range was complete per its disposition record; backfill ledger is a clean paper-trail confirmation.

**No `## ADR review` section** per D-47-C4 — backfill is retroactive paper-trail on a 2-year-old range (Phase 22 ship 2026-01-XX, Phase 34 ship 2026-05-12); fresh ADR verdict on it adds no signal beyond the Plan 47-01 UPST6 verdict (which already confirmed Option A `continue`). D-47-C4 NEGATIVE assertion preserved; falsifiable via `! grep -q "^## ADR review$"` in the close-gate.

## Reproduction

This audit is regenerable from the values in the YAML frontmatter above (D-47-A2 / D-47-E1):

```bash
git fetch upstream --tags
# Drift-tool script pinned at commit sha 0834aa664fbaf4c5e41af5debece292992211559
# (Phase 24 ship commit; unchanged through Phase 33 + 39 + 42 + 47):
make check-upstream-drift ARGS="--from v0.41.0 --to v0.43.0 --format json"
# (On Windows hosts where `make` is not on PATH, the Makefile target dispatches to
#  bash scripts/check-upstream-drift.sh ... — same shell command, same JSON output.)
```

**Raw JSON output path:** `ci-logs-local/drift/20260524T025014Z-v041-v043.json` (NOT committed per D-47-E1 / D-33-A2 inherited; `ci-logs-local/` is in `.gitignore`; `git check-ignore -v` confirms). The ledger below is the canonical artifact.

**Auditor-rerun:** A fresh auditor reproduces the input set by running the locked invocation against the same `range` (`v0.41.0..v0.43.0`) + `drift_tool_sh_sha` (`0834aa664fbaf4c5e41af5debece292992211559`) + `historical_absorption_phases: [22, 34]`. The `upstream_head_at_audit` field (`807fca38efc768c4e9856a0cb5c47d961b9287e5`) is captured for schema uniformity with the UPST6 ledger (D-47-A3) but is **informational** for this plan — the backfill range is fully historical (v0.41.0 + v0.43.0 tags resolve identically regardless of HEAD), so output is deterministic against the same drift-tool-sha and tag-pair. Output reproduces against the same git ref state.

Per D-11 (see `.planning/phases/24-parity-drift-prevention/24-CONTEXT.md` D-11), `*_windows.rs` and `crates/nono-cli/src/exec_strategy_windows/` are EXCLUDED from drift-tool output. The `windows-touch` column on commit rows (D-47-A5 inherited from D-42-C1) flags upstream commits adding NEW Windows code OUTSIDE the D-11-excluded paths — every commit in this audit returns `no` against the mechanical D-47-A5 heuristic.

**Inspection methodology** (mirrors Phase 33 + 39 + 42 + Plan 47-01 with D-47-C2 backfill extension): each commit's `subject` + `categories` + `files_changed[]` length was read from the drift JSON for every row; the fork main subject-line + D-19 `Upstream-commit:` trailer was matched via `git log main --grep="<upstream-subject-substring>"` + `git log -1 --format="%B" <fork-sha> | grep "^Upstream-commit:"` per D-47-C2 methodology. For commits with no fork-side match, the Phase 34 `34-PHASE-OUTCOMES.md` artifact (read via `git show 01abbdf4:.planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/34-PHASE-OUTCOMES.md`) provided the canonical `won't-sync` disposition reference. Per D-47-C4, NO `## ADR review` section appears in this ledger; per D-47-C4, the `## Empirical cross-check` section IS present.

**Cluster grouping rationale:** the 11 backfill commits cluster naturally into 4 themes mirroring Phase 34's cluster grouping (C2 CLI consolidation / C4 proxy-net / C5 keyring / C3 Unix-socket). Each backfill cluster aligns 1:1 with the corresponding Phase 34 absorption plan (or won't-sync disposition for C3). The 2 release-bump commits (v0.42.0 + v0.43.0) ride along with their respective semantic clusters per Phase 34 + 40 release-ride convention.

## Cluster Summary

| cluster_id | theme | commits | disposition | windows-touch | absorbed-via-summary | rationale |
|------------|-------|---------|-------------|---------------|----------------------|-----------|
| BC1 | CLI consolidation tail: cargo fmt + v0.43.0 release-ride | 2 | will-sync (retroactive paper-trail) | no | 2 phase-34-plan-01 | Phase 34 Plan 34-01 absorbed Cluster C2 CLI consolidation (6 upstream commits, v0.41.0); style + release-ride commits in v0.43.0 absorbed alongside. The release-bump (`30c0f76e chore: release v0.43.0`) preserved as empty cherry-pick (`dc5247bf`) with D-19 trailer for chain completeness per Phase 34 Plan 34-03 SUMMARY (release-ride is structurally cross-cluster — landed in C5 keyring plan since v0.43.0 closed the keyring cluster's tag window) |
| BC2 | Proxy/network hardening: NO_PROXY hole + --allow-connect-port + macOS fail-fast | 3 | will-sync (retroactive paper-trail) | no | 3 phase-34-plan-02 | Phase 34 Plan 34-02 absorbed Cluster C4 proxy-net hardening (4 commits, v0.42.0..v0.45.0; 3 of those land in the backfill v0.41.0..v0.43.0 range, the 4th `cb6b199c native-tls-roots` is v0.45.0 out of range). D-34-B2 surgical posture preserved: `--allow-connect-port` flows through `nono-proxy` only; zero WFP retrofit |
| BC3 | Keyring: optional feature flag + default + v0.43.0 release-ride | 3 | will-sync (retroactive paper-trail) | no | 3 phase-34-plan-03 | Phase 34 Plan 34-03 absorbed Cluster C5 keyring (8 commits, v0.43.0..v0.45.0; 3 of those land in the backfill range — `f5215917` + `7b58c3ee` + `30c0f76e`). `system-keyring` feature flag (on by default; opt-out for headless/container) lands per Phase 34 D-34-B2 surgical posture |
| BC4 | Unix-socket capability + release v0.42.0 (Phase 34 won't-sync) | 4 | won't-sync | no | 4 intentionally-skipped | Phase 34 D-34-A3 + C3 won't-sync per `34-PHASE-OUTCOMES.md` artifact: `UnixSocketCapability` + `UnixSocketMode` + `--allow-unix-socket` flag family + Linux seccomp af_unix plumbing — capability shape is Unix-specific (Windows IPC uses Named Pipes per Phase 18 AIPC); adding to `crates/nono/` would expose a no-op enum variant on the Windows backend violating fail-secure. Future re-evaluation trigger: cross-platform "stream socket" abstraction unifying Unix sockets + Named Pipes. Release v0.42.0 (`a87c6ae5`) rides along with C3 (was part of the v0.42.0 tag window; Cargo.toml version bump rejected per Phase 34 D-34-B2) |

### Cluster BC1: CLI consolidation tail (cargo fmt + v0.43.0 release-ride)

**Commits:** 2 (lead: `1f912e53 style: run cargo fmt`)
**Disposition:** will-sync (retroactive paper-trail)
**Windows-touch:** no
**Rationale:** Two commits ride along with Phase 34 Plan 34-01 (Cluster C2 CLI consolidation, 6 upstream commits v0.41.0) absorption. (a) `1f912e53` is a cargo-fmt cleanup pass that flowed into fork main as `03ab7006` (D-19 trailer match: `Upstream-commit: 1f912e53` confirmed via `git log -1 --format="%B" 03ab7006`). (b) `30c0f76e chore: release v0.43.0` is the release-bump for the v0.43.0 tag window; fork absorbed as empty cherry-pick `dc5247bf` per Phase 34 D-34-B2 Cargo.toml version-bump rejection — D-19 trailer preserved for chain completeness. **Absorbed-via attribution:** both commits trace to Phase 34 via fork-side D-19 trailers; the cargo-fmt cleanup lands under Plan 34-01 (the CLI consolidation plan where it originated), the release-ride under Plan 34-03 (the keyring plan where the v0.43.0 tag-window closes). Note: `81d8c3d0` and `322e2ddb` are also fork-side `style: run cargo fmt` commits but they trace to `Upstream-commit: 1c893465` (v0.47.0) and `Upstream-commit: 1d491b4d` (v0.52.0) respectively — out of the v0.41.0..v0.43.0 backfill range; excluded from backfill ledger by tag-range filter.
**Target phase:** N/A — already absorbed via Phase 34 (retroactive paper-trail; no Phase 48 forward action)

| sha | subject | upstream-tag | categories | files-changed | windows-touch | absorbed-via |
|-----|---------|--------------|------------|---------------|---------------|--------------|
| 1f912e53 | style: run cargo fmt | v0.43.0 | other | 1 | no | phase-34-plan-01-commit-03ab7006 |
| 30c0f76e | chore: release v0.43.0 | v0.43.0 | other | 1 | no | phase-34-plan-03-commit-dc5247bf |

### Cluster BC2: Proxy/network hardening (NO_PROXY hole + --allow-connect-port + macOS fail-fast)

**Commits:** 3 (lead: `8c818f84 feat(cli): add --allow-connect-port for outbound TCP port allowlisting`)
**Disposition:** will-sync (retroactive paper-trail)
**Windows-touch:** no
**Rationale:** Three commits absorbed via Phase 34 Plan 34-02 (Cluster C4 proxy-net hardening, originally 4 commits v0.42.0..v0.45.0; 3 fall within the v0.41.0..v0.43.0 backfill range, the 4th `cb6b199c feat(packages): use native tls root certificates` is v0.45.0 and out of range). (a) `ad23d794 fix(proxy): stop adding allow_domain hosts to NO_PROXY without direct TCP grants` closes the NO_PROXY hole — fork-side `02626ebe` with `Upstream-commit: ad23d794`. (b) `8c818f84 feat(cli): add --allow-connect-port` adds outbound TCP port allowlisting via a repeatable clap flag — fork-side `108d1139` with `Upstream-commit: 8c818f84`. (c) `cba186f4 fix(cli): fail fast on --allow-connect-port on macOS` adds macOS guard — fork-side `d2447525` with `Upstream-commit: cba186f4`. **D-34-B2 surgical posture preserved:** `--allow-connect-port` flows through `nono-proxy` only; zero WFP retrofit on Windows (`exec_strategy_windows/` last-touched SHA unchanged from baseline `2823ec29` per Phase 34 Plan 34-02 SUMMARY). All three carry verbatim D-19 trailers with lowercase `a` in `Upstream-author:` per Phase 34/40 convention.
**Target phase:** N/A — already absorbed via Phase 34 (retroactive paper-trail; no Phase 48 forward action)

| sha | subject | upstream-tag | categories | files-changed | windows-touch | absorbed-via |
|-----|---------|--------------|------------|---------------|---------------|--------------|
| 8c818f84 | feat(cli): add --allow-connect-port for outbound TCP port allowlisting | v0.43.0 | other,profile | 3 | no | phase-34-plan-02-commit-108d1139 |
| cba186f4 | fix(cli): fail fast on --allow-connect-port on macOS | v0.43.0 | other | 1 | no | phase-34-plan-02-commit-d2447525 |
| ad23d794 | fix(proxy): stop adding allow_domain hosts to NO_PROXY without direct TCP grants | v0.42.0 | other,proxy | 4 | no | phase-34-plan-02-commit-02626ebe |

### Cluster BC3: Keyring (optional feature flag + default + release-ride)

**Commits:** 3 (lead: `f5215917 feat: make system keyring optional for headless/container builds`)
**Disposition:** will-sync (retroactive paper-trail)
**Windows-touch:** no
**Rationale:** Three commits absorbed via Phase 34 Plan 34-03 (Cluster C5 keyring, originally 8 commits v0.43.0..v0.45.0; 3 land in the backfill v0.41.0..v0.43.0 range). (a) `f5215917 feat: make system keyring optional` introduces the `system-keyring` feature flag (on by default; controls whether OS keyring — macOS Keychain / Linux Secret Service via libdbus — is compiled in) — fork-side `459d47e8` with `Upstream-commit: f5215917`. (b) `7b58c3ee fix: set system-keyring as default feature for backward compatibility` makes `default = ["system-keyring"]` matching pre-PR behavior — fork-side `afde16f5` with `Upstream-commit: 7b58c3ee`. (c) `30c0f76e chore: release v0.43.0` empty-cherry-picked as `dc5247bf` per Cargo.toml version-bump rejection (also listed in BC1 — released within the C5 keyring tag-window per Plan 34-03 SUMMARY metadata `landed_commit_count: 8`). **Note on double-counted release commit:** `30c0f76e` appears in BC1 (rationale: cargo-fmt cleanup tail) and conceptually rides with BC3 (rationale: closes the v0.43.0 tag window for the keyring cluster); the commit-row table places it under BC1's tag-grouping to preserve the row-count gate (D-47-B4 step 2 exact coverage: 11 rows = 11 drift-tool unique commits), since each commit appears in exactly one cluster's commit-row table. The Phase 34 Plan 34-03 absorption attribution is documented via the BC1 absorbed-via cell pointing at `phase-34-plan-03-commit-dc5247bf`. **D-34-B2 surgical posture preserved:** Phase 34 Plan 34-03 added Dockerfile-headless (root, fork variant) for headless/container consumers; opt-out via `default-features = false`. All three carry verbatim D-19 trailers.
**Target phase:** N/A — already absorbed via Phase 34 (retroactive paper-trail; no Phase 48 forward action)

| sha | subject | upstream-tag | categories | files-changed | windows-touch | absorbed-via |
|-----|---------|--------------|------------|---------------|---------------|--------------|
| f5215917 | feat: make system keyring optional for headless/container builds | v0.43.0 | other | 3 | no | phase-34-plan-03-commit-459d47e8 |
| 7b58c3ee | fix: set system-keyring as default feature for backward compatibility | v0.43.0 | other | 1 | no | phase-34-plan-03-commit-afde16f5 |

### Cluster BC4: Unix-socket capability + release v0.42.0 (Phase 34 won't-sync)

**Commits:** 4 (lead: `85708cae feat(cli): add --allow-unix-socket flag family + profile schema`)
**Disposition:** won't-sync
**Windows-touch:** no
**Rationale:** Four commits explicitly `won't-sync` per Phase 34 D-34-A3 + Cluster C3 disposition (Unix-socket capability, v0.42.0). Per `34-PHASE-OUTCOMES.md` artifact (read via `git show 01abbdf4:.planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/34-PHASE-OUTCOMES.md`):

> Upstream adds `UnixSocketCapability` + `UnixSocketMode` + `--allow-unix-socket` flag family + Linux seccomp `af_unix` plumbing. The capability shape is Unix-specific (Windows IPC uses Named Pipes — see Phase 18 AIPC pipe/socket brokering); adding a `UnixSocketCapability` to `crates/nono/` would expose an enum variant that no Windows backend can honor and would violate D-19 (no library mutation in this audit; a sync-time addition would need its own Windows-no-op handling decision). Fork users on Windows do not consume Unix sockets; macOS users get unsigned Unix-socket access today via the broader macOS Seatbelt allowlist — a typed capability is not a regression.

**Decision rationale cites** (verbatim from `34-PHASE-OUTCOMES.md`):
- **D-19 / D-34-E2 (atomic commit-per-semantic-change; no library mutation in this audit):** A typed `UnixSocketCapability` lands in `crates/nono/src/capability.rs` (the library). Adding the enum variant would either expose a no-op match arm on the Windows backend (violating fail-secure: "On any error, deny access. Never silently degrade to a less secure state.") or require a parallel Windows IPC capability decision that is out of Phase 34 scope.
- **Phase 18 AIPC pipe/socket brokering** already addresses the fork's Windows IPC needs via Named Pipes. A Unix-socket-typed capability is not the right abstraction for the fork's Windows surface; macOS users already get Unix-socket access via the broader Seatbelt allowlist, so no fork-side user-visible regression results from non-porting.

**Future re-evaluation trigger:** if a future phase decides to define a cross-platform "stream socket" capability that abstracts over Unix sockets (Linux/macOS) and Named Pipes (Windows), upstream's `UnixSocketCapability` shape becomes a candidate to absorb as the Linux/macOS arm of that abstraction. Until then, the fork's Phase 18 AIPC Named-Pipe path is the canonical Windows IPC capability surface.

The release commit `a87c6ae5 chore: release v0.42.0` rides along with C3 (was the v0.42.0 tag-window's release bump); fork main has NO `chore: release v0.42.0` commit (verified via `git log main --grep="release v0.42" --oneline` returning zero rows) — consistent with both the Phase 34 D-34-B2 Cargo.toml version-bump rejection AND the C3 won't-sync disposition (no Plan 34-XX needed to absorb the v0.42.0 release-bump because the v0.42.0 tag-window was entirely won't-sync).
**Target phase:** N/A — `won't-sync` by design; no Phase 48 forward action; no Phase 22/34 absorption to paper-trail

| sha | subject | upstream-tag | categories | files-changed | windows-touch | absorbed-via |
|-----|---------|--------------|------------|---------------|---------------|--------------|
| 85708cae | feat(cli): add --allow-unix-socket flag family + profile schema | v0.42.0 | other,profile | 9 | no | intentionally-skipped |
| a9a8b6c2 | feat(capability): add UnixSocketCapability and UnixSocketMode | v0.42.0 | other | 3 | no | intentionally-skipped |
| 1d789aa6 | fix(supervisor(linux)): allow pathname af_unix sockets in network seccomp | v0.42.0 | other | 3 | no | intentionally-skipped |
| a87c6ae5 | chore: release v0.42.0 | v0.42.0 | other | 1 | no | intentionally-skipped |

## Empirical cross-check

Per D-47-C4 + D-47-D1, the audit walk spot-checks ≥4 fork-shared files for any upstream path the drift tool missed AND to retroactively close the `feedback_cluster_isolation_invalid` lesson on the backfill range (Phase 34 may have hit cross-cluster re-export deps it didn't recognize during the pre-D-47-D1..D4 era). D-47-C4 explicitly retains the empirical cross-check on backfill while skipping the `## ADR review` section. D-47-E12 preferential sampling for backfill targets Phase 22/34-era hot zones per CONTEXT § Claude's Discretion.

Methodology: for each sampled file, run `git log v0.41.0..v0.43.0 -- <file>` against `upstream/main` and confirm the drift tool's commit list covers every upstream commit touching that file. For backfill, additionally confirm the absorbed-via cell in the cluster commit-row tables matches the fork-side D-19 trailer attribution.

### File: crates/nono-cli/src/capability_ext.rs

- Walked upstream log: `git log v0.41.0..v0.43.0 -- crates/nono-cli/src/capability_ext.rs`
- Commits touching this file in range: **4** (`1f912e53`, `cba186f4`, `8c818f84`, `ad23d794`)
- Cluster mapping: BC1 (1: `1f912e53` cargo fmt) + BC2 (3: `cba186f4` + `8c818f84` + `ad23d794` proxy-net hardening)
- absorbed-via status sample: cargo-fmt → `phase-34-plan-01-commit-03ab7006`; 3 proxy-net → `phase-34-plan-02-commit-{d2447525,108d1139,02626ebe}`
- Drift-tool coverage: **PASS** — all 4 upstream commits touching this file appear in the drift JSON. `capability_ext.rs` was the highest-churn fork-shared file in this range (Phase 34 Plan 34-02 SUMMARY confirms `crates/nono-cli/src/capability_ext.rs` modified by the 4-commit Plan 34-02 cherry-pick chain + 1 fmt-fix follow-up `9e47c675`).

### File: crates/nono-cli/src/cli.rs

- Walked upstream log: `git log v0.41.0..v0.43.0 -- crates/nono-cli/src/cli.rs`
- Commits touching this file in range: **2** (`8c818f84`, `85708cae`)
- Cluster mapping: BC2 (1: `8c818f84` --allow-connect-port flag) + BC4 (1: `85708cae` --allow-unix-socket flag family)
- absorbed-via status sample: `8c818f84` → `phase-34-plan-02-commit-108d1139` (will-sync absorbed); `85708cae` → `intentionally-skipped` (Phase 34 C3 won't-sync; no fork-side absorption)
- Drift-tool coverage: **PASS** — both upstream commits appear in the drift JSON. cli.rs cross-platform clap-arg surface; the `--allow-unix-socket` flag intentionally absent from fork's CLI per Phase 34 C3 won't-sync disposition.

### File: crates/nono/src/capability.rs

- Walked upstream log: `git log v0.41.0..v0.43.0 -- crates/nono/src/capability.rs`
- Commits touching this file in range: **2** (`85708cae`, `a9a8b6c2`)
- Cluster mapping: BC4 (both commits — Unix-socket capability won't-sync)
- absorbed-via status sample: both → `intentionally-skipped` (Phase 34 C3 won't-sync)
- Drift-tool coverage: **PASS** — both upstream commits appear in the drift JSON. This is the highest-leverage file for the `feedback_cluster_isolation_invalid` retroactive closure: the `UnixSocketCapability` + `UnixSocketMode` enum additions in `a9a8b6c2` are exactly the "library-mutation introduces no-op enum variant on Windows backend" class of risk Phase 34 D-34-A3 + C3 won't-sync explicitly rejected; no cross-cluster re-export deps surfaced because the symbols are NOT in fork's `crates/nono/src/lib.rs` `pub use` surface (verified via `git grep -n "UnixSocketCapability\|UnixSocketMode" crates/nono/src/lib.rs` returning zero matches on current fork main). Retroactive empirical confirmation that Phase 34's structural rejection held: zero ghost-symbol re-exports in fork main.

### File: crates/nono-cli/src/profile/mod.rs

- Walked upstream log: `git log v0.41.0..v0.43.0 -- crates/nono-cli/src/profile/mod.rs`
- Commits touching this file in range: **2** (`8c818f84`, `85708cae`)
- Cluster mapping: BC2 (1: `8c818f84` NetworkConfig::connect_port profile field) + BC4 (1: `85708cae` --allow-unix-socket profile schema)
- absorbed-via status sample: `8c818f84` → `phase-34-plan-02-commit-108d1139` (will-sync absorbed; `NetworkConfig::connect_port` field landed per Plan 34-02 SUMMARY); `85708cae` → `intentionally-skipped` (Plan 34-02 SUMMARY decisions block notes "Cluster C3 (won't-sync) is the basis for resolving the ad23d794 capability_ext.rs conflict — drop upstream's warn!->info! adjustment in try_new_unix_socket_{file,dir} (those helpers don't exist in the fork)")
- Drift-tool coverage: **PASS** — both upstream commits appear in the drift JSON. `profile/mod.rs` is the canonical-sections hot zone (Phase 36 / 36.5 `CommandsConfig` + `FilesystemConfig.deny/bypass_protection` + `LegacyPolicyPatch` + `DeprecationCounter` extensions); the v0.41.0..v0.43.0 era predates Phase 36 by 1 year so no Phase 36-style absorption conflict on the backfill range. Empirical confirmation that Phase 34 Plan 34-02's "drop upstream's `try_new_unix_socket_{file,dir}` warn!->info! adjustment" decision held in fork main (verified: fork-side `02626ebe` does NOT touch any Unix-socket helper).

### File: crates/nono/src/keystore.rs

- Walked upstream log: `git log v0.41.0..v0.43.0 -- crates/nono/src/keystore.rs`
- Commits touching this file in range: **1** (`f5215917`)
- Cluster mapping: BC3 (`f5215917` keyring optional feature flag)
- absorbed-via status sample: `f5215917` → `phase-34-plan-03-commit-459d47e8` (will-sync absorbed; `#[cfg(feature = "system-keyring")]` gating landed per Plan 34-03 SUMMARY)
- Drift-tool coverage: **PASS** — single upstream commit touching this file appears in the drift JSON. Plan 34-03 SUMMARY confirms `crates/nono/src/keystore.rs` cfg-gated with explicit fail-closed fallback for headless builds; D-34-B2 surgical posture preserved.

**Findings summary:** All 5 sampled files PASS; drift tool's commit list is complete against the v0.41.0..v0.43.0 fork-shared surface for the sampled subsystems. **No drift-tool blind spots surfaced; no D-47-E10 quick-task spawn required.** The empirical cross-check on the backfill range structurally closes the `feedback_cluster_isolation_invalid` lesson retroactively: Phase 34's C3 won't-sync disposition explicitly considered the "library-mutation introduces no-op enum variant" class of risk that the v2.5 close hardened-via-`split` lesson formalized — `capability.rs` walk confirms the rejection held (`UnixSocketCapability` + `UnixSocketMode` never landed in fork's `crates/nono/src/lib.rs` re-export surface). Per D-47-C4 + D-47-D1, no `## Cross-cluster re-export deps detected` subsection appears on the backfill ledger — re-export scan applies to `will-sync` clusters in cycles where cherry-pick is the forward action; backfill `will-sync` rows are retroactive paper-trail (no forward cherry-pick), so the scan is structurally N/A.

## Phase 48 hand-off

Zero `absorbed-via: unmatched` rows detected across the v0.41.0..v0.43.0 backfill ledger. Phase 34 UPST3 (Plans 34-00..34-10; closed 2026-05-12 per commit `01abbdf4`) absorbed the full range per its disposition record: 7 commits via Plans 34-01 + 34-02 + 34-03 (`will-sync` retroactive paper-trail) and 4 commits via D-34-A3 C3 won't-sync (`intentionally-skipped`). The REQ-DRIFT-INGEST-01 deferral resolves with "no fork-side action needed" per ROADMAP § Phase 47 SC#4 most-likely-outcome.

**Phase 48 has NO backfill candidates to absorb alongside UPST6 work.** Phase 48 plan-phase consumes Plan 47-01's `DIVERGENCE-LEDGER.md` (42 commits / 9 clusters / 8 will-sync + 1 fork-preserve + 0 won't-sync + 0 split) as its sole authoritative input; this backfill ledger is documented paper-trail and structurally complete with no forward action items.

**v2.3 scope-lock 2026-04-29 REQ-DRIFT-INGEST-01 deferral closed.** The DRIFT-01/02 tooling (shipped Phase 24, v2.2 milestone) has now been successfully exercised on 3 audit ranges (Phase 33 v0.40.1..v0.52.0; Phase 39 v0.52.0..v0.53.0; Phase 42 v0.53.0..v0.54.0; Phase 47 Plan 47-01 v0.54.0..v0.57.0) PLUS the v0.41.0..v0.43.0 backfill range — first-real-load on a long-deferred range surfaced zero category miscategorizations or file-filter blind spots per D-47-E10; drift-tool category coverage is empirically validated against the historical Phase 34 absorption record.
