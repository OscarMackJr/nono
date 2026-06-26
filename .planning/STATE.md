---
gsd_state_version: 1.0
milestone: v3.3
milestone_name: UPST10 Upstream Sync (v0.64→v0.65.1) + First Real Release
status: Awaiting next milestone
stopped_at: Completed 97-03-PLAN.md
last_updated: "2026-06-26T18:32:29.578Z"
last_activity: 2026-06-26 — Milestone v3.3 completed and archived
progress:
  total_phases: 4
  completed_phases: 4
  total_plans: 16
  completed_plans: 16
  percent: 100
---

# Project State: nono — v3.3 UPST10 Upstream Sync (v0.64→v0.65.1) + First Real Release

## Project Reference

See: `.planning/PROJECT.md` (v3.3 milestone active 2026-06-25; v3.2 Phases 91-93 complete + archived; tag `v3.2` local). Phase numbering continues from Phase 93 (Phases 94-97 — NOT reset). Roadmap: `.planning/ROADMAP.md`. Requirements: `.planning/REQUIREMENTS.md`.

**Core Value:** Windows security must be as structurally impossible and feature-complete as Unix platforms. The fork stays current with upstream without regressing its Windows security model — and is, for the first time, genuinely releasable: a gated, signed, multi-registry pipeline prepared GREEN for a one-step operator push.

**Current Focus:** Milestone v3.3 complete + archived 2026-06-26 (tag `v3.3` local). Awaiting next milestone — `/gsd-new-milestone`.

## Current Position

Phase: Milestone v3.3 complete
Plan: —
Status: Awaiting next milestone
Last activity: 2026-06-26 — Milestone v3.3 completed and archived

## Performance Metrics

**Velocity:** (v3.3 — reset; populated as phases complete)

- Total plans completed: 16
- Average duration: —
- Total execution time: —

| Phase | Plan | Duration | Tasks | Files |
|-------|------|----------|-------|-------|

*Updated after each plan completion*
| Phase 95-upstream-absorb-fork-invariant-verify P01 | 180 | 2 tasks | 7 files |
| Phase 95-upstream-absorb-fork-invariant-verify P04 | 30 | 2 tasks | 2 files |
| Phase 95 P05 | 8 | 2 tasks | 1 files |
| Phase 95-upstream-absorb-fork-invariant-verify P06 | 18 | 2 tasks | 3 files |
| Phase 95 P07 | 30 | 1 task | 0 source files (verification gate) |
| Phase 96 P01 | 26 | 2 tasks | 3 files |
| Phase 96-cross-target-toolchain P02 | 14 | 2 tasks | 1 files |
| Phase 96-cross-target-toolchain P03 | 2 | 2 tasks | 2 files |
| Phase 97 P01 | 6 | 2 tasks | 7 files |
| Phase 97 P02 | 15 | 2 tasks | 2 files |
| Phase 97 P03 | 35 | 2 tasks | 1 files |
| Phase 97 P04 | 3 | 2 tasks | 2 files |

## Accumulated Context

### Key Decisions (v3.3 roadmap)

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

### Pending Todos

None yet.

### Blockers/Concerns

- **Repo stays PUBLIC**: verify no `build_notes/` or `.gsd/` files staged before any `git push` (minifilter-altitude approval pending). All tags remain LOCAL ONLY; push is operator-gated.
- **Upstream relocated**: canonical upstream is now `nolabs-ai/nono` (was `always-further/nono`); Phase 94 updates the remote and PROJECT.md.
- **Cross-target clippy**: XTGT-03 (apple-darwin) explicitly allows a documented hard-blocker outcome if osxcross/SDK is infeasible from Windows. Phase 96 resolves the outcome either way.
- **Cross-repo release**: nono-py at `../nono-py`, nono-ts at `../nono-ts`. Phase 97 version bump must touch both sibling repos.
- **PARTIAL→CI carry-forwards**: SEC-01/SEC-02 (v3.1), ZTL-04 AWS_* strip (v3.2) — still PARTIAL→CI; Phase 96 may resolve if linux-gnu toolchain clears them.
- **All commits DCO-signed**: `Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>` required on every commit including cherry-picks (use `-x` + manual DCO trailer).
- **nono-py PyPI blocker (97-03)**: `maturin build` exits 1 — nono-py `src/policy.rs:743` and `src/proxy.rs:206` are missing `endpoint_policy: None,` in `RouteConfig` struct initializers. Must be fixed in nono-py repo before the actual PyPI release.

### Quick Tasks Completed

| # | Description | Date | Commit | Directory |
|---|-------------|------|--------|-----------|
| 260624-p1c | Cargo Audit: bump quinn-proto past RUSTSEC-2026-0185 (remote memory exhaustion) | 2026-06-24 | 78b50f04 | [260624-p1c-cargo-audit-bump-quinn-proto-past-rustse](./quick/260624-p1c-cargo-audit-bump-quinn-proto-past-rustse/) |
| 260624-q98 | Remove orphan audit_ledger.rs + dead state_paths helpers (never compiled) | 2026-06-24 | e350df23 | [260624-q98-remove-orphan-audit-ledger-rs-and-its-de](./quick/260624-q98-remove-orphan-audit-ledger-rs-and-its-de/) |
| 260624-q9j | Fix red Docs Checks: force-add already-in-nav windows-win-1706-option-1-workstream.mdx | 2026-06-24 | 3475b470 | [260624-q9j-exclude-docs-cli-development-from-docs-c](./quick/260624-q9j-exclude-docs-cli-development-from-docs-c/) |
| 260625-crs | Phase 83 deferred code-review findings: WR-02/03/04/05 + IN-01/IN-03 (interpreter PATH-hijack, GetWindowsDirectoryW, canonical expander, validate(), gate probe, SID regex) | 2026-06-25 | 4af1e8f9 | [260625-crs-address-phase-83-code-review-deferred-fi](./quick/260625-crs-address-phase-83-code-review-deferred-fi/) |

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

**Operator-action carry-forwards (PREPARE-ONLY, outside any milestone):** (1) nono-py `RouteConfig` missing `endpoint_policy` field (`src/policy.rs:743` + `src/proxy.rs:206`) — fix before PyPI publish; (2) commit the v0.66.0 version bumps in sibling repos nono-py/nono-ts; (3) downstream crate `cargo publish --dry-run` stays PRE_PUBLISH_REGISTRY_BLOCKED until `nono 0.66.0` is on crates.io; (4) follow RELEASE-RUNBOOK.md for the tag push + publish.

## Session Continuity

Last session: 2026-06-26 — v3.3 milestone completion (all 4 phases verified, archived, tagged local)
Stopped at: Milestone v3.3 complete + archived
Resume file: None

## Operator Next Steps

- Start the next milestone with /gsd-new-milestone
