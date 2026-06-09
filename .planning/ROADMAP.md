---
milestone: v2.10
milestone_name: Kernel-Driver Spike + EDR UAT + macOS Upstream Parity
status: active
created: 2026-05-28
last_updated: 2026-06-06
granularity: standard
---

# Roadmap — nono

## Milestones

- ✅ **v1.0 Windows Alpha** — Phases 01-12 (shipped 2026-03-31) — see [`milestones/v1.0-*`](milestones/)
- ✅ **v2.0 Windows Gap Closure** — Phases 13-18 — see [`milestones/v2.0-ROADMAP.md`](milestones/v2.0-ROADMAP.md)
- ✅ **v2.1 Resource Limits / Extended IPC / Attach-Streaming** — see [`milestones/v2.1-ROADMAP.md`](milestones/v2.1-ROADMAP.md)
- ✅ **v2.2 Windows/macOS Parity Sweep** — see [`milestones/v2.2-ROADMAP.md`](milestones/v2.2-ROADMAP.md)
- ✅ **v2.3 Linux POC Unblock + Deferreds Closure** — see [`milestones/v2.3-ROADMAP.md`](milestones/v2.3-ROADMAP.md)
- ✅ **v2.4 Complete the Partial Ports + UPST4** — Phases 35, 36, 36.5, 39, 40 (shipped 2026-05-15) — see [`milestones/v2.4-ROADMAP.md`](milestones/v2.4-ROADMAP.md)
- ✅ **v2.5 Backlog Drain + UPST5** — Phases 37, 41, 42, 43 (shipped 2026-05-20) — see [`milestones/v2.5-ROADMAP.md`](milestones/v2.5-ROADMAP.md)
- ✅ **v2.6 UPST6 + v2.5 Drain** — Phases 44, 44.1, 45, 46, 47, 48, 49, 50 (shipped 2026-05-25) — see [`milestones/v2.6-ROADMAP.md`](milestones/v2.6-ROADMAP.md)
- ✅ **v2.7 Windows supervised-run hardening** — Phases 51, 52 (shipped 2026-05-26) — see [`milestones/v2.7-ROADMAP.md`](milestones/v2.7-ROADMAP.md)
- ✅ **v2.8 UPST7 + v2.7 Drain & Release** — Phases 53-59 (shipped 2026-06-06; tags `v2.8`+`v0.57.5`) — see [`milestones/v2.8-ROADMAP.md`](milestones/v2.8-ROADMAP.md)
- ✅ **v2.9 Windows Sandbox-the-Tools — Confined Coding Loop** — Phases 60, 61, 62 (published as `v0.62.2` 2026-06-06) — see [`milestones/v2.9-ROADMAP.md`](milestones/v2.9-ROADMAP.md)
- 🔄 **v2.10 Kernel-Driver Spike + EDR UAT + macOS Upstream Parity** — Phases 63-66 (active)

## Phases

<details>
<summary>✅ v2.8 UPST7 + v2.7 Drain & Release (Phases 53-59) — SHIPPED 2026-06-06</summary>

- [x] Phase 53: Release & Drain (3/4) — completed 2026-05-29 (shipped v0.57.5)
- [x] Phase 54: UPST7 Audit (1/1) — completed 2026-06-04
- [x] Phase 55: UPST7 Cherry-pick Wave (7/7) — completed 2026-06-05
- [x] Phase 56: Fine-grained Network Filtering (4/4) — completed 2026-06-05
- [x] Phase 57: Bitwarden Credential Source (1/1) — completed 2026-06-05
- [x] Phase 58: Session Lifecycle Hooks (3/3) — completed 2026-06-06
- [x] Phase 59: Supervisor IPC Robustness (3/3) — completed 2026-06-06

Audit: `tech_debt`, 10/10 reqs satisfied, 0 blockers. Full detail: [`milestones/v2.8-ROADMAP.md`](milestones/v2.8-ROADMAP.md).

</details>

<details>
<summary>✅ v2.9 Windows Sandbox-the-Tools — Confined Coding Loop (Phases 60-62) — PUBLISHED as v0.62.2 2026-06-06</summary>

- [x] Phase 60: Confined Coding Loop (3/3) — completed 2026-05-29
- [x] Phase 61: Ship/Release v2.9 (4/4) — completed 2026-06-06 (published v0.62.2)
- [x] Phase 62: WFP kernel network enforcement — Windows supervised (13/13) — completed 2026-06-03

Separate initiative from UPST7 (builds on merged PR #4). The v0.62.0/v0.62.1 release attempts failed on two latent cfg-gated cross-target compile errors (E0716 + edition-2024 let-chain), fixed in `4de294e8`+`7bb7c7e3` → v0.62.2 published. Full detail: [`milestones/v2.9-ROADMAP.md`](milestones/v2.9-ROADMAP.md).

</details>

### v2.10 Kernel-Driver Spike + EDR UAT + macOS Upstream Parity (Phases 63-66)

- [x] **Phase 63: Minifilter Spike Groundwork + macOS DIVERGENCE-LEDGER Audit** (completed 2026-06-08) - WDK/VM environment verified, altitude request sent, design doc written; macOS upstream commit inventory v0.57.0..v0.61.2 complete
- [x] **Phase 64: Minifilter Spike Implementation + macOS P1 Cherry-pick Wave** (completed 2026-06-09) - end-to-end pre-create interception + policy IPC roundtrip proven on test VM; P1 security/correctness macOS commits absorbed
- [ ] **Phase 65: Minifilter ADR + macOS Live Re-validation** - go/no-go ADR committed with latency data; macOS Seatbelt re-validated on real host with CI macOS build green (hard gate)
- [ ] **Phase 66: WR-02 EDR HUMAN-UAT** - ~10 pass/fail assertions recorded against real EDR runner; WR-02 closed or explicitly re-scoped

## Phase Details

<details>
<summary>✅ v2.8 UPST7 + v2.7 Drain & Release — Phase Details (archived)</summary>

See [`milestones/v2.8-ROADMAP.md`](milestones/v2.8-ROADMAP.md) for full phase detail blocks.

</details>

<details>
<summary>✅ v2.9 Windows Sandbox-the-Tools — Phase Details (archived)</summary>

See [`milestones/v2.9-ROADMAP.md`](milestones/v2.9-ROADMAP.md) for full phase detail blocks.

</details>

### Phase 63: Minifilter Spike Groundwork + macOS DIVERGENCE-LEDGER Audit
**Goal**: The minifilter spike has a working build environment and a written design doc before any driver code runs, and the macOS upstream commit inventory is complete so cherry-picks can begin in Phase 64
**Depends on**: Phase 62 (v2.9 shipped baseline)
**Requirements**: DRV-03 (partial — build pipeline documented), MACOS-01
**Success Criteria** (what must be TRUE):
  1. `msinfo32` HVCI/Secure Boot state is documented and a Hyper-V Secure-Boot-OFF VM is confirmed available for driver iteration; `bcdedit /enum all` TESTSIGNING state is recorded as a reproducibility artifact
  2. A WDK MSBuild project exists at `drivers/nono-fltmgr/` (`.vcxproj`, `.inf`, skeleton `.c` entry point) that compiles without errors to a `.sys` on the test-signing VM
  3. A written design doc (pre-code gate) specifies the ring-buffer + worker-thread IPC pattern, forbids driver-originated file I/O (`ZwCreateFile`), mandates a finite `FltSendMessage` timeout, and records the chosen altitude in the Activity-Monitor/FSFilter range with the Microsoft altitude-assignment request status (`fsfcomm@microsoft.com`, ~30 business-day lead)
  4. A `DIVERGENCE-LEDGER.md` for upstream `v0.57.0..v0.61.2` scoped to macOS-relevant paths is committed, with every commit classified (will-sync / fork-preserve / won't-sync / split), a `macos-only` column, and a diff-inspect note per the `feedback_cluster_isolation_invalid` lesson; the three P1 commits (`8f84d454`, `362ada22`, `8f1b0b74`) are identified and dispositioned `will-sync`
**Plans**: 3 plans
- [x] 63-01-PLAN.md — Author the drivers/nono-fltmgr/ WDK scaffold + DESIGN.md pre-code gate + ADR pointer stub (Wave 1, autonomous; DRV-03)
- [x] 63-02-PLAN.md — Provision the Azure test-signing VM, capture SC1, compile the scaffold to .sys, send the Microsoft altitude request (Wave 2, human-gated; DRV-03)
- [x] 63-03-PLAN.md — macOS DIVERGENCE-LEDGER audit v0.57.0..v0.61.2, three P1 commits will-sync, supersede Phase 54 C14 (Wave 1, autonomous; MACOS-01)

### Phase 64: Minifilter Spike Implementation + macOS P1 Cherry-pick Wave
**Goal**: The minifilter spike proves end-to-end pre-create interception with a user-mode policy roundtrip on the test VM, and the P1 macOS security/correctness commits land on the fork with unit-test coverage
**Depends on**: Phase 63
**Requirements**: DRV-01, DRV-02, DRV-03 (complete), MACOS-02
**Success Criteria** (what must be TRUE):
  1. A test-signed `nono-fltmgr.sys` installs on the Secure-Boot-OFF VM (`fltmc instances` shows the driver registered at the chosen altitude); a process attempting to open the deterministic deny-target path is refused at the kernel boundary (`STATUS_ACCESS_DENIED` returned to the caller)
  2. The minifilter's `IRP_MJ_CREATE` pre-operation sends a message over `\NonoPolicyPort` (via `FltSendMessage` with a finite timeout), and a Rust `fltmgr_client.rs` user-mode client (`#[cfg(windows)]`, `Win32_Storage_InstallableFileSystems` feature, `#[repr(C)]` message struct with a static layout assertion) receives the path+PID and returns an allow/deny decision that the driver enforces
  3. The three P1 macOS commits (`8f84d454` platform-rules-after-user-write-allows, `362ada22` and `8f1b0b74` symlink/`$PWD` CWD capture) are cherry-picked with verbatim D-19 `Upstream-commit:` trailers; unit tests assert Seatbelt rule ordering (deny rules appear after the allow rules they override — last-match-wins) and cover both the symlink path and canonical `/private/etc` path for every affected deny group
  4. The build pipeline for both the driver (`drivers/nono-fltmgr/`) and the Rust user-mode client (`fltmgr_client.rs`) is documented in `drivers/README.md`; the existing `nono-wfp-driver.sys` placeholder and MSI are untouched
**Plans**: 5 plans
- [x] 64-01-PLAN.md — Create nono-fltmgr-client crate + Wave 0 macOS test stubs (Wave 1, autonomous; DRV-02, MACOS-02)
- [x] 64-02-PLAN.md — Extend nono-fltmgr.c with pre-create callback + ring buffer + IPC port (Wave 1, autonomous; DRV-01, DRV-02)
- [x] 64-03-PLAN.md — Implement run_policy_client + cherry-pick 8f1b0b74+362ada22 (Wave 2, autonomous; DRV-02, MACOS-02)
- [x] 64-04-PLAN.md — Cherry-pick 8f84d454 + cross-target clippy + VM test-sign+load+deny proof (Wave 3, autonomous+human; DRV-01, DRV-03, MACOS-02)
- [x] 64-05-PLAN.md — Write drivers/README.md + make test phase close gate (Wave 4, autonomous; DRV-01..03, MACOS-02)

### Phase 65: Minifilter ADR + macOS Live Re-validation
**Goal**: The go/no-go ADR formalizes the spike verdict with latency data, and the macOS Seatbelt layer is confirmed correct on a real macOS host with CI green as a hard close gate
**Depends on**: Phase 64
**Requirements**: DRV-04, MACOS-03
**Success Criteria** (what must be TRUE):
  1. An ADR committed to `.planning/adr/` documents: the interception design, measured `FLT_PREOP_PENDING` round-trip latency (with the finite-timeout fail-open behavior for the spike), the `windows-drivers-rs`-not-viable decision, the FltMgr-vs-ETW rationale, the chosen altitude and official-assignment request status, and an explicit go or no-go recommendation for a production-driver milestone
  2. `sandbox_init()` succeeds with the updated Seatbelt profile on a real macOS host; `nono run --dry-run --profile claude-code` emits a profile where deny rules appear after the allow rules they override; `nono run --profile claude-code -- cat ~/.ssh/id_rsa` is blocked; both `/etc/hosts` and `/private/etc/hosts` are blocked
  3. The macOS CI build leg in `release.yml` is confirmed green before any release tag — this is a HARD close gate, not advisory; the cherry-pick checklist has been scanned for edition-2024 let-chains and E0716-class borrows; `make test-lib` passes on the macOS host
**Plans**: TBD

### Phase 66: WR-02 EDR HUMAN-UAT
**Goal**: The long-deferred WR-02 EDR validation is executed on a real EDR-instrumented host, producing concrete verdicts on nono's behavior and visibility; WR-02 is closed or explicitly re-scoped
**Depends on**: Phase 62 (v2.9 binaries; no code dependencies on Phases 63-65)
**Requirements**: EDR-01, EDR-02
**Success Criteria** (what must be TRUE):
  1. A HUMAN-UAT artifact (`.planning/phases/66-edr-human-uat/66-HUMAN-UAT.md`) records ~10 pass/fail assertions executed in two passes — no-exclusion first (to characterize false-positive exposure) then with-exclusion (to confirm suppression is sufficient) — against a real EDR runner (Sysmon as EDR-proxy and/or Microsoft Defender for Endpoint), installed via the production-signed MSI on a host where the EDR has been running for at least 24 hours; each assertion records the EDR product, version, and policy mode
  2. The UAT explicitly validates whether EDR DLL-injection into Low-IL children fails at the `NO_WRITE_UP` MIC boundary as designed, and whether the broker's `CreateProcessAsUserW` + `SetTokenInformation(IntegrityLevel)` sequence (MITRE T1134.002) triggers EDR alerts or quarantine
  3. WR-02 is formally closed in the planning artifacts with the recorded findings, or explicitly re-scoped with a concrete next step if the results are inconclusive; the UAT artifact distinguishes "EDR did not alert" from "EDR did not quarantine" for every test scenario
**Plans**: TBD

## Progress

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 63. Minifilter Spike Groundwork + macOS DIVERGENCE-LEDGER Audit | 3/3 | Complete    | 2026-06-08 |
| 64. Minifilter Spike Implementation + macOS P1 Cherry-pick Wave | 5/5 | Complete    | 2026-06-09 |
| 65. Minifilter ADR + macOS Live Re-validation | 0/TBD | Not started | - |
| 66. WR-02 EDR HUMAN-UAT | 0/TBD | Not started | - |

## Future Cycles

### UPST8 — Upstream v0.59.0… sync audit (placeholder)

**Goal**: Audit upstream `v0.59.0..<next-tag>` divergence per the Phase 33 ADR `continue` cadence rule. Inherits the audit-shape template from Phase 33 + 39 + 42 + 47 + 54 verbatim. The first deferred-from-UPST7 targets are **v0.60.0 (`9a05a4ff`), v0.61.0, and v0.61.1** (the 2026-06-04 UPST7 re-fetch surfaced all three past the locked `v0.57.0..v0.59.0` range; the deferred set is `v0.60.0..v0.61.1`, NOT v0.60.0 alone — and NOT the unrelated Feb-2026 v0.6.x tag line). Title may flip from `sync audit` to `sync execution` if the next cycle's commit set is small enough to skip a dedicated audit (auditor's call at UPST8 plan-phase). Note: v2.10 absorbs the macOS-relevant slice of v0.60.0..v0.61.2 (Phases 63-65); the non-macOS UPST8 clusters remain deferred here.
**Depends on**: Phase 55 (UPST7 cherry-pick wave closed; cadence rule preserves linear ordering)
**Plans**: 0 / TBD
**Reference**: `docs/architecture/upstream-parity-strategy.md` § Future audit cadence

UPST8 fires when the maintainer decides the accumulated cherry-pick labor (v0.60.0..v0.61.1 deferred at Phase 54; will grow before UPST8 fires) warrants absorbing.

### Carried v2-deferred requirements (from v2.8)

These were defined but deferred during v2.8 (not yet milestone-scoped):

- **REQ-WSRH-AUDIT-01** — profile-wide audit of which heavy-runtime binaries hit the `WriteRestricted` gate.
- **REQ-RLS-ATTEST-01** — evaluate `actions/attest-build-provenance` vs the existing sigstore/TUF + Authenticode pipeline.
- **REQ-UPST-RESID-01** — residual v0.44–v0.57 macОS-learn-diagnostics refactors (`b5f0a3ab`, `bbdf7b85`, `wiring.rs`).
- **REQ-DENY-PREFLIGHT-01** — Linux-host-gated `validate_deny_overlaps` preflight investigation (security equivalence already proven).
- **REQ-UNDO-TOCTOU-01** — full fd-relative TOCTOU hardening of `validate_restore_target` (standalone security phase, ~2-3 wk).

## Next

Phase 63 is the entry point. The two tracks (minifilter spike groundwork + macOS audit) are parallel-safe within Phase 63 — the audit requires only read-only git access; the `drivers/` scaffold is entirely new. Phase 66 (EDR UAT) has no code dependencies and can begin as soon as an EDR-instrumented host is available. Start with `/gsd:plan-phase 63`.

## References

- `.planning/PROJECT.md` — project context + current state.
- `.planning/MILESTONES.md` — shipped milestone history (v1.0 → v2.9).
- `.planning/REQUIREMENTS.md` — v2.10 requirements (DRV-01..04, EDR-01..02, MACOS-01..03).
- `.planning/research/SUMMARY.md` — HIGH-confidence research; build-order recommendations.
- `.planning/research/ARCHITECTURE.md` — integration points per theme.
- `.planning/research/PITFALLS.md` — pitfall→phase ownership; BSOD guards; cross-target drift guards.
- `.planning/milestones/v2.8-ROADMAP.md` / `v2.8-REQUIREMENTS.md` / `v2.8-MILESTONE-AUDIT.md` — archived v2.8.
- `.planning/milestones/v2.9-ROADMAP.md` / `v2.9-REQUIREMENTS.md` — archived v2.9.
