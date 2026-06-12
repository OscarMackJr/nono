---
gsd_state_version: 1.0
milestone: v2.11
milestone_name: Clean-Host Distribution Cleanup + UPST8
status: executing
last_updated: "2026-06-12T17:19:07.102Z"
last_activity: 2026-06-12 -- Phase 68 BLOCKED: macOS host UAT failed (RESL enforcement still not firing)
progress:
  total_phases: 4
  completed_phases: 1
  total_plans: 1
  completed_plans: 0
  percent: 0
---

# Project State: nono — v2.11 Clean-Host Distribution Cleanup + UPST8

## Project Reference

See: `.planning/PROJECT.md` (v2.11 milestone started 2026-06-11; v2.10 shipped + archived 2026-06-11). Phase numbering continues from Phase 66 (Phases 67-70).

**Core Value:** Windows security must be as structurally impossible and feature-complete as Unix platforms; every nono command that works on Linux/macOS should work on Windows with equivalent security guarantees, or be explicitly documented as intentionally unsupported with a clear rationale.

**Current Focus:** Phase 68 — macos-resl-enforcement-fix (DIAGNOSED multi-defect; RE-SCOPED to a planned phase — next `/gsd:plan-phase 68`)

## Current Position

Phase: 68 (macos-resl-enforcement-fix) — BLOCKED → RE-SCOPE. Debug `macos-resl-not-firing` (status: diagnosed) found the macOS supervised path has THREE foundational defects, broader than the planned 2-bug scope: **D1** `set_read_timeout`/SO_RCVTIMEO EINVAL on the AF_UNIX supervisor socket (core RESL path; todo 20260612-...-rcvtimeo-einval), **D2** `setrlimit(RLIMIT_AS)` fails in child → `--memory` broken (todo 20260612-...-rlimit-as), **D3** `--timeout`/`--max-processes` non-enforcement (original targets). D1+D2 PREDATE Phase 68. Phase 68's setpgid/NPROC fix is deployed on origin/main (`1b2e2ad0`/`f94c1c1b`/`3583bacc` + macOS compile fixes `53501113`/`fa6c2dc6`, head `173f8386`) but unobservable behind D1/D2. Per user decision 2026-06-12, fix re-scoped to planned work. NEXT: `/gsd:plan-phase 68` covering D1+D2+D3 (planning input = debug file DIAGNOSIS COMPLETE block). Load-bearing gate must be a real macOS build+test, NOT Windows `cargo check`.
Plan: 1 of 1 (NOT complete — Tasks 1+2+3-automated committed `1b2e2ad0`..`cc9f8c94` + compile fixes; fix deployed but blocked behind D1/D2; re-plan will decide keep/revise/gate)
Status: ROOT CAUSE FOUND (debug `macos-resl-not-firing`, status root-cause-found) — the 2026-06-12 macOS UAT tested a STALE binary: the fix commits (1b2e2ad0/f94c1c1b/3583bacc) were local-only/unpushed, so the Mac's `git pull origin main` (848ce71d) + `cargo build` compiled PRE-fix code (host printed the exact warning Phase 68 deleted; child PGID==parent PGID; only 4 tests not 5). NOT a code defect. **Fix now PUSHED to origin/main `63dfd9a5` (2026-06-12).** AWAITING Mac re-test against the deployed fix (`git pull` → verify fix landed → `cargo build` → `NONO_RESL_HOST_VALIDATED=1 cargo test -p nono-cli --test resl_nix_macos`). Separate confound still filed: audit_attestation set_read_timeout EINVAL (Phase 59 IPC).
Last activity: 2026-06-12 -- Phase 68-01 UAT root-caused to stale binary; fix pushed to origin 63dfd9a5; awaiting Mac re-test

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

**Last session:** 2026-06-12T17:19:07.089Z

**v2.11 roadmap complete (2026-06-11):** Phases 67-70 defined, 8/8 reqs mapped (100% coverage). ROADMAP.md + REQUIREMENTS.md traceability + STATE.md updated. Phases 67 (clean-host Win install: DIST-01/02 + TRUST-01/02) and 68 (macOS resl: RESL-MAC-01/02) are independent + host-gated + parallel-safe. Phases 69 (UPST8 audit: UPST8-01) → 70 (UPST8 sync: UPST8-02) are the linear audit-then-sync pair, cadence-ordered after Phase 55.

**Predecessor context (carried):** v2.10 shipped tag `v2.10` 2026-06-11; gate-65-A Seatbelt PASS; WR-02 CLOSED; ADR-65 Accepted (No-go/Conditional-go, DRV-PROD-01 deferred). The Phase 65 A5 finding (macOS resl enforcement doesn't fire) is now Phase 68. Repo STAYS PUBLIC (commit `74a47742` cancelled; `build_notes/`+`.gsd/` ignored, untracked).

## Operator Next Steps

- `/gsd:plan-phase 67` — clean-host Windows install (needs a clean Win11 host for the install + broker UAT; production-signed MSI, not dev-layout).
- `/gsd:plan-phase 68` — macOS resl enforcement fix (needs a real macOS host for `NONO_RESL_HOST_VALIDATED=1` re-validation). Parallel-safe with 67.
- `/gsd:plan-phase 69` then `70` — UPST8 audit-then-sync (host-agnostic; cross-target clippy via CI).
- Before any push: confirm no `build_notes/`/`.gsd/` staged — repo stays PUBLIC pending Microsoft minifilter-altitude approval.
