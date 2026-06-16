---
phase: 71-engine-agnostic-launch-productionization
plan: "04"
subsystem: cli-launch-integration
tags: [windows, engine-profiles, workspace, interpreter-coverage, rb3-gate, fail-secure, cli]
dependency_graph:
  requires: [71-01-windows_interpreters-field, 71-03-coverage-gate-primitives]
  provides:
    - --workspace flag: CLI flag for single-source-of-truth engine working directory
    - workspace→cwd wiring: child CWD = canonicalized absolute workspace (D-05/D-06)
    - workspace read+write grant: auto-grant at declare time, no prompt
    - aider profile hint: recommended_builtin_profile maps aider/aider.exe → aider
    - read_distlib_shebang: diagnostic-only interpreter path from PE-embedded shebang
    - resolve_interpreter_paths: shebang ∪ PATH resolver for windows_interpreters
    - interpreter coverage threading: ExecConfig.interpreters → validate_launch_paths
    - R-B3 pre-launch gate: path_has_write_owner check before AppliedLabelsGuard
  affects:
    - crates/nono-cli/src/cli.rs
    - crates/nono-cli/src/execution_runtime.rs
    - crates/nono-cli/src/exec_strategy_windows/mod.rs
    - crates/nono/src/lib.rs
tech_stack:
  added: []
  patterns:
    - "workspace as single source of truth: CLI flag → resolved_workdir → child CWD + writable grant"
    - "distlib shebang read: scan PE binary for #! marker, extract path bytes"
    - "TDD RED/GREEN: interpreter_resolve_tests + rb3_gate_tests written before/alongside impl"
    - "R-B3 gate: path_has_write_owner BEFORE AppliedLabelsGuard, fail-secure SandboxInit Err"
    - "ExecConfig extension: interpreters field threads resolved set into coverage gate"
key_files:
  created: []
  modified:
    - crates/nono-cli/src/cli.rs
    - crates/nono-cli/src/execution_runtime.rs
    - crates/nono-cli/src/launch_runtime.rs
    - crates/nono-cli/src/sandbox_prepare.rs
    - crates/nono-cli/src/exec_strategy_windows/mod.rs
    - crates/nono-cli/src/exec_strategy_windows/launch.rs
    - crates/nono-cli/src/exec_strategy_windows/network.rs
    - crates/nono/src/lib.rs
decisions:
  - "workspace auto-grants read+write without prompt (sandbox_prepare.rs) — operator intent is explicit (like --allow), no prompt needed; workspace != workdir semantically (workdir = $WORKDIR expansion hint; workspace = child CWD + writable grant)"
  - "resolved_workdir() prefers workspace over workdir — D-06 single source of truth: if workspace is declared, it IS the child CWD; workdir remains for profile $WORKDIR expansion only"
  - "read_distlib_shebang scans the full file for #! (not offset-based) — covers both old (header-only) and new (appended-ZIP) distlib stub variants"
  - "path_has_write_owner exported from nono::lib.rs for CLI consumption — previously library-internal; Plan 04 R-B3 gate needs it from the CLI crate"
  - "R-B3 gate uses NonoError::SandboxInit (not LabelApplyFailed) — fires pre-relabel so error is named before any HRESULT is seen"
  - "interpreter bare-name pass-through on unresolvable — rather than Err at resolution time, pass the bare name through so the coverage gate names it in the refusal (fail-secure, not silent)"
metrics:
  duration_minutes: 45
  completed_date: "2026-06-14"
  tasks_completed: 3
  tasks_total: 3
  files_changed: 8
---

# Phase 71 Plan 04: CLI Integration (workspace + interpreters + R-B3 gate) — Summary

**One-liner:** Wired `--workspace` as the single source of truth for child CWD and writable grant (D-05/D-06), threaded resolved interpreter paths (shebang assist ∪ PATH) into the coverage gate (D-07), and inserted the R-B3 pre-launch WRITE_OWNER gate before any relabel (D-08) — completing the ENG-01/ENG-02 fail-secure launch path.

## Tasks Completed

| Task | Name | Commit | Key Files |
|------|------|--------|-----------|
| 1 | --workspace flag + workspace→cwd + writable grant + aider hint | `2001bf97` | cli.rs, execution_runtime.rs, launch_runtime.rs, sandbox_prepare.rs |
| 2 | Resolve interpreters + thread into coverage gate | `870ce1f5` | exec_strategy_windows/mod.rs, execution_runtime.rs, launch.rs, network.rs |
| 3 | R-B3 pre-launch ownership gate at the chokepoint | `ddb335ab` | exec_strategy_windows/mod.rs, nono/src/lib.rs |

## What Was Built

### Task 1 — `--workspace` flag + CWD/grant wiring + aider hint

Added `pub workspace: Option<PathBuf>` to both `SandboxArgs` (cli.rs:1621) and `WrapSandboxArgs` (cli.rs:2029) with `#[arg(long, value_name = "DIR", help_heading = "FILESYSTEM")]` and doc-comment explaining it is the single source of truth for child CWD AND writable grant (D-06).

`resolved_workdir()` in `sandbox_prepare.rs` now prefers `workspace` over `workdir` (D-06: if workspace is declared, it IS the child CWD). `prepare_sandbox_with_context` auto-grants the canonicalized workspace as read+write when explicitly declared (no prompt — operator intent is explicit, like `--allow`).

`launch_runtime.rs` wires `workspace.or(workdir)` into `ExecutionFlags.workdir` so the Windows child's `lpCurrentDirectory` receives the canonicalized absolute workspace (kills the PowerShell→`C:\` relative-write trap, T-71-09).

`recommended_builtin_profile` extended with `"aider" | "aider.exe" => Some("aider")` (D-04). Test updated with 2 new aider assertions; passes.

`From<WrapSandboxArgs> for SandboxArgs` updated to forward `workspace`.

### Task 2 — Interpreter resolution + interpreter coverage threading

Added `interpreters: Vec<PathBuf>` field to `ExecConfig` with doc-comment (D-07/ENG-02).

**`read_distlib_shebang(exe_path: &Path) -> Option<PathBuf>`** (diagnostic-only utility, Windows-gated): scans the binary for `#!` bytes (covers both header-only and appended-ZIP distlib variants), extracts the interpreter path, returns `None` on any error or absent marker. NEVER auto-grants (D-07/T-71-13).

**`resolve_interpreter_paths(program: &Path, declared: &[String]) -> Vec<PathBuf>`** (diagnostic-only): for each declared bare name, tries (1) distlib shebang match by filename (case-insensitive), then (2) `which::which` PATH resolution, then (3) passes through bare name as-is for fail-secure gate naming.

`execution_runtime.rs` (Windows cfg-gated) resolves `profile.windows_interpreters` via `resolve_interpreter_paths` before building `ExecConfig`, then populates `interpreters: windows_resolved_interpreters`.

`prepare_live_windows_launch` now passes `&config.interpreters` (instead of `&[]`) to `Sandbox::validate_windows_launch_paths` — the Plan 03 coverage gate now receives the real resolved set.

`launch.rs` and `network.rs` test helpers updated with `interpreters: Vec::new()` (backward-compatible).

**Tests (6):** shebang_read_extracts_interpreter_from_fixture, shebang_read_returns_none_when_no_shebang, shebang_read_returns_none_for_missing_file, resolve_interpreter_paths_returns_candidate_paths_not_grants, resolve_interpreter_paths_falls_back_to_bare_name_when_unresolvable, resolve_interpreter_paths_empty_slice_returns_empty — all PASS.

### Task 3 — R-B3 pre-launch workspace ownership gate

Exported `path_has_write_owner` from `nono::lib.rs` (was library-internal; CLI needs it).

In `prepare_live_windows_launch`, inserted **R-B3 GATE A** between `validate_windows_command_args` (line 351) and `AppliedLabelsGuard::snapshot_and_apply` (line 413):
- Calls `nono::path_has_write_owner(config.current_dir)` — workspace == child CWD by D-06 construction
- On `false`: returns `Err(NonoError::SandboxInit(...))` with named diagnostic:
  - Names WRITE_OWNER (0x00080000) as the missing permission
  - Identifies the elevated-console/admin-ownership trap
  - Provides `%USERPROFILE%` and `%TEMP%` workspace alternatives
  - Provides `icacls ... /grant` fix for existing paths
  - States explicitly: "nono will NOT take ownership automatically" (D-08)
- Error variant: `SandboxInit` (not `LabelApplyFailed`) — fires before any relabel, so the failure is named, not opaque (Pitfall 2)

**Tests (3):** workspace_owned_by_current_user_passes_write_owner_check (PASS branch: user-owned tempdir), system_dir_lacks_write_owner_for_standard_user (FAIL branch: System32 non-elevated), rb3_error_message_names_cause_and_fix_without_auto_takeown (message assertions) — all PASS.

## Verification

```
cargo build -p nono-cli          # clean
cargo test -p nono-cli           # 1246 passed; 4 pre-existing failures unchanged
cargo build -p nono              # clean
cargo test -p nono               # 783 passed; 1 pre-existing try_set_mandatory_label failure
cargo test -p nono-cli recommended_builtin_profile_matches_known_agent_commands  # PASS
cargo test -p nono-cli "interpreter_resolve"   # 6/6 PASS
cargo test -p nono-cli "rb3_gate"              # 3/3 PASS
```

## Cross-Target Gate

`exec_strategy_windows/mod.rs` is `cfg(target_os = "windows")`-gated code. Per CLAUDE.md requirement, cross-target clippy verification against `x86_64-unknown-linux-gnu` and `x86_64-apple-darwin` is REQUIRED but the Windows host cannot cross-compile (ring/aws-lc-sys C-toolchain unavailable). Status: **PARTIAL** — Windows host build and tests are clean; Linux/macOS cross-target clippy deferred to live CI per `.planning/templates/cross-target-verify-checklist.md`.

`cli.rs`, `execution_runtime.rs`, `sandbox_prepare.rs`, `launch_runtime.rs`, and `nono/src/lib.rs` are platform-neutral. The `workspace` flag and `aider` hint changes compile on all platforms.

## Threat Mitigations Applied

| Threat | Status |
|--------|--------|
| T-71-09: PowerShell→C:\ relative-write trap | MITIGATED — workspace → config.current_dir (child lpCurrentDirectory) + absolute read+write grant |
| T-71-10: uncovered interpreter (partial confinement) | MITIGATED — resolve_interpreter_paths + threading into validate_launch_paths; shebang assist NEVER auto-grants |
| T-71-11: admin-owned workspace → opaque confinement failure | MITIGATED — R-B3 gate before AppliedLabelsGuard with named diagnostic; no auto-takeown |
| T-71-12: string starts_with path comparison | MITIGATED — all coverage checks go through covers_path (Plan 03, component-wise) |
| T-71-13: auto-widening from interpreter denial | MITIGATED — read_distlib_shebang + resolve_interpreter_paths return candidate paths only, no grants |
| T-71-SC: npm/pip/cargo installs | N/A — CLI plumbing edits only; which crate already in-tree |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] ExecConfig.interpreters missing from test helper initializers in launch.rs and network.rs**
- **Found during:** Task 2 (test compilation)
- **Issue:** `launch.rs::make_minimal_exec_config` and 3 `ExecConfig` literals in `network.rs` are exhaustive struct literals; adding `interpreters` caused E0063 compile errors
- **Fix:** Added `interpreters: Vec::new()` to each (backward-compatible; test helpers don't exercise interpreter coverage)
- **Files modified:** `exec_strategy_windows/launch.rs`, `exec_strategy_windows/network.rs`
- **Commit:** `870ce1f5`

**2. [Rule 2 - Missing] path_has_write_owner not in nono public API**
- **Found during:** Task 3 (build)
- **Issue:** `nono::sandbox::windows::path_has_write_owner` was library-internal; the CLI needed it as a public API for the R-B3 gate
- **Fix:** Added `path_has_write_owner` to the `#[cfg(target_os = "windows")] pub use sandbox::windows::{...}` re-export list in `nono/src/lib.rs`
- **Files modified:** `crates/nono/src/lib.rs`
- **Commit:** `ddb335ab`

## Known Stubs

None. All three gates (workspace grant, interpreter coverage threading, R-B3 ownership) are wired at the production call sites. The `interpreter_resolve_tests` fixtures use tempdir (not a real aider.exe) but the shebang format is correctly simulated; the production path exercises the same code.

## Threat Flags

None — changes are either Windows-only (`exec_strategy_windows/`) or CLI-plumbing only (no new network endpoints, auth paths, or schema changes). The `path_has_write_owner` re-export adds a new symbol to `nono::lib.rs` but it is already present in the library; the export does not introduce new functionality.

## Self-Check: PASSED

Files created/modified exist:
- `crates/nono-cli/src/cli.rs` — FOUND (workspace field on SandboxArgs + WrapSandboxArgs)
- `crates/nono-cli/src/execution_runtime.rs` — FOUND (aider hint + interpreter resolution)
- `crates/nono-cli/src/launch_runtime.rs` — FOUND (workspace→workdir priority)
- `crates/nono-cli/src/sandbox_prepare.rs` — FOUND (resolved_workdir + workspace grant)
- `crates/nono-cli/src/exec_strategy_windows/mod.rs` — FOUND (interpreters + R-B3 gate + tests)
- `crates/nono-cli/src/exec_strategy_windows/launch.rs` — FOUND (interpreters: Vec::new())
- `crates/nono-cli/src/exec_strategy_windows/network.rs` — FOUND (interpreters: Vec::new())
- `crates/nono/src/lib.rs` — FOUND (path_has_write_owner re-export)

Commits verified:
- `2001bf97` — FOUND (feat: --workspace flag + aider hint + workspace CWD/grant wiring)
- `870ce1f5` — FOUND (feat: interpreter resolve + thread into coverage gate)
- `ddb335ab` — FOUND (feat: R-B3 pre-launch workspace ownership gate)
