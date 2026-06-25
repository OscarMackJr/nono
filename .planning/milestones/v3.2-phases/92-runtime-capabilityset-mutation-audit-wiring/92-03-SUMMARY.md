---
phase: 92-runtime-capabilityset-mutation-audit-wiring
plan: "03"
subsystem: audit-wiring
tags: [audit, telemetry, policy-override, aud-01, aud-04, hmac-chain, cli, launch-runtime]
dependency_graph:
  requires:
    - 92-01  # SecurityEventType PolicyOverride* variants + EventIDs 10006-10010
    - 92-02  # nono-py emits --override-audit <base64-json> + --allow flags
  provides:
    - OverrideAuditMeta struct in crates/nono-cli/src/cli.rs (serde deny_unknown_fields)
    - --override-audit hidden SandboxArgs field in cli.rs
    - override_audit: Option<OverrideAuditMeta> threaded through ExecutionFlags
    - SECURITY_LAYER: OnceLock<SecurityEventLayer> in telemetry/mod.rs
    - emit_override_event() #[must_use] method on SecurityEventLayer (AUD-04 fail-closed)
    - AUD-04 pre-spawn gate in execution_runtime.rs::execute_sandboxed
  affects:
    - 92-04  # DF-01 dark factory gate depends on Plan 03 bilateral handshake
tech_stack:
  added:
    - base64 = "0.22" direct dep in nono-cli Cargo.toml (0.22.1 already in lockfile)
  patterns:
    - SecurityEventLayer Arc<Mutex<...>> inner for cheap Clone (shared chain state)
    - OnceLock<SecurityEventLayer> set once by init_tracing, read by execute_sandboxed
    - DECODE-ONCE: base64url-JSON decode at launch_runtime boundary; execute_sandboxed sees typed struct
    - AUD-04 fail-closed: emit_override_event returns Err on poisoned mutex; gate aborts before spawn
key_files:
  created: []
  modified:
    - crates/nono-cli/src/cli.rs
    - crates/nono-cli/src/launch_runtime.rs
    - crates/nono-cli/src/telemetry/mod.rs
    - crates/nono-cli/src/cli_bootstrap.rs
    - crates/nono-cli/src/agent_daemon/telemetry_init.rs
    - crates/nono-cli/src/execution_runtime.rs
    - crates/nono-cli/Cargo.toml
key-decisions:
  - "SecurityEventLayer.inner changed from Mutex<...> to Arc<Mutex<...>> + #[derive(Clone)] to share chain state between SECURITY_LAYER clone and tracing registry clone without requiring Arc<SecurityEventLayer> as Layer<S> (not supported in tracing-subscriber 0.3.23)"
  - "SECURITY_LAYER: OnceLock<SecurityEventLayer> (not OnceLock<Arc<SecurityEventLayer>>) because SecurityEventLayer is now Clone"
  - "base64 0.22 promoted to direct dep (already in lockfile); no new package legitimacy concern"
  - "emit_override_event uses #[allow(dead_code)] for nono-agentd binary: method IS used in nono binary + tests; dead_code fires only because execute_sandboxed is not compiled into agentd (multi-binary crate artifact, not actual dead code)"
  - "#[must_use = msg] instead of bare #[must_use] to avoid double_must_use clippy lint (Result is already must_use)"
requirements-completed:
  - AUD-01
  - AUD-02
  - AUD-03
  - AUD-04
metrics:
  duration: "~35 minutes"
  completed: "2026-06-22"
  tasks_completed: 2
  tasks_total: 2
  files_modified: 7
---

# Phase 92 Plan 03: nono-cli Override Wiring Summary

**nono-cli bilateral AUD-04 handshake wired: --override-audit base64url-JSON flag + OverrideAuditMeta struct in cli.rs, SECURITY_LAYER OnceLock + emit_override_event() on SecurityEventLayer, and pre-spawn gate in execute_sandboxed that aborts before any spawn if the PolicyOverrideVerified event cannot be committed to the HMAC chain.**

## Performance

- **Duration:** ~35 min
- **Started:** 2026-06-22T~13:30Z
- **Completed:** 2026-06-22T~14:05Z
- **Tasks:** 2
- **Files modified:** 7

## Accomplishments

- `OverrideAuditMeta` struct with `serde(deny_unknown_fields)` in `cli.rs`; `--override-audit` hidden `SandboxArgs` field; base64url-no-pad decode in `prepare_run_launch_plan` (DECODE-ONCE pattern); `ExecutionFlags.override_audit: Option<OverrideAuditMeta>` threaded through
- `emit_override_event()` `#[must_use]` method on `SecurityEventLayer`: locks `Arc<Mutex<SecurityEventLayerInner>>`, builds canonical chain bytes, calls `advance_chain`, emits `SecurityEvent` when telemetry enabled; returns `Err("mutex poisoned")` on poisoned lock (AUD-04 fail-closed)
- `SECURITY_LAYER: OnceLock<SecurityEventLayer>` in `telemetry/mod.rs`; populated in `init_tracing` (cli_bootstrap.rs) and `init_daemon_telemetry` (agent_daemon/telemetry_init.rs) via `security_layer.clone()`
- AUD-04 pre-spawn gate in `execute_sandboxed`: placed AFTER `start_proxy_runtime`, BEFORE `apply_pre_fork_sandbox`; calls `SECURITY_LAYER.get().emit_override_event(...)` when `flags.override_audit` is `Some`; returns `Err(NonoError::SandboxInit(...))` on any failure
- 7 new unit tests: 3 `OverrideAuditMeta` deserialization tests (valid, deny_unknown_fields, null zt_audit_hash) + 4 `emit_override_event` tests (chain advance, mutex poison, two-call advance, None zt_audit_hash)

## Task Commits

1. **Task 1: --override-audit flag, OverrideAuditMeta struct, ExecutionFlags threading** - `64922272` (feat)
2. **Task 2: emit_override_event method, SECURITY_LAYER OnceLock, AUD-04 pre-spawn gate** - `6480d897` (feat)

## Files Created/Modified

- `crates/nono-cli/src/cli.rs` - Added `OverrideAuditMeta` struct (deny_unknown_fields), `--override-audit` hidden SandboxArgs field, `WrapSandboxArgs` From impl `override_audit: None`, 3 deserialization tests
- `crates/nono-cli/src/launch_runtime.rs` - Added `override_audit: Option<OverrideAuditMeta>` to `ExecutionFlags` struct + defaults; added base64url-JSON decode in `prepare_run_launch_plan`
- `crates/nono-cli/src/telemetry/mod.rs` - Changed `SecurityEventLayer.inner` to `Arc<Mutex<...>>` + `#[derive(Clone)]`; added `SECURITY_LAYER` OnceLock static; added `emit_override_event()` method; added 4 unit tests
- `crates/nono-cli/src/cli_bootstrap.rs` - Populates `SECURITY_LAYER` via `security_layer.clone()` in `init_tracing`
- `crates/nono-cli/src/agent_daemon/telemetry_init.rs` - Populates `SECURITY_LAYER` via `security_layer.clone()` in `init_daemon_telemetry`
- `crates/nono-cli/src/execution_runtime.rs` - AUD-04 pre-spawn gate in `execute_sandboxed`
- `crates/nono-cli/Cargo.toml` - Added `base64 = "0.22"` direct dep

## Decisions Made

### SecurityEventLayer inner field change to Arc<Mutex<...>>

**Decision:** Change `SecurityEventLayer.inner` from `Mutex<SecurityEventLayerInner>` to `Arc<Mutex<SecurityEventLayerInner>>` and derive `Clone`, rather than wrapping in `Arc<SecurityEventLayer>` in the `OnceLock`.

**Rationale:** `tracing-subscriber 0.3.23` does NOT implement `Layer<S>` for `Arc<T>` (this was added in later releases — confirmed by the `SpyLayer` workaround in `agent_daemon/telemetry_init.rs`). Using `Arc<SecurityEventLayer>` would require changing `init_tracing_with_security`'s signature. The `Arc<Mutex<...>>` inner approach makes `SecurityEventLayer` itself `Clone` with O(1) cost; both the SECURITY_LAYER clone and the tracing registry copy share the same underlying chain state, which is the correctness requirement.

### DECODE-ONCE placement in launch_runtime.rs

**Decision:** The base64url-JSON decode of `--override-audit` happens in `prepare_run_launch_plan`, not in `execute_sandboxed`.

**Rationale:** `execute_sandboxed` receives `flags.override_audit: Option<OverrideAuditMeta>` (the typed struct). This aligns with 92-PATTERNS.md DECODE-ONCE rule and ensures type-checked access in the AUD-04 gate without any string manipulation at the security-critical spawn boundary.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] #[must_use] → #[must_use = "msg"] to avoid double_must_use clippy error**
- **Found during:** Task 2 (clippy verification with -D warnings)
- **Issue:** `Result<String, &'static str>` is already `#[must_use]`; bare `#[must_use]` on a function returning it triggers `clippy::double_must_use` which becomes an error under `-D warnings`
- **Fix:** Changed to `#[must_use = "AUD-04: Err means the audit record was not committed — callers MUST return Err before spawning (never silently proceed)"]`
- **Files modified:** `crates/nono-cli/src/telemetry/mod.rs`
- **Commit:** `6480d897`

**2. [Rule 1 - Bug] base64 not in nono-cli Cargo.toml (only a transitive dep)**
- **Found during:** Task 1 build
- **Issue:** `use base64::Engine as _` in `launch_runtime.rs` fails with E0432 — `base64` was only a transitive dep (via other crates), not a direct dep of nono-cli
- **Fix:** Added `base64 = "0.22"` to nono-cli Cargo.toml (0.22.1 was already in Cargo.lock; no new package install)
- **Files modified:** `crates/nono-cli/Cargo.toml`, `Cargo.lock`
- **Commit:** `64922272`

**3. [Rule 2 - Missing Critical] #[allow(dead_code)] on emit_override_event for nono-agentd**
- **Found during:** Task 2 clippy verification
- **Issue:** `emit_override_event` is used in `execution_runtime.rs` which is compiled only for the `nono` binary, not `nono-agentd`. The Rust compiler issues a `dead_code` warning for the daemon binary — not a real dead code issue, just a multi-binary compilation artifact
- **Fix:** Added `#[allow(dead_code)]` with an explanatory comment on the method; the method IS used in production (nono binary) and in unit tests
- **Files modified:** `crates/nono-cli/src/telemetry/mod.rs`
- **Commit:** `6480d897`

---

**Total deviations:** 3 auto-fixed (1 bug, 1 blocking, 1 missing critical)
**Impact on plan:** All auto-fixes necessary for correctness and compilation. No scope creep.

## Test Results

```
cargo test -p nono-cli --bin nono cli::
  126 passed; 0 failed (includes 3 new OverrideAuditMeta deserialization tests)

cargo test -p nono-cli --bin nono telemetry::
  37 passed; 0 failed (includes 4 new emit_override_event tests)

cargo test -p nono-cli
  1360 passed; 4 failed (4 pre-existing Windows baseline failures: profile_cmd init + 3
  protected_paths — documented in nono_cli_windows_baseline_test_failures.md, NOT regressions)

cargo build --workspace
  Finished (all 5 crates + bindings green)

cargo clippy --bin nono -- -D warnings -D clippy::unwrap_used
  Finished (0 errors, 0 warnings)

cargo clippy --bin nono-agentd -- -D warnings -D clippy::unwrap_used
  Finished (0 errors, 0 warnings)
```

## Cross-Target Clippy Status

**PARTIAL → CI** (consistent with Plans 01 and 02)

Attempted `cargo clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` — cross-toolchain (`x86_64-linux-gnu-gcc`) is not installed on this Windows 11 host. Same result for `--target x86_64-apple-darwin`.

The changes in `telemetry/mod.rs` are cfg-unconditional (no platform guards on `emit_override_event` or `SECURITY_LAYER`). The `SecurityEventLayer.inner` type change (Arc<Mutex<...>>) is also unconditional. Native Windows `cargo build --workspace` exits 0. Linux/macOS verification deferred to live CI per CLAUDE.md rule.

## Known Stubs

None — all fields are wired to real data. The AUD-04 gate calls the real `emit_override_event` which advances the real HMAC chain.

## Threat Flags

No new network endpoints, auth paths, file access patterns, or schema changes at trust boundaries beyond those documented in the plan's `<threat_model>`.

Threat dispositions per plan:
- T-92-FAILCLOSED: Mitigated — `emit_override_event` returns `Result`; gate returns `Err` before spawn on emit failure
- T-92-MUTEX: Mitigated — `map_err` on `lock()` converts poisoned mutex to `Err("mutex poisoned")`; AUD-04 caller treats Err as fatal
- T-92-META-FORGE: Accepted — nono-cli treats metadata as log-only (no re-verification); forged metadata is harmless to security
- T-92-WINDOW: Mitigated — Gate placed AFTER `start_proxy_runtime` (line 242) and BEFORE `apply_pre_fork_sandbox` (line 305)
- T-92-DENY_UNKNOWN: Mitigated — `serde(deny_unknown_fields)` on `OverrideAuditMeta`; deserialization Err → SandboxInit before spawn
- T-92-SC: Accepted — No new packages; base64 is existing-lockfile dep promoted to direct dep

## Self-Check: PASSED

| Item | Status |
|------|--------|
| crates/nono-cli/src/cli.rs modified (OverrideAuditMeta + field) | FOUND |
| crates/nono-cli/src/launch_runtime.rs modified (ExecutionFlags + decode) | FOUND |
| crates/nono-cli/src/telemetry/mod.rs modified (SECURITY_LAYER + emit_override_event) | FOUND |
| crates/nono-cli/src/cli_bootstrap.rs modified (SECURITY_LAYER.set) | FOUND |
| crates/nono-cli/src/agent_daemon/telemetry_init.rs modified (SECURITY_LAYER.set) | FOUND |
| crates/nono-cli/src/execution_runtime.rs modified (AUD-04 gate) | FOUND |
| 92-03-SUMMARY.md exists | FOUND |
| Commit 64922272 (Task 1) exists | FOUND |
| Commit 6480d897 (Task 2) exists | FOUND |
