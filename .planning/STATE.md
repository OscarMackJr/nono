---
gsd_state_version: 1.0
milestone: v3.5
milestone_name: Trusted Signing Go-Live + First Distributed Release
status: Roadmap approved — 7 phases (101-107), 10/10 requirements mapped, ready for plan-phase
stopped_at: Phase 101 context gathered
last_updated: "2026-07-02T18:38:58.493Z"
last_activity: 2026-07-02 — Roadmap created
progress:
  total_phases: 1
  completed_phases: 0
  total_plans: 0
  completed_plans: 0
---

# Project State: nono — v3.5 Trusted Signing Go-Live + First Distributed Release

## Project Reference

See: `.planning/PROJECT.md` (v3.5 milestone active 2026-07-02; v3.4 SHIPPED + archived, tag `v3.4` local. Phases 98-100 archived to `milestones/v3.4-ROADMAP.md`). Roadmap: `.planning/ROADMAP.md` (Phases 101-107, 10/10 requirements mapped). Requirements: `.planning/REQUIREMENTS.md` (defined below). Phase numbering continues 100 → 101+ (no reset).

**Core Value:** Windows security must be as structurally impossible and feature-complete as Unix platforms — and, for v3.5, actually *distributable*: a publicly-trusted-signed release that runs out-of-the-box on a clean host.

**Current Focus:** v3.5 — Trusted Signing Go-Live + First Distributed Release. Full go-live EXECUTE (operator-in-loop): resolve the Azure Trusted Signing verify-gate `UnknownError`, cut the first trusted-signed `0.66.1` release, publish live to crates.io/PyPI/npm (FUT-01), drain both host-gated clean-host todos on a fresh Azure Win11 VM (FUT-03), and close out.

## Current Position

Phase: 101 (Verify-Gate Hardening + Azure Profile Confirmation) — not started
Plan: —
Status: Roadmap approved — 7 phases (101-107), 10/10 requirements mapped, ready for plan-phase
Last activity: 2026-07-02 — Roadmap created

## Performance Metrics

**Velocity:** (v3.5 — reset; populated as phases complete)

- Total plans completed: 0
- Average duration: —
- Total execution time: —

| Phase | Plan | Duration | Tasks | Files |
|-------|------|----------|-------|-------|

*Updated after each plan completion*

## Accumulated Context

### Key Decisions (v3.3 roadmap — historical)

| Decision | Phase | Rationale |
|----------|-------|-----------|
| 4 phases (94-97), not 2-3 | all | Three distinct concerns (audit, absorb, cross-target, release) each have a clean delivery boundary and different risk profiles; collapsing absorb+release creates a dependency inversion (version bump must come after sync). |
| UPST10-04 (remote relocation) folded into Phase 94 | 94 | The `nolabs-ai/nono` rename is audit-setup work — done at audit-open when fetching commits; a separate phase would be artificial. |
| Version leapfrog (RLS-05) in Phase 97, after Phase 95 sync | 97 | Bump once, post-sync, to a clean ≥ 0.65.0; bumping mid-sync creates a rebasing treadmill and dirty Cargo.lock during cherry-picks. |
| Cross-target (Phase 96) sequenced after Phase 95 sync | 96 | XTGT clippy gates should run against the synced + post-sync tree, not a pre-sync snapshot that will change. |
| Release scope = PREPARE ONLY | 97 | Preserves LOCAL-ONLY posture; repo PUBLIC pending Microsoft minifilter-altitude approval; actual push/publish is operator-gated manual step outside this milestone. |
| D-02 confirmed: Cluster C (9b37dc52) structural no-op | 95-03 | Upstream CredentialProxyIntent refactor is structurally incompatible with fork's flat ProxyLaunchOptions; no code change; Phase 89 || !prepared.custom_credentials.is_empty() active predicate preserved |
| WR-01 gap closed: dynamic errno lost in post-fork static message | 95-06 | format!() heap allocation is unsafe in post-fork child; message type sufficient for operator diagnosis; errno inaccessible safely post-fork |
| CR-01 gap closed: evaluate() placed AFTER endpoint_rules check (additive) | 95-06 | Preserves backward compat for legacy routes while enforcing explicit deny rules; compile() wraps endpoint_rules as allow entries with deny-default |
| Cross-target clippy gate PARTIAL→CI (both Linux and macOS) | 95-07 | Rust targets installed; aws-lc-sys/ring require C cross-linker (x86_64-linux-gnu-gcc) absent; Docker Desktop not running; WSL absent; failure is C toolchain missing, not Rust clippy error in changed files; GH Actions decisive on HEAD be42a5af; Phase 96 resolution target |
| linux-gnu cross clippy gate GREEN locally (exit 0) — PARTIAL→CI retired for linux-gnu | 96-01 | `cross clippy` in pinned image `ghcr.io/cross-rs/x86_64-unknown-linux-gnu:0.2.5@sha256:9e5b39c0...`; first local run surfaced COMPILE errors (not lints) in cfg(linux) code: Phase 95 absorb of upstream ae77d198 (#1210) silently dropped fork invariants — SEC-01 AF_UNIX no-grant static-EPERM filter + cgroup v2 resource-enforcement module — and left stale audit/approval call sites. Restored verbatim from ae77d198^; aligned stale sites to converged API. All structural, no silencing allows. Native clippy+fmt still green. Windows clippy is structurally blind to cfg(linux) drift — this is the gate's whole value. |
| apple-darwin cross clippy gate LOCAL-RUNNABLE (exit 0) — PARTIAL→CI retired for apple-darwin too | 96-02 | zig 0.16.0 + cargo-zigbuild 0.23.0 (host installs); ONE bounded `cargo-zigbuild clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used` exited 0 with SDKROOT UNSET and no SDK extraction. The expected D-04(b) aws-lc-sys SDK-licensing wall did NOT materialize — zig's bundled macOS C target support satisfied the `aws-lc-sys 0.41.0`/`ring 0.17.14` build-dep probe (assumption A3 favorable branch). Working invocation is the direct-binary `cargo-zigbuild clippy …` form (NOT `cargo zigbuild clippy`, which mis-parses). Both cross-targets now provably local-runnable; XTGT-03 closed via D-04 clean-exit branch (not the hard-blocker). |
| XTGT-04 closed: verification protocol rewritten — both gates LOCAL-RUNNABLE, PARTIAL→CI demoted to documented-runner-failure fallback | 96-03 | Checklist (`.planning/templates/cross-target-verify-checklist.md`) rewritten as single source of truth (D-06): Q2 linux-gnu → `cross clippy` (bare `cargo clippy --target` removed); Q3 apple-darwin flipped to MUST-run-locally via direct-binary `cargo-zigbuild clippy` (SDKROOT unset), per 96-02 record. Auto-default-to-PARTIAL retired per-gate (D-07) — PARTIAL only on a *documented* runner failure (stopped daemon / absent-but-installable tool excluded). Added anti-patterns 5 (default-PARTIAL) + 6 (`cargo zigbuild clippy` mis-parse). CLAUDE.md bullet collapsed to a one-line pointer carrying both commands; security mandate + "Windows `cargo check` not a substitute" preserved. Docs-only, no source changes. |
| 3-crate publish set confirmed sufficient (nono, nono-proxy, nono-cli) | 97-02 | nono-shell-broker appears ONLY under `[target.'cfg(target_os = "windows")'.dev-dependencies]` in nono-cli/Cargo.toml — a version-pinned Windows dev-dep that cargo does not resolve during publish-verify and downstream consumers ignore entirely |
| release.yml homebrew download-url corrected to OscarMackJr/nono | 97-02 | T-97-07 mitigation: fork ships its own release tarball; always-further/nono is an abandoned org; nolabs-ai/nono is upstream — neither is the fork's release repo |
| cargo publish --dry-run PRE_PUBLISH_REGISTRY_BLOCKED for downstream workspace crates | 97-03 | cargo resolves deps from live crates.io index at package time; nono-proxy/shell-broker/cli exit 101 until nono 0.66.0 is published; only nono is always-runnable; downstream crates re-run after nono publish |
| nono-py RouteConfig missing endpoint_policy (hard blocker for PyPI release) | 97-03 | maturin build exits 1: endpoint_policy field added to nono-proxy RouteConfig in phase 95 absorb but nono-py src/policy.rs:743 and src/proxy.rs:206 were not updated; fix = add `endpoint_policy: None,` to both initializers |
| Release-readiness gate: policy violations return FAIL verdict; infrastructure failures throw | 97-04 | Enforces T-97-11/12/13 threat model — private-path leak is a FAIL verdict (operator can diagnose), not a harness error; command-not-found is a throw (harness-internal error, exit 4) |
| Publish set reconciled to 3 crates (nono → nono-proxy → nono-cli) — WR-02 | 97-04/close | Code review + verifier flagged a divergence: release.yml published 3 crates but the dry-run + runbook specified 4 (adding nono-shell-broker). Operator chose the 3-crate set — nono-shell-broker marked `publish=false` (internal Windows broker, ships inside the MSI, bin-only crate / no lib target), dropped from release-dry-run.ps1 + RELEASE-RUNBOOK.md; now consistent with release.yml's existing publish-crates job. Build clean; release-readiness gate re-PASS. Supersedes the 97-04 "4-crate set" entry. |

### Key Decisions (v3.5 roadmap)

| Decision | Phase | Rationale |
|----------|-------|-----------|
| Full go-live EXECUTE posture (operator-in-loop), not prepare-only | all | The Azure VM removes the clean-host access blocker (operator's corporate host is contaminated: POC cert imported + VC++ installed + corporate proxy/EDR/managed-trust confounds the chain-build/revocation failure being debugged). Milestone closes only when binaries are Verified-publisher + broker spawns on the VM. |
| Verify-gate debugging is CI-side (GitHub `windows-latest`), not user-host | all | Smoke test + `signtool /pa` re-verify run on GitHub's clean cloud runner; only the behavioral install/broker-spawn/no-VC++ tests need the Azure VM. Fork's autonomous deliverable = CI verify hardening + Azure-VM IaC + scripted gates. |
| Ship reproducible Azure-VM IaC (`az`/Bicep) + scripted `verify-dark.ps1` gates | — | Operator selected "Ship IaC + scripted gate" over runbook-only: a fresh Win11 VM (no POC cert, no VC++) stood up by script + unattended gates the operator runs over RDP, emitting machine-readable verdicts (`SKIP_HOST_UNAVAILABLE` when no VM). |
| FUT-01 live multi-registry publish IN scope | — | Operator broadened scope: actually publish `0.66.1` to crates.io + PyPI + npm in-milestone (v3.4 was PREPARE-ONLY). |
| 7 phases (101-107), spine 101→104→105/106→107, with 102 and 103 parallelizable | roadmap | Registry-ownership question is pre-resolved by the PUB-01 rename decision (fork publishes under owned `nono-sandbox` names, no upstream co-owner-grant preflight needed) — so the research's suggested "Phase 1: registry preflight" collapses into Phase 102 (the rename phase) rather than a separate go/no-go gate. Verify-gate hardening (101) must land before the release cut (104, "do not cut a release until smoke is green"); the rename (102) is independent of signing and parallelizable; Azure IaC + gates (103) reuse 101's shared verify helper but are otherwise independent infra work; live publish (105) needs both the rename (102) and a real release (104); clean-host UAT (106) needs the real release (104) but NOT the live registry publish (105) — the GitHub Release artifact is sufficient; close-out (107) is strictly gated on 106 PASS, never merely on 104 green (Pitfall 10). |
| CHOST-03 does not depend on PUB-02 | roadmap | Architecture research's suggested build order has Phase 8 (VM UAT) depend on Phases 2/3/6 (IaC, gates, release cut) but not Phase 7 (publish) — the VM stages GitHub Release artifacts directly, not registry-installed packages, so live publish is not a precondition for clean-host UAT. |
| Operator-in-loop checkpoints explicitly annotated per success criterion (not just per phase) | roadmap | Phases 104-107 each mix autonomous prep with operator-only actions (Azure profile config, tag push, registry publish commands, RDP session, secret deletion) — annotating at the success-criterion level (not phase level) keeps `/gsd:plan-phase` from mis-scoping an operator action as executor-automatable. |

*Further v3.5 decisions populated as phases complete.*

### Key Decisions (v3.4 roadmap — historical)

| Decision | Phase | Rationale |
|----------|-------|-----------|
| 3 phases (98-100), drain-then-sync shape | all | Audit (98) gates absorb (99), absorb completes before version bump (100) — same dependency-inversion protection as v3.3. #1225 HIGH-CONFLICT disposition settled in 98 before any cherry-pick begins. |
| #1225 ADR deferred to Phase 98 | 98 | The NetworkIntent refactor touches the core network-capability enum the Windows backends key off — adopt-vs-diverge decision must be deliberate and documented, not decided mid-absorb. Mirrors the v3.1 Phase 86 precedent. |
| Release reconcile in Phase 100, after sync | 100 | Version bump to 0.66.1 must follow the sync to avoid a dirty Cargo.lock during cherry-picks; PyPI blocker fixed in the same phase since it gates the release. |
| UPST11 window: 14 substantive + 6 noise = 20 total; 8 clusters A-H | 98-01 | D-12 closed: empirical accounting from drift tool (14) + git log enumeration (6 noise) = 20 total non-merge commits in v0.65.1..v0.66.0 |
| #1225 NetworkIntent refactor is CLI-side ONLY — no core library changes | 98-01 | actual-diff of 72bcfd66 shows 11 files all under crates/nono-cli/src/; preliminary 260629-toe scan suggested library touches but upstream chose CLI-side approach; conflict is fork CLI vs upstream CLI, not library-boundary issue |
| Cluster B (4 tool-sandbox commits) won't-sync: fork lacks tool-sandbox/ dir | 98-01 | Fork skipped tool-sandbox feature in Phase 94/95 Cluster B; all 4 commits (691e0f4f, 7011bc85, d2252225, 853d5236) patch absent files; carry-forward if feature adopted |
| Cluster A windows-touch=yes confirmed: 7 of 11 #1225-touched files have cfg(windows) | 98-01 | grep confirms capability_ext.rs, supervised_runtime.rs, command_runtime.rs, execution_runtime.rs, launch_runtime.rs, output.rs, terminal_approval.rs; cross-target clippy REQUIRED in Phase 99 if adopted |
| #1225 actual-diff confirmed CLI-side only; Cluster E verify→will-sync; ADR-98 sole Phase 99 blocking gate | 98-02 | git show on all 14 commits: no core library changes in window; c808f000 D-03 re-confirmed as will-sync (mechanical nolabs-ai URL update; package namespace identifiers preserved); per-cluster ADR risk matrix complete (Cluster A=H dominant; no TBD cells) |
| ADR-98 Decision: full-sync-adopt upstream #1225 NetworkIntent refactor (Option A) | 98-03 | CLI-only refactor; ADR-86 library boundary non-regressed (NetworkMode::ProxyOnly in crates/nono/src/ untouched); Windows WFP/AppContainer non-regressed (exec_strategy_windows/network.rs reads enforcement-time WindowsNetworkPolicyMode::ProxyOnly, not CLI intent type); Phase 86 convergence precedent applies; Phase 99 deviations: WSL2ProxyFallback preservation + CompiledEndpointPolicy compatibility in proxy_runtime.rs |
| Carve-out re-touch check finalized: 2 CLEAN, 3 HIT (with guard tests), 1 additive-HIT | 98-04 | CR-02+v3.2-override=clean; Cluster-F+endpoint-policy=HIT-expected-conflict-preserve-fork-expression; linux.rs=HIT-additive-only; guard tests named for all HITs; ADR-98 cross-referenced in endpoint-policy subsection (highest re-touch exposure) |
| Completeness sweep PASS: 14+6=20; no TBD; six verdicts; 0.66.1 floor anchored | 98-04 | All five assertions PASS; ledger is complete; Phase 99 gate document ready |
| Cluster D (5b8e94da): 9P warning via statfs/V9FS_MAGIC additive in linux.rs; fork GPU tests preserved | 99-04 | is_9p_path detection (statfs probe, V9FS_MAGIC 0x01021997) inserted before Landlock capability-add loop; warnings deduplicated per mount device ID; fork GPU tests unaffected |
| Cluster E (c808f000): always-further/nono→nolabs-ai GitHub URLs in 3 src/ files; package identifiers and production URLs preserved | 99-04 | profile/mod.rs, proxy_runtime.rs, route.rs had no matching lines in fork (test fixtures diverged); always-further/claude registry identifiers preserved; OscarMackJr/nono fork identity preserved |
| Cluster F (2e64798d): sigstore-trust-root =0.9.0 pinned; dual-version cascade (resolver kept sigstore-verify 0.8.0 + trust-root 0.8.0 for sigstore-verify; new trust-root 0.9.0 direct dep for nono); make build GREEN; cargo audit clean | 99-05 | sigstore-verify bump (upstream 9e084cbb) deferred to future sync phase; assumption A1 confirmed favorable: dual-version resolution is stable; no HIGH/CRITICAL advisories |
| Cluster G (a4d68189): X-Nono-Token stale claim removed from proxy README and token.rs; cli_bootstrap.rs deprecated flag warnings now 3-tuple with remove_by field (--proxy-credential → Will be removed in v1.0.0) | 99-05 | cli.rs ALIAS annotation absent in fork — skipped; behavioral change fully captured in cli_bootstrap.rs; 176/176 nono-proxy tests pass |
| RLS-10 leapfrog to 0.66.1: full 6-member set bumped (D-03 correction of stale 5-crate text) | 100-01 | nono-fltmgr-client IS one of the gate's 6 tracked crates and was included; 6 [package] versions + 6 internal path-dep pins → 0.66.1; tools/sign-fixture (0.1.0) + root repository/homepage (always-further/nono, Deferred Idea) untouched; Cargo.lock regenerated via `cargo build --workspace --all-targets` with exactly 6 workspace-member hunks, zero third-party drift; clippy/fmt/audit/nono+nono-ffi tests green |
| make ci not fully green on dev host — nono-cli test leg has 11 pre-existing env-state failures, NOT this plan's doing | 100-01 | Zero source files changed (Cargo.toml×6 + Cargo.lock only); failures are leftover dev-host state (my-agent.json dated May 26 → profile_cmd "already exists"; env-lock PoisonError cascade; 17-vs-1 session dirs) — documented in memory nono_cli_windows_baseline_test_failures.md as baseline; out of scope per deviation SCOPE BOUNDARY |
| ADR-100: #1245/#1251 ADAPTED not adopted verbatim — publish-crates idempotency clean 1:1 port; cross-compile job trigger rewritten from dead upstream 'chore: release v...' PR-title convention to workflow_dispatch | 100-02 | Fork's release model is manual tag-push/workflow_dispatch (release.yml), never a release-please-bot PR — verbatim adoption would add a permanently-false if: condition (silent false-assurance gap); #1251's pre-corrected quoted-string if: form applied inline from the start; release-readiness verify-dark gate + signed-MSI sign-before-harvest order (D-06) confirmed untouched by either hunk |
| Cross-repo binding bump finished 0.66.0→0.66.1 in both nono-py + nono-ts, one DCO-signed commit each (nono-py 84e8f18, nono-ts dd7d416); no 0.66.0 commit ever landed | 100-03 | D-01/D-02/D-07: both repos already had stale uncommitted 0.66.0 edits from a prior session — finished in place rather than starting from committed HEAD; publish stayed operator-gated (maturin build + npm publish --dry-run only, no push) |
| nono-py RouteConfig PyPI blocker (RLS-12) closed: endpoint_policy: None added at both RouteConfig construction sites (src/proxy.rs, src/policy.rs); Rule 3 auto-fix also added ProxyConfig.enable_h2: false (same-class binding drift, only surfaced by actually running maturin build, not by static grep) | 100-03 | maturin build now exits 0; both fields are no-op stubs matching nono-proxy's own defaults, not policy/business-logic changes; full endpoint_policy threading deferred to a named future phase per D-08 |
| RLS-13 closed: release-readiness gate re-pointed targetVersion=0.66.1/upstreamHighest=0.66.0; both mandatory pre-push gates re-run GREEN; RELEASE-RUNBOOK.md fully updated for the 0.66.1 tag | 100-04 | Rule 1 auto-fix: gate assertion (e) hardcoded a stale '0.66.0' Cargo.lock literal instead of referencing $targetVersion — Cargo.lock now contains zero 0.66.0 occurrences (all 6 crates at 0.66.1), so the old check would have false-FAILed the very re-green this plan exists to deliver; rewritten to reference $targetVersion |
| release-dry-run.ps1 twine-detection bug fixed: false-positived on Python's own "No module named twine" error text (contains the substring "twine") instead of checking $LASTEXITCODE, masking a real toolchain-absent SKIP as a hard FAIL | 100-04 | Discovered on first re-run after the version bump (pypi.twine_check FAILed with exit 1); switched detection to $LASTEXITCODE; re-run now correctly SKIPs with SKIP_HOST_UNAVAILABLE and the overall dry-run exits 0 with zero FAIL keys |
| Checkpoint clean-host-install gate verdict pre-resolved by orchestrator; SKIP_HOST_UNAVAILABLE/exit 3 operator-accepted ("Accept SKIP & close phase") and recorded verbatim on the msi-vcredist-prereq todo rather than re-run | 100-05 | Plan's `<how-to-verify>` treats SKIP_HOST_UNAVAILABLE/exit 3 as the expected, acceptable outcome per CONTEXT.md D-09; re-running would have been redundant |
| Neither host-gated todo marked resolved: msi-vcredist-prereq stays open pending a genuine clean Win11 VM run; poc-cert-broker-clean-host stays open pending both an actual 0.66.1 tag push (operator-gated) and a fix to the Azure Trusted Signing verify-gate UnknownError (quick 260630-trusted-signing-golive) | 100-05 | Fold-without-resolve pattern (mirrors v3.1 Phase 90 DRAIN) — a todo can be formally documented at milestone close without claiming false completion |

### Pending Todos

Two host-gated distribution todos are IN SCOPE for v3.5 (CHOST-03 / FUT-03 drain), to be run on the Azure Win11 VM (Phase 106):

- **`20260611-poc-cert-broker-clean-host.md`** — after a trusted-signed `0.66.1` release, confirm `nono run --profile claude-code` spawns the broker on a clean host with NO manual cert-trust step. Blocked until (1) the verify-gate `UnknownError` is fixed (Phase 101) and (2) a real trusted-signed `0.66.1` is cut (Phase 104).
- **`20260611-msi-vcredist-prereq.md`** — confirm the machine MSI installs on a fresh Win11 host with no VC++ redist (code fix `+crt-static` already shipped/verified; only the clean-VM empirical confirmation remains).

### Blockers/Concerns

- **Azure Trusted Signing verify-gate `UnknownError` (v3.5 headline blocker — Phase 101 target):** first live smoke run (`28467925298`, 2026-06-30) — OIDC login + Sign succeeded (signer `CN=TWGGLOBAL.onmicrosoft.com`) but Verify FAILED `Status: UnknownError`, issuer `CN=Microsoft Enterprise ID Verified Policy AOC CA 01`. Two causes to settle: (1) confirm the profile is **Public Trust** (public chain reads `Microsoft ID Verified CS EOC/AOC CA NN`, not `…Enterprise ID Verified Policy AOC CA…`); (2) `UnknownError` ≠ `UntrustedRoot` → chain couldn't be *built* on `windows-latest` (AOC intermediate absent / CRL-OCSP timeout) — re-verify with `signtool verify /pa /v`. `release.yml:259` uses the same fail-closed `-ne 'Valid'` check, so a release tag aborts at verify until this is `Valid`. Runbook: `.planning/quick/260630-trusted-signing-golive/AZURE-TRUSTED-SIGNING-GOLIVE-COOKBOOK.md`. See [[azure_trusted_signing_golive]].
- **Correct GitHub OIDC FIC subject** (both workflows): `repo:OscarMackJr/nono:environment:Development` — the old `260603-i31` cookbook's `oscarmackjr-twg` + `ref:` subject is STALE and causes `AADSTS700213`.
- **Operator's corporate host is NOT a valid clean host** — POC cert previously imported + VC++ installed + corporate proxy/EDR/managed-trust-store confound the exact chain-build/revocation failure being debugged. Clean-host UAT must run on a fresh Azure Win11 VM (off corporate LAN, clean revocation egress, never trusted the POC cert, no VC++) — Phase 106.
- **Base crate `0.66.1` already prepared (v3.4)** — all 6 workspace crates + both binding repos at `0.66.1`; both mandatory pre-push gates GREEN; RELEASE-RUNBOOK.md current. v3.5 executes the actual go-live from this prepared state. Do NOT publish `0.66.0`.
- **Repo stays PUBLIC** (minifilter altitude 377813.5 received 2026-07-01; go-private retired). Verify no `build_notes/`/`.gsd/` staged before any push. Unlike prior prepare-only tags, v3.5's tag push is *intended* (go-live).
- **All commits DCO-signed**: `Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>` on every commit.
- **Cross-target clippy MUST be GREEN** if any cfg-gated Unix code is touched: Docker `cross` (linux-gnu) + zig `cargo-zigbuild` (apple-darwin) exit 0 locally — PARTIAL→CI retired (v3.3 Phase 96). (v3.5 is primarily CI-yaml + PowerShell + IaC; likely low Unix-cfg exposure, but the rule stands.)
- **Registry ownership pre-resolved, not a live blocker** — crates.io `nono`/`nono-proxy`, PyPI `nono-py`, npm `nono-ts` are upstream-owned; the fork publishes under fork-owned `nono-sandbox` family names instead of requesting a co-owner grant (PUB-01, Phase 102). Each new name's availability must still be confirmed live before committing to the rename.
- **CLOSE-01 secret-retirement ordering is a structural gate, not a checklist item** — Phase 107 must NOT run until Phase 106 (CHOST-03) has a passed verdict; retiring POC secrets after only Phase 104 (release green) leaves no fallback signing path if clean-host UAT reveals a problem invisible on CI runners.

### Quick Tasks Completed

| # | Description | Date | Commit | Directory |
|---|-------------|------|--------|-----------|
| 260624-p1c | Cargo Audit: bump quinn-proto past RUSTSEC-2026-0185 (remote memory exhaustion) | 2026-06-24 | 78b50f04 | [260624-p1c-cargo-audit-bump-quinn-proto-past-rustse](./quick/260624-p1c-cargo-audit-bump-quinn-proto-past-rustse/) |
| 260624-q98 | Remove orphan audit_ledger.rs + dead state_paths helpers (never compiled) | 2026-06-24 | e350df23 | [260624-q98-remove-orphan-audit-ledger-rs-and-its-de](./quick/260624-q98-remove-orphan-audit-ledger-rs-and-its-de/) |
| 260624-q9j | Fix red Docs Checks: force-add already-in-nav windows-win-1706-option-1-workstream.mdx | 2026-06-24 | 3475b470 | [260624-q9j-exclude-docs-cli-development-from-docs-c](./quick/260624-q9j-exclude-docs-cli-development-from-docs-c/) |
| 260625-crs | Phase 83 deferred code-review findings: WR-02/03/04/05 + IN-01/IN-03 (interpreter PATH-hijack, GetWindowsDirectoryW, canonical expander, validate(), gate probe, SID regex) | 2026-06-25 | 4af1e8f9 | [260625-crs-address-phase-83-code-review-deferred-fi](./quick/260625-crs-address-phase-83-code-review-deferred-fi/) |
| 260629-toe | UPST11 v0.66.0 parity phase definition: reviewed upstream PR #1293, authored divergence ledger for v0.65.1→v0.66.0 (19 PRs) + fork-invariant gates + 3-wave structure; #1225 NetworkIntent flagged HIGH-CONFLICT; 0.66.0 version collision → next release ≥0.67.0 | 2026-06-29 | (docs) | [260629-toe-v066-parity](./quick/260629-toe-v066-parity/) |
| 20260701-fltmgr-altitude | Wire official Microsoft altitude 377813.5 for nono-fltmgr.sys: live artifacts (INF/DESIGN/README) already done in 2041fc62; reconciled ADR-65 §5 amendment + STATE approval-RECEIVED. PUBLIC→PRIVATE flip flagged as operator decision (un-taken) | 2026-07-01 | e8ea5508 | [20260701-fltmgr-altitude-assignment](./quick/20260701-fltmgr-altitude-assignment/) |
| 260630-trusted-signing-golive | Live smoke run `28467925298` on the Azure Trusted Signing path — OIDC login + Sign succeeded, Verify FAILED `UnknownError`; authored the go-live cookbook (`AZURE-TRUSTED-SIGNING-GOLIVE-COOKBOOK.md`) that seeds this milestone's Phase 101/104 gates | 2026-06-30 | (docs) | [260630-trusted-signing-golive](./quick/260630-trusted-signing-golive/) |

## Deferred Items

Items acknowledged and deferred at **v3.2 close (2026-06-23)** — `gsd-sdk query audit-open` reported 47 open artifacts, user acknowledged-all. All historical or host-gated; none blockers:

| Category | Item | Status | Deferred At |
|----------|------|--------|-------------|
| Historical | 36 open quick-tasks (Mar–Apr 2026 dates, all `missing`/cleaned-up) | Acknowledged | v3.2 close |
| Historical | 6 seeds SEED-001…006 (all consumed or dormant; SEED-005 = v3.2 scope, delivered) | Acknowledged | v3.2 close |
| Historical | 4 empty/"None" todo parse artifacts | Acknowledged | v3.2 close |
| Host-gated | OVERRIDE-02 (DF-02) live allow/revoke proof — needs ZT-Infra provisioner + openssl + elevated session; SKIP_HOST_UNAVAILABLE by design | Open (host-gated) | v3.2 close |
| PARTIAL→CI | Cross-target clippy (linux-gnu + apple-darwin) for ZTL-04 `AWS_*` strip | Open (CI-decisive; may resolve in Phase 96) | v3.2 close |

Prior carry-forwards from v3.1 close (2026-06-21): SEC-01/SEC-02 AF_UNIX+procfs guards (PARTIAL→CI), DRAIN-01/02/03 live host-gated UAT, 2 env-sensitive Phase-74 DACL-guard tests.

Items acknowledged and deferred at **v3.3 close (2026-06-26)** — `gsd-sdk query audit-open` reported 48 open artifacts (after resolving both HUMAN-UAT gaps), user acknowledged-all. All historical or future-scoped; none v3.3 blockers:

| Category | Item | Status | Deferred At |
|----------|------|--------|-------------|
| Historical | 40 open quick-tasks (Mar–Apr 2026 dates, all `missing`/cleaned-up) | Acknowledged | v3.3 close |
| Historical | 6 seeds SEED-001…006 (all consumed; SEED-006 = UPST9 window, delivered v3.1) | Acknowledged | v3.3 close |
| Future-scoped | 2 todos FUT-02 (poc-cert-broker clean-host) + FUT-03 (msi-vcredist-prereq) — Azure Trusted Signing distribution milestone | Acknowledged | v3.3 close |

**Resolved at v3.3 close (not deferred):** 95-HUMAN-UAT cross-target clippy CI-lane confirmation → superseded by Phase 96 local cross-target toolchain (both gates GREEN locally); 97-HUMAN-UAT WR-02 publish-set divergence → operator chose the 3-crate set, reconciled. The v3.1/v3.2 PARTIAL→CI cross-target carry-forwards are likewise retired — Phase 96 made both Unix clippy gates locally runnable.

**Operator-action carry-forwards into v3.4 (PREPARE-ONLY — outside any prior milestone):** (1) nono-py `RouteConfig` missing `endpoint_policy` field (`src/policy.rs:743` + `src/proxy.rs:206`) — **CLOSED 2026-07-02, Phase 100 Plan 03 (RLS-12)**: `endpoint_policy: None` added at both sites, `maturin build` exits 0; (2) downstream crate `cargo publish --dry-run` stays PRE_PUBLISH_REGISTRY_BLOCKED until `nono 0.66.1` is on crates.io; (3) follow updated RELEASE-RUNBOOK.md (at 0.66.1) for the tag push + publish.

Items acknowledged and deferred at **v3.4 close (2026-07-02)** — `gsd-sdk query audit-open` reported 50 open artifacts, user acknowledged-all. All historical, dormant, or host-gated; none v3.4 blockers:

| Category | Item | Status | Deferred At |
|----------|------|--------|-------------|
| Historical | 41 open quick-tasks (Mar–Apr 2026 dates, all `missing`/cleaned-up) | Acknowledged | v3.4 close |
| Dormant | 6 seeds SEED-001…006 (all consumed/dormant; delivered across prior milestones) | Acknowledged | v3.4 close |
| Host-gated | 2 todos — `msi-vcredist-prereq` (clean Win11 VM MSI install) + `poc-cert-broker-clean-host` (trusted-signed release + broker spawns out-of-box) — FUT-02/03 Azure Trusted Signing distribution; folded-without-resolve in Plan 100-05; both now IN SCOPE for v3.5 CHOST-03 (Phase 106) | In progress (v3.5 Phase 106) | v3.4 close |
| Product-decision | `99-HUMAN-UAT` partial (3 open scenarios): WR-01 unused `sigstore-trust-root 0.9.0` direct-dep pin (keep-vs-remove), WR-03 `validate_block_net_conflicts` vs strict-filter semantics (fail-closed today, consistency not vuln), WR-02 `--allow-http2` runtime no-op (`UpstreamPool` absorbed but not yet wired — advertise-or-wire before H2 release notes) | Open (non-blocking) | v3.4 close |

**Blocked-external (now the v3.5 headline target, not merely tracked):** the signed `0.66.1` release was blocked on the open Azure Trusted Signing verify-gate `UnknownError` (quick `260630-trusted-signing-golive`) — signing works, verify fails; v3.5 Phase 101 exists specifically to resolve this before Phase 104 cuts the release.

## Session Continuity

Last session: 2026-07-02T18:38:58.475Z
Stopped at: Phase 101 context gathered
Resume file: .planning/phases/101-verify-gate-hardening-azure-profile-confirmation/101-CONTEXT.md

## Operator Next Steps

- v3.5 roadmap approved 2026-07-02 — 7 phases (101-107), 10/10 requirements mapped, no orphans. Next: `/gsd:plan-phase 101` (or `/gsd:discuss-phase 101`) to begin Phase 101 (Verify-Gate Hardening + Azure Profile Confirmation).

</content>
