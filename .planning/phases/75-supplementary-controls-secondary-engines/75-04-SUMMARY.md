---
phase: "75"
plan: "04"
subsystem: "nono-ts"
tags: ["napi", "windows", "confinement", "bindings", "SUPP-03"]
dependency_graph:
  requires:
    - "nono 0.62.2 (local path dep)"
    - "napi 2 (existing)"
  provides:
    - "confinedRun (Shape A) napi export — Windows-gated"
    - "confine (Shape B) napi export — Windows-gated"
    - "JsExecResult napi(object) struct"
    - "Non-Windows stubs for confinedRun and confine"
  affects:
    - "../nono-ts bindings API surface"
key_files:
  created:
    - "../nono-ts/src/windows_confined_run.rs"
  modified:
    - "../nono-ts/src/lib.rs"
    - "../nono-ts/Cargo.toml"
    - "../nono-ts/Cargo.lock"
decisions:
  - "Used path dep { path = '../Nono/crates/nono', version = '0.62' } instead of registry-only because nono 0.62.0 on crates.io lacks cfg gating for Unix-only MetadataExt (mtime/mode) methods — causes Windows build failure. Local 0.62.2 has the fix. Mirrors nono-py precedent exactly."
  - "Extracted is_already_confined() helper to enable unit-testing the NONO_ALREADY_CONFINED guard without triggering process::exit() (which kills the test runner)."
  - "Added ENV_LOCK mutex to serialize env-var-mutating tests — cargo test's parallel runner caused race conditions in NONO_ALREADY_CONFINED and NONO_EXE tests."
metrics:
  duration: "~45 minutes"
  completed: "2026-06-15"
  tasks_completed: 2
  files_created: 1
  files_modified: 3
---

# Phase 75 Plan 04: nono-ts Parity (confinedRun + confine) Summary

**One-liner:** nono-ts gains confinedRun (Shape A spawn-confined) + confine (Shape B born-confined broker re-exec) napi exports with Windows cfg gating, non-Windows stubs, and nono pin bumped to 0.62 via local path dep.

## What Was Built

### `../nono-ts/src/windows_confined_run.rs` (new)

Direct port of `../nono-py/src/windows_confined_run.rs` with pyo3 → napi error type surface translation only. All logic is identical to nono-py.

- `#![cfg(windows)]` module gate
- `find_nono_exe()` — NONO_EXE env var first, then PATH search; fail-secure (never silently falls back)
- `build_nono_run_args()` — --profile / --allow / --allow-cwd CLI arg builder
- `confined_run()` (Shape A) — pub(crate), validated, delegates to nono.exe run
- `confine()` (Shape B) — NONO_ALREADY_CONFINED exact "1" guard first, then re-exec via nono.exe, then `std::process::exit()`
- `do_spawn_and_wait()` — spawn + capture stdout/stderr + optional timeout with background drain threads
- `is_already_confined()` — extracted guard helper (enables safe unit testing)
- 5 unit tests with ENV_LOCK serialization

### `../nono-ts/src/lib.rs` (modified)

Added after existing exports (comment delimiter: `// --- Windows confined execution (SUPP-03b) ---`):

- `JsExecResult` — `#[napi(object)]` struct with `stdout: Vec<u8>`, `stderr: Vec<u8>`, `exit_code: i32` (declared unconditionally for both Windows + non-Windows stub signatures)
- `#[cfg(target_os = "windows")] mod windows_confined_run;` declaration
- `confinedRun` Windows export: `#[napi(js_name = "confinedRun")] #[cfg(target_os = "windows")]`
- `confinedRun` non-Windows stub: returns `Err(GenericFailure, "confinedRun is Windows-only")`
- `confine` Windows export: `#[napi] #[cfg(target_os = "windows")]`
- `confine` non-Windows stub: returns `Err(GenericFailure, "confine is Windows-only")`

### `../nono-ts/Cargo.toml` (modified)

```toml
# Before:
nono = { version = "0.33.0" }

# After:
nono = { path = "../Nono/crates/nono", version = "0.62" }
```

napi/napi-derive/napi-build remain at version "2" (D-07: no napi 3 migration).

## Verification Results

| Check | Result |
|-------|--------|
| `cargo build --target x86_64-pc-windows-msvc` | PASS (nono 0.62.2 local, 42.77s) |
| `cargo test --target x86_64-pc-windows-msvc` | PASS (5/5 tests, 0 failed) |
| NONO_ALREADY_CONFINED guard uses `== Ok("1")` | CONFIRMED (line 342) |
| napi 2 pins unchanged | CONFIRMED |
| No `.unwrap()`/`.expect()` in production code | CONFIRMED |
| cross-target clippy Linux (`x86_64-unknown-linux-gnu`) | PARTIAL — see below |
| cross-target clippy macOS (`x86_64-apple-darwin`) | PARTIAL — see below |

### Unit Test Results

```
running 5 tests
test windows_confined_run::tests::test_confine_already_confined_returns_ok ... ok
test windows_confined_run::tests::test_confine_guard_exact_match ... ok
test windows_confined_run::tests::test_confined_run_requires_profile_or_allow ... ok
test windows_confined_run::tests::test_find_nono_exe_from_env_var ... ok
test windows_confined_run::tests::test_find_nono_exe_not_found_returns_err ... ok
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

## Cross-Target Clippy: PARTIAL

Per CLAUDE.md cross-target clippy rule, both cross-target runs were attempted from `../nono-ts/` directory:

- `cargo clippy --target x86_64-unknown-linux-gnu`: exit code 0 (no Rust lint errors), but `warning: build failed` from missing `x86_64-linux-gnu-gcc` C compiler (aws-lc-sys native dependency). Rust cfg-gated logic was type-checked; C link layer failed.
- `cargo clippy --target x86_64-apple-darwin`: exit code 0 (no Rust lint errors), but `warning: build failed` from missing macOS `cc` cross-compiler (aws-lc-sys). Same pattern.

**Verdict:** Cross-target Rust linting PASSED (exit 0); C cross-compiler toolchain absent on Win11 host. Deferred to live CI per `.planning/templates/cross-target-verify-checklist.md`.

CI gate required: both `x86_64-unknown-linux-gnu` and `x86_64-apple-darwin` targets must build and clippy clean in CI before this plan is considered fully verified.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] nono 0.62.0 (crates.io) fails to build on Windows**

- **Found during:** Task 1, initial `cargo build --target x86_64-pc-windows-msvc`
- **Issue:** `nono 0.62.0` on crates.io references `MetadataExt::mtime()` and `MetadataExt::mode()` (Unix-only `std::os::unix::fs::MetadataExt` trait) without cfg gating in `src/undo/snapshot.rs`. Windows build fails with 45 `E0599` errors (method not found in scope).
- **Fix:** Changed `nono = { version = "0.62" }` to `nono = { path = "../Nono/crates/nono", version = "0.62" }`. The local workspace at `0.62.2` has proper `#[cfg(unix)]` / `#[cfg(not(unix))]` gating in `metadata_mtime()` and `metadata_permissions()`. This matches nono-py's approach exactly (`nono = { path = "../Nono/crates/nono" }` — no version pin in nono-py).
- **Files modified:** `../nono-ts/Cargo.toml`

**2. [Rule 1 - Bug] test_confine_already_confined_returns_ok killed test runner**

- **Found during:** Task 1 test run
- **Issue:** `confine()` on the non-guard path calls `std::process::exit()`. When run in cargo test's parallel runner, a race between `test_confine_already_confined_returns_ok` (sets NONO_ALREADY_CONFINED=1) and `test_confine_guard_exact_match` (sets NONO_ALREADY_CONFINED=1extra) caused the guard to not fire in the intended test, leading nono.exe to be spawned, which then called `process::exit()` and killed the test runner.
- **Fix:** 
  1. Extracted `is_already_confined()` helper function that tests the guard condition without triggering the `process::exit()` path. `confine()` now calls this helper.
  2. Tests for the guard (`test_confine_already_confined_returns_ok`, `test_confine_guard_exact_match`) test `is_already_confined()` directly rather than calling full `confine()`.
  3. Added `static ENV_LOCK: Mutex<()>` to serialize all env-var-mutating tests, preventing race conditions in cargo test's parallel runner.
- **Files modified:** `../nono-ts/src/windows_confined_run.rs`

**3. [Rule 1 - Bug] `result.unwrap_err()` requires `T: Debug`**

- **Found during:** Task 1 first test run
- **Issue:** `result.unwrap_err()` in `test_confined_run_requires_profile_or_allow` required `JsExecResult: Debug`, but `#[napi(object)]` structs don't auto-derive Debug.
- **Fix:** Changed test to use `match result { Err(e) => { assert!(e.reason.contains(...)) }, Ok(_) => panic!(...) }` — avoids the `Debug` bound entirely.
- **Files modified:** `../nono-ts/src/windows_confined_run.rs`

## Security Notes

- T-75-04-01 (NONO_ALREADY_CONFINED infinite re-exec DoS): mitigated via exact "1" string equality. Negative test: `test_confine_guard_exact_match` verifies "1extra" does NOT match.
- T-75-04-02 (caller bypasses confine by pre-setting guard): accepted by design — the guard is correct for already-confined processes.
- T-75-04-03 (NONO_EXE points to malicious binary): mitigated via `is_file()` check in `find_nono_exe()`. Operator responsible for PATH integrity.
- T-75-04-SC (npm/pip/cargo install legitimacy): zero new external packages. Only pin change is internal: nono 0.33.0 → 0.62 (same crate, local path).

## Threat Flags

None — no new network endpoints, auth paths, file access patterns, or schema changes at trust boundaries introduced.

## Known Stubs

None — `confinedRun` and `confine` are fully implemented on Windows. The non-Windows stubs intentionally throw errors ("Windows-only") per the D-07 design constraint.

## Self-Check: PASSED

- [x] `../nono-ts/src/windows_confined_run.rs` exists with `find_nono_exe`, `confined_run`, `confine`, `is_already_confined`, 5 unit tests
- [x] `../nono-ts/src/lib.rs` contains `JsExecResult`, `confinedRun` (Windows + stub), `confine` (Windows + stub)
- [x] `../nono-ts/Cargo.toml` contains `version = "0.62"` and path dep
- [x] nono-ts commit `e218827` on branch `44-broker-ffi-lockstep` with DCO sign-off
- [x] `cargo build` green, `cargo test` 5/5 passed
- [x] Cross-target clippy: PARTIAL documented (CI gate required)
