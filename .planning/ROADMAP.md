---
milestone: v3.0
milestone_name: Enterprise Hardening I (Deploy, Control, Compliance)
status: in_progress
created: 2026-06-18
last_updated: 2026-06-18
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
- ✅ **v2.13 Carry-Forward Closeout (Dark Factory)** — Phases 76-81 (shipped 2026-06-18; tag `v2.13`) — see [`milestones/v2.13-ROADMAP.md`](milestones/v2.13-ROADMAP.md)
- 🚧 **v3.0 Enterprise Hardening I (Deploy · Control · Compliance)** — Phases 82-84 (in progress)

## Phases

<details>
<summary>✅ v2.13 Carry-Forward Closeout (Dark Factory) (Phases 76-81) — SHIPPED 2026-06-18</summary>

- [x] Phase 76: Self-Verifying Harness Foundation (2/2, DARK-01) — 2026-06-17
- [x] Phase 77: Copilot CLI End-to-End Confinement (4/4, CPLT-01/02/03) — 2026-06-17
- [x] Phase 78: Cross-Process Classification (2/2, CLAS-01/02) — 2026-06-18
- [x] Phase 79: WFP Egress Isolation + nono-ts Ergonomics (2/2, WFP-01/TSRG-01) — 2026-06-18
- [x] Phase 80: Clean-Host Install UAT (2/2, INST-01) — 2026-06-18
- [x] Phase 81: Milestone Close Aggregator (1/1, DARK-02) — 2026-06-18

10/10 requirements satisfied; milestone audit `tech_debt` (no requirement gaps, 0 wiring defects;
host-execution deferrals only). Dark Factory mandate met: every host-gated item collapses to a
single unattended `scripts/verify-dark.ps1` gate; the no-flag aggregator (`_aggregate.json`) is the
machine-readable close signal. Full detail: [`milestones/v2.13-ROADMAP.md`](milestones/v2.13-ROADMAP.md).

</details>

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

- [ ] Phase 67: Clean-Host Windows Install (DIST-01/02, TRUST-01/02) — host-gated UAT closed in v2.13 Phase 80
- [x] Phase 68: macOS Resource-Limit Enforcement Fix (2/2) — completed 2026-06-12
- [x] Phase 69: UPST8 Audit (1/1) — completed 2026-06-13
- [x] Phase 70: UPST8 Cherry-pick Sync (3/3) — completed 2026-06-13

Phases 68/69/70 complete; Phase 67 carried to v2.13 Phase 80. Full detail: [`milestones/v2.11-ROADMAP.md`](milestones/v2.11-ROADMAP.md).

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

### 🚧 v3.0 Enterprise Hardening I (Deploy · Control · Compliance) (In Progress)

**Milestone Goal:** Make nono deployable and governable across a corporate Windows fleet — silent fleet install via `msiexec /qn`, machine-policy-managed deny-by-default egress from a single `HKLM\SOFTWARE\Policies\nono` spine, and SIEM/EDR-forwardable security telemetry — all under the Dark Factory verification mandate.

- [x] **Phase 82: Fleet Deployment Infrastructure** — Silent MSI install, machine-wide PATH, ProgramData root, cert install, health command, Event Log source registration
 (completed 2026-06-18)
- [ ] **Phase 83: Machine Policy Spine + Egress Control** — HKLM reader (fail-secure), proxy+WFP unified from one source, ADMX template, AI-provider presets, DNS-component wildcard matching
- [ ] **Phase 84: SIEM/EDR Telemetry** — SecurityEventLayer tracing::Layer, structured Event Log events, HMAC tamper-evidence chain, secret redaction, tamper-evidence ADR

## Phase Details

### Phase 82: Fleet Deployment Infrastructure
**Goal**: An admin can silently install nono fleet-wide via `msiexec /qn /norestart` and every subsequent `nono run` works with no manual steps — machine-wide PATH, auto-provisioned user scratch space, trusted cert, health command, and the Event Log source registered at install
**Depends on**: Nothing (Phase 82 — continues from v2.13 Phase 81)
**Requirements**: DEPLOY-01, DEPLOY-02, DEPLOY-03, DEPLOY-04, DEPLOY-05, DEPLOY-06
**Success Criteria** (what must be TRUE):
  1. `msiexec /i nono.msi /qn /norestart` exits 0 (or 3010 on reboot-required) with no interactive prompts when run under a non-admin test account via ALLUSERS=1, and `nono health` reports install state in machine-readable JSON
  2. Any user can invoke `nono` from a new shell after MSI install with no per-user PATH edit; the machine-wide PATH entry is present in `HKLM\SYSTEM\CurrentControlSet\Control\Session Manager\Environment`
  3. On first `nono run` in a user context, a user-owned scratch workspace is auto-provisioned (not SYSTEM-owned) — the R-B3 user-ownership guard passes without a manual `nono setup` step
  4. The POC root certificate is silently installed into both `LocalMachine\Root` and `CurrentUser\Root` stores; TLS through the nono proxy succeeds from PowerShell (CryptoAPI), Node.js, and nono-cli (rustls/native-certs) with no manual cert import
  5. `verify-dark.ps1 --gate deploy-silent-install` emits a PASS verdict, covering: silent install under SYSTEM context, workspace owned by target user not SYSTEM, degraded-service path produces non-zero `nono health`, and TLS trust paths verified across all three client types
**Plans**: 4 plans
  - [x] 82-01-PLAN.md — MSI machine-global provisioning: ProgramData root, HKLM sentinel key, non-fatal cert CA (Root+TrustedPublisher), PATH/service/CRT verify, ADMX template (DEPLOY-01/02/04/05/06)
  - [x] 82-02-PLAN.md — First-run user-context provisioner: user-owned WRITE_OWNER scratch + CurrentUser\Root cert + NODE_EXTRA_CA_CERTS, reusable cert-import logic (DEPLOY-03/05)
  - [x] 82-03-PLAN.md — nono health: read-only tri-state JSON verdict (install/WFP/policy/scratch+cert+PATH) (DEPLOY-06)
  - [x] 82-04-PLAN.md — deploy-silent-install dark-factory gate: silent install, scratch ownership, degraded-health, three-client TLS (DEPLOY-01/02/03/05/06)
**UI hint**: no

### Phase 83: Machine Policy Spine + Egress Control
**Goal**: An admin can push a deny-by-default outbound egress allowlist to a fleet via GPO ADMX or Intune OMA-URI; every confined agent's traffic is filtered at both the proxy (L7) and kernel WFP (L3/4) layers from the same deserialized source, with no allowlist drift possible between layers
**Depends on**: Phase 82
**Requirements**: POLICY-01, POLICY-02, POLICY-03, EGRESS-01, EGRESS-02, EGRESS-03, EGRESS-04
**Success Criteria** (what must be TRUE):
  1. nono reads `HKLM\SOFTWARE\Policies\nono` at process/daemon startup using `KEY_WOW64_64KEY`; when the key is present it overrides the per-user profile; when the key is absent nono falls through to per-user config normally
  2. A deliberately unreadable (permission-denied) machine-policy key causes nono to abort with a typed `NonoError::PolicyLoadFailed` — it does not fall through to a permissive per-user state; `verify-dark.ps1 --gate egress-policy-deny` asserts non-zero exit on the corrupted-key path
  3. Injecting an HKLM allowlist containing only `*.anthropic.com` causes the nono-proxy to reject a request to an out-of-list domain AND the nono-wfp-service to block the corresponding AppContainer SID from reaching that domain — both layers verified from the same `MachineEgressPolicy` struct, not independent reads
  4. Wildcard FQDN matching uses DNS-component comparison: `api.anthropic.com` matches `*.anthropic.com`, but `anthropic.com` and `evilanthropic.com` and `anthropic.com.evil.com` are all correctly rejected
  5. The shipped GPO ADMX template (`nono.admx` + `nono.adml`) and documented Intune OMA-URI path let an admin push the egress allowlist to `HKLM\SOFTWARE\Policies\nono` with no manual registry editing; AI-provider built-in groups (`*.anthropic.com`, `*.openai.com`, `api.github.com`) are available as named presets
**Plans**: 4 plans
  - [x] 83-01-PLAN.md — Core lib spine: MachineEgressPolicy type + NonoError::PolicyLoadFailed + winreg fail-secure 64-bit reader + SC-4 DNS-component matrix (POLICY-01/02, EGRESS-03) — 2026-06-18
  - [x] 83-02-PLAN.md — Single-source hand-off: one daemon-startup read -> ProxyFilter deny-by-default + flip wfp_filter_add to force-through-proxy (POLICY-03, EGRESS-01/02)
  - [x] 83-03-PLAN.md — AI-provider presets + ADMX named toggles: egress groups in network-policy.json + token->FQDN expansion + GPO toggle here-strings (EGRESS-04)
  - [ ] 83-04-PLAN.md — Dark Factory gate egress-policy-deny (SC-2 corrupted-key non-zero exit; SC-3 dual-layer deny) + cross-target clippy verification (POLICY-02, EGRESS-02)
**UI hint**: no

### Phase 84: SIEM/EDR Telemetry
**Goal**: Every blocked or denied action (path-deny, network-deny, label-violation, hook fail-closed) is emitted as a structured security event to the Windows Application Event Log with named EventData fields, HMAC-chained within the session for tamper-evidence, and scrubbed of secrets and full paths — readable by Splunk and Microsoft Sentinel without custom parsers
**Depends on**: Phase 83
**Requirements**: TELEM-01, TELEM-02, TELEM-03, TELEM-04
**Success Criteria** (what must be TRUE):
  1. After a clean-host MSI install (no prior `wevtutil im`), triggering a sandbox denial produces an entry in the Windows Application Event Log under the nono source with a distinct EventID (10001-10005) and named EventData fields (`EventType`, `AgentPid`, `PathHash`, `Host`, `SessionId`, `ChainHead`) that Splunk `XmlWinEventLog` and Sentinel parse as columns
  2. The in-session HMAC-SHA256 chain field (`ChainHead`) is present in each event; the ADR explicitly records that the tamper boundary is Windows Event Forwarding to an external SIEM (external copy out of local attacker reach) and that cross-session/cryptographic-local anchoring is deferred to SEED-005
  3. No security event body contains a raw file path, full URL, or credential value — a blocked-action event for `C:\Users\alice\secret.txt` contains a hashed path identifier and a category tag (`workspace_file`), not the literal path
  4. The emitter is implemented as a `tracing::Layer` in `nono-cli/src/telemetry/` (not in the `nono` library's `DiagnosticFormatter`), registered in `init_tracing()`, and reads its channel/level config from the machine policy struct
  5. `verify-dark.ps1 --gate telemetry-event-emit` emits a PASS verdict, covering: clean-host event appearance in Application log with correct EventID and named fields, absence of raw path strings in the event body, and ETW provider emission detectable via `logman`
**Plans**: TBD
**UI hint**: no

## Progress

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 82. Fleet Deployment Infrastructure | 4/4 | Complete   | 2026-06-18 |
| 83. Machine Policy Spine + Egress Control | 3/4 | In Progress|  |
| 84. SIEM/EDR Telemetry | 0/TBD | Not started | - |

## References

- `.planning/PROJECT.md` — project context + current milestone scope.
- `.planning/MILESTONES.md` — shipped milestone history (v1.0 → v2.13).
- `.planning/REQUIREMENTS.md` — v3.0 requirements (DEPLOY-01..06, POLICY-01..03, EGRESS-01..04, TELEM-01..04).
- `.planning/research/SUMMARY.md` — four-researcher consensus on stack, pitfalls, and build order.
- `.planning/research/ARCHITECTURE.md` — integration points (machine.rs, telemetry/, ProxyConfig injection, nono-agentd capability builder).
- `.planning/research/PITFALLS.md` — 13 pitfalls with per-phase prevention mapping.
- `.planning/milestones/v2.13-ROADMAP.md` — archived v2.13 roadmap (Phases 76-81) with full phase details + success criteria.
- `.planning/templates/cross-target-verify-checklist.md` — mandatory Linux+macOS clippy protocol for cfg-gated Unix code.
- `proj/DESIGN-engine-abstraction.md` — E1-E5 engine-abstraction contract (Phase 72).
