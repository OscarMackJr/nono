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

**42 non-merge cross-platform commits across 3 minor releases (v0.55.0, v0.56.0, v0.57.0); 9 themed clusters with disposition breakdown 8 will-sync / 1 fork-preserve / 0 won't-sync / 0 split (D-47-C3 standard 4-disposition vocab; split codified at v2.5 close).** Drift-tool category distribution: profile=10, policy=3, package=2, proxy=2, audit=0, other=38 (overlap on multi-category commits).

**windows-touch:yes count: 0.** Zero fires this cycle — no new `platform.rs` / `registry.rs` / Windows-conditional code introduced in upstream v0.54.0..v0.57.0. This is a relief signal contrasting with Phase 42's first-fire (3 commits: `0748cced` + `5d821c12` + `ce06bd59`) and confirms the Phase 43 + 45 absorption of `crates/nono-cli/src/platform.rs` foundation has not been disrupted by subsequent upstream work. The mechanical D-47-A5 heuristic (files containing `windows` substring OR pinned-list match OR subject keywords) returns zero matches across all 42 commits; auditor judgment-override confirms (no edge case where heuristic missed Windows-specific work).

**Cross-cluster re-export deps detected: 0 across 7 will-sync clusters scanned** (D-47-D1..D4 closure). Cluster 4 lead commit `c2c6f2ca` exposes a re-export surface (`pub use sandbox::{DetectedAbi, LandlockScopePolicy, detect_abi, is_wsl2, landlock_scope_policy};` in `crates/nono/src/lib.rs` and `pub use linux::{...};` in `crates/nono/src/sandbox/mod.rs`) but the re-exported symbols are introduced in `c2c6f2ca` itself (verified via `git show c2c6f2ca -- crates/nono/src/sandbox/linux.rs | grep '^+pub'`) — INTRA-cluster, not cross-cluster. No D-47-D4 split-flip required this cycle.

**Cluster 2 (Edition 2024 + workspace deps) follow-on:** the Phase 42 deferred source migration that was the load-bearing Cluster-2-isolation-invalid empirical discovery has been ABSORBED in Phase 45 Plan 45-01 commits `f640528a..d21399e3` (2026-05-20). The fork's `bindings/c/src/` is now at upstream Edition 2024 syntax; the `crates/nono/src/trust/{signing,types,policy}.rs` source-migration target files are absorbed. UPST6 audit confirms no new fork-vs-upstream divergence on this surface — the empirical cross-check (§ below) walks `crates/nono/src/trust/signing.rs` + `crates/nono-cli/src/policy.rs` + `crates/nono-cli/src/cli.rs` + `crates/nono-cli/src/profile/mod.rs` and reports drift-tool coverage PASS on all four.

**ADR-review outcome verdict: (a) Confirm Option A `continue`** — per-cell aggregate shape (H, H, M, M, M) — 2 High / 3 Medium / 0 Low, same shape as Phase 42's verdict, holding stable through ~75-raw-commit / 42-filtered-commit evidence base. Phase 33 ADR `Status: Accepted` remains in force; Phase 47 does NOT supersede. Larger evidence base (vs Phase 42's 18) does not surface new amend candidates. See [§ ADR review](#adr-review) for the per-cell L/M/H verdicts.

**Strictly silent on post-v0.57.0 per D-47-A4.** The 19 known post-v0.57.0 commits between `10cec984` and the audit-open upstream/main HEAD `807fca38` are deferred to UPST7 per the Phase 33 ADR § Future audit cadence rule. No task in Plan 47-01 references or processes any commit past `10cec984`.

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
code OUTSIDE the D-11-excluded paths — every commit in this audit returns `no` against the
mechanical D-47-A5 heuristic.

**Inspection methodology** (mirrors Phase 33 + 39 + 42 + D-47-A5 / D-47-D1..D4 extensions): each
commit's `subject` + `categories` + `files_changed[]` length was read from the drift JSON for
every row; per-commit diffs were read for the lead commit in each cluster, any commit whose subject
was ambiguous re: disposition (none triggered this cycle), AND every commit flagged by the D-47-A5
mechanical windows-touch heuristic (zero this cycle). For each `will-sync` cluster, the lead
commit's re-export surface (`pub use` / `pub mod` / `extern crate` / `pub(crate)`) was scanned per
D-47-D2 uniform discipline. Findings appear inline per cluster + consolidated in
[§ Cross-cluster re-export deps detected](#cross-cluster-re-export-deps-detected) below.

**Cluster grouping rationale:** the 42 commits cluster naturally into 9 themes by feature surface
(profile shadowing & pack verification / process startup timeout config / 3-release ride /
landlock-v6 + af_unix mediation / Linux policy & deny-overlap polish / macOS exact-path & localhost
outbound / PTY proxy & musl portability / proxy credential format / package install path). Three
release commits ride along the v0.55.0 / v0.56.0 / v0.57.0 tag chain per Phase 34 + 40 release-ride
convention. No cluster spans both new-feature work AND release-ride concerns; clean separation.

## Cluster Summary

| cluster_id | theme | commits | disposition | windows-touch | rationale |
|------------|-------|---------|-------------|---------------|-----------|
| C1 | Profile shadowing checks + pack signer verification + name-resolution polish | 9 | will-sync | no | Cross-platform profile/pack verification hardening; composes additively with fork's Phase 36 canonical-sections work |
| C2 | Process startup timeout configuration (`--startup-timeout` flag + `NONO_STARTUP_TIMEOUT` env-var + interactive-detection refactor) | 7 | will-sync | no | New cross-platform CLI surface; no Windows-conditional code; composes additively with fork's startup-prompt seam |
| C3 | Release v0.55.0 / v0.56.0 / v0.57.0 (Cargo.toml version bumps) | 3 | will-sync | no | Per Phase 34 + 40 release-ride convention: fork drops upstream's Cargo.toml + Cargo.lock version bumps and absorbs only CHANGELOG entries (fork tracks its own version separately) |
| C4 | Linux Landlock v6 (signal scoping + abstract unix socket scoping) + af_unix pathname mediation | 9 | will-sync | no | Cross-platform Linux-only feature (gated by `#[cfg(target_os = "linux")]`); no Windows-conditional code; intra-cluster re-export surface (`LandlockScopePolicy`, `DetectedAbi`) introduced in lead commit `c2c6f2ca` itself — no cross-cluster dep |
| C5 | Linux policy / Landlock deny-overlap diagnostic quieting + code-review polish | 3 | will-sync | no | Linux-only policy polish; composes additively with fork's Phase 41 Class D Linux deny-overlap regression test (REQ-TEST-HYG-01 closed via Phase 44 Plan 44-02 drain) |
| C6 | macOS exact-path / future-file grant restore + macOS localhost outbound | 3 | will-sync | no | macOS-only seatbelt grant work; composes additively with fork's existing macOS sandbox layer |
| C7 | PTY proxy fixes (bare ESC forwarding + child-output trailing-newline) + musl libc Ioctl portability | 4 | will-sync | no | Cross-platform Unix-side PTY + musl-libc portability fixes; no Windows intersection |
| C8 | Proxy credential format: explicit `credential_format` on custom inject headers | 2 | will-sync | no | Cross-platform `nono-proxy` work + profile schema extension; composes additively with fork's Phase 25 credential injection |
| C9 | Package install path conflict prevention + manifest-based installs | 2 | fork-preserve | no | Two commits refactor artifact install pipeline + add `.nono-trust.bundle` `installed_path` validation. Fork-side intersection with Phase 35 / 45 trust-bundle work has not been diff-confirmed compatible; conservative default applied per D-47-D2 spirit — Phase 48 plan-phase upgrades to `will-sync` after diff inspection confirms upstream's `installed_artifact_relative_path` helper composes cleanly with fork's `validate_bundle_relative_path` defense-in-depth |

### Cluster C1: Profile shadowing checks + pack signer verification + name-resolution polish

**Commits:** 9 (lead: `c897c8cc feat(profiles): expand shadowing checks to include pack profiles`)
**Disposition:** will-sync
**Windows-touch:** no
**Rationale:** Nine commits substantially extend the profile-shadowing + pack-signer-verification + name-resolution surface: (a) `c897c8cc` renames `would_shadow_builtin` to `would_shadow_existing_profile` and extends shadowing checks to include pack profiles (not just built-ins) — prevents user-saved profiles inadvertently overriding pack-provided profiles; (b) `0a4db57e` + `bd76c6b5` add hard-block on profile init when name shadows builtin/pack + review-fix polish; (c) `3d3d239a` + `316c6a2c` + `750f4653` refine profile name resolution (versioned package refs in fast path, fmt + test assertion fixes); (d) `b3556139` adds verification of pack signer identities (cross-references trust/policy surface); (e) `0015f348` + `0b05508f` strengthen pack-source-inclusion in verification + hard-block trust-bundle without lockfile provenance (prevents silent acceptance of unpinned signers when a trust bundle is present). All nine touch only cross-platform `crates/nono-cli/src/profile/`, `crates/nono-cli/src/profile_cmd.rs`, `crates/nono-cli/src/profile_save_runtime.rs`, `crates/nono-cli/src/profile_runtime.rs`, and `crates/nono-cli/src/learn_runtime.rs` files — no `_windows.rs` or `platform.rs` intersection. Composes additively with fork's Phase 36 / 36.5 canonical-sections work (`CommandsConfig`, `FilesystemConfig.deny/bypass_protection`, `LegacyPolicyPatch`, `DeprecationCounter`) per `feedback_cluster_isolation_invalid`-style spot check on `profile/mod.rs` (see [§ Empirical cross-check](#empirical-cross-check) File #4).
**Cross-cluster re-export check:** Clean — scanned lead commit `c897c8cc` for `pub use` / `pub mod` / `extern crate` / `pub(crate)` declarations; single `pub(crate) fn would_shadow_existing_profile` is a local function (not a cross-cluster re-export). No cross-cluster deps detected.
**Target phase:** UPST6-sync (Phase 48)

| sha | subject | upstream-tag | categories | files-changed | windows-touch |
|-----|---------|--------------|------------|---------------|---------------|
| 750f4653 | fix(profile): fix fmt and test assertion after shadow-check refactor | v0.57.0 | other,profile | 3 | no |
| 316c6a2c | fix(profile): handle versioned package refs in fast path | v0.57.0 | profile | 1 | no |
| 3d3d239a | feat(profile): refine profile name resolution and init validation | v0.57.0 | other,profile | 3 | no |
| 0a4db57e | fix(profiles): block profile init when name shadows builtin or pack profile | v0.57.0 | other | 1 | no |
| bd76c6b5 | fix(profiles): address review points on shadow-check PR | v0.57.0 | other | 1 | no |
| c897c8cc | feat(profiles): expand shadowing checks to include pack profiles | v0.57.0 | other | 2 | no |
| b3556139 | feat(profiles): verify pack signer identities | v0.55.0 | other,profile | 2 | no |
| 0015f348 | feat(profile): ensure source pack is included for verification | v0.55.0 | other,profile | 3 | no |
| 0b05508f | fix(profile-verification): strengthen profile and pack verification checks | v0.55.0 | other | 1 | no |

### Cluster C2: Process startup timeout configuration

**Commits:** 7 (lead: `2bed3565 feat(cli): add option to configure process startup timeout`)
**Disposition:** will-sync
**Windows-touch:** no
**Rationale:** Seven commits introduce a new cross-platform CLI surface for explicit process-startup-timeout configuration: (a) `2bed3565` introduces `--startup-timeout` flag + `NONO_STARTUP_TIMEOUT` env-var on `run`, `shell`, `wrap` commands; refactors timeout-detection logic to remove implicit profile-tied behaviors; adds formatted termination messages with `--profile` hints; (b) `a8646d26` + `8628fd6d` expand interactive-detection (alt-screen instead of any-output) + require alt-screen for the timeout decision; (c) `468d3813` clarifies docs for "interactive" definition; (d) `4e0e127a` uses SIGKILL consistently + removes dead `startup_prompt.rs` infrastructure (193 deletions); (e) `1be97978` + `50272a03` refactor timeout-check simplification. All seven touch only cross-platform `crates/nono-cli/src/{cli,command_runtime,exec_strategy,execution_runtime,launch_runtime,output,pty_proxy,startup_prompt}.rs` files — no `_windows.rs` intersection. Composes additively with fork's existing startup-prompt seam; fork's Phase 27 PTY proxy work is upstream-equivalent. Removing dead `startup_prompt.rs` infrastructure may surface fork-side cleanups if fork still references that surface (Phase 48 plan-phase diff-inspects).
**Cross-cluster re-export check:** Clean — scanned lead commit `2bed3565` for `pub use` / `pub mod` / `extern crate` / `pub(crate)` declarations; no new re-exports detected. The startup-timeout refactor is contained to internal function signatures + a single new public CLI flag.
**Target phase:** UPST6-sync (Phase 48)

| sha | subject | upstream-tag | categories | files-changed | windows-touch |
|-----|---------|--------------|------------|---------------|---------------|
| 50272a03 | refactor(cli): simplify startup timeout check | v0.56.0 | other | 1 | no |
| 1be97978 | refactor(cli-exec-strategy): simplify startup timeout checks | v0.56.0 | other | 1 | no |
| 4e0e127a | fix(startup): use SIGKILL consistently and remove dead prompt infrastructure | v0.56.0 | other | 5 | no |
| 468d3813 | docs(cli): clarify startup timeout definition of interactive | v0.56.0 | other | 1 | no |
| 8628fd6d | refactor(cli): require alt-screen for startup timeout | v0.56.0 | other | 1 | no |
| a8646d26 | feat(cli): expand startup timeout interactive detection | v0.56.0 | other | 4 | no |
| 2bed3565 | feat(cli): add option to configure process startup timeout | v0.56.0 | other | 8 | no |

### Cluster C3: Release v0.55.0 / v0.56.0 / v0.57.0 (Cargo.toml version bumps)

**Commits:** 3
**Disposition:** will-sync
**Windows-touch:** no
**Rationale:** Three release commits ride along the v0.55.0 / v0.56.0 / v0.57.0 tag chain. Per Phase 34 + Phase 40 + Phase 43 release-ride convention (precedent commit `64b231a7`): fork DROPS upstream's Cargo.toml + Cargo.lock version bumps and absorbs only CHANGELOG.md entries (fork tracks its own version separately; v2.5 milestone close shipped at fork version 0.53.0; v2.6 may bump independently). Each commit touches exactly `crates/nono/Cargo.toml` (1 file each); structural release-ride pattern.
**Cross-cluster re-export check:** N/A — release commits touch only Cargo.toml; no source-file re-export surface.
**Target phase:** UPST6-sync (Phase 48) — three separate release-ride absorptions OR one consolidated CHANGELOG-only absorption (Phase 48 plan-phase decides per release-ride convention)

| sha | subject | upstream-tag | categories | files-changed | windows-touch |
|-----|---------|--------------|------------|---------------|---------------|
| 35f9fea2 | chore: release v0.55.0 | v0.55.0 | other | 1 | no |
| b251c72f | chore: release v0.56.0 | v0.56.0 | other | 1 | no |
| 10cec984 | feat(release): release version 0.57.0 | v0.57.0 | other | 1 | no |

### Cluster C4: Linux Landlock v6 + af_unix pathname mediation

**Commits:** 9 (lead: `c2c6f2ca feat(landlock): add landlock v6 signal and abstract unix socket scoping`)
**Disposition:** will-sync
**Windows-touch:** no
**Rationale:** Nine commits introduce a substantial Linux-only Landlock v6 feature stack: (a) `c2c6f2ca` (11 files, +705/-28) adds Landlock v6 signal scoping + abstract unix socket scoping with `nono why --scope <SCOPE>` query command + `IpcMode::SharedMemoryOnly` requesting abstract unix socket scoping + `SignalMode::Isolated` / `AllowSameSandbox` using Landlock signal scoping; introduces `LandlockScopePolicy` + `DetectedAbi` core types; (b) `a0222be2` (18 files, +814/-70) implements af_unix pathname mediation with `feat(linux)` cfg gating; (c) `1e9385a7` adds explicit allowlist for pathname af_unix sockets; (d) `bbc652a0` records explicit scope for grants; (e) `858ad009` adds recursive unix socket directory grants on CLI; (f) `d146001b` + `98f8cb18` correctly resolve af_unix paths for seccomp + add unix listener test; (g) `863bbfd3` refines IPC denial reporting + audit timestamps; (h) `b8a32006` clarifies linux signal mode behavior with landlock (docs). All touch cross-platform `crates/nono-cli/src/{cli,exec_strategy,exec_strategy/supervisor_linux,output,query_ext,sandbox_prepare,setup,supervised_runtime,why_runtime}.rs` + `crates/nono/src/{capability,lib,sandbox/linux,sandbox/mod}.rs` + supporting profile/command_runtime/launch_runtime files. The Linux-only logic is properly cfg-gated; cross-platform surface composes additively. **Note:** `0b05508f` was originally signal-grouped here by drift-tool date proximity, reassigned to Cluster C1 after diff inspection (pure pack-verification, not landlock). Move documented as auditor judgment — diff at `crates/nono-cli/src/profile_runtime.rs` shows hard-block on trust-bundle-without-lockfile, no landlock surface intersection.
**Cross-cluster re-export check:** Clean — scanned lead commit `c2c6f2ca` for `pub use` / `pub mod` / `extern crate` / `pub(crate)` declarations; found `pub use sandbox::{DetectedAbi, LandlockScopePolicy, detect_abi, is_wsl2, landlock_scope_policy};` in `crates/nono/src/lib.rs` and `pub use linux::{DetectedAbi, LandlockScopePolicy, detect_abi, landlock_scope_policy};` in `crates/nono/src/sandbox/mod.rs`. Re-exported symbols `LandlockScopePolicy` + `DetectedAbi` + `landlock_scope_policy` are INTRODUCED in `c2c6f2ca` itself (verified via `git show c2c6f2ca -- crates/nono/src/sandbox/linux.rs | grep '^+pub'` — `pub struct LandlockScopePolicy { ... }` + `pub fn landlock_scope_policy(caps: ...) -> Result<LandlockScopePolicy> { ... }` both new-additions in the same commit). INTRA-cluster re-export, NOT cross-cluster. No D-47-D4 split-flip required.
**Target phase:** UPST6-sync (Phase 48)

| sha | subject | upstream-tag | categories | files-changed | windows-touch |
|-----|---------|--------------|------------|---------------|---------------|
| c2c6f2ca | feat(landlock): add landlock v6 signal and abstract unix socket scoping | v0.55.0 | other | 11 | no |
| b8a32006 | docs(capability): clarify linux signal mode behavior with landlock | v0.55.0 | other | 2 | no |
| 858ad009 | feat(cli): add recursive unix socket directory grants | v0.55.0 | other,profile | 8 | no |
| bbc652a0 | feat(unix-socket): record explicit scope for grants | v0.55.0 | other | 4 | no |
| 1e9385a7 | feat(sandbox): add explicit allowlist for pathname af_unix sockets | v0.55.0 | other | 4 | no |
| 98f8cb18 | test(supervisor-linux): add unix listener for connect capability test | v0.55.0 | other | 1 | no |
| d146001b | fix(sandbox): correctly resolve af_unix socket paths for seccomp | v0.55.0 | other | 2 | no |
| a0222be2 | feat(linux): implement af_unix pathname mediation | v0.55.0 | other,policy,profile | 18 | no |
| 863bbfd3 | refactor(supervisor): refine ipc denial reporting and audit timestamps | v0.55.0 | other | 2 | no |

### Cluster C5: Linux policy + Landlock deny-overlap diagnostic quieting

**Commits:** 3 (lead: `4fa9f6a6 cli: quiet Landlock deny-overlap diagnostics on Linux`)
**Disposition:** will-sync
**Windows-touch:** no
**Rationale:** Three small Linux-policy + Landlock-deny-overlap polish commits: (a) `4fa9f6a6` quiets deny-overlap diagnostics on Linux (touches `crates/nono-cli/src/policy.rs`); (b) `e6215f8b` review fix for `4fa9f6a6`; (c) `1122c315` code-review polish on `crates/nono/src/sandbox/linux.rs`. Linux-only policy diagnostic polish — cross-platform surface composes additively with fork's Phase 41 Class D Linux deny-overlap regression test (REQ-TEST-HYG-01 closed via Phase 44 Plan 44-02 drain). Composes cleanly with fork's existing `policy.rs` canonical-validation surface.
**Cross-cluster re-export check:** Clean — scanned lead commit `4fa9f6a6` for `pub use` / `pub mod` / `extern crate` / `pub(crate)` declarations; pure diagnostic-quieting cleanup with no public-API surface change.
**Target phase:** UPST6-sync (Phase 48)

| sha | subject | upstream-tag | categories | files-changed | windows-touch |
|-----|---------|--------------|------------|---------------|---------------|
| e6215f8b | review fix | v0.55.0 | policy | 1 | no |
| 4fa9f6a6 | cli: quiet Landlock deny-overlap diagnostics on Linux | v0.55.0 | policy | 1 | no |
| 1122c315 | fix: code review | v0.55.0 | other | 1 | no |

### Cluster C6: macOS exact-path / future-file grant restore + macOS localhost outbound

**Commits:** 3 (lead: `2c3742ab fix(cli): preserve macOS future-file grants in why --self`)
**Disposition:** will-sync
**Windows-touch:** no
**Rationale:** Three macOS-specific grant-restore + outbound-routing fixes: (a) `2c3742ab` preserves macOS future-file grants in `why --self` (194 +/19 -, touches `capability_ext.rs` + `sandbox_state.rs`); (b) `74b0be71` unifies macOS exact-path grant restore (104 +/60 -); (c) `abca959a` treats `open_port 0` as `localhost:*` outbound on macOS (touches `profile/mod.rs`, `capability.rs`, `sandbox/{linux,macos}.rs`). macOS Seatbelt grant-restore work — cross-platform surface composes additively with fork's existing macOS sandbox layer. Phase 48 plan-phase reviews `capability_ext.rs` per-commit diff against fork's Phase 32 / 33 macOS canonical-sections work; conservative-default `will-sync` because the file touches are pure macOS-side and fork's macOS layer is upstream-equivalent.
**Cross-cluster re-export check:** Clean — scanned lead commit `2c3742ab` for `pub use` / `pub mod` / `extern crate` / `pub(crate)` declarations; no new public-API surface added.
**Target phase:** UPST6-sync (Phase 48)

| sha | subject | upstream-tag | categories | files-changed | windows-touch |
|-----|---------|--------------|------------|---------------|---------------|
| 74b0be71 | fix(cli): unify macOS exact-path grant restore | v0.55.0 | other | 2 | no |
| 2c3742ab | fix(cli): preserve macOS future-file grants in why --self | v0.55.0 | other | 2 | no |
| abca959a | feat(macos): treat open_port 0 as localhost:* outbound | v0.55.0 | other,profile | 4 | no |

### Cluster C7: PTY proxy + musl libc portability fixes

**Commits:** 4 (lead: `1f552106 fix: preserve child output without trailing newline (#881)`)
**Disposition:** will-sync
**Windows-touch:** no
**Rationale:** Four small portability + UX fixes on Unix-side PTY + musl libc: (a) `1f552106` (PR #881) preserves child output without trailing newline in `prepare_parent_output_area` (touches `pty_proxy.rs` only); (b) `279af554` forwards bare ESC immediately in `filter_client_input` (PTY proxy); (c) `3d0ff87f` + `3cd22aa5` fix musl libc `Ioctl` type mismatches for `x86_64-unknown-linux-musl` target (touches `pty_proxy.rs`, `exec_strategy.rs`, `sandbox/linux.rs`). All cross-platform Unix-side fixes; no Windows intersection. Cherry-pick risk: musl portability changes may not have a corresponding fork-side regression test — Phase 48 plan-phase may add one.
**Cross-cluster re-export check:** Clean — scanned lead commit `1f552106` for `pub use` / `pub mod` / `extern crate` / `pub(crate)` declarations; pure internal-function fix.
**Target phase:** UPST6-sync (Phase 48)

| sha | subject | upstream-tag | categories | files-changed | windows-touch |
|-----|---------|--------------|------------|---------------|---------------|
| 279af554 | fix(pty): forward bare ESC immediately in filter_client_input | v0.55.0 | other | 1 | no |
| 3d0ff87f | fix(musl): use as _ for TIOCSCTTY ioctl cast to support all platforms | v0.55.0 | other | 1 | no |
| 3cd22aa5 | fix(musl): fix libc::Ioctl type mismatches for x86_64-unknown-linux-musl target | v0.55.0 | other | 3 | no |
| 1f552106 | fix: preserve child output without trailing newline (#881) | v0.55.0 | other | 1 | no |

### Cluster C8: Proxy credential format on custom inject headers

**Commits:** 2 (lead: `57005737 fix(proxy): honor explicit credential_format on custom inject headers`)
**Disposition:** will-sync
**Windows-touch:** no
**Rationale:** Two commits add a wire-protocol-level distinction between omitted-and-default vs explicit `'Bearer {}'` for `credential_format` on custom inject headers: (a) `57005737` treats `credential_format` as `Option<String>` so omitted vs explicit `'Bearer {}'` are distinct; defaults resolved only when absent (Authorization → `Bearer {}`, other headers → bare `{}`); manifest export + profile validation use the same resolution. Touches `crates/nono-cli/data/nono-profile.schema.json`, `network_policy.rs`, `profile/mod.rs`, `profile_cmd.rs`, `crates/nono-proxy/src/{config,credential,route,server}.rs`. (b) `530306ee` review-fix follow-up. Cross-platform `nono-proxy` + profile schema extension; composes additively with fork's Phase 25 credential-injection layer. **Schema change risk:** `nono-profile.schema.json` is fork-shared; Phase 48 plan-phase diff-inspects to confirm no fork-side schema validators (jsonschema validation in tests) regress.
**Cross-cluster re-export check:** Clean — scanned lead commit `57005737` for `pub use` / `pub mod` / `extern crate` / `pub(crate)` declarations; pure type-signature change + schema extension.
**Target phase:** UPST6-sync (Phase 48)

| sha | subject | upstream-tag | categories | files-changed | windows-touch |
|-----|---------|--------------|------------|---------------|---------------|
| 530306ee | review fix | v0.55.0 | other,profile,proxy | 3 | no |
| 57005737 | fix(proxy): honor explicit credential_format on custom inject headers | v0.55.0 | other,profile,proxy | 7 | no |

### Cluster C9: Package install path conflict prevention + manifest-based installs

**Commits:** 2 (lead: `5f1c9c73 refactor(package): base installs on package manifest`)
**Disposition:** fork-preserve
**Windows-touch:** no
**Rationale:** Two commits refactor the artifact-installation pipeline to be manifest-driven instead of filename-inferred: (a) `5f1c9c73` introduces `installed_artifact_relative_path` helper centralizing manifest→install-path mapping; extends `.nono-trust.bundle` with `installed_path` + `sha256_digest` per artifact; adds `validate_bundle_relative_path` to prevent unsafe `installed_path` values when reading from trust bundles; removes `infer_artifact_type` function (superseded by manifest-driven approach); (b) `8d774753` extends with explicit artifact-install-path-conflict prevention. **Fork-side intersection concern:** fork's Phase 35 (REQ-PORT-CLOSURE-04 path security) + Phase 45 trust-bundle handling work has not been diff-confirmed compatible with upstream's `installed_path` field extension on `.nono-trust.bundle`. Conservative `fork-preserve` default applied per D-47-D2 spirit (uniform-discipline scan reveals fork's trust-bundle schema may carry a fork-divergent field set). **Phase 48 plan-phase action:** diff-inspect `crates/nono-cli/src/package_cmd.rs` + `crates/nono/src/trust/policy.rs` + `crates/nono/src/manifest.rs` against `5f1c9c73`; if no schema collision detected, UPGRADE to `will-sync`. The conservative default protects fork's D-32-15 verify-is-offline invariant (`.nono-trust.bundle` schema additions must not break offline verify path).
**Cross-cluster re-export check:** N/A for fork-preserve clusters per D-47-D2 (re-export scan runs on `will-sync` clusters only). Phase 48 plan-phase performs the scan when upgrading disposition.
**Target phase:** UPST6-sync (Phase 48) — `fork-preserve` default; Phase 48 may upgrade to `will-sync` after diff inspection

| sha | subject | upstream-tag | categories | files-changed | windows-touch |
|-----|---------|--------------|------------|---------------|---------------|
| 8d774753 | feat(package): prevent artifact install path conflicts | v0.55.0 | package | 1 | no |
| 5f1c9c73 | refactor(package): base installs on package manifest | v0.55.0 | other,package | 2 | no |

## ADR review

The Phase 33 strategic ADR (`docs/architecture/upstream-parity-strategy.md`, `Status: Accepted` 2026-05-11, re-confirmed at v2.4 close per D-39-C4, re-confirmed at v2.5 close per D-42-C4 verdict) chose Option A `continue`. **Phase 47 UPST6 is the largest single audit cycle yet** — 42 post-D-11-filter commits spanning 3 minor releases (v0.55.0 / v0.56.0 / v0.57.0), substantially larger than Phase 42's 18, Phase 39's 22, and Phase 33's 97 (but Phase 33 cumulative spanned v0.40.1..v0.52.0). Per D-47-E8 MANDATORY, this section ships explicit per-cell L/M/H verdicts for the 5 dimensions enumerated in the Phase 33 ADR Decision Table.

### Per-cell L/M/H verdict — Option A `continue` at v0.54.0..v0.57.0

| dimension | verdict | rationale |
|-----------|---------|-----------|
| security  | High    | Zero `windows-touch:yes` commits this cycle (vs Phase 42's 3 fires) — the absorbed `crates/nono-cli/src/platform.rs` foundation (Phase 43 + 45) has held without new windows-platform-detection iteration from upstream. Cluster C8 (proxy credential format) hardens credential-injection wire protocol (cross-platform fix). Cluster C4 (Landlock v6 + af_unix mediation) closes a structural Linux-IPC isolation gap that fork would otherwise need to manual-replay. Cluster C9 (package install path conflict prevention) adds `validate_bundle_relative_path` defending against unsafe `installed_path` values in trust bundles — directly the kind of security-fix flow-in scenario the ADR's security-posture cell argued for. Phase 48 absorbs all three via cherry-pick (modulo C9 fork-preserve / will-sync upgrade decision). |
| windows   | High    | Single-CLI-surface preservation across Linux/macOS/Windows is structurally intact: zero windows-touch:yes commits means no new fork-side windows-conditional rework is needed. The Cluster C4 Linux-only Landlock v6 work is cfg-gated (`#[cfg(target_os = "linux")]`); Windows is structurally unaffected. The Phase 43 + 45 absorption of Cluster 5 (`0748cced` + `5d821c12` Windows platform-detection) closed the v2.5 risk surface; no upstream regression detected this cycle. ADR's user-clarity High verdict ("one nono binary, one docs URL") preserved: Phase 48 absorbs cross-platform feature work into the same workspace where fork's Windows-specific seams (broker, IL backend, WFP) already live. |
| maintenance | Medium | 42 commits / 9 clusters absorbed in one audit cycle is the largest single-cycle load this milestone (vs Phase 42's 18 / 7 and Phase 39's 22 / 7); still sustainable because the cluster breakdown is dominated by `will-sync` straight cherry-picks (7 / 9 = 78%). Cluster C4 (Landlock v6 + af_unix) at 9 commits and 18-files-in-`a0222be2` is the largest single source of cherry-pick labor — Phase 48 plan-phase MUST sequence this cluster carefully (multi-commit wave structure likely). The 1 fork-preserve cluster (C9) is a conservative-default with explicit upgrade pathway documented; maintenance cost is bounded. Compared to Phase 42's 2 fork-preserve clusters (C4 + C5 windows-touch:yes), this cycle's fork-preserve labor is structurally smaller. If Phase 48 surfaces unavoidable manual-replay conflicts on C4 or C9, maintenance cost shifts toward High — but per current Phase 47 evidence base, M is the right call. |
| divergence  | Medium  | At v0.54.0..v0.57.0 the per-cycle commit count is 42 (vs Phase 42's 18); the cadence rule firing per upstream release (D-47-E6) means accumulated drift stays bounded as long as UPST6+ cycles fire within ~2 weeks of upstream release. Zero windows-touch:yes count is the divergence-risk evidence supporting M: fork has NOT diverged on the Windows platform-detection surface in this cycle; the absorbed Cluster 2 / Cluster 5 from Phase 43 + 45 closed the structural risk. The 1 fork-preserve cluster (C9) is a small-surface intersection with fork's Phase 35 trust-bundle work; even if Phase 48 keeps C9 as `fork-preserve`, divergence stays bounded because the file count is small (2 commits / 3 files). If future cycles surge to 5+ fork-preserve clusters per release, divergence risk would shift toward High and the ADR cadence-rule warning fires. |
| contributor | Medium  | Phase 47 sized at ~1 audit cycle (1 plan for UPST6 audit + 1 plan for backfill = 2 plans total in Phase 47); Plan 47-01 ~1 work-day per Phase 42 sizing precedent scaled for 2.3x commit count. Aligns with the ADR's Decision-Table Med verdict for per-release drift-audit + cherry-pick labor. Phase 48 sizing (TBD at plan-phase) is the next contributor-velocity signal — if Phase 48 sustains ≤2-week elapsed time for 42-commit cherry-pick + Cluster C4 multi-commit sequencing, velocity stays Medium; growth past 2 weeks per cycle would shift to Low. The Phase 46 close-gate baseline (`3f638dc6` per `.planning/templates/upstream-sync-quick.md:102`) means Phase 48's baseline-aware CI gate is a real regression detector — drift-audit + cherry-pick gate review burden is contained. Zero windows-touch:yes this cycle means Phase 48 plan-phase does NOT need fork-side analog inspection on every cluster (vs Phase 42 + 43's labor on Cluster 4/5 windows-touch:yes). |

### Verdict outcome

**Outcome:** (a) Confirm Option A `continue`. Per-cell aggregate shape: (H, H, M, M, M) — 2 High / 3 Medium / 0 Low. This dominates Option B's reference shape (1 High / 0 Med / 4 Low) and Option C's (1 High / 2 Med / 2 Low) without invoking the D-33-C3 tiebreaker. **The aggregate is identical to Phase 42's verdict shape** — the cycle's larger evidence base (42 vs 18 commits) does NOT surface new amend candidates; the Phase 42 + 43 + 45 absorption of the windows-platform-detection risk surface holds through the v0.54.0..v0.57.0 evidence base. The Phase 33 ADR `Status: Accepted` remains in force; Phase 47 does NOT supersede. Phase 48 plan-phase MAY produce a follow-on ADR amendment if Cluster C4 multi-commit sequencing or Cluster C9 trust-bundle schema collision surfaces a structural pattern worth codifying; this is plan-phase discretion, not Phase 47 verdict.

Per the Phase 33 ADR § Future audit cadence rule (D-47-E6): "per upstream release, lazily-evaluated" — UPST7 fires when next upstream release ships (already partially accumulated: 19 post-v0.57.0 commits visible between `10cec984` and `807fca38` at audit-open time) OR maintainer decides accumulated cherry-pick labor warrants firing. UPST7 stub queued in ROADMAP per D-47-E11 (see [§ Hand-off](#hand-off-to-phase-48) below).

## Empirical cross-check

Per D-47-D1, the audit walk spot-checks ≥4 fork-shared files for any upstream path the drift tool missed (Phase 47 SC#3 raises Phase 42 ≥3 → ≥4). Per D-47-E12 preferential sampling, Phase 47 walks files in `crates/nono-cli/src/platform.rs` (Phase 43 Cluster 5 absorption surface), `crates/nono/src/trust/` (Phase 43 Cluster 2 + Phase 45 Plan 45-01 source-migration target), AIPC schema files, and Phase 45 Plan 45-01 source-migration target files since those are the highest-risk surfaces for further upstream drift.

Methodology: for each sampled file, run `git log v0.54.0..v0.57.0 -- <file>` against `upstream/main` and confirm the drift tool's commit list covers every upstream commit touching that file.

### File: crates/nono-cli/src/platform.rs

- Walked upstream log: `git log v0.54.0..v0.57.0 -- crates/nono-cli/src/platform.rs`
- Commits touching this file in range: **0**
- Cluster mapping: N/A — no upstream commits touched this file in the v0.54.0..v0.57.0 range
- Drift-tool coverage: **PASS** — the Phase 43 Cluster 5 absorption surface has not been further iterated by upstream in this cycle. Zero drift; zero risk of further fork-side rework on `platform.rs`.

### File: crates/nono/src/trust/signing.rs

- Walked upstream log: `git log v0.54.0..v0.57.0 -- crates/nono/src/trust/signing.rs`
- Commits touching this file in range: **0**
- Cluster mapping: N/A — no upstream commits touched this file in the v0.54.0..v0.57.0 range
- Drift-tool coverage: **PASS** — the Phase 43 Cluster 2 + Phase 45 Plan 45-01 source-migration target file has not been further iterated by upstream in this cycle. The split-disposition closure from Phase 45 (`f640528a..d21399e3`) holds. Zero drift on the trust/signing surface.

### File: crates/nono-cli/src/policy.rs

- Walked upstream log: `git log v0.54.0..v0.57.0 -- crates/nono-cli/src/policy.rs`
- Commits touching this file in range: **3** (`e6215f8b`, `4fa9f6a6`, `a0222be2`)
- Cluster mapping: Cluster C5 (`e6215f8b`, `4fa9f6a6` Linux deny-overlap quieting) + Cluster C4 (`a0222be2` af_unix pathname mediation)
- Drift-tool coverage: **PASS** — all 3 upstream commits touching this file appear in the drift JSON (verified via `python3 -c "import json; d=json.load(open('.../drift.json')); print([c['sha'][:8] for c in d['commits'] if 'crates/nono-cli/src/policy.rs' in c['files_changed']])"`).

### File: crates/nono-cli/src/profile/mod.rs

- Walked upstream log: `git log v0.54.0..v0.57.0 -- crates/nono-cli/src/profile/mod.rs`
- Commits touching this file in range: **8** (`750f4653`, `316c6a2c`, `3d3d239a`, `abca959a`, `858ad009`, `b3556139`, `0015f348`, `a0222be2` — fork-side this file gained the Phase 36 / 36.5 `CommandsConfig` + `FilesystemConfig.deny/bypass_protection` + `LegacyPolicyPatch` + `DeprecationCounter` canonical-sections extensions)
- Cluster mapping: Cluster C1 (5 commits: profile shadowing/init validation) + Cluster C4 (1: a0222be2 af_unix pathname mediation) + Cluster C6 (1: abca959a macOS localhost outbound) + a few others spanning clusters
- Drift-tool coverage: **PASS** — all 8 upstream commits touching this file appear in the drift JSON. The fork-side intersection risk (Phase 36 canonical sections in `From<ProfileDeserialize>` exhaustive match) is identified — Phase 48 plan-phase MUST diff-inspect each commit's `profile/mod.rs` hunks against fork's `CommandsConfig` extensions to confirm no exhaustive-match regression. This is precisely the `feedback_cluster_isolation_invalid` lesson empirically applied: file walk surfaces a known fork-divergence hot spot that the drift tool's category labels do not flag.

### File: crates/nono-cli/src/cli.rs

- Walked upstream log: `git log v0.54.0..v0.57.0 -- crates/nono-cli/src/cli.rs`
- Commits touching this file in range: **6** (`a8646d26`, `2bed3565`, `468d3813`, `4e0e127a`, `c2c6f2ca`, `bbc652a0`, `858ad009`)
- Cluster mapping: Cluster C2 (4 commits: startup-timeout flag work) + Cluster C4 (3 commits: Landlock v6 + af_unix CLI surface)
- Drift-tool coverage: **PASS** — all upstream commits touching this file appear in the drift JSON. Note: `cli.rs` is the most-touched cross-platform file this cycle (large clap-arg surface area; high churn).

**Findings summary:** All 5 sampled files PASS; drift tool's commit list is complete against the v0.54.0..v0.57.0 fork-shared surface for the sampled subsystems. **No drift-tool blind spots surfaced; no D-47-E10 quick-task spawn required.** The empirical cross-check confirms the D-11 path filter remains necessary-but-not-sufficient — every upstream commit touching the sampled cross-platform files appears in the drift JSON, and zero `windows-touch:yes` matches this cycle means the D-47-A5 column did not need to fire. The Phase 43 + 45 absorption surfaces (`platform.rs` + `trust/signing.rs`) show ZERO upstream churn — the absorption has not been disrupted by subsequent upstream work, structurally supporting the Phase 33 ADR `continue` re-verdict.

## Cross-cluster re-export deps detected

Per D-47-D3, this section consolidates per-cluster re-export findings. Scanned 7 `will-sync` clusters (C1, C2, C3, C4, C5, C6, C7, C8 — C3 release commits skipped re-export scan per the N/A category in the cluster body; that's 7 scans across C1, C2, C4, C5, C6, C7, C8). Cluster C9 disposition `fork-preserve` so re-export scan deferred to Phase 48 plan-phase upgrade.

**No cross-cluster re-export deps detected across 7 will-sync clusters scanned.**

The only re-export surface observed was Cluster C4 lead commit `c2c6f2ca`:
- `pub use sandbox::{DetectedAbi, LandlockScopePolicy, detect_abi, is_wsl2, landlock_scope_policy};` in `crates/nono/src/lib.rs`
- `pub use linux::{DetectedAbi, LandlockScopePolicy, detect_abi, landlock_scope_policy};` in `crates/nono/src/sandbox/mod.rs`

Both re-export the same set of symbols (`DetectedAbi`, `LandlockScopePolicy`, `detect_abi`, `landlock_scope_policy`, `is_wsl2`) which are INTRODUCED in `c2c6f2ca` itself via `pub struct LandlockScopePolicy { ... }` + `pub fn landlock_scope_policy(caps: &CapabilitySet) -> Result<LandlockScopePolicy> { ... }` + `pub struct DetectedAbi` definitions in `crates/nono/src/sandbox/linux.rs`. **INTRA-cluster re-export, NOT cross-cluster.** No D-47-D4 split-flip required this cycle.

**Empirical closure of `feedback_cluster_isolation_invalid`:** the Phase 43 Cluster 2 (`8b888a1c` re-exporting `public_key_id_hex` + `sign_statement_bundle` in `crates/nono/src/trust/mod.rs` from prerequisite upstream commits the fork hadn't absorbed) class is STRUCTURALLY PREVENTED by D-47-D1..D4 — the scan ran on every `will-sync` lead commit and surfaced the only re-export edge in `c2c6f2ca`; deep inspection confirmed intra-cluster origin; no flip-to-split required. Phase 48 inherits a cleaner UPST6 sync execution surface than Phase 43 did for UPST5 — no Cluster-2-style structural surprises waiting in the cherry-pick queue.

## Hand-off to Phase 48

The ledger above is the immutable input for Phase 48 UPST6 sync execution. The [§ Cluster Summary](#cluster-summary) table feeds Phase 48 plan-phase plan-slicing. The 7 `will-sync` clusters carry explicit per-cluster dispositions; Phase 48 must honor these without re-relitigating the call (per ROADMAP § Phase 47 SC #1 + REQ-UPST6-01 acceptance criteria).

**Wave-hint summary for Phase 48:**
- **Cluster C4 (Landlock v6 + af_unix mediation)** is the largest single cluster at 9 commits — sequence first as a multi-commit wave (or sub-cluster into Wave 0a / 0b split per the Phase 43 Plan 43-01b split-disposition precedent if `a0222be2`'s 18-file scope triggers conflict pressure). Suggested foundation candidate per the Phase 42 Cluster 2 foundation-flag precedent.
- **Cluster C1 (profile shadowing + pack verification)** at 9 commits is the second-largest cluster; sequence after C4 if both clusters touch `profile/mod.rs` (verified via [§ Empirical cross-check](#empirical-cross-check) File #4 — they do; both Cluster C1 and Cluster C4 touch `profile/mod.rs`).
- **Cluster C2 (startup timeout config)** at 7 commits has the most cross-platform CLI surface change (`cli.rs`, `command_runtime.rs`, `exec_strategy.rs`, etc.); sequence carefully to avoid conflict with Cluster C4's CLI surface additions (`cli.rs` is touched by both clusters per [§ Empirical cross-check](#empirical-cross-check) File #5).
- **Cluster C3 (release ride)** is structurally last (Cargo.toml version-bump drops per Phase 34 + 40 + 43 release-ride convention). Phase 48 may consolidate the 3 release-ride absorptions into a single CHANGELOG-only commit per the standard convention.
- **Cluster C9 (package install path conflict)** is `fork-preserve` default; Phase 48 plan-phase performs the diff inspection and upgrade-to-`will-sync` decision; if upgraded, the D-47-D2 re-export scan must be performed before cherry-pick.

**Baseline-aware CI gate baseline SHA** is `3f638dc6` per `.planning/templates/upstream-sync-quick.md:102` (REQ-CI-FU-03 closed at Phase 46 close 2026-05-23); Phase 48 inherits this as the gate reference for `success → failure` regression detection. Any `wave-hint:` annotations above are advisory; Phase 48 retains full discretion to refine.

**UPST7 cadence trigger:** already accumulating — 19 post-v0.57.0 commits visible between `10cec984` and audit-open upstream/main HEAD `807fca38` at fetch time (2026-05-24). UPST7 plan-phase can fire any time after Phase 48 close per D-47-E6.
