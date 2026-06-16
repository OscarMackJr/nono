---
phase: 74-persistent-multi-tenant-daemon
plan: 08
subsystem: daemon
tags: [windows, appcontainer, nono-agentd, daemon, ergonomics]

# Dependency graph
requires:
  - phase: 74-07
    provides: control loop wired (launch + list handlers); SC1 end-to-end validated
provides:
  - daemon start returns the shell promptly (DETACHED_PROCESS + Stdio::null())
  - bare-exe resolution via SearchPathW before CreateProcessW
  - AgentTenant.engine_profile; agent list shows operator profile name (e.g. aider)
affects: [74-persistent-multi-tenant-daemon]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "SearchPathW probe-then-fill idiom for bare-name exe resolution on Windows"
    - "Raw CreateProcessW(bInheritHandles=FALSE) required for true console detach — std::process::Command forces bInheritHandles=TRUE on any stdio redirect"

key-files:
  created: []
  modified:
    - crates/nono-cli/src/agent_cli.rs
    - crates/nono-cli/src/agent_daemon/launch.rs
    - crates/nono-cli/src/agent_daemon/reap.rs
    - crates/nono-cli/src/agent_daemon/control_loop.rs
    - crates/nono-cli/src/agent_daemon/accept_loop.rs

key-decisions:
  - "Resolve exe via SearchPathW BEFORE AppContainer spawn so confinement applies to the resolved binary"
  - "engine_profile stored on AgentTenant at launch time — display field, never used for security decisions"
  - "Raw CreateProcessW(bInheritHandles=FALSE) is the only correct daemon-start approach; std::process::Command::Stdio::null() is insufficient because Rust forces bInheritHandles=TRUE on any stdio redirect"

patterns-established:
  - "SearchPathW probe-then-fill (probe with len=0 returns required size; then fill with that buffer)"

requirements-completed:
  - DMON-01
  - DMON-03

# Metrics
duration: 35min
completed: 2026-06-15
---

# Phase 74 Plan 08: Operator UX Polish (Gap Closure) Summary

**Three targeted ergonomic fixes: daemon now detaches cleanly from the operator console, bare exe names resolve via SearchPathW, and `agent list` shows the engine profile name instead of the internal AppContainer moniker.**

## Performance

- **Duration:** 55 min (initial 35 min + Fix 1 root-cause iteration)
- **Completed:** 2026-06-15
- **Tasks:** 1 (single atomic task with 3 sub-fixes; Fix 1 required a second commit for the true root-cause)
- **Files modified:** 5

## Accomplishments

- Fixed `nono daemon start` shell hang (root-cause fix): replaced `std::process::Command` spawn with raw `CreateProcessW` with `bInheritHandles=FALSE`. The initial fix (`Stdio::null()` + `DETACHED_PROCESS`) was insufficient because Rust's `Command` unconditionally sets `bInheritHandles=TRUE` whenever any stdio stream is redirected — including `Stdio::null()`. With `bInheritHandles=TRUE`, the daemon inherited the launching shell's inheritable handles (console stdout pipe) and held them open until daemon exit, blocking the shell. Raw `CreateProcessW(bInheritHandles=0)` is the only correct approach. Creation flags: `DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW`. Both `hProcess` and `hThread` closed immediately after spawn (no wait). SCM path unchanged.
- Fixed `nono agent launch -- notepad.exe` (bare name) returning raw "os error 2": added `resolve_exe_path` using `SearchPathW` (probe-then-fill idiom) in `launch.rs` before `spawn_appcontainer_process_suspended`. Absolute+existing paths skip the search. Unresolvable names return a clear human-readable error. Confinement unchanged.
- Fixed `nono agent list` showing `profile=nono.session.<id>` (internal AppContainer moniker): added `engine_profile: String` to `AgentTenant`, set from the operator-supplied profile name at `launch_agent` call time, displayed in `handle_list`.

## Task Commits

1. **Task 1: clean daemon-start detach (Stdio::null attempt) + bare-exe resolution + engine-profile in list** - `4471c9bc` (fix)
2. **Task 1 Fix 1 root-cause: replace Command spawn with raw CreateProcessW(bInheritHandles=FALSE)** - `583ad6f9` (fix)

## Files Created/Modified

- `crates/nono-cli/src/agent_cli.rs` - Replaced `Command` dev-layout spawn with raw `CreateProcessW(bInheritHandles=FALSE)` in new `daemon_start_raw_spawn()` function; `daemon_start()` now calls it for dev-layout path
- `crates/nono-cli/src/agent_daemon/launch.rs` - Added `SearchPathW` import + `resolve_exe_path()` function; `launch_agent` now accepts `engine_profile: String` param; `AgentTenant` construction sets `engine_profile`
- `crates/nono-cli/src/agent_daemon/reap.rs` - Added `engine_profile: String` field to `AgentTenant` struct with doc comment
- `crates/nono-cli/src/agent_daemon/control_loop.rs` - `handle_launch` passes `resolved_profile_name` as `engine_profile` to `launch_agent`; `handle_list` displays `tenant.engine_profile` instead of `tenant.profile_name`; test updated to assert `profile=aider`
- `crates/nono-cli/src/agent_daemon/accept_loop.rs` - Test `AgentTenant` constructions updated with `engine_profile` field

## Decisions Made

- Resolve exe FIRST, then confinement applies to the resolved binary (not the bare name). This preserves the existing exe-coverage/profile check applicability and does not weaken AppContainer boundaries.
- `engine_profile` is a display/bookkeeping field only — never used for security decisions. The AppContainer token + Job Object are the actual isolation boundary.
- Raw `CreateProcessW(bInheritHandles=FALSE)` is the only correct daemon-launch approach on Windows. `std::process::Command` with `Stdio::null()` is INSUFFICIENT because Rust sets `bInheritHandles=TRUE` unconditionally when any stdio stream is redirected — the kernel still inherits all inheritable handles from the parent, not just those explicitly redirected.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fix 1 (daemon start hang) required raw CreateProcessW, not Stdio::null()**
- **Found during:** Win11 live validation of the initial fix
- **Issue:** The initial Fix 1 (`Stdio::null()` + `DETACHED_PROCESS`) did not cure the hang because `std::process::Command` sets `bInheritHandles=TRUE` on every stdio redirect. The daemon still inherited the parent shell's inheritable handles. Hang persisted.
- **Root cause:** Win32 `CreateProcessW` parameter. `bInheritHandles=TRUE` → daemon inherits all inheritable handles in parent (including console stdout pipe) regardless of which handles were redirected. `bInheritHandles=FALSE` → daemon inherits zero handles.
- **Fix:** Replaced `Command` spawn with `daemon_start_raw_spawn()` using raw `windows_sys::Win32::System::Threading::CreateProcessW` with literal `0` for `bInheritHandles`. Added `SAFETY` docs on every `unsafe` block. Unwrap-free. Error path includes `GLE=<n>`.
- **Files modified:** `crates/nono-cli/src/agent_cli.rs`
- **Commit:** `583ad6f9`

## Issues Encountered

- The test for `list_returns_tenants_when_populated` was asserting `result.contains("nono.session.aaaa1234bbbb5678")` — updated to `result.contains("profile=aider")` to match the new `engine_profile` display behavior.
- `accept_loop.rs` tests also construct `AgentTenant` directly and needed `engine_profile` added (caught at compile time, fixed immediately).
- Fix 1 Stdio::null approach appeared sound but failed live validation on Win11; the kernel-level `bInheritHandles` flag is authoritative regardless of which file descriptors are passed.

## Win11 End-to-End Validation Session

Full PowerShell session (Win11 26200, release binary, `583ad6f9` — raw CreateProcessW fix):

```
PS> Get-Process nono-agentd -ErrorAction SilentlyContinue | Stop-Process -Force
PS> Get-Process notepad -ErrorAction SilentlyContinue | Stop-Process -Force
Cleanup done

# Fix 1: Shell returns promptly — SHELL-RETURNED prints within ~1s (no hang)
PS> & .\target\release\nono.exe daemon start; Write-Host 'SHELL-RETURNED'
[dev-layout] nono-agentd started as background process (pid=25792).
Use `nono daemon status` to confirm, `nono daemon stop` to stop.
SHELL-RETURNED

PS> & .\target\release\nono.exe daemon status
nono-agentd status: RUNNING

# Fix 2: Bare exe resolved via SearchPathW
PS> & .\target\release\nono.exe agent launch --profile aider -- notepad.exe
Launched agent:
  tenant_id=6a174769c9eee6f8c1e4544bdb5a1212
  profile=aider
  sid=S-1-15-2-1996277029-389678628-3672185711-3592747021-2732199625-3032937229-2384172808
  pid=16804

# Fix 3: List shows engine profile (aider) not nono.session.<id>
PS> & .\target\release\nono.exe agent list
Tenant agents (1):
  6a174769c9eee6f8  profile=aider  sid=S-1-15-2-1996277029-389678628-3672185711-3592747021-2732199625-3032937229-2384172808  pid=16804

# Stop + verify 0
PS> & .\target\release\nono.exe daemon stop
nono-agentd stopped (dev-layout): nono-agentd: shutdown initiated.
PS> Get-Process notepad,nono-agentd -ErrorAction SilentlyContinue | Stop-Process -Force
Cleanup complete
```

**Result: All 3 fixes pass on Win11. Fix 1 confirmed: SHELL-RETURNED printed within 1s.**

## Cross-Target Clippy Status

- **Windows host (--bin nono):** PASS — `cargo clippy -p nono-cli --bin nono -- -D warnings -D clippy::unwrap_used` clean
- **Linux (x86_64-unknown-linux-gnu):** PARTIAL — cross-toolchain (`x86_64-linux-gnu-gcc`) not installed on Win11 host; deferred to live CI per `.planning/templates/cross-target-verify-checklist.md`
- **macOS (x86_64-apple-darwin):** PARTIAL — same reason; deferred to live CI
- All changes are gated on `#[cfg(target_os = "windows")]`; non-Windows paths are unchanged

## Security Notes

No confinement regression. The SearchPathW resolution runs BEFORE `spawn_appcontainer_process_suspended`; the AppContainer token and Job Object are applied to the resolved absolute binary path. The `engine_profile` field is a bookkeeping/display field only — it is never used as an authorization or capability input. The DACL/SDDL/MIC boundaries are unchanged.

## Known Stubs

None.

## Threat Flags

None — no new network endpoints, auth paths, file access patterns, or schema changes at trust boundaries.

## Self-Check: PASSED

- `4471c9bc` commit exists in git log (initial task commit)
- `583ad6f9` commit exists in git log (Fix 1 root-cause: raw CreateProcessW)
- Modified files verified: agent_cli.rs, launch.rs, reap.rs, control_loop.rs, accept_loop.rs
- `cargo build --release -p nono-cli`: clean
- `cargo test -p nono-cli --bin nono-agentd`: 14/14 pass
- `cargo clippy -p nono-cli --bin nono -- -D warnings -D clippy::unwrap_used`: clean
- Win11 end-to-end (`583ad6f9`): all 3 SC passes (clean detach with SHELL-RETURNED prompt, bare-exe via SearchPathW, engine-profile in list)

---
*Phase: 74-persistent-multi-tenant-daemon*
*Completed: 2026-06-15*
