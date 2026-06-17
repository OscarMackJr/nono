---
milestone: v2.13
milestone_name: Carry-Forward Closeout (Dark Factory)
status: active
created: 2026-06-17
last_updated: 2026-06-17
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
- ✅ **v2.10 Kernel-Driver Spike + EDR UAT + macOS Upstream Parity** — Phases 63-66 (shipped 2026-06-11; tag `v2.10`) — see [`milestones/v2.10-ROADMAP.md`](milestones/v2.10-ROADMAP.md)
- ✅ **v2.11 Clean-Host Distribution Cleanup + UPST8** — Phases 67-70 (shipped 2026-06-13) — see [`milestones/v2.11-ROADMAP.md`](milestones/v2.11-ROADMAP.md)
- ✅ **v2.12 AI Agent Abstraction** — Phases 71-75 (shipped 2026-06-16; tag `v2.12`) — see [`milestones/v2.12-ROADMAP.md`](milestones/v2.12-ROADMAP.md)
- 🔵 **v2.13 Carry-Forward Closeout (Dark Factory)** — Phases 76-81 (active) — below

## Phases

### v2.13 Carry-Forward Closeout (Dark Factory) — Active

- [ ] **Phase 76: Self-Verifying Harness Foundation** - Build the shared scripted-gate framework (single-invocation unattended scripts emitting machine-readable pass/fail) that all host-gated phases depend on.
- [ ] **Phase 77: Copilot CLI End-to-End Confinement** - Fix Node-ESM/AppContainer ancestor `FILE_READ_ATTRIBUTES` + one-time-admin system-ancestor RA grant + scripted end-to-end proof.
- [ ] **Phase 78: Cross-Process Classification** - Add daemon control-pipe `Classify` verb so `nono classify <pid>` is authoritative cross-process, caller-gated, and tenant-safe.
- [ ] **Phase 79: WFP Egress Isolation + nono-ts Ergonomics** - Empirical per-agent WFP isolation test + `confinedRun` default-broker-arm and auto-coverage ergonomics for nono-ts.
- [ ] **Phase 80: Clean-Host Install UAT** - Verify the machine MSI installs and runs on a fresh Win11 host with no manual steps via the unattended clean-host harness.
- [ ] **Phase 81: Milestone Close Aggregator** - Collect all per-phase verdict artifacts into a single aggregator so v2.13 completion is evaluable from harness output alone.

<details>
<summary>✅ v2.12 AI Agent Abstraction (Phases 71-75) — SHIPPED 2026-06-16</summary>

- [x] Phase 71: Engine-Agnostic Launch Productionization (5/5, ENG-01/02/03) — 2026-06-14
- [x] Phase 72: nono-py Binding + In-Process-Exec Proof (4/4, ABI-01/02) — 2026-06-14
- [x] Phase 73: AI_AGENT Marker (3/3, MARK-01) — verified 2026-06-16
- [x] Phase 74: Persistent Multi-Tenant Daemon (8/8, DMON-01/02/03) — 2026-06-15 *(marquee)*
- [x] Phase 75: Supplementary Controls + Secondary Engines (8/8, SUPP-01/02/03) — 2026-06-16

12/12 requirements satisfied; milestone audit PASSED. Engine-neutral confinement + persistent
multi-tenant daemon + unforgeable AI_AGENT marker + per-agent WFP/demote + nono-ts parity.
SC3 re-scope: Copilot confine-only (Node-ESM/AppContainer limit); claude-code is Engine-2.
Full detail: [`milestones/v2.12-ROADMAP.md`](milestones/v2.12-ROADMAP.md).

</details>

<details>
<summary>✅ v2.11 Clean-Host Distribution Cleanup + UPST8 (Phases 67-70) — SHIPPED 2026-06-13</summary>

- [ ] Phase 67: Clean-Host Windows Install (DIST-01/02, TRUST-01/02) — host-gated UAT carried to v2.13 Phase 80
- [x] Phase 68: macOS Resource-Limit Enforcement Fix (2/2) — completed 2026-06-12
- [x] Phase 69: UPST8 Audit (1/1) — completed 2026-06-13
- [x] Phase 70: UPST8 Cherry-pick Sync (3/3) — completed 2026-06-13

Phases 68/69/70 complete; Phase 67 carries to v2.13 Phase 80. Full detail: [`milestones/v2.11-ROADMAP.md`](milestones/v2.11-ROADMAP.md).

</details>

<details>
<summary>✅ v2.10 Kernel-Driver Spike + EDR UAT + macOS Upstream Parity (Phases 63-66) — SHIPPED 2026-06-11</summary>

- [x] Phase 63: Minifilter Spike Groundwork + macOS DIVERGENCE-LEDGER Audit (3/3) — completed 2026-06-08
- [x] Phase 64: Minifilter Spike Implementation + macOS P1 Cherry-pick Wave (5/5) — completed 2026-06-09
- [x] Phase 65: Minifilter ADR + macOS Live Re-validation (4/4) — completed 2026-06-11
- [x] Phase 66: WR-02 EDR HUMAN-UAT (1/1) — completed 2026-06-11

9/9 reqs satisfied. Full detail: [`milestones/v2.10-ROADMAP.md`](milestones/v2.10-ROADMAP.md).

</details>

<details>
<summary>✅ v2.8 / v2.9 (Phases 53-62) — SHIPPED 2026-06-06</summary>

v2.8 UPST7 + v2.7 Drain & Release (Phases 53-59, tags `v2.8`+`v0.57.5`); v2.9 Windows Sandbox-the-Tools (Phases 60-62, published `v0.62.2`). Full detail: [`milestones/v2.8-ROADMAP.md`](milestones/v2.8-ROADMAP.md) / [`milestones/v2.9-ROADMAP.md`](milestones/v2.9-ROADMAP.md).

</details>

## Phase Details

### Phase 76: Self-Verifying Harness Foundation
**Goal**: Deliver the shared scripted-gate framework — single-invocation unattended scripts that emit machine-readable pass/fail verdicts — so every subsequent host-gated phase can drop interactive human UAT in favor of a scripted run.
**Depends on**: Nothing (foundation phase)
**Requirements**: DARK-01
**Success Criteria** (what must be TRUE):
  1. A harness script (e.g., `scripts/verify-dark.ps1`) executes on a Win11 host with no interactive prompts and exits with a machine-readable verdict (JSON output + structured exit code).
  2. The framework defines a per-item gate contract: each feature gate is a named, self-contained invocation that emits a typed verdict (`PASS`, `FAIL`, or `SKIP_HOST_UNAVAILABLE`).
  3. The `--gate <name>` selector exercises one gate in isolation without running the full suite, enabling per-phase development and CI partial-run scenarios.
  4. Running the harness on a host that lacks a required precondition (e.g., daemon not running, Copilot CLI not installed) emits `SKIP_HOST_UNAVAILABLE` rather than crashing or producing ambiguous output.
  5. The harness self-check gate (`--gate harness-self-check`) exits 0 with a `PASS` verdict on any Win11 host, confirming the framework itself is functional before any feature gates run.
**Plans**: TBD
**Host gate**: Real Win11 host (the harness must execute without operator prompts to be validated).
**Unattended gate**: `scripts/verify-dark.ps1 --gate harness-self-check` — exits 0 with JSON `PASS` verdict; this IS the gate for Phase 76.

### Phase 77: Copilot CLI End-to-End Confinement
**Goal**: GitHub Copilot CLI completes a real task end-to-end under AppContainer confinement — replacing the v2.12 confine-only re-scope — by fixing the Node-ESM `realpathSync`/`lstat` ancestor `FILE_READ_ATTRIBUTES` denial and providing a verified idempotent one-time-admin setup step for the system-ancestor ACL grants (`C:\`, `C:\Users`).
**Depends on**: Phase 76 (scripted gate for CPLT-03)
**Requirements**: CPLT-01, CPLT-02, CPLT-03
**Success Criteria** (what must be TRUE):
  1. `nono run --profile copilot-cli -- gh copilot suggest "list files"` completes and prints a suggestion without any `STATUS_ACCESS_DENIED` error or Node module-resolution crash under AppContainer.
  2. nono grants `FILE_READ_ATTRIBUTES` on every ancestor in the confined target's package SID path (up to drive root) at launch time, so Node-ESM `realpathSync`/`lstat` ancestor-chain walking succeeds under AppContainer.
  3. An idempotent `nono setup --copilot-ancestors` step (or equivalent CLI command) grants the package-SID RA on `C:\` and `C:\Users` — one-time-admin, documented as non-destructive (does not alter DACL deny entries or remove existing ACEs) — and is safe to run repeatedly.
  4. The Copilot end-to-end gate (`scripts/verify-dark.ps1 --gate copilot-e2e`) emits `PASS` on a host where the one-time-admin step has been run, with no operator interaction beyond that single invocation.
**Plans**: TBD
**Host gate**: Real Win11 host + GitHub Copilot CLI installed + one-time-admin step runnable.
**Unattended gate**: `scripts/verify-dark.ps1 --gate copilot-e2e` — replaces the interactive SC3 UAT from v2.12 Phase 75.

### Phase 78: Cross-Process Classification
**Goal**: An operator can authoritatively classify any running PID as `AI_AGENT` (or not) via `nono classify <pid>`, answered cross-process by the `nono-agentd` daemon control-pipe `Classify` verb, with the same caller-gating and SDDL posture as the existing control loop.
**Depends on**: Phase 74 daemon (already shipped — `nono-agentd` serves `\\.\pipe\nono-agentd-control`)
**Requirements**: CLAS-01, CLAS-02
**Success Criteria** (what must be TRUE):
  1. `nono classify <pid>` of a daemon-launched confined agent returns `AiAgent` from a separate non-daemon process without requiring the caller to have elevated privileges.
  2. `nono classify <pid>` of a non-agent PID (e.g., `notepad.exe`) returns `NotAnAgent` with no false-positive.
  3. The `Classify` verb enforces the same Low-IL-denying SDDL posture as the existing `nono-agentd-control` pipe — a caller running at Low IL is denied with a clear error rather than receiving a spoofable answer.
  4. A tenant cannot learn the classification of another tenant's agent: the `Classify` response for a PID the caller did not launch contains no cross-tenant SID disclosure.
**Plans**: TBD
**Host gate**: Real Win11 host with `nono-agentd` running (daemon from Phase 74 — already proven on Win11 26200).
**Unattended gate**: `cargo test --bin nono-agentd -- classify` exercises the cross-process path as a scripted gate; no interactive prompts required.

### Phase 79: WFP Egress Isolation + nono-ts Ergonomics
**Goal**: Prove per-agent WFP egress isolation empirically via an automated two-agent test (allowed vs. denied egress on the same host), and ship `confinedRun` ergonomics in nono-ts so callers get a working confined run with no manual profile or coverage flags.
**Depends on**: Phase 76 (WFP isolation gate uses harness framework); Phase 74 daemon (per-agent WFP is daemon-keyed — already shipped in Phase 75)
**Requirements**: WFP-01, TSRG-01
**Success Criteria** (what must be TRUE):
  1. An automated gate (`scripts/verify-dark.ps1 --gate wfp-egress-isolation`) launches two confined agents with distinct AppContainer package SIDs under a network-scoped profile — agent A's allowed egress to a local mock server succeeds while agent B (no `allow_domain`) is denied — both verdicts machine-verifiable in one unattended run.
  2. The `nono-ts` `confinedRun` function defaults to the Low-IL broker arm (`windows_low_il_broker: true`) without requiring the caller to set any profile flag, so `confinedRun({ target: "node" })` produces a confined child at Low IL out of the box.
  3. `confinedRun` auto-covers the target executable's directory (adds the directory of `target` to the allowed-read paths) so a caller does not need to manually specify coverage for the target binary.
  4. A nono-ts integration test (`confinedRun` with no profile flags on Windows) confirms the default-broker-arm path works end-to-end and passes in the napi build on the Win11 dev host.
**Plans**: TBD
**Host gate**: Real Win11 host (WFP gate requires live kernel WFP enforcement; nono-ts build requires Node + napi-rs on Windows).
**Unattended gate**: `scripts/verify-dark.ps1 --gate wfp-egress-isolation` for WFP-01; `npm test` or equivalent napi integration for TSRG-01.

### Phase 80: Clean-Host Install UAT
**Goal**: Verify the machine MSI installs and runs cleanly on a fresh Win11 host with no manual prerequisite steps, closing the Phase 67 v2.11 carry-forward with an unattended scripted gate rather than an interactive human UAT.
**Depends on**: Phase 76 (clean-host harness gate); no dependency on Phases 77-79.
**Requirements**: INST-01
**Success Criteria** (what must be TRUE):
  1. On a fresh Win11 host (no VC++ redistributable, no prior nono install, no pre-trusted cert), `msiexec /i nono-machine.msi /quiet` completes with exit code 0 and `nono --version` runs from a new PowerShell session.
  2. `nono-wfp-service` starts successfully or, if it fails transiently, the failure is non-fatal and does not roll back the MSI install — the machine is in a usable state regardless.
  3. The clean-host gate (`scripts/verify-dark.ps1 --gate clean-host-install`) runs on the clean host with no operator prompts and emits a machine-readable verdict.
  4. The gate emits `PASS` on a host that has never had nono installed, confirming the VC++ static-CRT or bundled-redist handling from Phase 67 is correctly packaged in the MSI.
**Plans**: TBD
**Host gate**: Clean Win11 host (no prior nono, no VC++ runtime, no pre-trusted cert — this is the definition of the gate).
**Unattended gate**: `scripts/verify-dark.ps1 --gate clean-host-install` on the clean host — single unattended invocation replaces the Phase 67 interactive UAT.

### Phase 81: Milestone Close Aggregator
**Goal**: Collect all per-phase verdict artifacts into a single milestone-close aggregator so v2.13 completion is evaluable from harness output alone — no human interpretation step required.
**Depends on**: Phases 76, 77, 78, 79, 80 (aggregates verdicts from all prior phases)
**Requirements**: DARK-02
**Success Criteria** (what must be TRUE):
  1. A single invocation (`scripts/verify-dark.ps1` with no gate selector, or `--all`) runs all registered gates in sequence and emits a structured summary (JSON with per-gate `PASS`/`FAIL`/`SKIP_HOST_UNAVAILABLE` verdicts and an overall milestone verdict).
  2. The aggregator's overall verdict is `PASS` only when every required gate emits `PASS`; any `FAIL` results in an overall `FAIL`; `SKIP_HOST_UNAVAILABLE` on host-gated items produces a qualified `PASS_WITH_SKIPS` that is still actionable (no ambiguity about why the skip occurred).
  3. The aggregator's output is consumable by a CI pipeline: exit 0 for `PASS`/`PASS_WITH_SKIPS`, non-zero for `FAIL` — no post-processing or human judgment required.
  4. The v2.13 close checklist is evaluable by pointing at the aggregator's output artifact — no ad-hoc per-phase SUMMARY.md scanning required.
**Plans**: TBD
**Host gate**: Same Win11 host that ran the individual gates (aggregator consumes their stored output).
**Unattended gate**: `scripts/verify-dark.ps1` (no flags) — this IS the aggregator. Its exit code and JSON output are the milestone completion signal.

## Progress Table

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 76. Self-Verifying Harness Foundation | 0/? | Not started | - |
| 77. Copilot CLI End-to-End Confinement | 0/? | Not started | - |
| 78. Cross-Process Classification | 0/? | Not started | - |
| 79. WFP Egress Isolation + nono-ts Ergonomics | 0/? | Not started | - |
| 80. Clean-Host Install UAT | 0/? | Not started | - |
| 81. Milestone Close Aggregator | 0/? | Not started | - |

## References

- `.planning/PROJECT.md` — project context + current milestone scope.
- `.planning/MILESTONES.md` — shipped milestone history (v1.0 → v2.12).
- `.planning/milestones/v2.12-ROADMAP.md` — archived v2.12 roadmap (Phases 71-75); Phase 75 § SC3 + 75-08 is the Copilot carry-forward origin.
- `.planning/templates/cross-target-verify-checklist.md` — mandatory Linux+macOS clippy protocol for cfg-gated Unix code.
- `proj/DESIGN-engine-abstraction.md` — E1-E5 engine-abstraction contract (Phase 72).
