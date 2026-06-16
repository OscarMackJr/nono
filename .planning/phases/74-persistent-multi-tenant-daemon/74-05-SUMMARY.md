---
phase: 74-persistent-multi-tenant-daemon
plan: "05"
subsystem: nono-cli
tags:
  - daemon-lifecycle
  - agent-management
  - cli-ux
  - windows
  - phase-74
dependency_graph:
  requires:
    - "74-04: nono-agentd daemon binary with accept loop + launch"
    - "74-03: agent_daemon module (DaemonState, AgentTenant)"
    - "74-02: library primitives (authenticate_pipe_client, AppContainer APIs)"
    - "73: classify_runtime (reused for PID inspection)"
  provides:
    - "nono daemon start|stop|status|install|uninstall CLI verbs"
    - "nono agent launch|list CLI verbs"
    - "DAEMON_CONTROL_PIPE_NAME constant (agent_cli.rs)"
  affects:
    - "crates/nono-cli/src/cli.rs (Commands enum, help templates)"
    - "crates/nono-cli/src/agent_cli.rs (new file)"
    - "crates/nono-cli/src/main.rs (mod agent_cli)"
    - "crates/nono-cli/src/app_runtime.rs (dispatch)"
    - "crates/nono-cli/src/startup_runtime.rs (update-check gate)"
    - "crates/nono-cli/src/cli_bootstrap.rs (verbosity match)"
tech_stack:
  added: []
  patterns:
    - "Windows SCM control via sc.exe (minimal Phase 74; Phase 75 to upgrade)"
    - "Named-pipe client with 4-byte LE length-prefix framing (matches socket_windows.rs)"
    - "cfg-gated Windows-only implementation with non-Windows diagnostic stubs"
    - "clap v4 nested Subcommand pattern (matches RollbackArgs/RollbackCommands)"
key_files:
  created:
    - crates/nono-cli/src/agent_cli.rs
  modified:
    - crates/nono-cli/src/cli.rs
    - crates/nono-cli/src/main.rs
    - crates/nono-cli/src/app_runtime.rs
    - crates/nono-cli/src/startup_runtime.rs
    - crates/nono-cli/src/cli_bootstrap.rs
decisions:
  - "Use sc.exe for daemon lifecycle in Phase 74; Phase 75 may upgrade to proper SCM Rust API"
  - "DAEMON_CONTROL_PIPE_NAME separate from capability pipe (control = nono-agentd-control, cap = nono-agentd-cap)"
  - "is_pipe_not_found helper enables caller-controlled error messaging (launch vs list differ)"
  - "Exclude daemon/agent from pre-exec update check (startup latency)"
metrics:
  duration_minutes: 108
  completed_date: "2026-06-15"
  tasks_completed: 2
  tasks_total: 2
  files_created: 1
  files_modified: 5
---

# Phase 74 Plan 05: Operator CLI Verb Surface (D-05) Summary

**One-liner:** `nono daemon start|stop|status|install|uninstall` + `nono agent launch|list` clap v4 verbs wired as thin SCM/pipe client over nono-agentd, with fail-secure deny when daemon is down.

## What Was Built

### Task 1: cli.rs + agent_cli.rs

Added the D-05 CLI verb surface to `cli.rs`:
- `Commands::Daemon(DaemonArgs)` with `DaemonCommands` enum: `Start`, `Stop`, `Status`, `Install`, `Uninstall`
- `Commands::Agent(AgentArgs)` with `AgentCommands` enum: `Launch(AgentLaunchArgs)`, `List` — explicitly NO `Query` variant (D-05 fence)
- `AgentLaunchArgs`: `--profile <engine>` + `#[arg(last = true)] cmd: Vec<String>`
- Updated both Windows and non-Windows `ROOT_HELP_TEMPLATE` with DAEMON & AGENTS section
- Added `classify`, `daemon`, `agent` to `ALL_SUBCOMMANDS` constant (classify was a Phase 73 carry-over gap)

Created `crates/nono-cli/src/agent_cli.rs`:
- `run_daemon(DaemonArgs)`: dispatches to `daemon_start/stop/status/install/uninstall` 
- `run_agent(AgentArgs)`: dispatches to `agent_launch/agent_list`
- Windows: `daemon_install` builds sc.exe invocation with `type= userservice` (ADR-74 D1; T-74-05-04 mitigation mandatory)
- Windows: `windows_control_pipe_request` — 5-second timeout (T-74-05-02), 4-byte LE length-prefix framing (matches `socket_windows.rs`)
- Non-Windows: all verbs print a diagnostic / return `Ok(())` or `Err(...)` as appropriate
- `is_pipe_not_found` helper: distinguishes GLE=2 (daemon not running) from other pipe errors

9 new tests in `agent_cli::tests`:
- `daemon_subcommand_parses_start/stop/status`
- `agent_launch_parses_profile_and_cmd`
- `agent_list_parses`
- `no_agent_query_verb_exists` (D-05 fence: must fail to parse)
- `control_pipe_name_consistency`
- `is_pipe_not_found_recognizes_gle2`
- `is_pipe_not_found_returns_false_for_other_errors`

### Task 2: main.rs, app_runtime.rs, startup_runtime.rs, cli_bootstrap.rs

- `main.rs`: added `mod agent_cli;` (not cfg-gated; non-Windows handled internally)
- `app_runtime.rs`: added `use crate::agent_cli;` + `Commands::Daemon` and `Commands::Agent` dispatch arms (following Classify pattern)
- `startup_runtime.rs`: added `Commands::Daemon(_) | Commands::Agent(_)` to update-check deny-list
- `cli_bootstrap.rs`: added `Commands::Daemon(_) | Commands::Agent(_)` to exhaustive verbosity match (verbosity = 0)

**Bonus fix:** `cli::tests::test_root_help_lists_all_commands` and `test_subcommand_help_structure` were failing before this plan (Phase 73's `classify` command was not in `ALL_SUBCOMMANDS` or `ROOT_HELP_TEMPLATE`). Both are now fixed as part of adding daemon/agent.

## Verification Results

### Build
- `cargo build -p nono-cli` — PASS (both `nono` and `nono-agentd` binaries)
- `cargo clippy -p nono-cli -- -D warnings -D clippy::unwrap_used` — PASS

### Tests
- 9 new agent_cli tests — ALL PASS
- `cargo test -p nono-cli --bin nono` — 1258 PASS / 4 FAIL (pre-existing baseline failures)

Pre-existing failures (documented in `nono_cli_windows_baseline_test_failures.md`):
- `protected_paths::tests::blocks_parent_directory_capability`
- `protected_paths::tests::blocks_child_directory_capability`
- `protected_paths::tests::requested_path_blocks_nonexistent_child_under_protected_root`
- `profile_cmd::tests::test_init_allowed_when_pack_has_same_short_name`

**No new test failures introduced.**

### Help Output
- `nono --help` lists `classify`, `daemon`, `agent` under appropriate sections
- `nono daemon --help` shows `start|stop|status|install|uninstall` subcommands
- `nono agent --help` shows `launch|list` but NOT `query` (D-05 fence confirmed)

### Cross-Target Clippy
**Status: PARTIAL / DEFERRED TO CI**

Rust targets `x86_64-unknown-linux-gnu` and `x86_64-apple-darwin` are installed but C cross-compiler (`x86_64-linux-gnu-gcc`) is absent — `ring` and `aws-lc-sys` build scripts fail. This is the same constraint documented in `feedback_clippy_cross_target.md`. Live CI (GitHub Actions with Linux runner) is the load-bearing signal for cross-target correctness.

The `agent_cli.rs` code uses `#[cfg(target_os = "windows")]` throughout — non-Windows code paths are pure Rust (no `ring`/`aws-lc-sys` deps), so the cfg-gated Unix branches should compile cleanly on CI.

## Commits

| Task | Description | Commit |
|------|-------------|--------|
| 1 | Add Daemon/Agent subcommands to cli.rs + create agent_cli.rs | `ee736894` |
| 2 | Wire dispatch in main.rs, app_runtime.rs, startup_runtime, cli_bootstrap | `c76232c9` |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed missing `classify` in ALL_SUBCOMMANDS and ROOT_HELP_TEMPLATE**
- **Found during:** Task 1 (running `cli::tests::test_root_help_lists_all_commands`)
- **Issue:** Phase 73 added `Commands::Classify` but did not update `ALL_SUBCOMMANDS` or `ROOT_HELP_TEMPLATE`; the test was failing before this plan
- **Fix:** Added `classify` to `ALL_SUBCOMMANDS` and both Windows/non-Windows `ROOT_HELP_TEMPLATE` (under EXPLORATION & DEBUGGING section)
- **Files modified:** `cli.rs`

**2. [Rule 2 - Missing] Fixed missing exhaustive match arm in cli_bootstrap.rs**
- **Found during:** Task 2 (cargo build)
- **Issue:** `cli_bootstrap::cli_verbosity` matches exhaustively on `Commands`; new variants caused compile error
- **Fix:** Added `Commands::Daemon(_) | Commands::Agent(_) => 0` arm
- **Files modified:** `cli_bootstrap.rs`

**3. [Rule 3 - Blocking] Fixed GENERIC_READ/GENERIC_WRITE import paths**
- **Found during:** Task 1 (cargo build)
- **Issue:** `windows_sys::Win32::Storage::FileSystem` does not export `GENERIC_READ`/`GENERIC_WRITE` in windows-sys 0.59; they are generic access mask constants, not file-system-specific
- **Fix:** Defined them as local `const` values (0x8000_0000 / 0x4000_0000 per Win32 ACCESS_MASK reference, matching `nono::supervisor::policy` constants)
- **Files modified:** `agent_cli.rs`

**4. [Rule 1 - Bug] Fixed EXAMPLES flag validation in test_subcommand_help_structure**
- **Found during:** Task 1 test run
- **Issue:** The initial EXAMPLES text `aider --model gpt4` contained `--model`, which the test validates against `nono agent launch`'s known flags (it doesn't have `--model`)
- **Fix:** Rewrote example to `aider --model gpt4` → `aider` (the model flag belongs to aider, not to nono agent)
- **Files modified:** `cli.rs`

## Known Stubs

**Phase 74 control pipe (INTENTIONAL):**
- `windows_control_pipe_request` attempts a real pipe connection to `\\.\pipe\nono-agentd-control`
- The daemon (nono-agentd.rs) does NOT yet listen on this control pipe — it only listens on `nono-agentd-cap` (capability pipe)
- `nono agent launch` and `nono agent list` will return "pipe not available" until Phase 75 wires the control protocol into nono-agentd
- This is by design: the CLI surface is established now; daemon-side control listener is Phase 75 scope

## Security Review (D-05 Threat Model)

| Threat | Status |
|--------|--------|
| T-74-05-01: Unknown profile spoofing | DEFERRED — daemon validates on Phase 75 connect |
| T-74-05-02: DoS via blocking pipe | MITIGATED — 5-second WaitNamedPipeW timeout in `windows_control_pipe_request` |
| T-74-05-03: agent list SID disclosure | ACCEPTED — same security context as operator; by design |
| T-74-05-04: install grants wrong service account | MITIGATED — `type= userservice` always present in `sc create` args |

## Threat Flags

None — no new network endpoints, auth paths, or schema changes introduced. The control pipe client is an outbound-only operation (CLI → daemon) with no new listener surface.

## Self-Check: PASSED

- `crates/nono-cli/src/agent_cli.rs` — FOUND
- `crates/nono-cli/src/cli.rs` — FOUND (modified)
- `crates/nono-cli/src/main.rs` — FOUND (modified)
- `crates/nono-cli/src/app_runtime.rs` — FOUND (modified)
- Commit `ee736894` — FOUND (`git log --oneline`)
- Commit `c76232c9` — FOUND (`git log --oneline`)
- `cargo test -p nono-cli --bin nono -- agent_cli` — 9 tests PASS
- `cargo build -p nono-cli` — PASS
