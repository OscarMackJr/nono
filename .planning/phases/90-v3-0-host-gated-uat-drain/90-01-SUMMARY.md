---
phase: 90-v3-0-host-gated-uat-drain
plan: 01
subsystem: nono-agentd / telemetry
tags: [drain-04, daemon-telemetry, security-event-layer, hmac-chain, windows-service]
dependency_graph:
  requires: []
  provides: [DRAIN-04]
  affects:
    - crates/nono-cli/src/bin/nono-agentd.rs
    - crates/nono-cli/src/agent_daemon/mod.rs
    - crates/nono-cli/src/agent_daemon/telemetry_init.rs
    - crates/nono-cli/src/telemetry/mod.rs
tech_stack:
  added: []
  patterns:
    - "#[path]-include idiom for daemon binary modules (mirrors agent_daemon pattern)"
    - "SpyLayer delegation wrapper for testing Arc-incompatible tracing layers"
    - "#[cfg(test)] test accessor to avoid dead_code lint on pub(crate) methods"
    - "OnceLock double-init guard for tracing subscriber in foreground-fallback path"
key_files:
  created:
    - crates/nono-cli/src/agent_daemon/telemetry_init.rs
  modified:
    - crates/nono-cli/src/bin/nono-agentd.rs
    - crates/nono-cli/src/agent_daemon/mod.rs
    - crates/nono-cli/src/telemetry/mod.rs
decisions:
  - "chain_sequence() gated #[cfg(test)] to avoid dead_code lint; test accessor not production code"
  - "SpyLayer delegation wrapper used instead of Arc<SecurityEventLayer> (Arc<L: Layer> not implemented in tracing-subscriber 0.3.23)"
  - "Cross-target clippy PARTIAL→CI: ring/aws-lc-sys C-linker absent on Windows dev host"
metrics:
  duration: "~13 minutes"
  completed: "2026-06-20"
  tasks: 3
  files: 4
requirements: [DRAIN-04]
---

# Phase 90 Plan 01: Daemon Telemetry Wiring (DRAIN-04) Summary

**One-liner:** Register `SecurityEventLayer` in `nono-agentd` via a minimal daemon-side init helper threaded from the SOLE HKLM read, with a non-host-gated D-01 test proving in-process `nono_security::network_deny` events advance the HMAC chain (sequence 0 → 1).

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 0 | Telemetry reachability probe + chain_sequence accessor | `2573af29` | nono-agentd.rs, telemetry/mod.rs |
| 1 | D-03 tuple extension + D-02 daemon init + D-01 test | `29499244` | agent_daemon/mod.rs, telemetry_init.rs, nono-agentd.rs |
| 2 | Native clippy clean + cross-target PARTIAL→CI | `bdbe237c` | telemetry_init.rs, telemetry/mod.rs |

## Verification

- `cargo build -p nono-cli --bin nono-agentd`: PASS (no E0433/E0583 unresolved-module errors)
- `cargo test -p nono-cli --bin nono-agentd`: **69 passed, 0 failed**
  - `telemetry_init::tests::d01_network_deny_advances_chain_sequence_to_one`: PASS
  - `telemetry_init::tests::opt_out_disabled_layer_does_not_advance_chain`: PASS
  - `telemetry::tests::chain_sequence_genesis_is_zero`: PASS
- SOLE-read: exactly one `nono::read_machine_egress_policy()` call in agent_daemon/mod.rs (line 363)
- `init_daemon_telemetry` called in both `run_service` and `run_foreground_mode` after policy resolution
- Native clippy (`cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used`): CLEAN
- Cross-target clippy: **PARTIAL→CI** (see below)

## Must-Haves Verified

| Truth | Status |
|-------|--------|
| nono-agentd registers SecurityEventLayer in both service and foreground modes | PASS — OnceLock guard; init_daemon_telemetry called in both run_service and run_foreground_mode |
| In-process network_deny event reaching the layer advances the HMAC chain | PASS — D-01 test: sequence 0→1 via SpyLayer→SecurityEventLayer::on_event→advance_chain |
| Layer honors HKLM policy.telemetry enabled opt-out and min_severity threshold | PASS — opt-out test: enabled=false → sequence stays 0 (T-90-03) |
| Daemon performs exactly one read_machine_egress_policy() call | PASS — SOLE-read preserved; telemetry threaded from existing return |
| Non-host-gated test proves event reaches on_event (chain sequence == 1) | PASS — telemetry_init::tests::d01_network_deny_advances_chain_sequence_to_one |

| Artifact | Status |
|----------|--------|
| crates/nono-cli/src/agent_daemon/telemetry_init.rs (fn init_daemon_telemetry + D-01 test) | CREATED |
| crates/nono-cli/src/bin/nono-agentd.rs (mod telemetry + mod telemetry_init + init calls) | MODIFIED |
| crates/nono-cli/src/agent_daemon/mod.rs (TelemetryConfig in return tuple) | MODIFIED |
| crates/nono-cli/src/telemetry/mod.rs (chain_sequence accessor) | MODIFIED |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Arc<SecurityEventLayer> does not implement Layer<S> in tracing-subscriber 0.3.23**
- **Found during:** Task 1 RED phase test compilation
- **Issue:** Assumption A2 in RESEARCH.md was incorrect — `Arc<L: Layer<S>>` impl is absent from tracing-subscriber 0.3.23; compiler error E0277 on `.with(arc_layer)` in `with_default`
- **Fix:** Introduced `SpyLayer` — a thin delegation wrapper struct that owns a `SecurityEventLayer`, delegates `on_event` to it, and mirrors the `chain_sequence()` value into a shared `Arc<Mutex<u64>>` readable after the `with_default` closure. Avoids any unsafe code.
- **Files modified:** `crates/nono-cli/src/agent_daemon/telemetry_init.rs`
- **Commit:** `29499244`

**2. [Rule 1 - Bug] chain_sequence() dead_code lint in native clippy**
- **Found during:** Task 2 native clippy run
- **Issue:** `pub(crate) fn chain_sequence()` was flagged as "never used" by clippy because Rust's dead_code lint considers test-only usage non-production; the method was only called from `#[cfg(test)]` blocks
- **Fix:** Gated the method with `#[cfg(test)]` in `telemetry/mod.rs` — it is a test accessor, not a production API. Added `chain_sequence_genesis_is_zero` test in `telemetry/mod.rs` to also exercise it from the `nono` binary compilation unit. Per CLAUDE.md: "Avoid `#[allow(dead_code)]`. If code is unused, either remove it or write tests that use it."
- **Files modified:** `crates/nono-cli/src/telemetry/mod.rs`
- **Commit:** `bdbe237c`

**3. [Rule 1 - Bug] Duplicate trait imports in test module (`prelude::*` unused)**
- **Found during:** Task 2 native clippy run
- **Issue:** The test mod used `use super::*` which already imported `tracing_subscriber::prelude::*` from the outer module scope; adding it again in the test block triggered `unused import: prelude::*`
- **Fix:** Removed duplicate import; test module relies on `use super::*` for trait access
- **Files modified:** `crates/nono-cli/src/agent_daemon/telemetry_init.rs`
- **Commit:** `bdbe237c`

## Cross-Target Clippy Disposition

**PARTIAL→CI** for both cross-target legs.

| Target | Result | Reason |
|--------|--------|--------|
| x86_64-unknown-linux-gnu | PARTIAL→CI | `failed to run custom build command for ring v0.17.14` — C cross-linker (`x86_64-linux-gnu-gcc`) absent on Windows dev host |
| x86_64-apple-darwin | PARTIAL→CI | Same C-linker failure (aws-lc-sys/ring cannot build without macOS-targeting C toolchain) |

Both Rust targets ARE installed (`rustup target list --installed` confirms both). The failure is a C build system constraint, not a Rust code error. Per `.planning/templates/cross-target-verify-checklist.md`:

> Cross-target clippy gate SKIPPED on Windows dev host due to missing toolchain (x86_64-{unknown-linux-gnu | apple-darwin}). The live GH Actions {Linux Clippy | macOS Clippy} lane on the head SHA is the decisive signal per .planning/templates/cross-target-verify-checklist.md. REQ marked PARTIAL pending CI confirmation.

DRAIN-04 cross-target: **PARTIAL→CI**. Decisive gates: GH Actions Linux/macOS clippy lanes on head SHA `bdbe237c`.

## Known Stubs

None. All wiring is functional: `init_daemon_telemetry` constructs a real `SecurityEventLayer`, registers it via the tracing subscriber global, and the D-01 test proves events reach `on_event`.

## Threat Flags

No new threat surface introduced. Changes wire an existing, audited layer (`SecurityEventLayer`) into the daemon's process. The layer's HMAC chain, path-hashing, scrub_value, and ETW/Application-Log sinks are unchanged. The `resolve_machine_egress_policy` Err→abort posture is preserved.

## Self-Check: PASSED

**Files exist:**
- `crates/nono-cli/src/agent_daemon/telemetry_init.rs`: FOUND
- `crates/nono-cli/src/bin/nono-agentd.rs`: FOUND (modified)
- `crates/nono-cli/src/agent_daemon/mod.rs`: FOUND (modified)
- `crates/nono-cli/src/telemetry/mod.rs`: FOUND (modified)

**Commits exist:**
- `2573af29`: FOUND (Task 0 — reachability probe)
- `29499244`: FOUND (Task 1 — TelemetryConfig thread + init helper + D-01 test)
- `bdbe237c`: FOUND (Task 2 — native clippy clean + cross-target PARTIAL→CI)

**Test result:** 69 passed, 0 failed
**SOLE-read:** Exactly one `nono::read_machine_egress_policy()` call confirmed
**Native clippy:** CLEAN (0 errors)
