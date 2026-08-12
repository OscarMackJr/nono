---
phase: 117-fail-direction-contract-startup-self-attestation
plan: 37
subsystem: telemetry
tags: [windows, d-28, information-disclosure, gap-closure, round-4, cr-05, cint-02]

# Dependency graph
requires:
  - phase: 117-fail-direction-contract-startup-self-attestation
    provides: "117-35's --no-fail-fast baseline, for reconciling the full-suite result"
provides:
  - "write_security_event_log's NULL-handle stderr fallback renders a downgraded_layers-redacted
    copy of the event — the fourth D-28 leak site, four call frames downstream of the function
    CR-03 patched"
  - "build_redacted_fallback_message — one function shared by the NULL branch and its test, so the
    redaction is perturbation-provable despite the FFI branch being undrivable from a unit test"
affects: [117-44]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "When the real branch cannot be driven from a test, extract the logic into a function BOTH
      the branch and the test call — a test that re-implements the logic passes after the
      production copy is removed, which is a test that cannot fail"
    - "Channel asymmetry is expressed in the TYPE: taking &SecurityEvent rather than a pre-built
      body: &str makes it impossible for a caller to hand one string to two channels with
      different audiences"

key-files:
  created: []
  modified:
    - crates/nono-cli/src/telemetry/windows.rs

key-decisions:
  - "Deviated from the plan's prescribed test shape. It said to build the redacted clone in the
    test 'the same way the fallback branch does'; that test would pass even with the production
    redaction removed. Extracted build_redacted_fallback_message so both paths share one
    implementation and the mandated perturbation proof actually bites."
  - "Ran both cross-target clippy gates even though the plan exempts this file (0 Unix cfg blocks,
    not under exec_strategy_windows/). Moving build_event_payload's calls inside a
    #[cfg(target_os = \"windows\")] function is exactly what makes a helper dead on non-Windows."
  - "The Event Log write keeps the FULL event including downgraded_layers. D-27 makes the
    Application Event Log the legitimate operator/audit channel; only the shared console is
    D-28-scoped. Redacting both would have destroyed the audit record to fix a disclosure."
---

# Plan 117-37: CR-05 — Execution Summary

**Executed:** 2026-08-12 · **Wave:** 16 · **Requirement:** CINT-02

## The leak

`apply_startup_attestation_gate` → `emit_attestation_event` → `emit_security_event` →
`write_security_event_log` → NULL-handle branch → `eprintln!` of the full serialized
`SecurityEvent` **including `downgraded_layers`**, onto the supervisor's stderr — the channel the
confined child shares on the non-detached-stdio path.

Round 3's CR-03 enumeration stopped at `apply_startup_attestation_gate`'s function boundary. It
correctly gated the three emission sites *inside* that function on one shared predicate and never
followed the call chain four frames down. The fallback is a raw `eprintln!`, not a `tracing` call,
so `log_target_is_private()` — the predicate CR-02/CR-03 built their entire approach around —
**structurally cannot gate it**. It was invisible to that approach by construction.

## Task 1 — redaction at the render site

`write_security_event_log`'s signature changed from `body: &str` to `event: &SecurityEvent`. That
is the load-bearing part: the two channels this function writes to have different audiences, and a
pre-built string cannot be simultaneously correct for both.

| Channel | Audience | Content |
|---|---|---|
| `ReportEventW` → Application Event Log | operator / audit (D-27) | **full event**, `downgraded_layers` included |
| `eprintln!` → supervisor stderr | shared with the confined child | **redacted** copy, `downgraded_layers: None` |

`emit_security_event` now passes the event through and no longer pre-builds a payload; the
`let _ = payload;` non-Windows suppression is gone with it. `build_event_payload` gained
`#[cfg_attr(not(target_os = "windows"), allow(dead_code))]`, matching the three existing precedents
in this file (`EVENT_LOG_SOURCE`, `EventLogLevel`, `build_event_log_message`).

## Independently re-derived enumeration (required by Task 1)

Re-ran `grep -rn "downgraded_layers" crates/ bindings/ --include=*.rs` in this session rather than
trusting the plan's account:

| Site | Value | Kind |
|---|---|---|
| `exec_strategy_windows/attestation_downgrade_event.rs:146` | `Some(downgraded_layers_value.clone())` | **the one production producer** |
| `telemetry/event.rs:377` | `Some("AppContainerProfile,DaclPackageSidGrant")` | test |
| `telemetry/mod.rs:418`, `:610` | `None` | production |
| `telemetry/windows.rs:262` | `None` | test |
| `telemetry/event.rs:403`, `:568` | `None` | test |

The plan's claim is confirmed. **The fix covers the class by construction**: it redacts inside the
shared fallback function, not at any call site, so a future producer of `downgraded_layers: Some(..)`
inherits the redaction without needing to know about it.

## Deviation: the prescribed test could not fail

The plan's `<action>` said to construct the redacted clone in the test "the same way the fallback
branch does". Written that way, the test re-implements the redaction — so removing the production
redaction leaves it **passing**, and the plan's own mandated perturbation proof would have been
impossible to satisfy honestly.

Extracted `build_redacted_fallback_message(level, event_id, event)`. The NULL branch calls it; the
test calls it. Its doc comment records why it exists as a separate function.

### Perturbation proof

Replaced the redacting clone with a plain `event.clone()` inside the shared helper:

```
thread '...null_handle_fallback_rendering_never_carries_a_downgraded_layer_name' panicked at
crates\nono-cli\src\telemetry\windows.rs:372:9:
the stderr fallback rendering must not carry a LayerId name (D-28) — got:
[WARN] source=nono event_id=10011 {"EventType":"layer_attestation_downgraded","AgentPid":4242,
"SessionId":"test-session","ChainHead":"000...000","TimestampUnixMs":0,
"DowngradedLayers":"MandatoryIntegrityLabel,AppContainerProfile"}
test result: FAILED. 0 passed; 1 failed
```

That is CR-05's exact payload reaching the shared channel. Reverted; re-run green.

The test additionally asserts (a) the field NAME `downgraded_layers` is absent, since
`skip_serializing_if` omits it when `None`, and (b) a **non-vacuity** check that the *unredacted*
rendering DOES contain the layer name — without which the primary assertions could be satisfied by
a payload that never carried the names at all.

## Verification

| Gate | Command | Result |
|---|---|---|
| Target module | `cargo test -p nono-sandbox-cli --bin nono --no-fail-fast -- --test-threads=1 telemetry::windows::` | **4 passed / 0 failed** |
| Wider module | `... -- --test-threads=1 telemetry::` | **43 passed / 0 failed** |
| Format | `cargo fmt --all -- --check` | clean |
| Cross-target clippy (linux-gnu) | `cross clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` | **PASS** — exit 0, 0 warnings, 0 errors |
| Cross-target clippy (apple-darwin) | `cargo-zigbuild clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used` | **PASS** — exit 0, 0 warnings, 0 errors |

Per the plan's instruction to record rather than silently skip:
`grep -c 'cfg(target_os = "linux")\|cfg(target_os = "macos")' crates/nono-cli/src/telemetry/windows.rs`
→ **0**, and the file is not under `exec_strategy_windows/` or `bindings/c/src/`, so CLAUDE.md's
MUST rule does not compel the cross-target gates here. They were run anyway because this change
moves calls behind a Windows-only `cfg`. Both passed; the `cfg_attr` added to `build_event_payload`
was defensive and its necessity was not counterfactually tested.

## Follow-on note for 117-44

117-44 (WR-26) needs to know whether the Application Event Log actually received a record, and
condition 3 of its three emptying conditions is exactly this NULL-handle branch. This plan did NOT
add a success/failure return to `write_security_event_log` — it is still `-> ()`. 117-44 will need
to add one, and its `<interfaces>` already instructs it to read this file's landed shape first.
