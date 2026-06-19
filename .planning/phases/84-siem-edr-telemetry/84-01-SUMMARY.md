---
phase: 84-siem-edr-telemetry
plan: "01"
subsystem: telemetry
tags: [schema, telemetry, hmac, security-event, machine-policy]
dependency_graph:
  requires: [83-01, 83-02, 83-03, 83-04]
  provides: [TelemetryConfig, TelemetrySeverity, SecurityEvent, SecurityEventType, PathCategory, SecurityEventLayer, ChainState]
  affects: [crates/nono/src/machine_policy.rs, crates/nono/src/error.rs, crates/nono/src/lib.rs, crates/nono-cli/src/telemetry/]
tech_stack:
  added: [sha2 (path hash placeholder for HMAC chain — replaced in Plan 02), rand (RngExt for key/salt generation), zeroize (ChainState key), tracing-subscriber (Layer trait)]
  patterns: [schema-first, HMAC domain separation, path-salted-hash, tracing::Layer, zeroize-on-drop]
key_files:
  created:
    - crates/nono-cli/src/telemetry/mod.rs
    - crates/nono-cli/src/telemetry/event.rs
    - crates/nono-cli/src/telemetry/windows.rs
    - crates/nono-cli/src/telemetry/syslog.rs
  modified:
    - crates/nono/src/machine_policy.rs
    - crates/nono/src/error.rs
    - crates/nono/src/lib.rs
    - crates/nono-cli/src/main.rs
    - crates/nono-cli/src/agent_daemon/mod.rs
decisions:
  - "D-MSRV: MSRV bump 1.77→1.82 DEFERRED to Plan 84-02 (tracing-etw 0.2.3 requires 1.82); do not touch Cargo.toml/CLAUDE.md MSRV in Plan 01"
  - "D-HMAC-PLACEHOLDER: sha2-based advance_chain placeholder accepted for Plan 01; replaced with Hmac<Sha256> in Plan 02 after operator checkpoint (hmac not yet in Cargo.toml)"
  - "D-CLASSIFY-MULTIPASS: classify_path uses multi-pass component loop (credential > temp > system > home) to ensure /var/lib/keystore/* is CredentialPath despite 'lib' appearing first"
  - "D-DEAD-CODE: #![allow(dead_code)] on telemetry/ modules — schema-only plan; all types exercised in tests, binary wiring in Plan 02"
metrics:
  duration: ~35m
  completed: 2026-06-19
  tasks_completed: 3
  files_created: 4
  files_modified: 5
---

# Phase 84 Plan 01: SecurityEvent Schema + TelemetryConfig + HMAC Chain State Summary

Schema-first discipline: TelemetryConfig struct in MachineEgressPolicy, SecurityEvent/SecurityEventType/PathCategory types in telemetry/event.rs, per-session Zeroizing HMAC chain state in telemetry/mod.rs, and stub emitters for Windows/syslog — no emitter code yet, all contracts locked for Plans 02-04.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Package Legitimacy Gate | (checkpoint resolved — no commit) | hmac 0.13.0 / tracing-etw 0.2.3 / eventlog 0.4.0 all APPROVED |
| 2 | TelemetryConfig extension to MachineEgressPolicy | `5bf2ef4e` | machine_policy.rs, error.rs, lib.rs |
| 3 | SecurityEvent schema + HMAC chain state + redaction helpers | `c4993ae8` | telemetry/{mod,event,windows,syslog}.rs, main.rs, agent_daemon/mod.rs |

## What Was Built

### Task 1: Package Legitimacy Gate (APPROVED)
- `hmac` 0.13.0 — RustCrypto/MACs, legitimate, not yanked. APPROVED.
- `tracing-etw` 0.2.3 — Microsoft/ETW-Rust, legitimate, not yanked. APPROVED. (MSRV 1.82 — D-MSRV recorded for Plan 02.)
- `eventlog` 0.4.0 — locryus/eventlog, MIT/Apache-2.0, legitimate, not yanked. APPROVED.

### Task 2: TelemetryConfig + error variants
- `TelemetrySeverity` enum (Debug/Info/Warning/Error), default = Warning.
- `TelemetryConfig` struct: `enabled=true`, `channel="Application"`, `min_severity=Warning` (D-13 default-ON semantics).
- `#[serde(default)] pub telemetry: TelemetryConfig` added to `MachineEgressPolicy`.
- `is_unconfigured()` remains egress-only — telemetry NOT counted (invariant 3 / CR-02).
- Windows reader `parse_telemetry_config()` reads `Telemetry\` sub-key; malformed values degrade to default + eprintln (D-14, not D-07 abort).
- `NonoError::TelemetryUnavailable` and `TelemetryConfigInvalid` added (non-fatal per D-03/D-14).
- `TelemetryConfig` and `TelemetrySeverity` re-exported from `crates/nono/src/lib.rs`.
- **21/21** machine_policy tests pass including 5 new Phase 84 TDD tests.

### Task 3: SecurityEvent schema + HMAC chain state
- **telemetry/event.rs**: `SecurityEventType` (PathDeny/NetworkDeny/LabelViolation/HookFailClosed/TelemetryDegraded), `event_id_for()` mapping to EventIDs 10001-10005, `PathCategory` (CredentialPath/SystemPath/Temp/UserHome/WorkspaceFile/Other), `classify_path()` multi-pass component-level classification, `path_hash_for()` SHA-256(salt||path)[0..16] hex, `SecurityEvent` struct with `#[serde(rename_all = "PascalCase")]`.
- **telemetry/mod.rs**: `TELEMETRY_EVENT_DOMAIN = b"nono.telemetry.event.alpha\n"`, `TELEMETRY_CHAIN_DOMAIN = b"nono.telemetry.chain.alpha\n"` (distinct from audit — D-06), `ChainState` with `Zeroizing<[u8;32]>` key + explicit `Drop::drop` zeroize, `advance_chain()` sha2 placeholder (TODO(84-02) for `Hmac<Sha256>`), `SecurityEventLayer` tracing::Layer filtering `nono_security::*`.
- **telemetry/windows.rs**: no-op stub, TODO(84-02) for real emit.
- **telemetry/syslog.rs**: `#[cfg(unix)]` stub, TODO(TELEM-FU-01).
- **23/23** telemetry tests pass.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] classify_path single-pass ordering hit /var/lib/keystore wrong category**
- **Found during:** Task 3 RED (test `classify_keystore_path_is_credential` failed)
- **Issue:** Single-pass component loop returned `SystemPath` for `/var/lib/keystore/tokens.db` because `lib` appeared before `keystore` in the path, and SystemPath was checked in the same loop after Credential
- **Fix:** Refactored `classify_path` to use three sequential passes: (1) all-components credential check, (2) all-components temp check, (3) all-components system-path check. Credential takes absolute priority regardless of component position.
- **Files modified:** `crates/nono-cli/src/telemetry/event.rs`
- **Commit:** `c4993ae8`

**2. [Rule 1 - Bug] Existing MachineEgressPolicy struct initializers in tests missing `telemetry` field**
- **Found during:** Task 2 compile — `cargo test -p nono machine_policy` failed E0063
- **Issue:** Adding `telemetry: TelemetryConfig` to `MachineEgressPolicy` broke 4 existing test initializers in `machine_policy.rs` and 1 in `agent_daemon/mod.rs` that didn't specify all fields
- **Fix:** Added `..Default::default()` to each affected initializer
- **Files modified:** `crates/nono/src/machine_policy.rs`, `crates/nono-cli/src/agent_daemon/mod.rs`
- **Commit:** `5bf2ef4e`, `c4993ae8`

**3. [Rule 2 - Missing dead_code handling] telemetry/ schema-only modules produced clippy errors with -D warnings**
- **Found during:** Task 3 clippy verification
- **Issue:** Plan 01 is schema-only; all public items are exercised in tests but not yet wired from the binary path (wiring happens Plans 02-04). `-D warnings` fails with `dead_code` for binary mode.
- **Fix:** Added `#![allow(dead_code)]` with explanatory comments to `telemetry/mod.rs`, `event.rs`, and `windows.rs`. Pattern is consistent with `crates/nono-cli/src/agent_daemon/launch.rs` and `launch_runtime.rs` in the existing codebase.
- **Files modified:** `crates/nono-cli/src/telemetry/mod.rs`, `event.rs`, `windows.rs`

**4. [Rule 3 - rand 0.10 API] rand::RngCore not available in rand 0.10**
- **Found during:** Task 3 build
- **Issue:** rand 0.10 (in nono-cli Cargo.toml) uses `rand::RngExt::fill` not `rand::RngCore::fill_bytes`
- **Fix:** Used `rand::RngExt` with `rng.fill(&mut key_bytes[..])` pattern
- **Files modified:** `crates/nono-cli/src/telemetry/mod.rs`

## Key Decisions

| Decision | Rationale |
|----------|-----------|
| D-MSRV: MSRV bump 1.77→1.82 deferred to Plan 84-02 | tracing-etw 0.2.3 requires Rust 1.82; Plan 01 does not add tracing-etw to Cargo.toml so no MSRV conflict exists yet |
| D-HMAC-PLACEHOLDER: sha2 placeholder in advance_chain | hmac crate not yet in Cargo.toml (operator checkpoint Task 1); sha2 (already in workspace) used as placeholder with domain separators preserved; Plan 02 replaces with Hmac<Sha256> |
| D-CLASSIFY-MULTIPASS: multi-pass classify_path | Single-pass ordering was fragile; multi-pass ensures credential paths win regardless of component position in the path string |

## Cross-Target Verification

**Status: PARTIAL** (Windows dev host cannot run Linux/macOS clippy)

- `cargo clippy -p nono-cli --bin nono -- -D warnings -D clippy::unwrap_used` — PASS on Windows host
- `cargo clippy --workspace --target x86_64-unknown-linux-gnu` — DEFERRED to CI (no cross-toolchain on Windows host; per CLAUDE.md MUST/NEVER rule + `feedback_clippy_cross_target`)
- The telemetry module uses no `#[cfg(target_os = "linux")]` or `#[cfg(target_os = "macos")]` blocks — only `#[cfg(unix)]` in syslog.rs stub and `#[cfg(target_os = "windows")]` in windows.rs. Risk of cross-target failure is LOW.

## Known Stubs

| Stub | File | Reason |
|------|------|--------|
| `advance_chain` sha2 placeholder | `telemetry/mod.rs:L182` | `hmac` crate not yet in Cargo.toml — Plan 02 adds it after operator checkpoint and replaces placeholder with `Hmac<Sha256>` |
| `emit_security_event` no-op | `telemetry/windows.rs` | Plan 01 is schema-only; real `RegisterEventSourceW`+`ReportEventW`+ETW emit in Plan 02 |
| `emit_syslog_event` no-op | `telemetry/syslog.rs` | RFC 5424 syslog is TELEM-FU-01, explicitly deferred |
| `SecurityEventLayer::on_event` not registered | `main.rs` | Layer not added to `init_tracing()` yet — Plan 02 wires it |

## Threat Surface Scan

No new network endpoints, auth paths, file access patterns, or schema changes at trust boundaries were introduced beyond what is documented in the plan's `<threat_model>`. The telemetry module does not open any new file handles, sockets, or registry keys in Plan 01 (all emitters are stubs). The sha2-placeholder `advance_chain` processes only in-memory bytes and produces no I/O.

## Self-Check: PASSED

All created files exist on disk. Both task commits (`5bf2ef4e`, `c4993ae8`) verified in git log. 21/21 machine_policy tests + 23/23 telemetry tests pass. `cargo clippy -p nono-cli --bin nono -- -D warnings -D clippy::unwrap_used` clean.
