---
phase: 51
plan: 03
subsystem: nono-cli/exec_strategy_windows
tags: [windows, broker, no-pty, BrokerLaunchNoPty, HANDLE_LIST, write-deny, MIC, integration-test]
dependency_graph:
  requires: ["WindowsTokenArm::BrokerLaunchNoPty (51-01)", "nono-shell-broker --no-pty (51-02)", "DetachedStdioPipes (Phase 17)"]
  provides: ["BrokerLaunchNoPty spawn arm in spawn_windows_child", "write_deny_low_il_broker_no_pty_tests integration test"]
  affects: ["crates/nono-cli/src/exec_strategy_windows/launch.rs", "crates/nono-cli/src/execution_runtime.rs"]
tech_stack:
  added: []
  patterns: ["PROC_THREAD_ATTRIBUTE_HANDLE_LIST handle gating", "anonymous-pipe detached stdio", "real-spawn write-deny integration test with non-vacuousness exit-code gate"]
key_files:
  created:
    - .planning/phases/51-no-pty-low-il-broker-token-routing-write-deny-preservation/51-03-SUMMARY.md
  modified:
    - crates/nono-cli/src/exec_strategy_windows/launch.rs
    - crates/nono-cli/src/execution_runtime.rs
decisions:
  - "BrokerLaunchNoPty wired as a new `else if matches!(arm, BrokerLaunchNoPty)` branch in spawn_windows_child's no-PTY path; mirrors the BrokerLaunch (PTY) arm with 3 DetachedStdioPipes child-end handles instead of 2 ConPTY handles"
  - "Shared post-CreateProcess block (close_child_ends -> job -> resume) reused for the broker path; the arm only flips child-end handles non-inheritable, letting the existing close_child_ends() close them — no double-close, architecturally correct timing"
  - "write-deny test asserts broker exit_code == 1 (non-vacuousness gate) AND fixture content unchanged; the broker propagates cmd.exe ERRORLEVEL (1=denied, 0=write-succeeded/breach, 2=broker-never-spawned/vacuous)"
  - "Executed INLINE by the execute-phase orchestrator after two gsd-executor subagents failed to obtain Bash permission (workflow stall-recovery: switch to inline execution)"
metrics:
  duration: "~60 minutes (inline)"
  completed: "2026-05-26"
  tasks_completed: 3
  tasks_total: 3
  files_modified: 2
---

# Phase 51 Plan 03: BrokerLaunchNoPty Spawn Arm + Write-Deny Integration Test

**One-liner:** `spawn_windows_child` now dispatches a `BrokerLaunchNoPty` arm that spawns the Medium-IL broker with anonymous-pipe stdio (3 handles gated by `PROC_THREAD_ATTRIBUTE_HANDLE_LIST`, `--no-pty` command line), and a real-spawn integration test cryptographically proves a Low-IL child cannot write a Medium-IL file (kernel MIC `NO_WRITE_UP`, REQ-WSRH-05) with an `exit_code == 1` non-vacuousness gate.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 0 | Pre-build `nono-shell-broker.exe` (release, msvc) | (build step, no commit) | — |
| 1 | `BrokerLaunchNoPty` spawn arm | `fcba74dd` | launch.rs |
| 1b | rust-1.95 clippy fixes in merged 51-01 code | `0b1372d3` | execution_runtime.rs |
| 2 | `write_deny_low_il_broker_no_pty_tests` integration test | `12e69940` | launch.rs |

## What Was Built

### Task 0 — Broker pre-build
`cargo build -p nono-shell-broker --target x86_64-pc-windows-msvc --release` → artifact at `target/x86_64-pc-windows-msvc/release/nono-shell-broker.exe` (the two-candidate lookup target for the integration test).

### Task 1 — BrokerLaunchNoPty spawn arm
Added a dedicated arm to the no-PTY `else` branch of `spawn_windows_child` (`else if matches!(arm, WindowsTokenArm::BrokerLaunchNoPty)`), structurally mirroring the Phase 31 `BrokerLaunch` (PTY) arm but for the no-PTY supervised path:

1. **Broker path resolution** — sibling `nono-shell-broker.exe` via `current_exe().parent()`; `BrokerNotFound` if absent.
2. **Authenticode self-trust-anchor** — `verify_broker_authenticode` (dev-build skip via `is_dev_build_layout`), identical invariant to BrokerLaunch (T-51C-04).
3. **Anonymous-pipe stdio** — `DetachedStdioPipes::create()`; the 3 child-end handles (`stdin_read`, `stdout_write`, `stderr_write`) flipped inheritable.
4. **HANDLE_LIST gating** — `PROC_THREAD_ATTRIBUTE_HANDLE_LIST` containing exactly those 3 handles (T-51C-03), `size_of_val(&inherit_handles[..])`.
5. **Broker command line** — `--shell <prog>` + `--shell-arg <a>`* + `--no-pty` + 3× `--inherit-handle 0x{:016x}` + `--cwd <dir>` via `build_broker_command_line`.
6. **CreateProcessW** (null `h_token` — Medium-IL broker self-degrades to Low IL), `CREATE_SUSPENDED | CREATE_UNICODE_ENVIRONMENT | EXTENDED_STARTUPINFO_PRESENT`, `bInheritHandles=1`.
7. **Cleanup** — `DeleteProcThreadAttributeList`; child-end handles unflipped non-inheritable on both success and error paths. The shared post-block (`close_child_ends()` → `AssignProcessToJobObject` → `apply_resource_limits` → `ResumeThread`) handles closing the supervisor's child-end copies.
8. **Relay** — `detached_stdio = Some(pipes)` returned so `execute_supervised` wires the stdout/stderr relay (PATTERNS.md confirmed `attach_detached_stdio` is unconditional).

Every Win32 failure is fail-closed (`NonoError::SandboxInit`) — no silent fallback to WriteRestricted/Null (T-51C-02). Every `unsafe` block carries a `// SAFETY:` comment.

### Task 2 — Write-deny integration test
`write_deny_low_il_broker_no_pty_tests::write_deny_low_il_broker_no_pty_prevents_child_write_to_medium_il_file` (`#[cfg(all(test, target_os = "windows"))]`):

- Resolves the broker via the two-candidate lookup; **hard-fails** (panic, no `#[ignore]`) if absent (D-08 / T-51C-06).
- Creates a Medium-IL `%TEMP%` fixture containing `b"sentinel"` (T-51C-05: `%TEMP%`, not a drive-root path, per the WRITE_OWNER label-apply limitation).
- Creates 3 inheritable anonymous pipe pairs; spawns the broker with `--no-pty` + the 3 child-end handle values + `cmd.exe /c "echo x > <fixture>"`, `CREATE_SUSPENDED` then `ResumeThread`, and `WaitForSingleObject(INFINITE)`.
- **Non-vacuousness gate:** asserts the broker `exit_code == 1`. The broker propagates its child's exit code (broker-internal errors exit `2`); cmd.exe sets `ERRORLEVEL 1` when the `>` redirect target cannot be opened. So `exit 0` = write succeeded (breach), `exit 1` = child ran and was denied (correct), `exit 2` = broker never spawned the child (vacuous). Empirically observed `exit_code=1`.
- **Belt-and-suspenders:** asserts the fixture content is still `b"sentinel"` (unmodified).

## Verification Results

```
write_deny_low_il_broker_no_pty_tests ........ 1 passed   (observed BROKER_EXIT_CODE=1, fixture_len=8)
broker_dispatch_tests ........................ 5 passed   (Phase 31 no-regression)
pty_token_gate_tests ......................... 8 passed   (incl. 51-01 BrokerLaunchNoPty selection + BrokerLaunch precedence)
```

- `cargo build -p nono-cli --target x86_64-pc-windows-msvc` exits 0
- `cargo test -p nono-cli --bin nono --target x86_64-pc-windows-msvc write_deny_low_il_broker_no_pty` exits 0
- `cargo clippy -p nono-cli --target x86_64-pc-windows-msvc -- -D warnings -D clippy::unwrap_used` exits 0
- `grep verify_broker_authenticode launch.rs`: 3 (fn def + BrokerLaunch + BrokerLaunchNoPty)
- `grep PROC_THREAD_ATTRIBUTE_HANDLE_LIST launch.rs`: 7
- `grep 'DetachedStdioPipes::create' launch.rs`: 9
- No `#[ignore]` attribute in the write-deny module

## Cross-Target Clippy (CLAUDE.md MUST)

`launch.rs` and `execution_runtime.rs` are cfg-gated Windows files. Host clippy (`x86_64-pc-windows-msvc`) is clean. Cross-target clippy for `x86_64-unknown-linux-gnu` / `x86_64-apple-darwin` is **DEFERRED to live CI — PARTIAL** per `.planning/templates/cross-target-verify-checklist.md`: the Unix cross-toolchains are not installed on this Windows dev host. The `BrokerLaunchNoPty` arm and the write-deny test are themselves `target_os = "windows"`-gated, so they do not compile on Unix targets; the host-target clippy pass is the strongest available local signal. Phase 51 Wave 3 (Plan 51-04) owns the cross-target sweep.

## Deviations from Plan

### Execution mode: inline (orchestrator), not subagent
Two `gsd-executor` subagents spawned for this plan returned without Bash access (each made a single tool call then requested Bash permission). Per the execute-phase stall-recovery contract ("kill and switch to inline execution"), Plan 51-03 was executed inline by the orchestrator on the `main` working tree. All tasks, commits, and verification are identical to the subagent contract; tracking (ROADMAP/STATE) is updated by the orchestrator in sequential mode.

### Auto-fixed: rust-1.95 clippy lints in merged 51-01 code (`0b1372d3`)
Host clippy with rust 1.95.0 surfaced 5 pre-existing lints in the merged Wave 1 (51-01) code — 4× `doc_list_item_without_indentation` in the `BrokerLaunchNoPty` doc comment (launch.rs) and 1× `unnecessary_map_or` in `execution_runtime.rs`. These were not caught by the post-merge gate (build+test only, no clippy) nor by the 51-01 executor (different lint surface). Fixed: blank doc separator line + `map_or(false, ..)` → `is_some_and(..)` (behavior identical). Required for the Wave 3 clippy sweep to pass.

## Security Analysis (Threat Model Coverage)

| Threat ID | Status |
|-----------|--------|
| T-51C-01 | MITIGATED — write-deny test proves Low-IL child cannot write Medium-IL file (kernel MIC pre-DACL), `exit_code==1` non-vacuous gate |
| T-51C-02 | MITIGATED — every Win32 failure in the arm returns `Err(NonoError::SandboxInit)`; no fallback match arm |
| T-51C-03 | MITIGATED — `PROC_THREAD_ATTRIBUTE_HANDLE_LIST` gates exactly the 3 pipe handles; flipped inheritable before, non-inheritable after, on all paths |
| T-51C-04 | MITIGATED — `verify_broker_authenticode` applied on the BrokerLaunchNoPty path (dev-build skip parity) |
| T-51C-05 | MITIGATED — test fixture uses `std::env::temp_dir()` (`%TEMP%`), never a drive-root path |
| T-51C-06 | MITIGATED — test panics (hard-fail) if broker artifact missing; no `#[ignore]` |
| T-51C-07 | MITIGATED — BrokerLaunch (PTY) arm structurally unchanged; broker_dispatch_tests (5) + pty_token_gate_tests (8) pass |
| T-51C-SC | ACCEPTED — no new dependencies |

## Known Stubs

None — the BrokerLaunchNoPty spawn arm is fully wired (pipe creation, HANDLE_LIST gating, broker invocation, relay return) and the write-deny test is a real-spawn integration test, not a stub.

## Threat Flags

None — no new network endpoints or auth paths. The arm operates within the established nono.exe → broker trust boundary; the only new external interaction is the documented Authenticode-verified broker spawn.

## Self-Check: PASSED

- [x] `crates/nono-cli/src/exec_strategy_windows/launch.rs` modified (arm + test)
- [x] `crates/nono-cli/src/execution_runtime.rs` clippy fix
- [x] Commit `fcba74dd` (feat arm) exists
- [x] Commit `0b1372d3` (clippy fix) exists
- [x] Commit `12e69940` (write-deny test) exists
- [x] write-deny test passes (exit_code==1, fixture unchanged)
- [x] broker_dispatch_tests (5) + pty_token_gate_tests (8) pass — no regression
- [x] Host clippy clean; cross-target PARTIAL (deferred to CI per checklist)
- [x] No `#[ignore]` in the write-deny module
