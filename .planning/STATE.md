---
gsd_state_version: 1.0
milestone: v3.0
milestone_name: Enterprise Hardening I
status: executing
stopped_at: Phase 84 structurally verified 5/5 (CR-01 + WR-01/02 closed inline); human_needed — live SIEM gate host-gated; awaiting approve to mark complete
last_updated: "2026-06-19T03:35:00.000Z"
last_activity: 2026-06-19 -- Phase 84 gap closure: CR-01 gate + WR-01 config-from-policy + WR-02 level-filter fixed; nono-ffi build regression fixed; re-verified 5/5 structural
progress:
  total_phases: 3
  completed_phases: 2
  total_plans: 12
  completed_plans: 12
  percent: 67
---

# Project State: nono — v3.0 Enterprise Hardening I (Deploy · Control · Compliance)

## Project Reference

See: `.planning/PROJECT.md` (v3.0 milestone started 2026-06-18; v2.13 Phases 76-81 complete, shipped + archived). Phase numbering continues from Phase 81 (Phases 82-84 — NOT reset to 1).

**Core Value:** Windows security must be as structurally impossible and feature-complete as Unix platforms — and that confinement must be deployable and governable across a corporate Windows fleet.

**Current Focus:** Phase 84 — siem-edr-telemetry (structurally verified 5/5; human_needed live SIEM UAT host-gated)

## Current Position

Phase: 84 (siem-edr-telemetry) — all 4 plans executed; re-verified 5/5 structural after gap closure
Plan: 4 of 4 executed; CR-01 + WR-01 + WR-02 all CLOSED inline 2026-06-19
Status: human_needed — 3 live-host UAT items (84-HUMAN-UAT.md) host-gated per Dark Factory policy; awaiting operator "approved" to mark phase/milestone complete. Follow-up: daemon-side telemetry emission (not a Phase 84 criterion).
Last activity: 2026-06-19

Progress: [█████████░] ~95% (Phase 84 structural pass; live UAT + milestone close pending)

## Performance Metrics

**Velocity:**

- Total plans completed: 9 (4 Phase 82 + 1 Phase 83)
- Average duration: ~7m (Phase 83 Plan 01)
- Total execution time: 7m (Phase 83 Plan 01 only)

| Phase | Plan | Duration | Tasks | Files |
|-------|------|----------|-------|-------|
| 83 | 01 | 7m | 3 | 6 |
| 83 | 03 | 25m | 2 | 6 |
| 84 | 02 | 65m | 2 | 7 |

*Updated after each plan completion*
| Phase 83 P02 | 45m | 2 tasks | 3 files |
| Phase 83 P04 | 30m | 2 tasks | 1 files |
| Phase 84 P01 | 35m | 3 tasks | 9 files |
| Phase 84 P02 | 65m | 2 tasks | 7 files |
| Phase 84 P03 | 30m | 2 tasks | 4 files |
| Phase 84 P04 | 3m | 2 tasks | 1 files |

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

### Decisions (Phase 84 Plan 04)

| Decision | Phase | Rationale |
|----------|-------|-----------|
| Gate auto-discovered by verify-dark.ps1 scripts/gates/*.ps1 scan (D-04); no ValidateSet update | 84-04 | Auto-discovery mechanism confirmed working; telemetry-event-emit.ps1 found without any hardcoded addition |
| EventID 10003 excluded from gate per Option B carry-forward | 84-04 | LabelViolation is RESERVED-but-unemitted in Phase 84; gate uses range 10001-10005 accepting whichever of the three wired EventIDs appears |
| Cross-target clippy PARTIAL: C linker absent for Linux+macOS cross targets | 84-04 | Same pre-existing issue as Phase 83; aws-lc-sys/ring require x86_64-linux-gnu-gcc/cc; deferred to live CI |

### Decisions (Phase 84 Plan 03)

| Decision | Phase | Rationale |
|----------|-------|-----------|
| OPTION B label-violation: EventID 10003 RESERVED-but-unemitted in Phase 84 | 84-03 | IL denials surface as NonoError::LabelApplyFailed (aborting session via ?) or as path-deny DenialRecords at exec_strategy layer; no distinct label_violation event emittable; Plan 84-04 gate excludes EventID 10003 |
| hook_fail_closed wired at hook script write failure | 84-03 | Script write failure = PreToolUse security hook cannot run = fail-closed; most security-relevant fail-closed site in hooks.rs |
| D-07 ADR delivered: tamper boundary = WEF; in-session HMAC only | 84-03 | docs/adr/telemetry-tamper-evidence.md records scope honestly: WEF is the real tamper boundary; local admin can clear Application log; SEED-005 deferred for cryptographic-local anchoring |

### Decisions (Phase 84 Plan 02)

| Decision | Phase | Rationale |
|----------|-------|-----------|
| D-MSRV executed: MSRV bumped 1.77->1.82 in CLAUDE.md atomically with tracing-etw 0.2.3 dep addition | 84-02 | tracing-etw 0.2.3 requires Rust 1.82; bumped in CLAUDE.md Technology Stack section per D-MSRV pre-approved decision |
| ETW emit via tracing::warn!(target: nono_security) inside emit_security_event | 84-02 | Simpler than OnceLock approach; the tracing-etw LayerBuilder registered in init_tracing() intercepts the warn! call automatically without per-event provider handle |
| init_registry() helper with fmt_layer.with_filter(env_filter) pattern | 84-02 | Avoids S-type mismatch when env_filter changes the registry subscriber type; security layer always active regardless of log level |
| EVENT_ID_* in event.rs (schema) as single source of truth; windows.rs imports via schema_event_id_for | 84-02 | No duplication; tests in both files use the same values; prevents drift between emit and schema |

### Decisions (Phase 84 Plan 01)

| Decision | Phase | Rationale |
|----------|-------|-----------|
| D-MSRV: MSRV bump 1.77→1.82 DEFERRED to Plan 84-02 | 84-01 | tracing-etw 0.2.3 requires Rust 1.82; Plan 01 does not add tracing-etw to Cargo.toml so no MSRV conflict exists yet. Plan 02 edits CLAUDE.md and workspace Cargo.toml atomically when adding the dep. |
| D-HMAC-PLACEHOLDER: sha2-based advance_chain placeholder in Plan 01 | 84-01 | hmac crate not yet in Cargo.toml (operator checkpoint passed for crates but not yet added); sha2 used as placeholder preserving domain separators; Plan 02 replaces with Hmac<Sha256> |
| D-CLASSIFY-MULTIPASS: classify_path uses multi-pass component loop | 84-01 | Single-pass ordering was fragile (/var/lib/keystore returned SystemPath because 'lib' appeared before 'keystore'); multi-pass ensures CredentialPath wins regardless of component position |
| Package legitimacy: hmac 0.13.0 / tracing-etw 0.2.3 / eventlog 0.4.0 APPROVED | 84-01 | Operator-verified all three crates via crates.io API before any Cargo.toml edits (Task 1 checkpoint resolved) |

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
| nono-ffi | E0004 non-exhaustive match (TelemetryUnavailable + TelemetryConfigInvalid) | Open | 84-04 |

## Session Continuity

Last session: 2026-06-19T00:10:00.000Z
Stopped at: Phase 84 Plan 04 complete — telemetry-event-emit Dark Factory gate (SC-1/SC-3/SC-5) + cross-target clippy PARTIAL
Resume file: .planning/phases/84-siem-edr-telemetry/84-04-SUMMARY.md
