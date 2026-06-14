---
gsd_state_version: 1.0
milestone: v2.11
milestone_name: Clean-Host Distribution Cleanup + UPST8
status: milestone_complete
last_updated: "2026-06-13T02:47:12.771Z"
last_activity: 2026-06-13
progress:
  total_phases: 4
  completed_phases: 4
  total_plans: 6
  completed_plans: 6
  percent: 100
---

# Project State: nono — v2.11 Clean-Host Distribution Cleanup + UPST8

## Project Reference

See: `.planning/PROJECT.md` (v2.11 milestone started 2026-06-11; v2.10 shipped + archived 2026-06-11). Phase numbering continues from Phase 66 (Phases 67-70).

**Core Value:** Windows security must be as structurally impossible and feature-complete as Unix platforms; every nono command that works on Linux/macOS should work on Windows with equivalent security guarantees, or be explicitly documented as intentionally unsupported with a clear rationale.

**Current Focus:** Phase 70 complete — UPST8 sync done; awaiting human-verify checkpoint for C2 network policy security; Phases 67 (clean-host Win install) and 68 (macOS resl fix) pending host-gated UAT

## Current Position

Phase: 70
Plan: Not started
Status: Milestone complete
Last activity: 2026-06-13

### v2.11 Phase Summary (active)

| Phase | Goal | Requirements | SC | Status | Host gate |
|-------|------|--------------|----|--------|-----------|
| 67 | Clean-Host Windows Install — machine MSI completes on a fresh Win11 host (VC++ handled, service-start non-fatal); interim auditable broker-trust helper + docs | DIST-01, DIST-02, TRUST-01, TRUST-02 | 4 | ⬜ Not started | Clean Win11 host (no VC++, no pre-trusted cert); production-signed MSI |
| 68 | macOS Resource-Limit Enforcement Fix — `--timeout` + `--max-processes` actually fire on a real macOS host (watchdog + `RLIMIT_NPROC`) | RESL-MAC-01, RESL-MAC-02 | 4 | ⬜ Not started | Real macOS host (`NONO_RESL_HOST_VALIDATED=1`) |
| 69 | UPST8 Audit — DIVERGENCE-LEDGER for the non-macOS slice of upstream `v0.60.0..v0.61.2` | UPST8-01 | 4 | ⬜ Not started | Host-agnostic |
| 70 | UPST8 Cherry-pick Sync — absorb will-sync commits (D-19 trailers, invariants preserved, suite green) | UPST8-02 | 4 | ⬜ Not started | Host-agnostic; cross-target clippy via CI |

**Dependencies:** Phases 67 and 68 are independent and parallel-safe (each host-gated). Phase 69 → 70 is the UPST8 audit-then-sync pair (linear; cadence-ordered after Phase 55, mirroring Phase 54/55).

### Host-availability gates (v2.11)

| Phase | Gate | Notes |
|-------|------|-------|
| 67 | Clean Windows 11 host with NO VC++ x64 runtime and NO pre-trusted nono cert | Must install + run the **production-signed** machine MSI, not a dev-layout binary (the D-32-12 broker trust gate only fires from a signed Program-Files install). Verify clean-uninstall leaves no orphaned WFP filters / service registration. |
| 68 | Real macOS host for `NONO_RESL_HOST_VALIDATED=1` re-validation | `macos_timeout_kills_at_deadline` + `macos_max_processes_blocks_on_rlimit_nproc` must PASS; CI runners cannot validate (they hang — the two tests stay env-gated off the runner). Closes the Phase 65 gate-65-A "A5" finding. |
| 70 | Cross-target clippy (Linux + macOS) | Per `.planning/templates/cross-target-verify-checklist.md`; Windows dev host can't cross-compile (ring/aws-lc-sys C-toolchain) → CI is the load-bearing signal. Phase 68 also touches cfg-gated Unix code → same gate. |

<details>
<summary>v2.10 Phase Summary (shipped 2026-06-11 — historical; archived at `milestones/v2.10-ROADMAP.md`)</summary>

| Phase | Goal | Requirements | Status |
|-------|------|--------------|--------|
| 63 | Minifilter spike groundwork (WDK/VM/design doc) + macOS DIVERGENCE-LEDGER audit | DRV-03 (partial), MACOS-01 | ✅ Complete |
| 64 | Minifilter spike implementation (intercept + deny + IPC roundtrip on test VM) + macOS P1 cherry-pick wave | DRV-01, DRV-02, DRV-03 (complete), MACOS-02 | ✅ Complete |
| 65 | Minifilter go/no-go ADR + macOS live re-validation HUMAN-UAT (CI macOS green — HARD gate) | DRV-04, MACOS-03 | ✅ Complete — D-11c green + latency captured + gate-65-A Seatbelt PASS; **ADR Accepted** `563df1ed` (No-go/Conditional-go); resl A5 macOS-enforcement defect filed → v2.11 (Phase 68) |
| 66 | WR-02 EDR HUMAN-UAT (no new code; real EDR host required) | EDR-01, EDR-02 | ✅ Complete — **WR-02 CLOSED** (validated under Sysmon+Defender EDR-proxy) |

</details>

## Key Decisions

### v2.11 decisions

| Decision | Phase | Rationale |
|----------|-------|-----------|
| DIST + TRUST kept together in one phase (67) | 67 | The MSI install fix and the interim broker-trust path are the same "make the public release work on a clean Win11 host" story; both are exercised in the same clean-host UAT. |
| Real publicly-trusted signing (Azure Trusted Signing) is OUT OF SCOPE | — | BLOCKED on an incoming cert; deferred to the next (enterprise distribution) milestone. v2.11 TRUST reqs are the interim cert-import helper + docs only — never weakening the D-32-12 gate. |
| macOS resl fix is a real nono supervisor/setrlimit bug fix, not a test-gate change | 68 | The Phase 65 A5 finding: the watchdog/`RLIMIT_NPROC` path genuinely doesn't fire on macOS. Test re-gating (the v2.10 CI-green workaround) is NOT the fix. |
| D-01: baseline+N RLIMIT_NPROC bounding — parent reads per-UID count via sysctl(KERN_PROC_UID) before fork | 68 | Matches Linux pids.max intent; accepted UID-wide race (D-03). `uid_process_count()` fails closed on sysctl error. |
| D-04: setpgid(0,0) in supervised child arm — child becomes own pgrp leader | 68 | Fixes timeout watchdog kill miss; watchdog kill(-pgrp) now targets only agent tree. Tolerated on failure (WR-04 skip-on-Err is the safety net). |
| D-10: cross-target clippy PARTIAL/deferred-to-CI for Phase 68 | 68 | Windows dev host cannot cross-compile (ring/aws-lc-sys C toolchain missing); GH Actions Linux + macOS Clippy lanes are the load-bearing signal. |
| UPST8 scoped to the non-macOS slice of `v0.60.0..v0.61.2` only | 69-70 | The macOS slice of that window was already absorbed in v2.10 (Phases 63-65 / MACOS-01..03). |
| UPST8 follows the Phase 54/55 audit-then-sync two-phase shape | 69-70 | Mirrors every prior UPST cycle (33/34, 39/40, 42/43, 47, 54/55); cadence-ordered linearly after Phase 55. |
| D-70-01: UPST8-01 acceptance criteria extended to v0.62.0 upper bound | 70 | Phase 69 DIVERGENCE-LEDGER D-01 found 3 tail commits (v0.61.2..v0.62.0) outside original scope; scope extended |
| cc21229f adapted inline (collect_ignored_denial_paths absent from fork) | 70 | Upstream helper + SandboxArgs::suppress_save_prompt not yet in fork; inlined using existing canonicalize_suppress_path, cfg-gated on non-Windows |
| 20cc5df9 sandbox_state.rs conflict: HEAD side taken for domain_endpoint_state_tests | 70 | Fork's Phase 56 module not in upstream; profile_save_runtime.rs registry-ref feature auto-merged cleanly |
| db073750 D-20 manual replay (C4): execute_with_options deferred, ExecuteOptions added | 70 | Fork's wiring.rs is yaml_merge system; upstream's WriteFile execute system not yet ported (v2.5-FU-3); ExecuteOptions used from install_package for forward-compat API |

### v2.10 decisions

| Decision | Phase | Rationale |
|----------|-------|-----------|
| Minifilter driver is C/C++ MSBuild in `drivers/nono-fltmgr/`, NOT a Cargo crate | 63 | `windows-drivers-rs` is early-stage/KMDF-only; WDK requires C/C++ |
| Spike driver NOT bundled in the MSI; `nono-wfp-driver.sys` placeholder unchanged | 63 | Test-signed driver requires TESTSIGNING ON; safe only on dedicated test VM |
| Dedicated `\NonoPolicyPort` FilterCommunicationPort; does NOT reuse WFP named pipe | 63-64 | FltMgr IPC is synchronous kernel APC-based; cannot layer on tokio async pipe |
| `FltSendMessage` finite timeout (fail-open on timeout) for the spike | 63 | Prevents system hang if supervisor not running; production ADR decides fail-direction |
| Altitude in Activity-Monitor/FSFilter range (NOT AV range 320000-329998) | 63 | AV-range altitude risks EDR driver disruption and altitude collision |
| macOS CI build leg green is HARD close gate for Phase 65 | 65 | v2.9 shipped two cfg-gated compile errors on release tags because Windows host never compiles macOS branches |
| EDR UAT runs no-exclusion first, then with-exclusion | 66 | Running only-with-exclusion proves nothing; characterize alerts before suppressing them |

### Key Decisions (carried from v1.0)

- **Supervisor-Broker Pattern:** Research confirms this is the only way to manage elevated tasks like WFP while maintaining user-level CLI (2026-04-04).
- **WFP as Primary Network Backend:** Moving away from temporary firewall rules for true kernel-level enforcement (2026-04-04).
- **Named Job Objects:** Chosen for agent lifecycle management to ensure atomic stop/list capabilities (2026-04-04).
- **SID-Based Filtering:** Prioritized over App-ID to ensure child processes inherit network restrictions (2026-04-04).
- **Double-Launch Strategy:** Used `DETACHED_PROCESS` to decouple the supervisor from the parent terminal (2026-04-04).
- **Restricted Tokens:** Used to apply the session-unique SID to the process tree (2026-04-04).
- **RFC 3161 Timestamping:** Upgraded from legacy /t to /tr + /td sha256 (2026-04-05).
- **WFP Startup Orphan Sweep:** Enumerates NONO_SUBLAYER_GUID filters and removes stale ones at startup (2026-04-05).
- **Machine MSI Owns EventLog Registration:** SYSTEM\CurrentControlSet\Services\EventLog\Application\nono-wfp-service (2026-04-05).
- **MSRV 1.77:** Bumped from 1.74 to align with windows-sys 0.59 (2026-04-05).
- **WaitNamedPipeW Readiness Probe:** run_detached_launch() uses WaitNamedPipeW(50ms) per iteration on Windows (2026-04-05).
- **Single SID Generation Point:** Session SID generated once at ExecConfig construction (2026-04-06).
- **Driver Gate Removed:** activate_policy_mode no longer checks for a kernel driver binary artifact (2026-04-06).

## Accumulated Context

### Constraints active this milestone

- **Repo MUST stay PUBLIC** until Microsoft approves the minifilter altitude — verify no `build_notes/`/`.gsd/` staged before any `git push` (the "go-private" commit `74a47742` was cancelled 2026-06-11).
- **Cross-target clippy (Linux + macOS) is a MUST** per CLAUDE.md for any cfg-gated Unix code (fires on Phases 68 and 70); Windows dev host can't cross-compile → CI is the load-bearing signal. Scan every macOS cherry-pick for edition-2024 let-chains + E0716 (the class that broke v0.62.0/v0.62.1 release tags).
- **TRUST helper never weakens the D-32-12 gate** — it imports a cert into trust stores (operator-auditable); it never bypasses the gate or trusts an unsigned/wrong-signer binary.

### Pitfall guards carried forward (UPST + macOS)

- **macOS cross-target drift (Pitfall 9):** scan every macOS cherry-pick for edition-2024 let-chains and E0716 patterns; CI macOS green required before tag.
- **Seatbelt deny-after-allow ordering (Pitfall 10):** unit tests must assert ordering, not just rule presence.
- **macOS `/private/etc` symlink drift (Pitfall 11):** emit both symlink and canonical path for every macOS deny path.
- **DIVERGENCE-LEDGER cluster isolation can be empirically false (`feedback_cluster_isolation_invalid`):** UPST8 audit (Phase 69) must diff-inspect re-export surfaces, not just `--name-only`.

### Plan 70-03 Close — C2 Network Policy Security Hardening (2026-06-13)

- **C2 commits:** 0fb59375 (fork: `1f5b6193`) + bd4c469a (fork: `35282744`)
- **Security effect:** Embedded profiles (opencode/developer/codex/claude-code) no longer carry implicit credential routes; proxy now enforces deny-by-default under network.block via ProxyConfig.strict_filter + HostFilter::new_strict()
- **Conflicts in bd4c469a:** 4 files (sandbox_prepare.rs, launch_runtime.rs, proxy_runtime.rs, main.rs); 4 auto-merged; upstream refactored helpers (cwd_access_requirement etc.) rejected (Edition 2021 incompatible; fork has equivalent inline logic)
- **RouteStore/CredentialStore decoupling (Phase 56):** PRESERVED — server.rs auto-merged cleanly; no regression
- **Test result:** 779+1219+162 pass; 5 pre-existing failures (red->red carry-forward); 0 new regressions
- **Cross-target clippy:** PARTIAL — Windows host cannot cross-compile; GH Actions CI is load-bearing signal
- **D-70-E1 C2:** PASS (0 Windows-only files); D-70-E1 phase-wide: PASS (0 Windows-only files across all 5 cherry-picks)
- **Phase-wide D-19/D-20 count:** 5 (all will-sync commits absorbed: C3×2 + C4×1 D-20 + C2×2)
- **UPST8-02:** SATISFIED — all 5 will-sync commits on main with correct trailers
- **Human-verify checkpoint:** AWAITING — automated checks PASS; awaiting user confirmation

### Plan 69-01 Close — UPST8 Audit (2026-06-13)

- **Range:** `9a05a4ff..52809dda` (v0.60.0..v0.62.0, D-01 corrected; SC says v0.61.2 ceiling — flagged for +3 update to REQUIREMENTS.md UPST8-01)
- **upstream_head_at_audit:** `849cda42c0541f18915708cd3ff31d61c12d136d` | **refetch_date:** 2026-06-13 | **drift_tool_sh_sha:** `0834aa66` (pin held)
- **9 unique commits** (drift-tool count), **4 clusters**. Disposition breakdown: **will-sync 3** (C2 network-policy security, C3 profile/diagnostic features, C4 nono-pull recovery) · **won't-sync 1** (C1 release bumps) · **split 0** · **fork-preserve 0**.
- **windows-touch:yes = 0** — no Phase 70 cross-target clippy work required for any cluster.
- **macOS-overlap:** 7 of 9 commits are in the overlap range (v0.60.0..v0.61.2); all 7 carry Phase 63 pointers. 2 tail commits (db073750, 52809dda) have no Phase 63 pointer and contain no macOS-relevant code — "macOS un-audited" flag vacuously satisfied.
- **Cross-cluster re-export deps:** 1 field-existence ordering dep — C2 (bd4c469a) → C3 (cc21229f) via `suppressed_system_service_operations` in `PreparedSandbox`. Cherry-pick ordering: absorb C3 before C2 in Phase 70. No re-export-isolation failures; no split flips. Mirrors Phase 54 C5→C3 dep.
- **Empirical cross-check:** 6 files walked (filter.rs, server.rs, policy.rs, net_filter.rs, network_policy.rs, sandbox_prepare.rs); zero drift-tool gaps; all PASS.
- **ADR review outcome:** (a) Confirm. Phase 33 ADR Option A 'continue' — 5-dimension L/M/H (security=M, windows=L, maintenance=L, divergence=L, contributor=L). Does NOT supersede Phase 33 ADR.
- **SC divergence flag:** ROADMAP says v0.61.2 ceiling; D-01 extended to v0.62.0; REQUIREMENTS.md UPST8-01 acceptance language should be updated to reflect v0.62.0 (+3 tail commits).
- **Zero-source-edits invariant honored:** `git diff plan_base_sha..HEAD -- crates/ bindings/ scripts/ Makefile` returns 0 lines.
- **DCO sign-off:** All commits carry `Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>`.
- **Next:** Phase 70 (UPST8 Cherry-pick Sync) — absorb will-sync clusters C2, C3, C4 with D-19 trailers; cherry-pick order: C3 before C2 (ordering constraint).

### Plan 54-01 Close — UPST7 Audit (2026-06-04) — UPST8 predecessor context

- **Range:** v0.57.0..v0.59.0 | **upstream_head_at_audit:** `48d39f36` | **refetch_date:** 2026-06-04 | **drift_tool_sh_sha:** `0834aa66` (pin held).
- **40 unique commits**, **14 clusters**. Disposition breakdown: **will-sync 8** · **split 3** · **won't-sync 3**. **windows-touch:yes = 2** (C2, C8).
- **ADR review:** **(a) confirm** Phase 33 Option A `continue` (5-dim L/M/H). Does NOT supersede Phase 33 ADR.
- **v0.60.0 scope deferred to UPST8:** the v0.57.0..v0.59.0 audit kept range; **v0.60.0..v0.61.1 deferred to UPST8** (re-fetch surfaced v0.60.0 `9a05a4ff` + v0.61.0 + v0.61.1). **v2.11 Phase 69 extends the upper bound to `v0.61.2` and scopes to the non-macOS surface** (the macOS slice of v0.60.0..v0.61.2 was absorbed in v2.10 Phases 63-65). Re-fetch upstream at Phase 69 audit-open and record the new head SHA.
- **Zero-source-edits invariant honored** for the audit; drift re-run idempotent; DCO sign-off on all commits.

## Deferred Items

### v2.10 close (acknowledged 2026-06-11)

Pre-close `audit-open` reported **65 open items**; user chose "Acknowledge all & proceed". Breakdown: **35** `missing`/`unknown` quick-task slugs (pre-v2.5 stragglers, carried since prior closes) + **17** UAT gaps (phases 35/36/37/41/43/44/45/48/49/50/55/56/57/60/62/65/66) + **5** verification gaps (phases 41/44/49/56/57) + **5** dormant seeds (001-silent-enterprise-deployment, 002-network-egress-hardening, 003-siem-edr-telemetry, 004-multi-engine-pluggability, 005-zt-infra-attestation) + **3** new v2.11 carry-forward todos. The 3 todos are the only genuinely new items and are now scoped into v2.11:

| Todo | Headline | v2.11 phase |
|------|----------|-------------|
| `20260611-poc-cert-broker-clean-host` | v0.62.2 signed with untrusted POC cert → broker non-functional out-of-box on a clean Windows host (most consequential for distribution) | Phase 67 (TRUST-01/02, partial close — real signing is enterprise-milestone-gated) |
| `20260611-msi-vcredist-prereq` | MSI doesn't bundle/declare VC++ x64 runtime → 1603 on a clean host | Phase 67 (DIST-01) |
| `20260611-macos-resl-enforcement-broken` | macOS `--timeout`/`RLIMIT_NPROC` enforcement doesn't fire on a real host (REQ-RESL-NIX-03; the Phase 65 gate-65-A A5 finding) | Phase 68 (RESL-MAC-01/02) |

The Phase 65/66 UAT-gap rows reflect the macOS-resl A5 finding (now Phase 68) and the EDR-proxy-vs-cloud-EDR caveat (WR-02 closed "under EDR-proxy", MDE re-run is an EDR-agnostic follow-up) — both already characterized, neither an undocumented regression.

### v2.9 + v2.8 close (acknowledged 2026-06-06)

Pre-close `audit-open` reported 55 open items; user chose "Acknowledge all & proceed". Breakdown: 35 `missing` quick-task slugs (pre-v2.5 stragglers, carried since prior closes) + 15 UAT gaps + 5 verification gaps (pre-v2.0 bookkeeping carried since the v2.2/v2.4 closes).

### v2.7 close (acknowledged 2026-05-28)

Pre-close `audit-open` reported 45 open items; user chose "Acknowledge all & proceed". Breakdown:

- **~42 historical (already deferred at prior closes):** 29 `missing` quick-task slugs (pre-v2.5 stragglers) + 10 UAT gaps + 3 verification gaps (pre-v2.0 phases 01/07/13/17/18 bookkeeping, carried since the v2.2/v2.4 closes).
- **Genuine new carry-forwards (resolved in v2.8):** WFP elevated live-uninstall UAT, v0.57.3 MSI rebuild, untagged post-`637a426c` fixes, 3 pending todos — all closed by v2.8 Phase 53.

### Phase-level verification gaps (3 — orchestrator post-merge)

| Phase | Item | Disposition |
|-------|------|-------------|
| 37 | VERIFICATION.md `status: human_needed` — Success Criterion 6 (`.github/workflows/phase-37-linux-resl.yml` live run on `ubuntu-24.04`) | Orchestrator post-merge push triggers workflow; structurally complete + YAML-valid + commits unpushed per worktree-mode discipline. |
| 41 | VERIFICATION.md `status: human_needed` — cross-target Linux/macOS clippy + 8 GH Actions lanes on HEAD `b78dba87` | Decisive close signal lives in GH Actions; Windows host cannot run cross-target clippy. Class E env_vars flake explicitly deferred per Plan 41-10 disposition. |
| 43 | VERIFICATION.md `status: human_needed` — umbrella PR open + baseline-aware CI lane diff vs `13cc0628` | 6 PR-SECTION.md contribution artifacts staged; orchestrator concatenates + `gh pr create` post-merge. |

### Partial UAT scenarios (13 across 3 phases)

| Phase | File | Open scenarios |
|-------|------|----------------|
| 37 | `.planning/phases/37-linux-resl-backends-pkgs-auto-pull/37-HUMAN-UAT.md` | 5 |
| 41 | `.planning/phases/41-ci-cleanup-v24-broker-code-review-closure/41-HUMAN-UAT.md` | 6 |
| 43 | `.planning/phases/43-upst5-sync-execution/43-HUMAN-UAT.md` | 2 |

### Historical quick-task slugs (21 — pre-v2.5 stragglers)

Pre-v2.5 task slugs marked `missing` or `unknown` in `.planning/quick/`. Most pre-date the v2.5 milestone by months and appear to be cleanup debris from prior milestones.

```
260405-v0e-investigate-and-fix-exec-strategy-rs-unc
260405-vjj-fix-pr-555-signoffs-and-merge-conflicts-
260406-ajy-assess-windows-functional-equivalence-to
260406-bem-research-and-roadmap-windows-gap-closure
260410-nlt-fix-three-uat-gaps-in-phase-10-etw-learn
260412-ajy-safe-layer-roadmap-input
260417-kem-fix-envvarguard-migration-migrate-48-fla
260417-wla-fix-windows-createprocess-handle-uaf
260419-cmp-upstream-036-windows-parity
260424-upr-review-upstream-037-to-040
260428-rsu-refresh-stack-onto-upstream-tip
260508-m99-broker-process-poc-minimal-rust-binary-t
260509-rib-clean-up-windows-poc-handoff-mdx-apply-9
260509-s9m-verify-that-the-sigstore-functionality-i
260509-stb-fix-windows-poc-handoff-mdx-block-net-se
260510-im9-investigate-windows-test-failures-in-cra
260511-jxg-cmd-unc-cwd-supervised
260511-jxk-label-guard-drop-on-sigint
260513-f5n-update-the-poc-runbook-windows-poc-hando
260514-0gu-bump-fork-version-0-37-1-to-0-53-0
260516-mxw-fix-handletarget-import-linux
```

## Quick Tasks Completed

| Date | Slug | Deliverable |
|------|------|-------------|
| 2026-06-08 | vm-driver-signing-runbook | `64-SC1-VM-RUNBOOK.md` — junior-friendly cookbook for the Phase 64 Track A VM minifilter test-signing + deny harness (Phase 63 UAT lessons baked in) |
| 2026-06-10 | cargo-audit-rustls-webpki | Bumped `sign-fixture` sigstore `0.7.0→0.8.0` (commit `4aaa0508`) — drops vulnerable `rustls-webpki 0.102.8` + `sigstore 0.7.0` subtree; `cargo audit` exit 0. Real remediation over `.cargo/audit.toml` ignore. |

## Session Continuity

**Last session:** 2026-06-13T02:47:12.755Z

**v2.11 roadmap complete (2026-06-11):** Phases 67-70 defined, 8/8 reqs mapped (100% coverage). ROADMAP.md + REQUIREMENTS.md traceability + STATE.md updated. Phases 67 (clean-host Win install: DIST-01/02 + TRUST-01/02) and 68 (macOS resl: RESL-MAC-01/02) are independent + host-gated + parallel-safe. Phases 69 (UPST8 audit: UPST8-01) → 70 (UPST8 sync: UPST8-02) are the linear audit-then-sync pair, cadence-ordered after Phase 55.

**Predecessor context (carried):** v2.10 shipped tag `v2.10` 2026-06-11; gate-65-A Seatbelt PASS; WR-02 CLOSED; ADR-65 Accepted (No-go/Conditional-go, DRV-PROD-01 deferred). The Phase 65 A5 finding (macOS resl enforcement doesn't fire) is now Phase 68. Repo STAYS PUBLIC (commit `74a47742` cancelled; `build_notes/`+`.gsd/` ignored, untracked).

## Operator Next Steps

- `/gsd:plan-phase 67` — clean-host Windows install (needs a clean Win11 host for the install + broker UAT; production-signed MSI, not dev-layout).
- `/gsd:plan-phase 68` — macOS resl enforcement fix (needs a real macOS host for `NONO_RESL_HOST_VALIDATED=1` re-validation). Parallel-safe with 67.
- `/gsd:plan-phase 69` then `70` — UPST8 audit-then-sync (host-agnostic; cross-target clippy via CI).
- Before any push: confirm no `build_notes/`/`.gsd/` staged — repo stays PUBLIC pending Microsoft minifilter-altitude approval.
