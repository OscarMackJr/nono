---
phase: 25-cross-platform-resl-aipc-unix-design
plan: 01
subsystem: sandbox
tags: [linux, macos, cgroup-v2, setrlimit, resource-limits, cli, landlock]

requires:
  - phase: 16-resource-limits
    provides: ResourceLimits struct (cpu_percent, memory_bytes, timeout, max_processes) populated from RunArgs

provides:
  - Linux kernel-level resource enforcement via cgroup v2 delegated hierarchies (memory.max, cpu.max, pids.max, cgroup.kill)
  - macOS kernel-level enforcement via setrlimit(RLIMIT_AS, RLIMIT_NPROC) + supervisor SIGKILL watchdog
  - CgroupSession RAII struct with Drop-based cleanup and async-signal-safe child placement
  - MacosResourceLimits pre_exec applier + spawn_macos_timeout_watchdog
  - parse_cpu_percent clap wrapper that rejects --cpu-percent on macOS at parse time
  - apply_resource_limits_unix dispatch helper for Direct + Supervised execution paths
  - Integration test coverage: resl_nix_linux.rs (5 tests) + resl_nix_macos.rs (4 tests)
  - NonoError::NotSupportedOnPlatform struct variant for platform-specific feature rejection

affects: [26-aipc-unix, exec_strategy, launch_runtime, supervised_runtime, execution_runtime]

tech-stack:
  added: [nix::resource feature (setrlimit on Linux + macOS), cgroup v2 pseudo-filesystem I/O]
  patterns:
    - "RAII cgroup lifecycle: CgroupSession creates cgroup in parent, Drop removes it after child reaped"
    - "Async-signal-safe child placement: place_self_in_cgroup_raw uses only raw libc write() in post-fork child"
    - "Cfg-gated function params: extra #[cfg] params on execute_supervised avoid ExecConfig shape change"
    - "Defense-in-depth rejection: MacosResourceLimits::new re-checks cpu_percent even though clap rejects it first"

key-files:
  created:
    - crates/nono-cli/src/exec_strategy/supervisor_macos.rs
    - crates/nono-cli/tests/resl_nix_linux.rs
    - crates/nono-cli/tests/resl_nix_macos.rs
  modified:
    - crates/nono-cli/src/exec_strategy/supervisor_linux.rs
    - crates/nono-cli/src/exec_strategy.rs
    - crates/nono-cli/src/exec_strategy_windows/mod.rs
    - crates/nono-cli/src/execution_runtime.rs
    - crates/nono-cli/src/supervised_runtime.rs
    - crates/nono-cli/src/launch_runtime.rs
    - crates/nono-cli/src/cli.rs
    - crates/nono-cli/Cargo.toml
    - crates/nono/src/error.rs
    - bindings/c/src/lib.rs

key-decisions:
  - "Added NonoError::NotSupportedOnPlatform { feature: String } as a struct variant alongside existing UnsupportedPlatform(String) to give FFI consumers a typed feature field for platform-specific rejections"
  - "Cfg-gated parameters on execute_supervised and execute_direct rather than threading them through ExecConfig — avoids touching the large ExecConfig construction surface on Windows"
  - "warn_unix_resource_limits and collect_unix_resource_limit_warnings removed entirely (option (a) from plan) — dead code would fail clippy::dead_code per CLAUDE.md policy"
  - "FFI bindings (nono-ffi): NotSupportedOnPlatform maps to ErrUnsupportedPlatform — same error class, different message; backward-compatible for FFI consumers"
  - "Direct strategy timeout note: --timeout in Direct mode is not enforced (no supervisor watchdog); this is documented in execute_direct doc comment and acceptable per the plan's dispatch_layout"

patterns-established:
  - "Async-signal-safe cgroup placement: prepare all paths as null-terminated Vec<u8> in parent; use only libc::write() in child"
  - "Integration tests with skip guards: require_cgroup_v2! macro skips gracefully on CI without cgroup v2 delegation"
  - "Pre-exec resource limits: guard kept alive until exec() replaces process on Direct; kept alive until child reaped on Supervised"

requirements-completed: [RESL-NIX-01, RESL-NIX-02, RESL-NIX-03]

duration: ~120min
completed: 2026-05-10
---

# Phase 25 Plan 01: RESL-NIX Summary

**Linux cgroup v2 delegated-hierarchy + macOS setrlimit enforcement for `--memory`, `--cpu-percent`, `--max-processes`, `--timeout` flags; removes all "not enforced on linux/macos" no-op warnings from Phase 16.**

## Performance

- **Duration:** ~120 min
- **Started:** 2026-05-10T14:30:00Z (prior session)
- **Completed:** 2026-05-10T17:09:45Z
- **Tasks:** 8
- **Files modified:** 19 (16 modified, 3 created)

## Accomplishments

- Linux: `CgroupSession` creates a cgroup v2 child under the user's systemd-delegated hierarchy, writes `memory.max` / `cpu.max` / `pids.max`, places the child PID via async-signal-safe `place_self_in_cgroup_raw`, and atomically kills via `cgroup.kill` at timeout deadline. RAII `Drop` removes the cgroup directory.
- macOS: `MacosResourceLimits` applies `setrlimit(RLIMIT_AS, RLIMIT_NPROC)` in a `pre_exec` hook; `spawn_macos_timeout_watchdog` sends SIGKILL to the child process group at the `Instant` deadline. `--cpu-percent` rejected at clap parse time.
- Dispatch wired into both `execute_direct` (pre_exec hook) and `execute_supervised` (inline child fork branch + parent watchdog).
- `grep -nE "is not enforced on (linux|macos)" crates/nono-cli/src/` returns **zero matches**.
- `cargo clippy --workspace -D warnings -D clippy::unwrap_used` and `cargo fmt --all --check` both clean.
- Integration tests: `resl_nix_linux.rs` (5 tests, all gated on cgroup v2 availability) + `resl_nix_macos.rs` (4 tests).

## Task Commits

All tasks committed in a single atomic commit (this plan was a continuation from a prior context window; all work accumulated before first commit):

1. **Tasks 1-8: Full plan implementation** - `2823ec29` (feat)

## Files Created/Modified

- `crates/nono-cli/src/exec_strategy/supervisor_linux.rs` - Added `mod cgroup` submodule: `CgroupSession` (detect, new, apply_limits, install_pre_exec, place_self_in_cgroup_raw, kill_all, disarm, Drop), unit tests (4 detect_from_str), integration tests (4 cgroup lifecycle, gated)
- `crates/nono-cli/src/exec_strategy/supervisor_macos.rs` (NEW) - `MacosResourceLimits` + `spawn_macos_timeout_watchdog`, macOS-gated unit tests
- `crates/nono-cli/src/exec_strategy.rs` - `UnixResourceLimitGuard` enum, `apply_resource_limits_unix`, `spawn_linux_timeout_watchdog`, updated `execute_direct` (new Unix params), updated `execute_supervised` (new Unix params + pre-fork cgroup/rlimit setup + watchdog spawn); removed `warn_unix_resource_limits` + `collect_unix_resource_limit_warnings`
- `crates/nono-cli/src/execution_runtime.rs` - Wire Unix resource limits into non-Windows Direct + Supervised call sites
- `crates/nono-cli/src/supervised_runtime.rs` - Wire resource_limits + resource_session_id into non-Windows execute_supervised call; update stale doc comment
- `crates/nono-cli/src/launch_runtime.rs` - Update `ResourceLimits` doc comment to reflect cross-platform enforcement reality
- `crates/nono-cli/src/cli.rs` - `parse_cpu_percent` wrapper with macOS compile-time rejection
- `crates/nono-cli/Cargo.toml` - Added `resource` feature to nix for both linux and macos targets
- `crates/nono/src/error.rs` - Added `NotSupportedOnPlatform { feature: String }` variant
- `bindings/c/src/lib.rs` - Map `NotSupportedOnPlatform` to `ErrUnsupportedPlatform` in FFI match
- `crates/nono-cli/src/exec_strategy_windows/mod.rs` - Removed dead stubs `collect_unix_resource_limit_warnings` + `warn_unix_resource_limits`; updated module comment
- `crates/nono-cli/tests/resl_nix_linux.rs` (NEW) - 5 integration tests: OOM kill, pids.max, timeout, atomic grandchild kill, no-warning assertion
- `crates/nono-cli/tests/resl_nix_macos.rs` (NEW) - 4 integration tests: cpu_percent rejection, timeout, no-warning assertion, RLIMIT_NPROC

## Decisions Made

1. **`NonoError::NotSupportedOnPlatform { feature: String }` struct variant** added alongside existing `UnsupportedPlatform(String)`. The distinction: `UnsupportedPlatform` is for platform-level cgroup detection failures (runtime); `NotSupportedOnPlatform` is for features intentionally unsupported on a specific OS (e.g., `--cpu-percent` on macOS).

2. **Cfg-gated parameters on `execute_supervised` / `execute_direct`** rather than adding fields to `ExecConfig`. This avoids touching the ~15 `ExecConfig` construction sites across the codebase. Windows has its own `execute_supervised` in `exec_strategy_windows/mod.rs` which already takes `resource_limits` directly; this approach is consistent.

3. **Removed `warn_unix_resource_limits` + `collect_unix_resource_limit_warnings` entirely** (plan option (a)). Keeping them as dead stubs would trigger `clippy::dead_code` (per CLAUDE.md: "avoid #[allow(dead_code)]").

4. **FFI mapping**: `NotSupportedOnPlatform` maps to `ErrUnsupportedPlatform` — semantically the closest existing code. The error message string carries the feature name, so FFI consumers can distinguish the variants via `nono_last_error()`.

5. **Direct strategy + --timeout**: Documented in `execute_direct` doc comment that timeout is NOT enforced in Direct mode (no supervisor). The plan's `dispatch_layout` acknowledged this: "Direct gets the guard but no watchdog." This is a known limitation, not a bug.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Added NotSupportedOnPlatform variant to nono-ffi match**
- **Found during:** Task 7 (Wire dispatch + warning removal) — workspace-level cargo check
- **Issue:** Adding `NotSupportedOnPlatform` to `NonoError` caused non-exhaustive match in `bindings/c/src/lib.rs:75`. The plan listed `exec_strategy.rs` and `launch_runtime.rs` as `files_modified` but the FFI exhaustive-match breakage was an implicit dependency.
- **Fix:** Added `NotSupportedOnPlatform { .. } => NonoErrorCode::ErrUnsupportedPlatform` with doc comment explaining the mapping rationale.
- **Files modified:** `bindings/c/src/lib.rs`
- **Verification:** `cargo check --workspace` clean.
- **Committed in:** `2823ec29`

---

**Total deviations:** 1 auto-fixed (Rule 2 - missing critical FFI error mapping)
**Impact on plan:** Necessary for workspace compilation correctness. No scope creep.

## Issues Encountered

- **Context window split**: The plan was started in a prior conversation that ran out of context mid-Task 7. The continuation agent picked up from `supervised_runtime.rs` non-Windows call site, which was the last incomplete piece.
- **Worktree creation**: The specified worktree (`agent-a1170c67365199107`) did not exist at session start; it was created from the base commit specified in the executor prompt.

## Known Stubs

None. All resource limit enforcement paths are wired end-to-end. The Direct strategy + `--timeout` limitation is intentional and documented (not a stub — it's a documented scope boundary per the plan's `dispatch_layout`).

## Threat Flags

None. No new network endpoints, auth paths, or trust boundaries introduced. The new cgroup pseudo-filesystem writes are within the user's own delegation scope (T-25-01-03 accepted in the plan's threat model).

## Next Phase Readiness

- Phase 25-02 (AIPC Unix futures ADR) can proceed independently — it does not depend on this plan's enforcement paths.
- The `ResourceLimits` struct shape is unchanged; any phase referencing it remains compatible.
- cgroup v2 detection fail-fast (`NonoError::UnsupportedPlatform("cgroup_v2: ...")`) is surfaced before any child spawn — non-systemd deployments will see a clear error message.

## Self-Check

**Files exist:**
- `crates/nono-cli/src/exec_strategy/supervisor_macos.rs`: FOUND (new file in commit)
- `crates/nono-cli/tests/resl_nix_linux.rs`: FOUND (new file in commit)
- `crates/nono-cli/tests/resl_nix_macos.rs`: FOUND (new file in commit)

**Commits exist:**
- `2823ec29`: FOUND (feat(25-01): implement Linux cgroup v2 + macOS setrlimit resource enforcement)

**Warning grep:** `grep -nE "is not enforced on (linux|macos)" crates/nono-cli/src/` returns ZERO matches.

## Self-Check: PASSED

---
*Phase: 25-cross-platform-resl-aipc-unix-design*
*Completed: 2026-05-10*
