---
gsd_state_version: 1.0
milestone: v3.0
milestone_name: Enterprise Hardening I
status: executing
stopped_at: Phase 84 context gathered
last_updated: "2026-06-19T02:30:16.856Z"
last_activity: 2026-06-19 -- Phase 84 planning complete
progress:
  total_phases: 3
  completed_phases: 2
  total_plans: 12
  completed_plans: 8
  percent: 67
---

# Project State: nono — v3.0 Enterprise Hardening I (Deploy · Control · Compliance)

## Project Reference

See: `.planning/PROJECT.md` (v3.0 milestone started 2026-06-18; v2.13 Phases 76-81 complete, shipped + archived). Phase numbering continues from Phase 81 (Phases 82-84 — NOT reset to 1).

**Core Value:** Windows security must be as structurally impossible and feature-complete as Unix platforms — and that confinement must be deployable and governable across a corporate Windows fleet.

**Current Focus:** Phase 83 — machine-policy-spine-egress-control

## Current Position

Phase: 84
Plan: Not started
Status: Ready to execute
Last activity: 2026-06-19 -- Phase 84 planning complete

Progress: [██████████] 100%

## Performance Metrics

**Velocity:**

- Total plans completed: 9 (4 Phase 82 + 1 Phase 83)
- Average duration: ~7m (Phase 83 Plan 01)
- Total execution time: 7m (Phase 83 Plan 01 only)

| Phase | Plan | Duration | Tasks | Files |
|-------|------|----------|-------|-------|
| 83 | 01 | 7m | 3 | 6 |
| 83 | 03 | 25m | 2 | 6 |

*Updated after each plan completion*
| Phase 83 P02 | 45m | 2 tasks | 3 files |
| Phase 83 P04 | 30m | 2 tasks | 1 files |

## Accumulated Context

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

### Decisions (Phase 83)

| Decision | Phase | Rationale |
|----------|-------|-----------|
| D-07: absent→Ok(None) / present-but-broken→Err(PolicyLoadFailed) | 83-01 | Fail-secure: once HKLM key exists ANY read/parse error aborts; implemented via raw_os_error()==2 for absent |
| D-09: KEY_WOW64_64KEY on all registry opens | 83-01 | Forces 64-bit hive view; prevents 32-bit Intune MDM write to WOW6432Node making key appear absent |
| D-10: winreg 0.56 Windows-only dep (operator-approved) | 83-01 | Single crate approach; io::Error maps cleanly onto D-07 taxonomy; never unconditional dep |
| D-13 Option A: enumerate N×REG_SZ subkey values (not REG_MULTI_SZ) | 83-01 | Matches shipped Phase-82 ADMX <list> shape; less churn than changing ADMX |
| D-14: existing HostFilter leading-dot ends_with+len> form retained | 83-01 | Already passes full SC-4 matrix; sc4_dns_component_matrix codifies the contract |
| D-11: ADMX named toggles write group TOKENS (anthropic/openai/github-api), not literal FQDNs | 83-03 | Token indirection decouples fleet ADMX template from FQDN lists; nono expands at runtime so provider hosts update without re-issuing ADMX |
| D-12 (corrected): preset token->FQDN map in embedded network-policy.json groups (not policy.json) | 83-03 | network-policy.json carries domain host[] groups (correct schema); policy.json carries only filesystem allow/deny semantics |

### Pending Todos

None.

### Blockers/Concerns

- **Cross-target clippy required**: any cfg-gated Unix code touched in this milestone MUST be verified via `cargo clippy --workspace --target x86_64-unknown-linux-gnu` AND `--target x86_64-apple-darwin`; Windows-host `cargo check` is not a substitute (CLAUDE.md MUST/NEVER rule; `feedback_clippy_cross_target`).
- **Repo stays PUBLIC**: verify no `build_notes/` or `.gsd/` files staged before any `git push` (minifilter-altitude approval pending).

## Deferred Items

Items acknowledged and carried forward from v2.13 close (2026-06-18):

| Category | Item | Status | Deferred At |
|----------|------|--------|-------------|
| Host-execution | stale `C:\Program Files\nono\nono.exe` (no `agent` subcommand) → aggregate FAIL on dev host; fix: prepend `target\release` to PATH | Open | v2.13 close |
| Host-execution | CPLT-03 Copilot CLI literal PASS gated by GitHub org policy | Open | v2.13 close |
| Host-execution | INST-01 live clean-VM PASS (needs fresh Win11 VM + rebuilt MSI post Phase 80) | Open | v2.13 close |
| Distribution | DIST-SIGN-01 untrusted-POC-cert broker path not exercised by clean-host gate | Open | v2.13 close |
| Historical | 44 pre-v2.13 open artifacts (see v2.13 STATE.md) | Acknowledged | v2.13 close |

## Session Continuity

Last session: 2026-06-19T01:43:04.393Z
Stopped at: Phase 84 context gathered
Resume file: .planning/phases/84-siem-edr-telemetry/84-CONTEXT.md
