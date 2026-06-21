---
phase: 90-v3-0-host-gated-uat-drain
reviewed: 2026-06-20T00:00:00Z
depth: standard
files_reviewed: 4
files_reviewed_list:
  - crates/nono-cli/src/agent_daemon/telemetry_init.rs
  - crates/nono-cli/src/bin/nono-agentd.rs
  - crates/nono-cli/src/agent_daemon/mod.rs
  - crates/nono-cli/src/telemetry/mod.rs
findings:
  critical: 0
  warning: 4
  info: 3
  total: 7
status: issues_found
---

# Phase 90: Code Review Report

**Reviewed:** 2026-06-20T00:00:00Z
**Depth:** standard
**Files Reviewed:** 4
**Status:** issues_found

## Summary

Reviewed the DRAIN-04 daemon-side telemetry wiring: `init_daemon_telemetry`
(`telemetry_init.rs`), its two call sites in `nono-agentd.rs`, the extended
`resolve_machine_egress_policy` return tuple (`agent_daemon/mod.rs`), and the
`chain_sequence()` test accessor (`telemetry/mod.rs`).

The core security invariants hold: the SOLE HKLM read is preserved (exactly one
`nono::read_machine_egress_policy()` call in `agent_daemon/mod.rs:363`),
`telemetry_config` is threaded from the existing return rather than via a second
read, the `Err → abort` fail-secure path for egress is unchanged, and there is
no `.unwrap()`/`.expect()` in production code (all are confined to `#[cfg(test)]`
blocks). The OnceLock double-init guard and `try_init()` are correct mechanically.

No BLOCKER-class defects were found. The findings are quality/robustness gaps:
a silent fail-open on telemetry *registration* (the phase's whole purpose is to
make telemetry reach the daemon, yet a failed registration is swallowed without
even a degraded-mode warning), a D-01 test that does not actually exercise the
production registration path it claims to validate, a stale doc comment claiming
the non-Windows arm is a "no-op" when it registers a live subscriber, and an
unreachable double-init scenario the guard documents as the reason it exists.

## Warnings

### WR-01: Telemetry subscriber-registration failure is silently swallowed (fail-open on security observability)

**File:** `crates/nono-cli/src/agent_daemon/telemetry_init.rs:59-82`
**Issue:** `init_daemon_telemetry` discards the `Result` of `try_init()` in every
arm (`let _ = ... .try_init();`). If a global subscriber was already set in this
process by any other code path, `try_init()` returns `Err` and the
`SecurityEventLayer` is **never installed** — yet the daemon continues running
with no log line indicating that security telemetry is dark. For a
security-critical telemetry path (SIEM/EDR forwarding of `nono_security::*`
denials), a silent registration failure is a fail-open on observability: the
admin believes events are flowing when they are not. Contrast with the ETW arm,
which at least emits an `eprintln!` on failure, and with `telemetry/mod.rs`'s
own design note that telemetry degradation should surface a `TelemetryDegraded`
event. Note also that this is the only signal the daemon emits about telemetry
state at all — there is no "telemetry active" confirmation line either.
**Fix:** Distinguish "second call, intentional no-op" (already guarded by the
OnceLock above) from "registration genuinely failed", and warn on the latter:
```rust
match base.try_init() {
    Ok(()) => {}
    Err(e) => eprintln!(
        "nono-agentd: telemetry: SecurityEventLayer registration FAILED ({e}); \
         security events will NOT be forwarded (fail-open observability gap)"
    ),
}
```
Because the OnceLock guard has already returned early for the second call, any
`Err` reaching `try_init()` here is a genuine failure worth surfacing.

### WR-02: D-01 test does not exercise the production `init_daemon_telemetry` path

**File:** `crates/nono-cli/src/agent_daemon/telemetry_init.rs:120-191`
**Issue:** The phase deliverable is wiring `SecurityEventLayer` into the daemon
via `init_daemon_telemetry`. The D-01 test
(`d01_network_deny_advances_chain_sequence_to_one`) builds a *separate* `SpyLayer`
that wraps a fresh `SecurityEventLayer` and registers it directly with
`tracing_subscriber::registry().with(spy)` — it never calls
`init_daemon_telemetry` at all. As a result, none of the production wiring is
covered: the OnceLock guard, the ETW-arm composition, the non-Windows arm, and
the registry composition inside the function are all untested. The test
effectively re-proves `SecurityEventLayer::on_event` advances the chain — which
`telemetry/mod.rs` already covers (`advance_chain_*`, `chain_sequence_genesis_is_zero`).
The genuinely new code (`init_daemon_telemetry`) has zero direct coverage. This
is a test-validity gap, not a runtime bug, but it means the "telemetry is
reachable from the daemon" claim rests on a test that bypasses the reachability
code.
**Fix:** Add a test that calls `init_daemon_telemetry(config, session_id)` and
asserts it does not panic and is idempotent across two calls (exercising the
OnceLock guard). Since `try_init()` makes a global-subscriber test order-dependent,
at minimum assert the double-call returns cleanly. Keep the existing `SpyLayer`
test for chain-advance, but rename it so it does not imply it validates the daemon
init path.

### WR-03: Stale doc comment claims the non-Windows arm is a "no-op" when it registers a live subscriber

**File:** `crates/nono-cli/src/agent_daemon/telemetry_init.rs:42-44,56-62`
**Issue:** The module/function docs state: "On non-Windows targets the function
is a no-op so the source file compiles cross-platform for clippy." This is false.
The `#[cfg(not(target_os = "windows"))]` arm (lines 56-62) constructs a real
`SecurityEventLayer::new(config, session_id)` and calls
`tracing_subscriber::registry().with(security_layer).try_init()` — it installs a
global subscriber. Only the ETW *arm* is Windows-gated; the security layer is
registered on all platforms. A reader trusting the doc would wrongly believe
Linux/macOS daemon builds emit no telemetry and would not look for a global
subscriber being set. Given CLAUDE.md's emphasis on auditable, explicit
security-relevant behavior, the comment actively misleads.
**Fix:** Correct the doc to: "On non-Windows targets, the `SecurityEventLayer` is
still registered (so chain advancement is exercised cross-platform); only the
Windows-specific ETW provider arm is skipped." Update the function-level
`# Platform` section (lines 40-44) the same way.

### WR-04: OnceLock guard documents a double-init scenario that cannot occur, masking the real risk

**File:** `crates/nono-cli/src/agent_daemon/telemetry_init.rs:15-20,45-52` and `crates/nono-cli/src/bin/nono-agentd.rs:185-192,293-299`
**Issue:** The guard's rationale (telemetry_init.rs:15-20) states `run_service()`
"may fall through to `run_foreground_mode()`" and "both code paths call this
helper", so the OnceLock prevents a double-init panic. Tracing the actual control
flow contradicts this: `run_service()` (which holds the first init call at
nono-agentd.rs:189) is invoked **only** by the SCM via
`ffi_service_main → service_main → run_service`. The fallback in
`run_service_mode` (nono-agentd.rs:241-252) calls `run_foreground_mode()` only
when `service_dispatcher::start` returns `Err` — i.e. when the SCM never connected
and `run_service` was therefore **never** entered. The two init sites are mutually
exclusive in every real path, so the documented "double-init across the fallback"
cannot happen. The guard is harmless but its stated justification is incorrect,
which is a maintenance hazard: a future refactor that *does* introduce a real
second call (e.g. a re-init after config reload) would be reasoned about against a
false mental model. Separately, `try_init()` (not `init()`) already makes a second
call non-panicking even without the OnceLock, so the guard's "avoid panic" framing
is redundant with the mechanism actually chosen.
**Fix:** Either keep the guard but correct the comment to state it is defensive
against a *hypothetical* future second call (not the current fallback path, which
is mutually exclusive), or document that `try_init()` alone already guarantees
no-panic and the OnceLock additionally guarantees no *duplicate* layer if a real
second call is ever introduced. Do not claim the current fallback triggers a
double init.

## Info

### IN-01: Daemon session_id is derived solely from PID, which is reused across restarts

**File:** `crates/nono-cli/src/bin/nono-agentd.rs:191,298`
**Issue:** `session_id` is `format!("nono-agentd-service-{}", std::process::id())`
(and the `-foreground-` variant). Windows reuses PIDs, so two daemon lifetimes can
share the same `session_id`. The HMAC chain key is ephemeral per process so chains
remain cryptographically independent, but a SIEM correlating by `session_id`
string could conflate two distinct daemon runs. Low risk given the key rotation,
but the identifier is weaker than the "opaque per-session identifier" the doc
implies.
**Fix:** Mix in a startup nonce, e.g. append a few random hex bytes
(`getrandom::fill`) or the start timestamp, to the PID-based id.

### IN-02: `min_severity` default-ON parity is asserted but the daemon path is only structurally tested

**File:** `crates/nono-cli/src/agent_daemon/mod.rs:406,650-670`
**Issue:** `resolve_machine_egress_policy`'s absent branch returns
`nono::TelemetryConfig::default()` (default-ON, matching `main.rs` CLI parity at
main.rs:173-177). The test `machine_policy_handoff_absent_falls_through_to_per_user`
only asserts `telemetry_cfg.enabled` inside an `if !active` block, so on a host
where a machine policy key *does* exist the telemetry assertion is silently
skipped. The wholesale-override branch (`Some(policy)`) telemetry threading
(`policy.telemetry.clone()` at mod.rs:370) has no direct unit assertion that the
returned config equals the policy's telemetry. Coverage gap, not a defect.
**Fix:** Add a unit test that constructs a `MachineEgressPolicy` with a non-default
`telemetry` (e.g. `enabled: false`) and asserts the resolver returns that exact
config in the third tuple element, independent of host registry state.

### IN-03: Fail-secure divergence between CLI and daemon on a malformed telemetry key is intentional but undocumented at the daemon call site

**File:** `crates/nono-cli/src/bin/nono-agentd.rs:173-183,284-291`
**Issue:** In `main.rs:168-177` a malformed/absent machine policy is non-fatal for
telemetry (D-14 degrade-to-default). In the daemon, the same
`read_machine_egress_policy()` failure aborts startup fail-secure (because egress
enforcement drives the abort). This means a corrupt `HKLM\...\nono\Telemetry`
value that breaks the whole-key parse will take the daemon down, whereas the CLI
would keep running. This is the correct posture (the daemon must fail secure on
egress), and telemetry is merely co-resident on the same read — but the divergence
from the CLI's D-14 contract is not noted at the daemon call site and could
surprise a maintainer debugging a daemon that refuses to start after a telemetry
mis-config.
**Fix:** Add a one-line comment at nono-agentd.rs:173 and :284 noting that, unlike
the CLI (D-14 non-fatal telemetry), the daemon's egress fail-secure (D-07)
subsumes telemetry — any HKLM read error aborts startup by design, telemetry
included.

---

_Reviewed: 2026-06-20T00:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
