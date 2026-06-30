---
gsd_state_version: 1.0
milestone: v3.4
milestone_name: UPST11 Upstream Sync to v0.66.0 + Release-Reconcile
status: active
last_updated: "2026-06-30T01:50:13.352Z"
last_activity: 2026-06-30
progress:
  total_phases: 3
  completed_phases: 0
  total_plans: 0
  completed_plans: 0
  percent: 0
---

# Project State: nono — v3.4 UPST11 Upstream Sync to v0.66.0 + Release-Reconcile

## Project Reference

See: `.planning/PROJECT.md` (v3.4 milestone active 2026-06-30; v3.3 Phases 94-97 complete + archived; tag `v3.3` local). Phase numbering continues from Phase 97 (Phases 98-100 — NOT reset). Roadmap: `.planning/ROADMAP.md`. Requirements: `.planning/REQUIREMENTS.md`.

**Core Value:** Windows security must be as structurally impossible and feature-complete as Unix platforms. The fork stays current with upstream without regressing its Windows security model — and turns the v3.3 prepare-only pipeline into a genuinely operator-pushable `0.66.1` release.

**Current Focus:** Milestone v3.4 active 2026-06-30 (roadmap created, Phases 98-100 defined). Phase 98 (UPST11 Divergence Audit) is next.

## Current Position

Phase: 98 — UPST11 Divergence Audit
Plan: —
Status: Not started
Last activity: 2026-06-30 — v3.4 roadmap created (Phases 98-100)

## Performance Metrics

**Velocity:** (v3.4 — reset; populated as phases complete)

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

### Key Decisions (v3.4 roadmap)

| Decision | Phase | Rationale |
|----------|-------|-----------|
| 3 phases (98-100), drain-then-sync shape | all | Audit (98) gates absorb (99), absorb completes before version bump (100) — same dependency-inversion protection as v3.3. #1225 HIGH-CONFLICT disposition settled in 98 before any cherry-pick begins. |
| #1225 ADR deferred to Phase 98 | 98 | The NetworkIntent refactor touches the core network-capability enum the Windows backends key off — adopt-vs-diverge decision must be deliberate and documented, not decided mid-absorb. Mirrors the v3.1 Phase 86 precedent. |
| Release reconcile in Phase 100, after sync | 100 | Version bump to 0.66.1 must follow the sync to avoid a dirty Cargo.lock during cherry-picks; PyPI blocker fixed in the same phase since it gates the release. |

### Pending Todos

None yet.

### Blockers/Concerns

- **Repo stays PUBLIC**: verify no `build_notes/` or `.gsd/` files staged before any `git push` (minifilter-altitude approval pending). All tags remain LOCAL ONLY; push is operator-gated.
- **#1225 HIGH-CONFLICT**: fork has deep `NetworkMode::ProxyOnly` usage; adopt-vs-fork-divergence call deferred to Phase 98 ADR. This item alone justifies the separate audit phase.
- **Cross-target clippy MUST be GREEN**: Docker `cross` (linux-gnu) + zig `cargo-zigbuild` (apple-darwin) must exit 0 locally — PARTIAL→CI is not the default (retired in v3.3 Phase 96). #1225, #1207, #1213, #1249 all touch cfg-gated Unix code.
- **Version collision at 0.66.0**: fork is at crate 0.66.0; upstream also shipped 0.66.0. The fork bumps to 0.66.1 in Phase 100 — do NOT publish 0.66.0 from the fork.
- **nono-py PyPI blocker (RLS-12)**: `maturin build` exits 1 — nono-py `src/policy.rs:743` and `src/proxy.rs:206` missing `endpoint_policy: None,` in `RouteConfig` initializers. Phase 100 closes this.
- **All commits DCO-signed**: `Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>` required on every commit including cherry-picks (use `-x` + manual DCO trailer).
- **Release scope = PREPARE ONLY**: actual tag push + registry publish remain an operator-gated manual step outside this milestone.

### Quick Tasks Completed

| # | Description | Date | Commit | Directory |
|---|-------------|------|--------|-----------|
| 260624-p1c | Cargo Audit: bump quinn-proto past RUSTSEC-2026-0185 (remote memory exhaustion) | 2026-06-24 | 78b50f04 | [260624-p1c-cargo-audit-bump-quinn-proto-past-rustse](./quick/260624-p1c-cargo-audit-bump-quinn-proto-past-rustse/) |
| 260624-q98 | Remove orphan audit_ledger.rs + dead state_paths helpers (never compiled) | 2026-06-24 | e350df23 | [260624-q98-remove-orphan-audit-ledger-rs-and-its-de](./quick/260624-q98-remove-orphan-audit-ledger-rs-and-its-de/) |
| 260624-q9j | Fix red Docs Checks: force-add already-in-nav windows-win-1706-option-1-workstream.mdx | 2026-06-24 | 3475b470 | [260624-q9j-exclude-docs-cli-development-from-docs-c](./quick/260624-q9j-exclude-docs-cli-development-from-docs-c/) |
| 260625-crs | Phase 83 deferred code-review findings: WR-02/03/04/05 + IN-01/IN-03 (interpreter PATH-hijack, GetWindowsDirectoryW, canonical expander, validate(), gate probe, SID regex) | 2026-06-25 | 4af1e8f9 | [260625-crs-address-phase-83-code-review-deferred-fi](./quick/260625-crs-address-phase-83-code-review-deferred-fi/) |
| 260629-toe | UPST11 v0.66.0 parity phase definition: reviewed upstream PR #1293, authored divergence ledger for v0.65.1→v0.66.0 (19 PRs) + fork-invariant gates + 3-wave structure; #1225 NetworkIntent flagged HIGH-CONFLICT; 0.66.0 version collision → next release ≥0.67.0 | 2026-06-29 | (docs) | [260629-toe-v066-parity](./quick/260629-toe-v066-parity/) |

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

**Operator-action carry-forwards into v3.4 (PREPARE-ONLY — outside any prior milestone):** (1) nono-py `RouteConfig` missing `endpoint_policy` field (`src/policy.rs:743` + `src/proxy.rs:206`) — closed by Phase 100 (RLS-12); (2) downstream crate `cargo publish --dry-run` stays PRE_PUBLISH_REGISTRY_BLOCKED until `nono 0.66.0` is on crates.io; (3) follow updated RELEASE-RUNBOOK.md (at 0.66.1) for the tag push + publish.

## Session Continuity

Last session: 2026-06-30 — v3.4 roadmap created (Phases 98-100 defined, requirements mapped)
Stopped at: Roadmap created; Phase 98 ready to plan
Resume file: None

## Operator Next Steps

- Run `/gsd:plan-phase 98` to plan the UPST11 Divergence Audit phase
