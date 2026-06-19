---
phase: 84-siem-edr-telemetry
plan: "02"
subsystem: telemetry
tags: [hmac, etw, event-log, dual-emit, tracing, init-tracing, msrv]
dependency_graph:
  requires: [84-01]
  provides: [emit_security_event, advance_chain_hmac, init_tracing_telemetry, etw_layer]
  affects:
    - crates/nono-cli/Cargo.toml
    - crates/nono-cli/src/telemetry/windows.rs
    - crates/nono-cli/src/telemetry/mod.rs
    - crates/nono-cli/src/cli_bootstrap.rs
    - crates/nono-cli/src/main.rs
    - CLAUDE.md
tech_stack:
  added: [hmac 0.13.0 (RustCrypto/MACs), tracing-etw 0.2.3 (Microsoft/ETW-Rust), eventlog 0.4.0 (locryus)]
  patterns: [dual-emit (ETW + Application log), HMAC-SHA256 keyed chain, RegisterEventSourceW/ReportEventW, tracing-subscriber registry composition, per-layer filter]
key_files:
  created: []
  modified:
    - crates/nono-cli/Cargo.toml
    - CLAUDE.md
    - crates/nono-cli/src/telemetry/windows.rs
    - crates/nono-cli/src/telemetry/mod.rs
    - crates/nono-cli/src/telemetry/event.rs
    - crates/nono-cli/src/cli_bootstrap.rs
    - crates/nono-cli/src/main.rs
decisions:
  - "D-MSRV executed: MSRV bumped 1.77→1.82 in CLAUDE.md atomically with tracing-etw 0.2.3 dep addition"
  - "ETW emit via tracing::warn!(target: nono_security) inside emit_security_event — tracing-etw layer in registry picks up automatically (simpler than OnceLock approach)"
  - "init_registry() helper with fmt_layer.with_filter(env_filter) pattern avoids S-parameter mismatch in generic subscriber composition"
  - "EVENT_ID_* constants live in event.rs (schema); windows.rs imports them via schema_event_id_for delegation — single source of truth"
  - "EventLogLevel::Information retained for future use; #[allow(dead_code)] applied to avoid spurious warning"
metrics:
  duration: ~65m
  completed: 2026-06-19
  tasks_completed: 2
  files_created: 0
  files_modified: 7
---

# Phase 84 Plan 02: Real dual-emit backend — HMAC chain + RegisterEventSourceW + ETW + init_tracing registration

Real Hmac<Sha256> keyed chain replaces the sha2 placeholder; RegisterEventSourceW/ReportEventW write JSON payloads (D-02) to the Phase-82-registered nono Application source at EventIDs 10001-10005; tracing-etw LayerBuilder registered in init_tracing() for ETW dual-emit (D-01); SecurityEventLayer wired in all three subscriber arms.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Add Cargo deps + MSRV bump | `c55c0397` | nono-cli/Cargo.toml, CLAUDE.md, Cargo.lock |
| 2 (RED) | Failing tests for HMAC chain + windows.rs constants + emit non-fatal | `48af8357` | telemetry/windows.rs, telemetry/mod.rs |
| 2 (GREEN) | Real dual-emit backend + init_tracing wiring | `89e8c907` | cli_bootstrap.rs, main.rs, telemetry/{event,mod,windows}.rs |

## What Was Built

### Task 1: Cargo dependencies + MSRV bump (D-MSRV)

- `hmac = "0.13"` added to `nono-cli` `[dependencies]` (unconditional — HMAC chain is cross-platform)
- `tracing-etw = "0.2"` and `eventlog = "0.4"` added under `[target.'cfg(target_os = "windows")'.dependencies]`
- `CLAUDE.md` Technology Stack: "Minimum Rust version: 1.82" (was 1.77); per D-MSRV decision recorded in STATE.md during Plan 84-01
- 23 new packages locked in Cargo.lock

### Task 2: Real dual-emit backend (TDD)

**telemetry/windows.rs — real emitter:**
- `EVENT_LOG_SOURCE = "nono"` (Phase-82-registered Application source)
- `EventLogLevel` enum (Information/Warning); `event_id_for()` delegates to `event.rs` schema (single source of truth)
- `write_security_event_log()` (cfg(windows)): RegisterEventSourceW/ReportEventW/DeregisterEventSource with `// SAFETY:` comments per CLAUDE.md; NULL-handle → `eprintln!` + `return` (D-03 / Pitfall 10 prevention)
- `build_event_payload()`: `serde_json::to_string(&event).unwrap_or_else(|e| ...)` — serialization failure → fallback JSON, never silent drop
- `emit_security_event()`: Application log write (cfg(windows)) + `tracing::warn!(target: "nono_security", ...)` with SC-1 named fields (event_type, event_id, agent_pid, path_hash, host, session_id, chain_head, timestamp_unix_ms)

**telemetry/mod.rs — real Hmac<Sha256> advance_chain:**
- sha2 placeholder fully replaced: `HmacSha256::new_from_slice(key)` → `update(TELEMETRY_CHAIN_DOMAIN)` → `update(&prev_head)` → `update(TELEMETRY_EVENT_DOMAIN)` → `update(event_bytes)` → `finalize()`
- `match`-based `InvalidLength` fallback: `eprintln!` + zeroed-key degrade (D-14) — no `.unwrap()` or `.expect()`
- Plan-01 `#![allow(dead_code)]` removed from mod.rs and event.rs (layer now wired)

**cli_bootstrap.rs — init_tracing signature extended (TELEM-04 SC-4):**
- New signature: `pub(crate) fn init_tracing(cli: &Cli, telemetry_config: Option<TelemetryConfig>)`
- Session ID: 16 hex chars from `rand::rng().fill()` (unconditional dep)
- `SecurityEventLayer::new(config, session_id)` built before subscriber composition
- `init_registry()` helper uses `fmt_layer.with_filter(env_filter)` pattern to handle tracing-subscriber generic type composition correctly
- On Windows: `tracing_etw::LayerBuilder::new("nono").build()` registered in all three arms; failure continues without ETW (D-03 non-fatal)

**main.rs:**
- `init_tracing(&cli, None)` — `None` uses `TelemetryConfig::default()` (D-13 default-ON)

**All 27 telemetry tests PASS. `cargo clippy --bin nono -D warnings -D clippy::unwrap_used`: CLEAN.**

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Type inference] init_registry generic subscriber composition**
- **Found during:** GREEN build — `fmt::Layer<Registry, ...>` parameterized on `S=Registry` didn't satisfy `Layer<Layered<EnvFilter, Registry>>` after `.with(env_filter)` changed S
- **Fix:** Applied `fmt_layer.with_filter(env_filter)` (per-layer filter) instead of `.with(env_filter)` on the registry, keeping S as `Registry` at all composition points
- **Files modified:** `crates/nono-cli/src/cli_bootstrap.rs`
- **Commit:** `89e8c907`

**2. [Rule 3 - Missing trait impl] Arc<SecurityEventLayer> doesn't implement Layer<S>**
- **Found during:** GREEN build — tracing-subscriber does not provide `Layer<S>` for `Arc<L>` generically
- **Fix:** Removed Arc wrapping. Since only one of the three match arms runs per process, SecurityEventLayer is passed directly into init_registry. Created init_registry() as a generic helper so the layer is consumed exactly once.
- **Files modified:** `crates/nono-cli/src/cli_bootstrap.rs`
- **Commit:** `89e8c907`

**3. [Rule 1 - Bug] EVENT_ID_* duplicate constants between event.rs and windows.rs**
- **Found during:** GREEN build — windows.rs originally defined its own EVENT_ID_* constants causing dead_code warnings when event.rs defines the authoritative ones
- **Fix:** windows.rs imports `event_id_for as schema_event_id_for` from event.rs and delegates; EVENT_ID_* in windows.rs tests import from event.rs
- **Files modified:** `crates/nono-cli/src/telemetry/windows.rs`, `event.rs`
- **Commit:** `89e8c907`

### Scope Deviations

- **ETW layer type complexity:** The plan suggested `OnceLock<EtwLayer>` or forwarding. Instead used `tracing_etw::LayerBuilder::new("nono").build()` inline inside `init_registry()` on Windows — this is simpler and avoids OnceLock complexity (approved by the plan's alternate suggestion). The `tracing::warn!(target: "nono_security", ...)` in `emit_security_event` is the ETW emission path; the layer intercepts it.

## Key Decisions

| Decision | Rationale |
|----------|-----------|
| ETW via tracing::warn! not direct EtwProvider | Simpler; the registered LayerBuilder intercepts the warn! call automatically; avoids OnceLock and per-event provider handle |
| init_registry() with per-layer filter (fmt_layer.with_filter(env_filter)) | Avoids S-type mismatch when env_filter changes the registry's subscriber type; security layer always active regardless of log level |
| EVENT_ID_* in event.rs (schema) as single source of truth | windows.rs imports them; no duplication; tests in both files use the same values |
| D-MSRV: CLAUDE.md updated 1.77→1.82 atomically with Cargo.toml dep add | D-MSRV decision recorded in STATE.md; workspace Cargo.toml already had rust-version = "1.95" but documentation was stale |

## Cross-Target Verification

**Status: PARTIAL** (Windows dev host cannot run Linux/macOS clippy)

- `cargo clippy -p nono-cli --bin nono -- -D warnings -D clippy::unwrap_used` — PASS on Windows host (clean)
- `cargo test -p nono-cli --bin nono -- telemetry` — 27/27 PASS on Windows host
- `cargo clippy --workspace --target x86_64-unknown-linux-gnu` — DEFERRED to CI (no cross-toolchain on Windows host; per CLAUDE.md MUST/NEVER rule)
- cfg-gated code: `#[cfg(target_os = "windows")]` in windows.rs and cli_bootstrap.rs; `#[cfg(not(target_os = "windows"))]` stubs in windows.rs; syslog.rs has `#[cfg(unix)]` stub. Risk: LOW (same pattern as Plan 01, which compiled clean on CI).

## Known Stubs

| Stub | File | Reason |
|------|------|--------|
| `emit_syslog_event` no-op | `telemetry/syslog.rs` | RFC 5424 syslog is TELEM-FU-01, explicitly deferred |
| `init_tracing` passes `None` for telemetry_config | `main.rs` | MachineEgressPolicy not available at process startup; Plans 03-04 may wire this |

## Threat Surface Scan

No new network endpoints, auth paths, file access patterns, or schema changes at trust boundaries beyond what was planned:
- `RegisterEventSourceW`/`ReportEventW` — within the plan's threat model (T-84-07/T-84-08); NULL-handle fallback (D-03) prevents blocking
- `tracing::warn!(target: "nono_security", ...)` — fields are pre-scrubbed/hashed SecurityEvent fields; no raw paths or URLs (T-84-10 mitigated)
- HMAC key material — Zeroizing<[u8;32]>, zeroed on drop (T-84-09 mitigated; match-based InvalidLength handling, no panic)

## Self-Check: PASSED

- `crates/nono-cli/src/telemetry/windows.rs` — EXISTS, contains EVENT_LOG_SOURCE="nono" and RegisterEventSourceW
- `crates/nono-cli/src/telemetry/mod.rs` — EXISTS, contains HmacSha256 (Hmac<Sha256>), no TODO(84-02)
- `crates/nono-cli/src/cli_bootstrap.rs` — EXISTS, contains SecurityEventLayer and Option<TelemetryConfig>
- Commits `c55c0397`, `48af8357`, `89e8c907` all present in git log
- 27/27 telemetry tests PASS
- cargo clippy CLEAN (0 warnings, 0 errors)
