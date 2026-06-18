---
gsd_state_version: 1.0
milestone: v2.13
milestone_name: Carry-Forward Closeout (Dark Factory)
status: verifying
last_updated: "2026-06-18T13:31:43.379Z"
last_activity: 2026-06-18
progress:
  total_phases: 6
  completed_phases: 6
  total_plans: 13
  completed_plans: 13
  percent: 100
---

# Project State: nono — v2.13 Carry-Forward Closeout (Dark Factory)

## Project Reference

See: `.planning/PROJECT.md` (v2.13 milestone started 2026-06-17; v2.12 Phases 71-75 complete, shipped + archived). Phase numbering continues from Phase 75 (Phases 76-81 — NOT reset to 1).

**Core Value:** Windows security must be as structurally impossible and feature-complete as Unix platforms — and that confinement must apply to *any* AI agent engine, not just Claude Code.

**Current Focus:** Phase 81 — milestone-close-aggregator

## Current Position

Phase: 81 (milestone-close-aggregator) — EXECUTING

- 78-01 (wave 1, autonomous): daemon `ControlRequest::Classify` verb + `handle_classify` against the shared `agent_registry`; pure `classify_response_string` (verdict-only, NO package SID — SC4); unit gate `cargo test --bin nono-agentd -- classify`. **COMPLETE** (`aaafe4ff`).
- 78-02 (wave 2): `classify_daemon_request` + daemon-first `app_runtime.rs` dispatch + structural fallback; `windows_control_pipe_request`/`is_pipe_not_found` promoted to `pub(crate)`; SC1/SC2/SC4 integration test (gated `NONO_DAEMON_INTEGRATION_TESTS=1`); live-daemon host PASS on Win11 26200. **COMPLETE** (`0f8cdeb7`, `ad284903`).
- CLAS-01/CLAS-02 both satisfied. SC1 (AiAgent cross-process, NON-optional) + SC2 + SC3 (VERIFIED_BY_SDDL) + SC4 (no SID) all PASS.
- Key decision: integration test helper re-implements pipe framing (Rust pub(crate) not visible to integration test binaries); daemon-absent uses "daemon-absent" sentinel string for fallback routing.
- Cross-target clippy: PARTIAL — cfg(windows)-only new code; deferred to CI per CLAUDE.md rule.

---
Phase: 77 (copilot-cli-end-to-end-confinement) — ✅ COMPLETE + VERIFIED (passed, 2026-06-17)
Plan: 1 of 1
Status: Phase complete — ready for verification
Last activity: 2026-06-18
NEXT: /gsd:plan-phase 78  (Cross-Process Classification — CLAS-01/02; independent of 77/79/80, depends on Phase 74 daemon)

**Phase 77 close notes (durable):**

- CPLT-01 RA guard now walks BOTH the binary chain AND the `--workspace` chain (`snapshot_and_apply_targets`, dedup, per-chain D-04 stop) — `dacl_guard.rs` + wired in `mod.rs`.
- CPLT-02 `nono setup --grant-ancestors` (ALL APPLICATION PACKAGES `S-1-15-2-1`, RA-only on `C:\`+`C:\Users`) verified working + idempotent on live Win11; the grant is durable (persists on the host).
- The gate `scripts/gates/copilot-e2e.ps1` was hardened live: killed a critical false-PASS (PASSed on nono "Profile not found"), added WinGet exe + node-interpreter coverage resolution, `--workspace` + R-B3 `/setowner` ownership, `--allow-all-tools` (Copilot's `-p` alone hangs), and org-policy → SKIP detection.
- **Literal green `copilot-e2e` PASS requires a GitHub account/host where Copilot CLI is org-enabled** (carry-forward — not a nono defect; the gate emits PASS there, SKIP on org-restricted).
- On Win11 dev host the gate must run with the fresh release build on PATH (`target\release` prepended) — the installed `C:\Program Files\nono\nono.exe` was a stale v0.57.5 without the copilot-cli profile.

### v2.13 Phase Summary (active)

| Phase | Goal | Requirements | SC | Status | Host gate | Unattended gate |
|-------|------|--------------|----|--------|-----------|-----------------|
| 76 | Self-Verifying Harness Foundation — build the scripted-gate framework; all host-gated phases depend on it | DARK-01 | 5 | ⬜ Not started | Real Win11 host | `verify-dark.ps1 --gate harness-self-check` |
| 77 | Copilot CLI End-to-End Confinement — fix Node-ESM ancestor RA + one-time-admin setup + scripted proof | CPLT-01, CPLT-02, CPLT-03 | 4 | ✅ Complete (verified; CPLT-03 host-PASS = reasoned SKIP, org-policy) | Win11 + Copilot CLI + admin | `verify-dark.ps1 --gate copilot-e2e` |
| 78 | Cross-Process Classification — daemon Classify verb + caller-gating + tenant safety | CLAS-01, CLAS-02 | 4 | ✅ Complete (SC1/SC2/SC3/SC4 PASS, Win11 26200 host-verified 2026-06-17) | Win11 + nono-agentd running | `cargo test --bin nono-agentd -- classify` |
| 79 | WFP Egress Isolation + nono-ts Ergonomics — empirical two-agent WFP test + confinedRun defaults | WFP-01, TSRG-01 | 4 | ⬜ Not started | Win11 + Node/napi | `verify-dark.ps1 --gate wfp-egress-isolation` |
| 80 | Clean-Host Install UAT — MSI installs clean on fresh Win11 host via scripted gate | INST-01 | 4 | ⬜ Not started | Clean Win11 host (no prior nono) | `verify-dark.ps1 --gate clean-host-install` |
| 81 | Milestone Close Aggregator — collect all verdicts into one unattended close signal | DARK-02 | 4 | ⬜ Not started | Win11 (after all prior gates run) | `verify-dark.ps1` (no flags) |

**Dependencies:**

- **76 FIRST** (harness foundation — Phases 77, 79, 80 depend on it for their scripted gates).
- **77** depends on Phase 76. Copilot end-to-end uses the harness for its gate script.
- **78** is independent of 77/79/80 — depends only on Phase 74 daemon (already shipped). Can run in parallel with 77/79/80.
- **79** depends on Phase 76 (WFP gate uses harness); TSRG-01 is independent but natural companion to WFP-01.
- **80** depends on Phase 76 (clean-host gate uses harness). No dependency on 77/79.
- **81 LAST** (aggregator — depends on all prior phases having run their gates).

### Host-availability gates (v2.13)

| Phase | Gate | Notes |
|-------|------|-------|
| 76 | Real Win11 host | Harness itself must run on Win11 to validate no-prompt behavior. |
| 77 | Win11 + Copilot CLI + one-time-admin | `gh copilot suggest` must be installed; one-time-admin RA grant runnable. |
| 78 | Win11 + nono-agentd running | Daemon from Phase 74 must be live; no extra host requirements. |
| 79 | Win11 + Node/napi-rs build | WFP kernel enforcement + nono-ts napi build both require live Win11. |
| 80 | Clean Win11 host | No prior nono install, no VC++ redist — the definition of the gate. |
| 81 | Win11 (same host) | Aggregator runs after per-phase gates have deposited verdict artifacts. |

## Key Decisions

### v2.13 decisions

| Decision | Phase | Rationale |
|----------|-------|-----------|
| Phase 76 (DARK-01 harness) is the foundation phase — precedes all other phases | all | Every host-gated item (Copilot, WFP, clean-host) depends on the harness for its scripted gate. Building the framework first lets Phases 77/79/80 write gates against a stable contract rather than each reinventing a one-off script. |
| DARK-01 and DARK-02 are split into their own phases (76 and 81) | 76, 81 | DARK-01 is infrastructure that other phases consume; coupling it to a feature phase would make that phase's gate depend on half-built infrastructure. DARK-02 (aggregator) is definitionally last and a poor co-traveler with any feature work. |
| CLAS-01/02 (Phase 78) is NOT harness-gated | 78 | Cross-process classification is verifiable via `cargo test --bin nono-agentd` without an interactive harness. The daemon test binary (established in Phase 74) is the unattended gate. |
| WFP-01 and TSRG-01 are in the same phase (79) | 79 | Both are supplementary/tooling items with no hard dependency between them; both require a Win11 host with napi/node; shipping them together keeps the phase count at a manageable 6 for 10 requirements and avoids a single-requirement phase for either. |
| Phase 80 (clean-host) has no dependency on Phases 77/79 | 80 | A clean-host install gate depends only on the MSI (already built in Phase 67) and the harness (Phase 76). Copilot confinement and nono-ts ergonomics are irrelevant to whether the MSI installs clean on a fresh host. |
| DARK-02 aggregator is Phase 81 (last) | 81 | The aggregator collects verdicts from all prior phases; it cannot be correct until all gates have been defined and run. |

### Key Decisions (carried from v2.12 — still load-bearing)

- **Dark Factory mandate:** every historically host-gated item ships a scripted unattended gate that emits a machine-readable verdict. Interactive human UAT is NOT a valid verification mechanism for v2.13.
- **Composition over green-field:** no new wire protocol, no new frameworks; Phase 74 daemon + Phase 62 WFP + Phase 75 nono-ts napi-2 shapes are all proven. Deltas are a Classify verb, RA ancestor grant, napi ergonomics, and a PowerShell harness.
- **Cross-target clippy (Linux + macOS) is a MUST** per CLAUDE.md for any cfg-gated Unix code touched; Windows dev host cannot compile cfg(unix); CI is the load-bearing signal. Phase 77 (ancestor RA grant) and Phase 78 (Classify verb) touch Windows-only code — verify CI stays green.
- **Repo MUST stay PUBLIC** until Microsoft approves the minifilter altitude (verify no `build_notes/`/`.gsd/` staged before any push).

## Accumulated Context

### Codebase facts carried into v2.13 (do not re-derive)

- Daemon `nono-agentd` already serves `\\.\pipe\nono-agentd-control` with Low-IL-denying SDDL (Phase 74). CLAS adds a `Classify` verb to that existing `control_loop.rs`.
- AppContainer/Low-IL broker arm (`windows_low_il_broker:true`), WFP kernel enforcement (Phase 62), and AI_AGENT marker (Phase 73, daemon-minted AppContainer SID) are all shipped and proven.
- nono-ts win32 napi loader + cfg(windows) exports must be regenerated on Windows (napi-rs 2 kept; pyo3 0.28 kept; windows-sys 0.59 kept — no dep bumps planned).
- Per-agent WFP add@launch/remove@reap is already in Phase 75 code (`SUPP-02`); WFP-01 needs a network-scoped test profile + empirical isolation test, not a new WFP integration.
- The Node-ESM `realpathSync lstat('C:\')` root cause is documented in spike 75-08: `FILE_READ_ATTRIBUTES` is denied for every ancestor up to drive root; fix = nono grants ancestor-chain RA at launch + one-time-admin `C:\`/`C:\Users` ACL grant (package-SID, non-destructive).

### Pitfall guards carried into v2.13

- **AppContainer per-agent SID:** `CreateAppContainerProfile`, NOT derive-only (else `CreateProcessW` `ERROR_FILE_NOT_FOUND`). (memory `windows_appcontainer_wfp_validated`)
- **Env baseline for CLR/PowerShell children:** preserve `SystemRoot`/`windir`/`SystemDrive` (else CLR `0xFFFF0000`). (memory `windows_hook_interpreter_spawn_gotchas`)
- **Cap-pipe DACL handshake:** Low-IL/AppContainer rendezvous (package-SID READ grant before the blocking `ConnectNamedPipe`). (memory `windows_appcontainer_cap_pipe_reachability`)
- **`nono classify` in-process vs cross-process:** Phase 73 shipped in-process structural classify (non-authoritative by design). Phase 78's `Classify` verb is the authoritative cross-process path via the daemon pipe. Never conflate the two.

## Deferred Items

### v2.12 carry-forwards (resolved in v2.13)

All four are now tracked as v2.13 phases:

- Copilot CLI end-to-end → Phase 77 (CPLT-01/02/03)
- A1 empirical WFP isolation → Phase 79 (WFP-01)
- Cross-process authoritative classify → Phase 78 (CLAS-01/02)
- nono-ts `confinedRun` ergonomics → Phase 79 (TSRG-01)
- Phase 67 clean-host install → Phase 80 (INST-01)

### Explicit v2.13 out-of-scope (deferred)

- **UPST9 upstream sync** — deferred to its own later cadence.
- **Enterprise-hardening horizon** — SEED-001/002/003/005, Azure Trusted Signing — separate enterprise milestone.
- **WR-02 EDR HUMAN-UAT / Gap 6b production kernel driver** — v3.0 deferrals re-affirmed.

### Historical (acknowledged at prior closes)

Prior-close audit-open backlogs (v2.12: carry-forwards resolved above; v2.10: 65 items; v2.9/v2.8: 55; v2.7: 45) — mostly pre-v2.5 `missing` quick-task slugs + historical UAT/verification bookkeeping. Carried, none blocking.

## Session Continuity

**Last session:** 2026-06-18T13:31:43.362Z

**v2.13 roadmap created (2026-06-17):** 6 phases (76-81), 10/10 requirements mapped. ROADMAP.md + REQUIREMENTS.md traceability + STATE.md updated. Build order: 76 (foundation) → 77/78/79/80 (78 is independent of harness; 77/79/80 depend on 76) → 81 (aggregator, last). Dark Factory mandate: every host-gated item has an unattended scripted gate as its verification mechanism.

**Predecessor context (carried):** v2.12 shipped + archived 2026-06-16 (12/12 reqs, audit PASSED, tag `v2.12`). PR #8 merged to main (`05c489b6`). PR #9 (`fix/ci-green-all-targets`) merged to main (`be0bca05`) — Clippy(ubuntu+macos)+Rustfmt now GREEN on main.

## Operator Next Steps

- **NEXT: `/gsd:plan-phase 76`** — Phase 76: Self-Verifying Harness Foundation.
- Before any push: confirm no `build_notes/`/`.gsd/` staged — repo stays PUBLIC pending Microsoft minifilter-altitude approval.
- Current branch is `fix/ci-green-all-targets` (PR #9). Ensure you're on a clean branch for v2.13 work.

## Quick Tasks Completed

| Date | Slug | Outcome |
|------|------|---------|
| 2026-06-15 | zt-infra-e5-poc-runbook | Advisory + docs: zt-infra integration is dormant SEED-005 (P3). Added `proj/POC-zt-infra-e5-local-provisioner.md`. Commits `0f5f3b93`+`1cd0e996`. |
