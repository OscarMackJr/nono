---
phase: 75-supplementary-controls-secondary-engines
plan: "02"
subsystem: agent_daemon
tags: [supp-01, demote, il-drop, wfp-cut, windows, incident-response]
dependency_graph:
  requires:
    - 75-01  # wfp_filter_remove pub(crate) export
  provides:
    - ControlRequest::Demote deserialization + dispatch
    - handle_demote() with SUPP-01 leak limits + D-03 WFP-cut
    - AgentCommands::Demote CLI verb with leak-limits help text
    - agent_demote() control-pipe client function
  affects:
    - crates/nono-cli/src/agent_daemon/control_loop.rs
    - crates/nono-cli/src/agent_cli.rs
    - crates/nono-cli/src/cli.rs
tech_stack:
  added: []
  patterns:
    - TokenGuard RAII for OpenProcessToken handle
    - DupHandleGuard RAII for DuplicateHandle before lock release (Pitfall 1)
    - Spike-002 Win32 IL-drop: OpenProcessToken + CreateWellKnownSid(WinLowLabelSid) + SetTokenInformation(TokenIntegrityLevel)
    - Non-fatal wfp_filter_remove call for D-03 WFP-cut after IL-drop
key_files:
  created: []
  modified:
    - crates/nono-cli/src/agent_daemon/control_loop.rs
    - crates/nono-cli/src/agent_cli.rs
    - crates/nono-cli/src/cli.rs
decisions:
  - "D-03 WFP-cut: handle_demote calls wfp_filter_remove after IL-drop (non-fatal warning on failure; WFP service startup sweep backstops stale filters)"
  - "Pitfall 1 mitigation: DuplicateHandle the process HANDLE before releasing tenants lock, then perform Win32 IL-drop on the duplicate"
  - "TokenGuard + DupHandleGuard RAII on all exit paths — no .unwrap()/.expect() in production code"
  - "SE_GROUP_INTEGRITY cast to u32 (windows-sys 0.59 exposes it as i32 = 32; value is non-negative, cast is safe; #[allow(clippy::cast_sign_loss)] annotation added)"
  - "Cross-target clippy: PARTIAL — Linux cross-toolchain absent on Win11 host; new cfg-gated code deferred to CI; all new code is #[cfg(target_os = 'windows')] or #[cfg(not(target_os = 'windows'))]"
metrics:
  duration: "~45 minutes"
  completed: "2026-06-15"
  tasks_completed: 2
  tasks_total: 2
  files_modified: 3
---

# Phase 75 Plan 02: SUPP-01 Demote Verb Summary

**One-liner:** Post-hoc IL-drop incident-response lever (`nono agent demote`) wiring Win32 OpenProcessToken+SetTokenInformation(TokenIntegrityLevel) from the daemon's control loop to the operator CLI, with D-03 WFP-cut and spike-002 leak limits documented at the verb.

## What Was Built

### Task 1: ControlRequest::Demote + handle_demote (control_loop.rs)

Added the `Demote { tenant_id: String }` variant to `ControlRequest` in `windows_impl` of `control_loop.rs`. The serde tag `"demote"` deserializes from `{"action":"demote","tenant_id":"..."}` wire JSON.

`handle_demote(state, tenant_id)` function:
1. Locks `state.tenants`, looks up the tenant. Unknown tenant_id returns a clear `"error: tenant_id '...' not found"` without any Win32 call.
2. Clones raw HANDLE via `DuplicateHandle` (Pitfall 1 — releases lock before Win32 calls) with `DupHandleGuard` RAII.
3. Calls `demote_tenant_il(dup_handle)` — the spike-002 Win32 IL-drop path.
4. On IL-drop success: calls `crate::agent_daemon::launch::wfp_filter_remove(package_sid, tenant_id)` (D-03 WFP-cut). Non-fatal: logs `tracing::warn!` on failure.
5. Does NOT call `tenants.remove()` — agent stays in map after demote (D-03).

`demote_tenant_il(process_handle)` private function:
- `OpenProcessToken(TOKEN_ADJUST_DEFAULT | TOKEN_QUERY)` with `TokenGuard` RAII
- `CreateWellKnownSid(WinLowLabelSid)` into stack buffer
- `SetTokenInformation(token, TokenIntegrityLevel, &low_label, struct_size + sid_len)`
- All unsafe blocks have `// SAFETY:` documentation
- Uses `saturating_add` for the total_size calculation (no arithmetic overflow risk)

Spike-002 leak limits documented in `handle_demote` doc comment per D-01:
1. Handles opened BEFORE the IL-drop continue at Medium IL
2. Already-started child processes are NOT retroactively affected
3. IL-drop may sever legitimate handles (agent may crash)
4. Outbound network NOT auto-blocked — SUPP-02 WFP filter removed separately (D-03)
5. Demote is one-way — no API to raise IL back to Medium from outside

### Task 2: AgentCommands::Demote + agent_demote (cli.rs, agent_cli.rs)

Added `AgentCommands::Demote { tenant_id: String }` as a third variant in `cli.rs`. The clap doc comment includes all 5 SUPP-01 leak limits in `--help` output (per D-01).

`agent_demote(tenant_id: String) -> Result<()>` in `agent_cli.rs`:
- Windows path: serializes `{"action":"demote","tenant_id":"..."}` and sends over `windows_control_pipe_request`; prints the response
- Non-Windows stub: returns `Err(NonoError::SandboxInit("nono agent demote is Windows-only ..."))`
- `run_agent` dispatch: `AgentCommands::Demote { tenant_id } => agent_demote(tenant_id)`

## Verification

### Automated Test Results (Windows host, x86_64-pc-windows-msvc)

| Test | Status |
|------|--------|
| `agent_daemon::control_loop::tests::demote_returns_err_for_unknown_tenant` | PASS |
| `agent_daemon::control_loop::tests::demote_does_not_reap_tenant_from_map` | PASS |
| `agent_cli::tests::agent_demote_parses` | PASS |
| Full nono-cli suite (1261 tests) | PASS (4 pre-existing failures unchanged) |

`nono agent demote --help` shows the leak-limits paragraph in the command description (verified via `cargo run --bin nono -- agent demote --help`).

### Cross-Target Clippy

**Status: PARTIAL — deferred to CI**

The Linux cross-toolchain (`x86_64-linux-gnu-gcc`) is absent on this Win11 host. `cargo clippy --target x86_64-unknown-linux-gnu` fails at the C-toolchain step for `aws-lc-sys`. Per CLAUDE.md:

> If the cross-toolchain is not installed, the related verification REQ MUST be marked PARTIAL and deferred to live CI.

All new production code in `control_loop.rs` is inside `mod windows_impl` which is gated with `#[cfg(target_os = "windows")]`. The non-Windows code paths (test helpers, cross-platform test stubs) compile without Win32 imports. No Unix cfg-branches were touched.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] SE_GROUP_INTEGRITY type mismatch (i32 vs u32)**
- **Found during:** Task 1 build
- **Issue:** `SE_GROUP_INTEGRITY` is exposed as `i32 = 32` in windows-sys 0.59, but `TOKEN_MANDATORY_LABEL.Label.Attributes` is `u32`. Spike-002 source uses `SE_GROUP_INTEGRITY as u32`.
- **Fix:** Added `#[allow(clippy::cast_sign_loss)]` block with `SE_GROUP_INTEGRITY as u32` cast and a comment explaining the value (32i32) is non-negative and safe to cast.
- **Files modified:** `control_loop.rs`

**2. [Rule 1 - Bug] DuplicateHandle import path**
- **Found during:** Task 1 build
- **Issue:** Used `windows_sys::Win32::System::Threading::DuplicateHandle` but the function is in `windows_sys::Win32::Foundation::DuplicateHandle`.
- **Fix:** Corrected the fully-qualified path in the unsafe call.
- **Files modified:** `control_loop.rs`

**3. [Rule 1 - Bug] GetLengthSid import path**
- **Found during:** Task 1 build investigation
- **Issue:** PATTERNS.md cited `windows_sys::Win32::System::Sid::GetLengthSid` but the actual location in windows-sys 0.59 is `windows_sys::Win32::Security::GetLengthSid`.
- **Fix:** Used the correct import path confirmed via `grep` in the windows-sys 0.59 source.
- **Files modified:** `control_loop.rs`

## Known Stubs

None. No hardcoded placeholder values, TODO comments, or unconnected data flows in the new code.

## Threat Flags

No new network endpoints, auth paths, file access patterns, or schema changes at trust boundaries beyond what the plan's threat model covers (T-75-02-01 through T-75-02-SC all handled by the existing Medium-IL control pipe SDDL from Phase 74 T-74-07-01, the tenant_id lookup guard, and the non-fatal WFP-cut warning pattern).

## Self-Check: PASSED

- `crates/nono-cli/src/agent_daemon/control_loop.rs` — FOUND (modified)
- `crates/nono-cli/src/agent_cli.rs` — FOUND (modified)
- `crates/nono-cli/src/cli.rs` — FOUND (modified)
- Task 1 commit `923ae5f7` — FOUND (`git log --oneline -5`)
- Task 2 commit `b1ae0d6f` — FOUND (`git log --oneline -5`)
- All 3 new SUPP-01 unit tests PASS
- `nono agent demote --help` shows leak limits paragraph
- No .unwrap()/.expect() in new production code paths
- All unsafe blocks have `// SAFETY:` comments
- DCO sign-off on both commits
