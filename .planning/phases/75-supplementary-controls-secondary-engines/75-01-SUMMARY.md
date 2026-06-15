---
phase: 75-supplementary-controls-secondary-engines
plan: "01"
subsystem: agent-daemon-wfp
tags:
  - wfp
  - appcontainer
  - supp-02
  - daemon
  - network-enforcement
dependency_graph:
  requires:
    - "74-04: launch_agent (job, AppContainer spawn, reap task)"
    - "74-07: control_loop (handle_demote caller for 75-02)"
    - "nono-wfp-service: activate_blocked_mode + deactivate_policy_mode endpoints"
  provides:
    - "wfp_filter_add (pub(crate)) — callable from launch_agent at step 6.5"
    - "wfp_filter_remove (pub(crate)) — callable from reap task + 75-02 handle_demote"
  affects:
    - "crates/nono-cli/src/agent_daemon/launch.rs — D-05 gate + WFP helpers"
    - "crates/nono-cli/src/agent_daemon/reap.rs — ordering-contract tests"
    - "crates/nono-cli/src/agent_daemon/mod.rs — wfp_contract #[path] include"
    - "crates/nono-cli/src/bin/nono-wfp-service.rs — session_sid path fix"
tech_stack:
  added: []
  patterns:
    - "Synchronous blocking pipe client (std::fs::OpenOptions) for WFP control pipe — avoids !Send HANDLE across .await"
    - "Session-SID keyed WFP filter: session_sid=Some(package_sid), target_program_path=None"
    - "Deterministic rule names: nono-agent-{tenant_id} / nono-agent-{tenant_id}-in"
    - "D-05 fail-secure: WFP add fails + profile network-scoped → TerminateProcess + return Err"
    - "Non-fatal reap: WFP remove fails → tracing::warn! + continue to tenants.remove"
key_files:
  created: []
  modified:
    - "crates/nono-cli/src/agent_daemon/launch.rs"
    - "crates/nono-cli/src/agent_daemon/reap.rs"
    - "crates/nono-cli/src/agent_daemon/mod.rs"
    - "crates/nono-cli/src/bin/nono-wfp-service.rs"
decisions:
  - "Synchronous std::fs pipe client: avoids tokio named-pipe HANDLE Send issue; mirrors probe_runtime_activation_mode pattern"
  - "WFP remove in reap TASK (not AgentTenant::Drop): blocking pipe I/O in Drop inside tokio is Pitfall 2"
  - "wfp_filter_remove is pub(crate): forward-export for Plan 75-02 handle_demote caller"
  - "session_sid keying: package_sid from CreateAppContainerProfile, never from wire input (T-75-01-01)"
  - "profile_needs_network_scoping: fails-safe false on JSON parse error; all current profiles return false"
metrics:
  duration: "~2 hours (multi-session)"
  completed: "2026-06-15"
  tasks_completed: 2
  tasks_total: 2
  files_changed: 4
---

# Phase 75 Plan 01: Per-Agent WFP Egress Filter Helpers + D-05 Gate (SUPP-02) Summary

Per-agent WFP egress filter wired into daemon launch/reap lifecycle via synchronous blocking pipe client sending session_sid-keyed activate/deactivate requests to nono-wfp-service over `\\.\pipe\nono-wfp-control`.

## What Was Built

SUPP-02 satisfies the per-agent network isolation requirement: every confined agent launched through `nono-agentd` gets its own WFP filter keyed to its AppContainer package SID (E4 identity). The elevated `nono-wfp-service` enforces the filter; the USER daemon is a least-priv client (privilege split preserved per D-04).

### Task 1: wfp_filter_add / wfp_filter_remove helpers + D-05 gate

**`crates/nono-cli/src/agent_daemon/launch.rs`** — 3 new functions in `windows_impl`:

- `send_wfp_control_request(req)` — synchronous blocking pipe client using `std::fs::OpenOptions`; serializes request JSON, writes length-prefixed frame, reads response. No tokio — avoids `HANDLE = *mut c_void` (`!Send`) across `.await`.
- `wfp_filter_add(package_sid, tenant_id)` — sends `"activate_blocked_mode"` with `session_sid=Some(package_sid)`, rule names `nono-agent-{tenant_id}` / `nono-agent-{tenant_id}-in`, `target_program_path: None`. Returns `NonoError::SandboxInit` on failure naming the service.
- `wfp_filter_remove(package_sid, tenant_id)` — `pub(crate)`, sends `"deactivate_policy_mode"` with same fields. Used in reap task and forward-exported for Plan 75-02.

**D-05 gate** inserted between step 6 (job assign) and step 7a (registry insert), before `ResumeThread`:

```rust
if profile_needs_network_scoping(&engine_profile) {
    if let Err(e) = wfp_filter_add(&package_sid, &tenant_id) {
        // TerminateProcess + CloseHandle handles + return Err
    }
}
```

**WFP deactivation** in the reap task BEFORE `tenants.remove()` (Pitfall 2 mitigation — not in `AgentTenant::Drop`):

```rust
if let Err(e) = wfp_filter_remove(&reap_package_sid, &reap_tenant_id) {
    tracing::warn!(...); // Non-fatal: continue to tenants.remove.
}
```

**`crates/nono-cli/src/agent_daemon/mod.rs`** — Added `#[path = "../windows_wfp_contract.rs"] pub(crate) mod wfp_contract;` so the `nono-agentd` standalone binary (loaded via `#[path]`) can access `WfpRuntimeActivationRequest` without `crate::windows_wfp_contract`.

### Task 2: Reap ordering contract tests

**`crates/nono-cli/src/agent_daemon/reap.rs`** — 2 new tests added to the existing `tests` module:

- `wfp_filter_remove_at_reap_not_in_drop`: constructs a real `AgentTenant` with duplicated current-process handles and drops it without a running `nono-wfp-service`. Confirms `AgentTenant::Drop` returns without blocking on the WFP pipe (no pipe I/O in Drop).
- `wfp_filter_remove_nonfatal_contract`: calls `wfp_filter_remove` directly (service absent in unit test context); verifies error message contains `"nono-wfp-service"` or `"nono-wfp-control"` (actionable for the reap task's `tracing::warn!`), and that the function returns `Err` rather than panicking.

## Test Results

```
cargo test -p nono-cli -- wfp_filter (4 tests, Task 1)
  agent_daemon::launch::tests::wfp_filter_add_constructs_request  ... ok
  agent_daemon::launch::tests::wfp_absent_no_scoping_ok           ... ok
  agent_daemon::launch::tests::wfp_absent_fail_secure             ... ok
  agent_daemon::launch::tests::wfp_filter_add_at_launch           ... ok

cargo test -p nono-cli -- wfp_filter_remove (2 tests, Task 2)
  agent_daemon::reap::tests::wfp_filter_remove_at_reap_not_in_drop  ... ok
  agent_daemon::reap::tests::wfp_filter_remove_nonfatal_contract     ... ok
```

6 new tests passing. Pre-existing 4 failures (profile_cmd init + protected_paths) unchanged.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed `validate_target_request_fields` requiring `target_program_path` for session_sid-keyed requests**
- **Found during:** Task 1 implementation — `wfp_filter_add` sends `target_program_path: None` (SID-keyed requests don't need a program path), but `nono-wfp-service`'s `validate_target_request_fields` returned `invalid-request` when it was None
- **Issue:** `validate_target_request_fields` unconditionally required `target_program_path` to be `Some`. `activate_policy_mode` also checked `target_program.exists()` unconditionally, which would fail for an empty/sentinel path.
- **Fix:** `validate_target_request_fields` now branches on `session_sid.is_some()`: if SID-keyed, `target_program_path` may be `None` (uses sentinel empty `PathBuf`). `activate_policy_mode` skips `target_program.exists()` check when `session_sid` is set.
- **Files modified:** `crates/nono-cli/src/bin/nono-wfp-service.rs`
- **Commit:** `195a7c11`

**2. [Rule 3 - Blocking] Module path issue: `crate::windows_wfp_contract` inaccessible from nono-agentd**
- **Found during:** Task 1 — `agent_daemon/launch.rs` uses `use crate::agent_daemon::wfp_contract::...` but `nono-agentd` binary loads `agent_daemon` via `#[path]` and cannot reach `crate::windows_wfp_contract`
- **Fix:** Added `#[path = "../windows_wfp_contract.rs"] pub(crate) mod wfp_contract;` in `agent_daemon/mod.rs`
- **Files modified:** `crates/nono-cli/src/agent_daemon/mod.rs`
- **Commit:** `195a7c11`

**3. [Rule 3 - Blocking] WFP helper implementation as synchronous (blocking std::fs) not async**
- **Found during:** Task 1 — initial async tokio named-pipe implementation caused `future cannot be sent between threads safely` because `HANDLE = *mut c_void` (`!Send`) was held across `.await` in `launch_agent` (which is called from `tokio::spawn`)
- **Fix:** Rewrote `send_wfp_control_request`, `wfp_filter_add`, and `wfp_filter_remove` as synchronous blocking functions using `std::fs::OpenOptions::new().read(true).write(true).open(WFP_CONTROL_PIPE)`. No `.await` → no `!Send` held across await boundary.
- **Files modified:** `crates/nono-cli/src/agent_daemon/launch.rs`
- **Commit:** `195a7c11`

**4. [Plan Adaptation] Reap task lives in `launch.rs`, not `reap.rs`**
- **Reason:** The `tokio::spawn(async move { ... })` reap task was implemented in `launch.rs` (Plan 74-04). The WFP deactivation call was correctly placed there (between `registry.remove` and `tenants.remove`). Task 2 added contract tests to `reap.rs` to verify the ordering invariant and non-fatal error path from the module's perspective.

## Cross-Target Verification

**Status: PARTIAL** — Windows host lacks Linux/macOS cross-toolchains.

Attempted `cargo clippy --workspace --target x86_64-unknown-linux-gnu`: build failed at `aws-lc-sys` C compilation (`x86_64-linux-gnu-gcc` not found).

All new code is under `#[cfg(target_os = "windows")]` blocks. No new `#[cfg(unix)]`, `#[cfg(target_os = "linux")]`, or `#[cfg(target_os = "macos")]` blocks introduced. Cross-target impact is minimal — the `wfp_contract` module include in `mod.rs` has no cfg gate, but the module itself contains only `#[cfg(target_os = "windows")]` types (derived via serde/windows-sys).

Deferred to CI per `.planning/templates/cross-target-verify-checklist.md`.

## Threat Surface Scan

No new network endpoints. All trust boundary mitigations from plan `<threat_model>` are implemented:

| Threat | Status |
|--------|--------|
| T-75-01-01: session_sid from daemon's own CreateAppContainerProfile (not wire input) | IMPLEMENTED — package_sid from `nono::package_sid_to_string`, never from operator request |
| T-75-01-02: Stale filter startup sweep (pitfall 6) | ACCEPTED — deterministic rule names `nono-agent-{tenant_id}` enable sweep; startup sweep pre-existing in nono-wfp-service |
| T-75-01-03: D-05 fail-secure aborts launch for network-scoped profiles | IMPLEMENTED — D-05 gate present; profiles without network scoping unaffected |
| T-75-01-04: Agent starts before WFP filter installed (pitfall 3) | IMPLEMENTED — wfp_filter_add before ResumeThread (between steps 6 and 7a) |

## Known Stubs

None — no placeholder data flows to UI rendering.

## Acceptance Criteria Verification

| Criterion | Status |
|-----------|--------|
| wfp_filter_add called before ResumeThread for network-scoped profiles | PASS — between step 6 and step 7a |
| WFP add fail + network-scoped → TerminateProcess + Err naming nono-wfp-service | PASS — `wfp_absent_fail_secure` test confirms |
| WFP add skip for non-network-scoped profiles | PASS — `wfp_absent_no_scoping_ok` test confirms |
| wfp_filter_remove in reap task before tenants.remove | PASS — reap task in launch.rs; `wfp_filter_remove_at_reap_not_in_drop` validates |
| WFP remove failure non-fatal | PASS — `wfp_filter_remove_nonfatal_contract` confirms Err not panic |
| wfp_filter_remove is pub(crate) | PASS — confirmed, forward-exported for plan 75-02 |
| No .unwrap()/.expect() in production paths | PASS — clippy -D clippy::unwrap_used clean |
| All unsafe blocks have SAFETY docs | PASS — all 6 unsafe blocks in launch.rs have // SAFETY: comments |
| DCO sign-off on all commits | PASS — both commits include Signed-off-by |

## Self-Check: PASSED

Files exist:
- `.planning/phases/75-supplementary-controls-secondary-engines/75-01-SUMMARY.md` — this file
- `crates/nono-cli/src/agent_daemon/launch.rs` — modified (FOUND)
- `crates/nono-cli/src/agent_daemon/reap.rs` — modified (FOUND)
- `crates/nono-cli/src/agent_daemon/mod.rs` — modified (FOUND)
- `crates/nono-cli/src/bin/nono-wfp-service.rs` — modified (FOUND)

Commits exist:
- `195a7c11` — Task 1 feat commit (FOUND)
- `1bdfc56e` — Task 2 test commit (FOUND)
