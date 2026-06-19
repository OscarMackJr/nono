---
gsd_state_version: 1.0
milestone: v3.1
milestone_name: UPST9 Upstream Sync (v0.62-v0.64) + v3.0 Drain
status: ready_to_plan
stopped_at: Phase 85 context gathered
last_updated: "2026-06-19T15:48:20.375Z"
last_activity: 2026-06-19 -- Phase 85 execution started
progress:
  total_phases: 6
  completed_phases: 1
  total_plans: 1
  completed_plans: 0
  percent: 17
---

# Project State: nono — v3.1 UPST9 Upstream Sync (v0.62→v0.64) + v3.0 Drain

## Project Reference

See: `.planning/PROJECT.md` (v3.1 milestone started 2026-06-19; v3.0 Phases 82-84 complete, shipped + archived; tag `v3.0` local). Phase numbering continues from Phase 84 (Phases 85–90 — NOT reset to 1). Scope source: `.planning/seeds/SEED-006-upst9-v0.62-v0.64-sync-window.md`. Roadmap: `.planning/ROADMAP.md`.

**Core Value:** Windows security must be as structurally impossible and feature-complete as Unix platforms — kept current with upstream `always-further/nono` without regressing the fork's Windows security model.

**Current Focus:** Phase 85 — UPST9 Divergence Audit

## Current Position

Phase: 86
Plan: Not started
Status: Ready to plan
Last activity: 2026-06-19

Progress: [______________________________] 0/6 phases

## Performance Metrics

**Velocity:** (v3.1 — reset; populated as phases complete)

- Total plans completed: 1
- Average duration: —
- Total execution time: —

| Phase | Plan | Duration | Tasks | Files |
|-------|------|----------|-------|-------|

*Updated after each plan completion*

## Accumulated Context

### Decisions (v3.1 roadmap)

| Decision | Phase | Rationale |
|----------|-------|-----------|
| Audit first (Phase 85) gates every cherry-pick | 85 | DIVERGENCE-LEDGER dispositions for themes A–M gate all downstream sync work (Phase 42→43, 47→48 precedent). |
| Library-boundary convergence is its own phase, sequenced right after the audit | 86 | ~2200 LOC of audit + structured-diagnostics moved into the core `nono` crate, adopt-upstream; highest-risk cluster, touches FFI + Windows diagnostic paths + proxy `ProxyDiagnostic`. SEC/FEAT/PROXY diagnostics-touching work depends on it landing first. |
| Security fix (SEC-01/02) its own phase, flagged security-priority | 87 | AF_UNIX datagram bypass (#1096) + procfs-remap dedup guard (#1064); cfg-gated Unix edits require cross-target clippy. |
| FEAT + DEPS folded into one additive cherry-pick wave | 88 | All additive, low-conflict; PTY ctrl-z fix (DEPS-01) + 9 dep bumps (DEPS-02) ride with them; `make ci` is the single gate. |
| PROXY its own phase, diff-inspect-careful | 89 | PROXY-02 touches the fork-divergent TLS-interception surface (Phase 34 C11 `fork-preserve`); depends on Phase 86 ProxyDiagnostic + Phase 88 AWS/customCredentials. |
| DRAIN independent (Phase 90), runs last/parallel | 90 | v3.0 host-gated UAT debt; DRAIN-04 (daemon SecurityEventLayer wiring) is real code, others are scripted-gate + operator-gated live UAT. |

### Decisions (v3.0)

| Decision | Phase | Rationale |
|----------|-------|-----------|
| Build order is deployment → policy spine → telemetry | 82→83→84 | MSI provisions the HKLM sentinel key and Event Log source that Phases 83 and 84 test against. Policy spine must exist before egress or telemetry can read from it. |
| Proxy and WFP wired to HKLM in one atomic phase (83) | 83 | Splitting proxy and WFP wiring across phases creates the allowlist-drift false-security state (Pitfall 2). Both layers read from the same MachineEgressPolicy struct in the same phase. |
| Stay WiX MSI; MSIX out of scope | 82 | MSIX cannot package the LocalSystem nono-wfp-service or kernel driver. WiX MSI CI pipeline is already proven (Phases 53/61). |
| Scratch space provisioned at first-run, not MSI time | 82/83 | MSI runs as SYSTEM; %LOCALAPPDATA% resolves to SYSTEM profile path, making every user R-B3 ownership guard fail (Pitfall 4). MSI creates only C:\ProgramData\nono\; user scratch is created at first run in user context. |
| Application Event Log source (no wevtutil manifest) for v3.0 | 84 | Custom channel requires wevtutil im at install; silent drop on missing registration. Application log source is proven in nono-wfp-service.rs and works without a manifest. Defer custom manifest to future SIEM schema phase. |
| Tamper-evidence = external SIEM forwarding; local HMAC deferred | 84 | Local HMAC key in HKLM is deletable by local admin — defeats the claim. v3.0 tamper boundary is Windows Event Forwarding to SIEM. SEED-005 ZT-Infra addresses cryptographic-local anchoring. ADR required as first Phase 84 deliverable. |
| Dark Factory verification carries forward from v2.13 | all | Every phase ships a verify-dark.ps1 gate as its verification mechanism. Milestone closes on the no-flag aggregator. True fleet/SIEM/EDR live UAT is host-gated tech-debt. |

### Pending Todos

None.

### Blockers/Concerns

- **Cross-target clippy required**: any cfg-gated Unix code touched in this milestone (esp. Phase 87 SEC-01/02 → `crates/nono/src/sandbox/linux.rs`, `crates/nono-cli/src/exec_strategy/supervisor_linux.rs`, `crates/nono/src/capability.rs`) MUST be verified via `cargo clippy --workspace --target x86_64-unknown-linux-gnu` AND `--target x86_64-apple-darwin`; Windows-host `cargo check` is not a substitute (CLAUDE.md MUST/NEVER rule; `feedback_clippy_cross_target`).
- **Library-boundary risk (Phase 86)**: adopt-upstream of the core-crate audit + diagnostics refactor is the highest near-term merge risk against fork-divergent surfaces (TLS intercept, FFI); guard the `nono-ffi` exhaustive-match arms (the `--bin nono` gate hides those — use `--workspace --all-targets`).
- **Repo stays PUBLIC**: verify no `build_notes/` or `.gsd/` files staged before any `git push` (minifilter-altitude approval pending).
- **Milestone-marker only**: no crate publish; a future release must leapfrog the crate version to ≥ `0.65.0`.

### Quick Tasks Completed

| # | Description | Date | Commit | Directory |
|---|-------------|------|--------|-----------|
| 260619-duw | Scope upstream v0.62..v0.64 delta and plant SEED-006 | 2026-06-19 | f3f792dd | [260619-duw-scope-upstream-v0-62-v0-64-delta-and-pla](./quick/260619-duw-scope-upstream-v0-62-v0-64-delta-and-pla/) |

## Deferred Items

Items acknowledged and carried forward from v2.13 close (2026-06-18):

| Category | Item | Status | Deferred At |
|----------|------|--------|-------------|
| Host-execution | stale `C:\Program Files\nono\nono.exe` (no `agent` subcommand) → aggregate FAIL on dev host; fix: prepend `target\release` to PATH | Open | v2.13 close |
| Host-execution | CPLT-03 Copilot CLI literal PASS gated by GitHub org policy | Open | v2.13 close |
| Host-execution | INST-01 live clean-VM PASS (needs fresh Win11 VM + rebuilt MSI post Phase 80) | Open | v2.13 close |
| Distribution | DIST-SIGN-01 untrusted-POC-cert broker path not exercised by clean-host gate | Open | v2.13 close |
| Historical | 44 pre-v2.13 open artifacts (see v2.13 STATE.md) | Acknowledged | v2.13 close |
| nono-ffi | E0004 non-exhaustive match (PolicyLoadFailed + TelemetryUnavailable/ConfigInvalid) | RESOLVED `f96aba8a` (84 close) | 84-04 |

Items acknowledged and deferred at v3.0 milestone close (2026-06-19) — see `.planning/v3.0-MILESTONE-AUDIT.md`. **The first six rows below are now folded into v3.1 Phase 90 (DRAIN-01..04):**

| Category | Item | Status | Deferred At |
|----------|------|--------|-------------|
| Host-execution | DEPLOY-01 clean-VM silent `msiexec /qn` install + exit codes → v3.1 DRAIN-01 | Open (host-gated) | v3.0 close |
| Host-execution | DEPLOY-03 live R-B3 user-owned WRITE_OWNER scratch verification → v3.1 DRAIN-01 | Open (host-gated) | v3.0 close |
| Host-execution | DEPLOY-05 three-client (CryptoAPI/Node/rustls) TLS-through-proxy round-trip → v3.1 DRAIN-01 | Open (host-gated) | v3.0 close |
| Host-execution | EGRESS-02 dual-layer (proxy + kernel WFP) live block proof → v3.1 DRAIN-02 | Open (host-gated) | v3.0 close |
| Host-execution | TELEM-01/04 live `telemetry-event-emit` gate PASS + admin opt-out/min_severity HKLM→emit (`84-HUMAN-UAT.md`) → v3.1 DRAIN-03 | Open (host-gated) | v3.0 close |
| Cross-phase | Daemon-side telemetry: `nono-agentd` registers no SecurityEventLayer → daemon-launched agent denials emit no `nono_security::*` events → v3.1 DRAIN-04 (real code) | Open (folded into v3.1) | v3.0 close |
| Historical | 48 open artifacts at v3.0 close (35 quick_tasks mostly pre-v2.13 strays + 5 seeds + 4 todos + 2 uat_gaps + 2 verification_gaps) — overwhelmingly historical/future-seed | Acknowledged | v3.0 close |

## Session Continuity

Last session: 2026-06-19T15:10:50.947Z
Stopped at: Phase 85 context gathered
Resume file: .planning/phases/85-upst9-divergence-audit/85-CONTEXT.md

## Operator Next Steps

- Review the v3.1 roadmap (`.planning/ROADMAP.md`), then run `/gsd:plan-phase 85` to plan the UPST9 Divergence Audit.
