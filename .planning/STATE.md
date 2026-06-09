---
gsd_state_version: 1.0
milestone: v2.10
milestone_name: Kernel-Driver Spike + EDR UAT + macOS Upstream Parity
status: executing
last_updated: "2026-06-08T22:25:48.176Z"
last_activity: 2026-06-08 -- Phase 64 execution started
progress:
  total_phases: 4
  completed_phases: 1
  total_plans: 8
  completed_plans: 3
  percent: 38
---

# Project State: nono — v2.10 Kernel-Driver Spike + EDR UAT + macOS Upstream Parity

## Project Reference

See: `.planning/PROJECT.md` (v2.10 milestone started 2026-06-06; v2.8 + v2.9 shipped 2026-06-06, archived). Phase numbering continues from Phase 62 (Phases 63-66). Roadmap: 4 phases defined.

**Core Value:** Windows security must be as structurally impossible and feature-complete as Unix platforms; every nono command that works on Linux/macOS should work on Windows with equivalent security guarantees, or be explicitly documented as intentionally unsupported with a clear rationale.

**Current Focus:** Phase 64 — minifilter-spike-implementation-macos-p1-cherry-pick-wave

## Current Position

Phase: 64 (minifilter-spike-implementation-macos-p1-cherry-pick-wave) — EXECUTING
Plan: 1 of 5
Status: Executing Phase 64
Last activity: 2026-06-08 -- Phase 64 execution started

Progress: ░░░░░░░░░░ 0% (0/4 phases complete)

### v2.10 Phase Summary (active)

| Phase | Goal | Requirements | Status |
|-------|------|--------------|--------|
| 63 | Minifilter spike groundwork (WDK/VM/design doc) + macOS DIVERGENCE-LEDGER audit | DRV-03 (partial), MACOS-01 | Not started |
| 64 | Minifilter spike implementation (intercept + deny + IPC roundtrip on test VM) + macOS P1 cherry-pick wave | DRV-01, DRV-02, DRV-03 (complete), MACOS-02 | Not started |
| 65 | Minifilter go/no-go ADR + macOS live re-validation HUMAN-UAT (CI macOS green — HARD gate) | DRV-04, MACOS-03 | Not started |
| 66 | WR-02 EDR HUMAN-UAT (no new code; real EDR host required) | EDR-01, EDR-02 | Not started |

### Host-availability gates

| Phase | Gate | Notes |
|-------|------|-------|
| 63-B | Hyper-V Secure-Boot-OFF VM with WDK installed | Check HVCI state with `msinfo32` before starting; HVCI default-on on Win11 26200 silently rejects test-signed drivers |
| 65-A | Real macOS host for `sandbox_init()` re-validation | `make test-lib` + live `nono run` required; CI macOS build is the HARD close gate |
| 66 | Real Windows host with EDR agent running ≥24 hours | Must use production-signed MSI, not dev-layout binary; Sysmon free EDR-proxy fallback if MDE unavailable |

## Key Decisions

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

### Pitfall guards active this milestone

- **BSOD from IRQL violation (Pitfall 1):** ring-buffer + worker-thread IPC design must be in the Phase 63 plan before any driver code is written; `NonPagedPoolNx` only in callback context
- **Own-I/O recursion BSOD (Pitfall 2):** no driver-originated file I/O (`ZwCreateFile`); all logging via `FltSendMessage` to user mode
- **System hang from blocking `FltSendMessage` (Pitfall 3):** finite `Timeout` mandatory; `STATUS_TIMEOUT` = permit-and-log for the spike
- **TESTSIGNING + HVCI silent failure (Pitfall 4):** document `msinfo32` state at Phase 63 start; use Hyper-V VM
- **Altitude conflict with EDR (Pitfall 5):** choose Activity-Monitor/FSFilter range; enumerate with `fltmc filters` before deploy
- **macOS cross-target drift (Pitfall 9):** scan every macOS cherry-pick for edition-2024 let-chains and E0716 patterns; CI macOS green required before tag
- **Seatbelt deny-after-allow ordering (Pitfall 10):** unit tests must assert ordering, not just rule presence
- **macOS `/private/etc` symlink drift (Pitfall 11):** emit both symlink and canonical path for every macOS deny path

### Cross-phase parallelism notes

- **Phase 63:** Track A (macOS audit: `git log v0.57.0..v0.61.2` scoped to macOS paths — any host, read-only) is fully parallel to Track B (minifilter groundwork: requires WDK VM). No file overlap.
- **Phase 66** (EDR UAT) has zero code dependencies on Phases 63-65. Can start as soon as an EDR host is available. Recommended: schedule after Phase 64-A so any new macOS profile content can be included in the UAT scope, but this is advisory, not blocking.

### Plan 54-01 Close — UPST7 Audit (2026-06-04)

- **Range:** v0.57.0..v0.59.0 | **upstream_head_at_audit:** `48d39f36` | **refetch_date:** 2026-06-04 | **drift_tool_sh_sha:** `0834aa66` (pin held).
- **40 unique commits** (drift source of truth; the 260527-sgo gap analysis under-counted at ~19), **14 clusters**. Disposition breakdown: **will-sync 8** (allow_domain, proxy-502, bw://, profile JSONC/target_binary/opencode, pack-hints, diagnostic polish, timeout constants, policy-test) · **split 3** (C2 supervisor named-socket IPC→Ph59, C5 TLS-intercept ordering→Ph56, C8 session hooks→Ph58) · **won't-sync 3** (release commits, C13 sigstore→reclassified split, C14 macOS-only). **windows-touch:yes = 2** (C2, C8).
- **ADR review:** **(a) confirm** Phase 33 Option A `continue` (5-dim L/M/H: security M, windows L, maintenance M, divergence M, contributor L). Does NOT supersede Phase 33 ADR.
- **Cross-cluster re-export:** pub-use scan **clean** (no Phase-43-class trap); one **function-call** prereq C5→C3 (`partition_allow_domain`).
- **SC4 TLS-intercept verdict:** **fork-preserve** — fork `RouteStore`/`CredentialStore` already decouples endpoint-before-credential; upstream `tls_intercept/` absent in fork; `credential.rs` untouched by `22e6c40` (byte-identical preserved). Small `proxy_runtime.rs` filter-allowlist port rides WITH Phase 56 allow_domain. (Phase 56 prerequisite note delivered.)
- **Empirical cross-check:** 5 fork-shared files walked (route.rs, credential.rs, keystore.rs, profile/mod.rs, platform.rs); **zero drift-tool gaps** (route.rs/credential.rs fork-original → 0 upstream commits; profile/mod.rs 6 merges correctly excluded; platform.rs 0 → no java-dev cluster in range).
- **v0.60.0 scope:** range kept v0.57.0..v0.59.0; **v0.60.0..v0.61.1 deferred to UPST8** (re-fetch surfaced v0.60.0 `9a05a4ff` + v0.61.0 + v0.61.1 — larger than the v0.60.0-alone set the plan anticipated). UPST8 stub appended to ROADMAP § Future Cycles (commit `0b49c697`).
- **Zero-source-edits invariant honored:** `git diff plan_base_sha(eb8c9b82)..HEAD -- crates/ bindings/ scripts/ Makefile` = 0. Drift re-run idempotent (exit 0, 40 commits). DCO sign-off on all commits.

## Deferred Items

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

## Session Continuity

**Last session:** 2026-06-08T21:20:01.666Z

**Resume with:** `/gsd:plan-phase 63`

**Open questions to resolve at plan-phase:**

- HVCI/VM host state on Win11 26200 — check `msinfo32` before Phase 63 plan
- Which EDR product is available for Phase 66 (MDE trial needs an M365 tenant ~1 day; Sysmon-only is the fallback)
- macOS host availability for Phase 65 live `sandbox_init()` re-validation
- Exact macOS-relevant upstream commit set produced by Phase 63 audit
