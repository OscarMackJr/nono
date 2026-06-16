---
phase: 72-nono-py-binding-in-process-exec-proof
plan: 02
subsystem: nono-py
tags: [windows, nono-py, pyo3, binding, confined_run, confine, shape-a, shape-b, appcontainer, low-il]

# Dependency graph
requires:
  - phase: 72-01
    provides: "Born-confined soundness PASS verdict on real Win11 Build 26200"
provides:
  - "confined_run (Shape A): Windows PyO3 pyfunction wrapping nono.exe run"
  - "confine (Shape B): born-confined self-re-exec with NONO_ALREADY_CONFINED guard"
  - "ExecResult pyclass on Windows (parallel to Unix sandboxed_exec.ExecResult)"
  - "Platform-conditional __init__.py exports (confined_run/confine on win32, sandboxed_exec on Unix)"
  - "nono = path dep to v0.62.2 local workspace (crates.io only has 0.62.0)"
affects:
  - 72-04-langchain-proof

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Shape A spawn-confined: Command::new(nono_exe).arg(run).arg(--profile)...arg(--).arg(exe) + py.detach(GIL release)"
    - "Shape B born-confined: NONO_ALREADY_CONFINED=1 guard first; spawn nono.exe with NONO_ALREADY_CONFINED=1 env; std::process::exit(child_code)"
    - "Path dep fallback: nono = { path = ../Nono/crates/nono } when crates.io pin 0.62.2 not published"
    - "cfg-gate platform fork: #![cfg(unix)] on sandboxed_exec.rs, #![cfg(windows)] on windows_confined_run.rs"

key-files:
  created:
    - ../nono-py/src/windows_confined_run.rs
    - .planning/phases/72-nono-py-binding-in-process-exec-proof/72-02-SUMMARY.md
  modified:
    - ../nono-py/Cargo.toml
    - ../nono-py/src/sandboxed_exec.rs
    - ../nono-py/src/lib.rs
    - ../nono-py/src/policy.rs
    - ../nono-py/src/proxy.rs
    - ../nono-py/src/undo.rs
    - ../nono-py/python/nono_py/__init__.py

key-decisions:
  - "Path dep fallback (D-11 deviation): crates.io nono = 0.62.2 does not exist (highest is 0.62.0); used path = ../Nono/crates/nono instead; documented as deviation per build_verification instructions"
  - "No --caps-json flag on nono-cli surface (confirmed in cli.rs); confine() caps param converts to explicit --allow flags per path dep inspection"
  - "API drift fixes (Rule 1+3): nono-py was authored against 0.57.0 API; path dep to 0.62.2 exposes 4 breaking changes (ConnectIntercept removed, RouteConfig fields removed, ProxyConfig fields removed, SessionMetadata rollback_status added); all fixed as blocking auto-fixes"
  - "Cross-target clippy: PARTIAL — x86_64-linux-gnu-gcc absent on Windows dev host; aws-lc-sys/ring C-linking blocks cross-target check; deferred to live CI per CLAUDE.md"

requirements-completed: [ABI-01]

# Metrics
duration: "~2 hours (includes API drift fix round)"
completed: 2026-06-14
---

# Phase 72 Plan 02: Windows nono-py Binding (confined_run + confine) Summary

**Windows PyO3 binding for confined_run (Shape A) and confine (Shape B) implemented, tested, and committed to the nono-py repo on branch `44-broker-ffi-lockstep`. Cargo build PASS with path dep to nono v0.62.2 local workspace; 2 unit tests passing.**

## Performance

- **Duration:** ~2 hours (includes API drift fix round-trip)
- **Started:** 2026-06-14
- **Completed:** 2026-06-14
- **Tasks:** 2 (Task 1: pin bump + windows_confined_run.rs + sandboxed_exec.rs gate; Task 2: lib.rs registration + __init__.py exports)
- **Files modified:** 7 (nono-py repo: Cargo.toml, windows_confined_run.rs, sandboxed_exec.rs, lib.rs, policy.rs, proxy.rs, undo.rs, __init__.py)

## Accomplishments

### Task 1: Pin bump + windows_confined_run.rs + sandboxed_exec.rs gate

- Bumped nono/nono-proxy from 0.57.0 to path dep (v0.62.2 local workspace; crates.io pin fallback per D-11)
- Created `src/windows_confined_run.rs` with `#![cfg(windows)]` file-level gate:
  - `find_nono_exe()`: checks `NONO_EXE` env var first, then `PATH` scan for `nono.exe`; fails loudly if `NONO_EXE` is set but invalid (fail-secure)
  - `ExecResult` pyclass with `__repr__` (parallel to Unix `sandboxed_exec.ExecResult`)
  - `confined_run(py, exe, args, allow, profile, cwd, timeout_secs)`: Shape A; GIL released via `py.detach(|| do_spawn_and_wait(...))`; validates allow/profile not both None
  - `confine(profile, allow, caps)`: Shape B; `NONO_ALREADY_CONFINED=1` guard is FIRST operation; caps converted to `--allow` flags (no `--caps-json` in CLI); calls `std::process::exit(child_code)` after spawn+wait
  - `do_spawn_and_wait()`: handles optional timeout with 10ms poll + kill on deadline (exit 124)
  - Unit tests: `test_find_nono_exe_from_env_var` + `test_find_nono_exe_not_found_returns_err` — both PASS
- Added `#![cfg(unix)]` as first file-level attribute to `src/sandboxed_exec.rs` (cross-target clippy gate)

**nono-py repo commit:** `015baf4`

### Task 2: lib.rs registration + __init__.py exports

- `src/lib.rs` mod declarations: `#[cfg(unix)] mod sandboxed_exec;` + `#[cfg(windows)] mod windows_confined_run;`
- `_nono_py()` module function: ExecResult class registration guarded (`#[cfg(not(windows))]` for sandboxed_exec variant, `#[cfg(windows)]` for windows_confined_run variant); `sandboxed_exec` function guarded `#[cfg(not(windows))]`; `confined_run` and `confine` registered under `#[cfg(windows)]`
- `python/nono_py/__init__.py`: removed `sandboxed_exec` from the unconditional flat import; added platform-conditional block (`if _sys.platform == "win32"` imports `confined_run, confine`; `else` imports `sandboxed_exec`); `__all__` extended platform-conditionally

**Note:** Task 2 lib.rs work was done in the same pass as Task 1 (no separate commit possible since both were needed for the build to succeed).

## Cargo Build Result

**PASS** — `cargo build` exits 0; `cargo test` exits 0 with 2 tests passing.

Path dep fallback used: `nono = { path = "../Nono/crates/nono" }` and `nono-proxy = { path = "../Nono/crates/nono-proxy" }`.

## Task Commits (nono-py repo)

1. `015baf4` — `feat(72-02): add Windows confined_run + confine binding (Shape A + Shape B)`
2. `774d152` — `fix(72-02): fix nono-proxy + nono API drift from 0.57.0 -> 0.62.2 path dep`

## Files Created/Modified

**nono-py repo (`C:\Users\OMack\nono-py`):**
- `src/windows_confined_run.rs` — NEW; `#![cfg(windows)]`; confined_run + confine + find_nono_exe + ExecResult + unit tests
- `Cargo.toml` — nono/nono-proxy pins bumped to path deps
- `src/sandboxed_exec.rs` — `#![cfg(unix)]` gate added at file level
- `src/lib.rs` — cfg-gated mod declarations and pymodule registrations
- `python/nono_py/__init__.py` — platform-conditional exports
- `src/policy.rs` — API drift fix (RouteConfig removed fields)
- `src/proxy.rs` — API drift fix (RouteConfig/ProxyConfig removed fields, ConnectIntercept removed)
- `src/undo.rs` — API drift fix (ConnectIntercept removed, rollback_status added)

**nono repo (this repo):**
- `.planning/phases/72-nono-py-binding-in-process-exec-proof/72-02-SUMMARY.md` — This file

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Published crates.io pin 0.62.2 does not exist**
- **Found during:** Task 1, Step 1 (first cargo build attempt)
- **Issue:** `cargo build` failed with "candidate versions found which didn't match: 0.62.0, 0.61.2..." — the fork's published crate version on crates.io is 0.62.0, not 0.62.2
- **Fix:** Applied path dep fallback as specified in `build_verification` instructions: `nono = { path = "../Nono/crates/nono" }` and `nono-proxy = { path = "../Nono/crates/nono-proxy" }`
- **Files modified:** `Cargo.toml`
- **Commit:** `015baf4`

**2. [Rule 1 - Bug + Rule 3 - Blocking] nono-py API drift from 0.57.0 → 0.62.2**
- **Found during:** Task 1 (build attempt with path deps)
- **Issue:** 4 breaking API changes between nono 0.57.0 (nono-py was authored against) and 0.62.2 local workspace:
  1. `NetworkAuditMode::ConnectIntercept` variant removed (proxy.rs line 28, undo.rs line 486)
  2. `nono_proxy::config::RouteConfig` lost fields `proxy`, `tls_client_cert`, `tls_client_key` (proxy.rs, policy.rs)
  3. `nono_proxy::ProxyConfig` lost fields `intercept_ca_dir`, `intercept_parent_ca_pems`; gained `strict_filter` (proxy.rs)
  4. `nono::undo::SessionMetadata` gained new required field `rollback_status: RollbackStatus` (undo.rs)
- **Fix:** Removed removed fields from struct constructors, getters, From impls, and serde structs; imported `RollbackStatus`; initialized `rollback_status: RollbackStatus::Available` in SessionMetadata::new
- **Files modified:** `src/policy.rs`, `src/proxy.rs`, `src/undo.rs`
- **Commit:** `774d152`

**3. [Rule 3 - Blocking] Rust edition 2024 requires unsafe for set_var/remove_var**
- **Found during:** Task 1 unit test compilation (`cargo test`)
- **Issue:** `std::env::set_var` and `std::env::remove_var` require `unsafe {}` blocks in Rust edition 2024; test code called them outside unsafe context
- **Fix:** Wrapped all env var mutations in the test module with `unsafe {}` blocks; added `// SAFETY:` comments noting test-only + no concurrent access
- **Files modified:** `src/windows_confined_run.rs`
- **Commit:** `015baf4`

**4. [Rule 2 - Missing critical functionality] lib.rs registration needed alongside sandboxed_exec.rs gate**
- **Found during:** Task 1, Step 2 (first build after adding #![cfg(unix)])
- **Issue:** Adding `#![cfg(unix)]` to sandboxed_exec.rs without guarding `mod sandboxed_exec` and `add_class::<sandboxed_exec::ExecResult>` in lib.rs caused compile errors on Windows
- **Fix:** Combined Task 1 and Task 2 lib.rs changes into a single pass so both the module declaration and the registration were guarded together. Structurally this was a Task 2 action performed during Task 1's build-fix cycle; both are in the same commit
- **Files modified:** `src/lib.rs`
- **Commit:** `015baf4`

## Cross-Target Clippy Status

**PARTIAL — deferred to live CI.**

`x86_64-linux-gnu-gcc` is not installed on this Windows dev host. The cross-target clippy attempt (`cargo clippy --target x86_64-unknown-linux-gnu`) failed in the `aws-lc-sys` build step with `ToolNotFound: failed to find tool "x86_64-linux-gnu-gcc"`. This is the documented blocker for cross-target checks on this machine (see `feedback_clippy_cross_target.md`).

The structural cfg gates are correct:
- `sandboxed_exec.rs` has `#![cfg(unix)]` at file level — the Unix-only `libc::fork`/`libc::execve` imports are gated
- `windows_confined_run.rs` has `#![cfg(windows)]` at file level — pure `std::process::Command` (no Unix APIs)
- `lib.rs` has `#[cfg(unix)] mod sandboxed_exec` and `#[cfg(windows)] mod windows_confined_run`

Live CI on a Linux runner will exercise the Unix path and catch any drift.

## Known Stubs

None. `confined_run` and `confine` are fully wired to `nono.exe run` CLI invocations. The `caps` parameter in `confine()` converts to explicit `--allow` flags (no stub — no `--caps-json` flag exists in the CLI, so this is the correct complete implementation).

## Threat Flags

No new network endpoints, auth paths, file access patterns, or schema changes outside what the plan's `<threat_model>` covers. `windows_confined_run.rs` invokes the existing `nono.exe` CLI broker (T-72-02-03 mitigated by NONO_EXE validation; T-72-02-01/04 mitigated by NONO_ALREADY_CONFINED string equality guard; T-72-02-05 mitigated by `py.detach(|| ...)`).

## Next Phase Readiness

72-04 (LangChain proof + UAT) is unblocked:
- `confined_run` and `confine` are available in the `_nono_py` module on Windows
- The proven invocation pattern from 72-01 (`nono run --profile langchain-python --allow <ws> --allow <python-dir> -- python.exe; cwd=ws`) is reproduced exactly in `confined_run`'s Command chain
- `confine(profile='langchain-python', allow=[ws])` is the correct Shape B entry point for the LangChain example

---
*Phase: 72-nono-py-binding-in-process-exec-proof*
*Completed: 2026-06-14*
