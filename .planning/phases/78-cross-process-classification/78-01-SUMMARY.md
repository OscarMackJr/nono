---
phase: 78-cross-process-classification
plan: "01"
subsystem: agent_daemon/control_loop
tags: [classify, daemon, control-pipe, sc4, clas-01, clas-02]
dependency_graph:
  requires: [phase-74-daemon, nono::AgentRegistry, nono::AgentClassification]
  provides: [ControlRequest::Classify, classify_response_string, handle_classify, handle_classify_testable]
  affects: [crates/nono-cli/src/agent_daemon/control_loop.rs]
tech_stack:
  added: []
  patterns:
    - pure-response-formatter (classify_response_string bypasses OS call for testability)
    - testable-shim pattern (handle_classify_testable / classify_response_string_testable)
key_files:
  created: []
  modified:
    - crates/nono-cli/src/agent_daemon/control_loop.rs
decisions:
  - "ControlRequest made pub(crate) + derive(Debug) so the deserialize test can reference the type from the sibling test module"
  - "classify_response_string uses `..` wildcard (not `package_sid: _`) to ensure 'package_sid' string literal does not appear in the function body (strict SC4 acceptance criterion)"
  - "classify_response_string_testable shim added alongside handle_classify_testable so tests (a)/(b) call the pure formatter without needing pub visibility on the private fn"
  - "All 4 tests gated #[cfg(target_os = 'windows')] — consistent with existing windows_impl test idiom; cross-target CI will not see breakage since the functions only exist on Windows"
metrics:
  duration: "~20 minutes"
  completed: "2026-06-17"
  tasks_completed: 1
  tasks_total: 1
  files_changed: 1
---

# Phase 78 Plan 01: Classify Verb — Daemon Control Pipe Summary

**One-liner:** Daemon-side `Classify` verb over the existing Medium-IL control pipe — pure `classify_response_string` formatter enforces SC4 (verdict-only, no package SID), 4 unit tests form the unattended `cargo test --bin nono-agentd -- classify` gate.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Add ControlRequest::Classify, classify_response_string, handle_classify, unit tests | `aaafe4ff` | `control_loop.rs` |

## Unattended Gate Output

```
cargo test --bin nono-agentd -- classify

running 4 tests
test agent_daemon::control_loop::tests::classify_non_appcontainer_pid_returns_not_an_agent ... ok
test agent_daemon::control_loop::tests::classify_request_deserializes ... ok
test agent_daemon::control_loop::tests::classify_response_aiagent_omits_package_sid ... ok
test agent_daemon::control_loop::tests::classify_response_notanagent ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 28 filtered out; finished in 0.00s
```

Full suite (cargo test --bin nono-agentd): 32/32 PASS — zero regressions.

## Verification Checklist

- [x] `ControlRequest::Classify { pid: u32 }` variant exists in enum
- [x] `classify_response_string` is a private pure function taking `&AgentClassification` and returning String
- [x] `handle_classify_testable` is pub(crate) and callable from test module
- [x] `cargo test --bin nono-agentd -- classify` passes all 4 tests
- [x] Test (a) asserts: result == "AiAgent", !result.contains("S-1-15-2"), !result.to_lowercase().contains("sid") — SC4 load-bearing proof
- [x] `grep -n "package_sid" control_loop.rs` — "package_sid" does NOT appear in `classify_response_string` or `handle_classify` function bodies (uses `..` wildcard)
- [x] Dispatch match contains `ControlRequest::Classify { pid } => handle_classify(&state, pid)` arm
- [x] `cargo clippy --bin nono-agentd -- -D warnings -D clippy::unwrap_used` PASS
- [x] `CONTROL_PIPE_SDDL` unchanged — still `"D:P(A;;GA;;;SY)(A;;GA;;;BA)(A;;GA;;;OW)S:(ML;;NW;;;ME)"` (SC3 invariant)
- [x] No `cfg(not(target_os = "windows"))` stub added — mirrors handle_list/handle_demote pattern exactly
- [x] Cross-target clippy: PARTIAL — all new code is cfg(windows)-gated via windows_impl module boundary; deferred to CI per CLAUDE.md cross-target rule

## Deviations from Plan

### Implementation Adjustments

**1. [Rule 1 - Precision] Used `..` wildcard instead of `package_sid: _` in AiAgent arm**
- **Found during:** acceptance criteria review
- **Issue:** Plan says `package_sid: _` (named discard) but the acceptance criteria grep check requires "package_sid" must NOT appear in the function body. `package_sid: _` would have been flagged.
- **Fix:** Used `AiAgent { .. }` (wildcard) — discards all fields without naming them. Strictly cleaner for SC4.
- **Files modified:** control_loop.rs

**2. [Rule 2 - Testability] Added classify_response_string_testable shim**
- **Found during:** test implementation
- **Issue:** Tests (a) and (b) call `classify_response_string` directly but the function is private to `windows_impl`. The test module is a sibling, not a child.
- **Fix:** Added `classify_response_string_testable` as a `#[cfg(test)] pub(crate)` shim, mirroring the existing `handle_list_testable` / `handle_demote_testable` pattern.
- **Files modified:** control_loop.rs

**3. [Rule 2 - Testability] Made ControlRequest pub(crate) + derive(Debug)**
- **Found during:** test (c) implementation
- **Issue:** Test (c) needs to match on `ControlRequest::Classify { pid }` from the sibling test module. Private enum → unreachable from `tests`.
- **Fix:** Added `pub(crate)` to `ControlRequest` and `#[derive(Debug)]` (needed for the panic message in the non-matching arm).
- **Files modified:** control_loop.rs

**4. [Decision] Tests (a), (b), (c) gated #[cfg(target_os = "windows")]**
- **Issue:** Plan said "do not gate (a), (b), (c)" but all three call into `windows_impl` functions (which only compile on Windows). Cross-platform versions would require non-Windows stubs — explicitly out of scope per plan ("no non-Windows arm, mirror handle_list/handle_demote exactly").
- **Outcome:** Gated all 4 tests with `#[cfg(target_os = "windows")]`, consistent with all existing tests that call `windows_impl` functions. CI does not break on Linux/macOS because the tests simply don't compile there (they have no non-Windows dependencies to break).

## Known Stubs

None — the Classify verb is fully wired. End-to-end SC1 (AiAgent from a real cross-process caller) is covered in Plan 02's integration test (gated `NONO_DAEMON_INTEGRATION_TESTS=1`).

## Threat Flags

None — all new code is inside the existing `windows_impl` module, behind the existing Medium-IL-only control pipe SDDL. No new network endpoints, auth paths, or trust boundaries introduced.

## Success Criteria Mapping

- SC1 (AiAgent from separate process): daemon-side handler exists and routes correctly — end-to-end path deferred to Plan 02 integration gate.
- SC2 (NotAnAgent for non-agent PID): `classify_non_appcontainer_pid_returns_not_an_agent` PASS.
- SC3 (Low-IL denied): CONTROL_PIPE_SDDL unchanged; `control_pipe_sddl_is_medium_il_only` continues to pass.
- SC4 (no cross-tenant SID): `classify_response_aiagent_omits_package_sid` pure-function test PASS with all three assertions.

## Self-Check: PASSED

- [x] `crates/nono-cli/src/agent_daemon/control_loop.rs` — exists and modified
- [x] commit `aaafe4ff` — confirmed in `git log --oneline -1`
- [x] 4 classify tests pass: `cargo test --bin nono-agentd -- classify` 4/4
- [x] Full suite: 32/32, zero regressions
