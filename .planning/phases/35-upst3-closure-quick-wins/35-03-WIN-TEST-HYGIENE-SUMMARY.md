---
phase: 35-upst3-closure-quick-wins
plan: "03"
subsystem: nono-cli
tags:
  - phase-35
  - port-closure
  - windows
  - test-hygiene
  - json-shape
  - fork-local-regression
  - p34-defer-01-1
  - p34-defer-09-3
  - p34-defer-10-1
dependency_graph:
  requires:
    - 35-02-LINUX-LANDLOCK-PROFILES (wave peer — runs in parallel)
    - 35-01-WIN-ENV-FILTER (wave peer — runs in parallel)
  provides:
    - Clean JSON shape for policy show/diff --json (no Debug-format leakage)
    - Typeable suggested_flag values from query_path (no UNC verbatim prefix)
    - Phase 34 deferred-items ledger closure (D-35-D4)
  affects:
    - crates/nono-cli/src/query_ext.rs
    - crates/nono-cli/src/profile_cmd.rs
    - .planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/deferred-items.md
tech_stack:
  added: []
  patterns:
    - serde_json::Map insertion with omit-when-None semantics for Option<...> enum fields
    - strip_verbatim_prefix helper reuse across multiple emission sites
    - Platform-agnostic test assertions via production-code helper reuse
key_files:
  created: []
  modified:
    - crates/nono-cli/src/query_ext.rs
    - crates/nono-cli/src/profile_cmd.rs
    - .planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/deferred-items.md
decisions:
  - "Computed expected test values from production helpers (strip_verbatim_prefix + suggested_flag_parts) rather than hard-coding platform-specific path strings — makes test_query_path_denied cross-platform without #[cfg] gates"
  - "Used unwrap_or_else (not Result propagation) for inject_mode in diff_custom_credentials_json to avoid changing that function's return type — InjectMode Serialize cannot fail in practice; fallback is unreachable"
  - "Pre-computed serde_json::to_value results before json!() macro in diff_to_json to enable ? propagation — json!() macro does not support ? operator"
metrics:
  duration: "~35 minutes"
  completed: "2026-05-12"
  tasks_completed: 3
  tasks_total: 3
  files_modified: 3
commits:
  task_1_unc_strip: "d8cb250b"
  task_2_json_shape: "66d7a386"
  task_3_closure_ledger: "a444558c"
---

# Phase 35 Plan 03: WIN-TEST-HYGIENE Summary

**One-liner:** Fixed Windows UNC-prefix UX bug in suggested_flag emission and replaced format!("{:?}") JSON Debug-leakage with serde_json::Map + omit-when-None in profile show/diff --json output.

## What Was Done

### Task 1: Strip UNC verbatim prefix in query_path suggested_flag emission (D-35-C1)

**Commit:** `d8cb250b`

Wrapped both `suggested_flag_for_path(&canonical, ...)` call sites in `query_path` with the existing `strip_verbatim_prefix` helper (introduced by in-fork commit `400f8c90` for the sensitive-path check). Both call sites: insufficient_access branch (near-miss case) and path_not_granted branch (terminal deny).

On Windows, `Path::canonicalize` returns UNC verbatim form (`\\?\C:\...`), making suggested flags untypeable by operators. The helper strips `\\?\`, `\\?\UNC\`, and `\??\` prefixes; the non-Windows arm is the identity no-op. No `#[cfg]` gate at the call sites.

Also updated two tests that were asserting the buggy UNC-prefixed output:
- `test_query_path_denied`: now computes expected_flag via `strip_verbatim_prefix` + `suggested_flag_parts` — cross-platform deterministic without any `#[cfg]` gate
- `test_query_path_reports_near_miss_with_source_and_fix`: now applies `strip_verbatim_prefix` to `test_file_canon` before asserting the expected flag

**Closes:** P34-DEFER-01-1, P34-DEFER-09-3 (transitive carry-forward duplicate)

### Task 2: Replace format!("{:?}") JSON-emission with serde_json::Map insertion (D-35-C2 + D-35-C3)

**Commit:** `66d7a386`

Full audit of all in-scope `format!("{:?}")` / `format!("{:#?}")` JSON-emission sites in `profile_cmd.rs` per PATTERNS.md scope table. Three functions refactored:

**profile_to_json (line 1041):**
- Changed return type from `serde_json::Value` to `Result<serde_json::Value>`
- Replaced `serde_json::json!({...})` security block with `serde_json::Map` insertion
- Omit-when-None semantics for 4 Option<...> fields: `signal_mode`, `process_info_mode`, `ipc_mode`, `wsl2_proxy_policy`
- `workdir.access` (non-Optional): `serde_json::to_value` emits `"readwrite"`/`"readonly"` via existing `#[serde(rename_all = "lowercase")]`
- `cmd_show` call site updated to propagate via `?`

**diff_to_json (line 1777):**
- Changed return type to `Result<serde_json::Value>`
- Pre-computed `wsl2_proxy_policy` (both profiles) and `workdir.access` (both profiles) before `json!()` macro call to enable `?` propagation
- None policy values emit as JSON null (diff shape preserved with profile1/profile2 keys)
- `cmd_diff` call site updated to propagate via `?`

**diff_custom_credentials_json (inject_mode at line ~1991):**
- `InjectMode` carries `#[serde(rename_all = "snake_case")]` so `serde_json::to_value` emits `"header"` / `"url_path"` etc.
- Used `unwrap_or_else` fallback (not propagation) since this function returns `serde_json::Value` and the Serialize impl cannot fail in practice

**Out-of-scope preserved (D-35-C3):** Lines ~1297-1318 in `cmd_diff` body — `diff_scalar_option` stdout printers (colored human-readable output, NOT JSON emission) — untouched.

Both regression tests pass:
- `test_policy_show_json_no_rust_debug_syntax`: 1 passed
- `test_policy_diff_json_no_rust_debug_syntax`: 1 passed

**Closes:** P34-DEFER-10-1 (entire format!("{:?}") JSON-leak regression class per D-35-C3 full audit)

### Task 3: Append Phase 35 closure section to Phase 34 deferred-items ledger (D-35-D4)

**Commit:** `a444558c`

Appended a `## Phase 35 closure` section at the bottom of `.planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/deferred-items.md` with five sub-entries:

| Ticket | Closed By | Closing Commit | Summary |
|--------|-----------|----------------|---------|
| P34-DEFER-01-1 | Plan 35-03 Task 1 | `d8cb250b` | UNC prefix strip in query_path suggested_flag |
| P34-DEFER-08a-1 | Plan 35-01 | `6a4d9932` | Windows env-filter wiring |
| P34-DEFER-09-1 | Plan 35-02 Task 1 | `327fe104` | D-19 cherry-pick of upstream bdf183e9 |
| P34-DEFER-09-3 | Plan 35-03 Task 1 (transitive) | `d8cb250b` | Carry-forward duplicate of P34-DEFER-01-1 |
| P34-DEFER-10-1 | Plan 35-03 Task 2 | `66d7a386` | JSON Debug-leak full audit + Map replacement |

Pre-existing 13 P34-DEFER-* entries unchanged (purely additive).

## Verification Gate Disposition (D-35-D2)

| Gate | Status | Notes |
|------|--------|-------|
| 1. `cargo test --workspace` (Windows host) | PASS | 944+ tests pass; 0 failed across nono-cli |
| 2. `cargo clippy --workspace` (Windows host) | PASS | 0 warnings, 0 errors |
| 3. Cross-target Linux clippy | Not run on this Windows host | Requires Linux cross-compilation toolchain; covered by CI lanes |
| 4. Cross-target macOS clippy | Not run on this Windows host | Covered by CI lanes |
| 5. `cargo fmt --all -- --check` | PASS | Clean after `cargo fmt --all` |
| 6. Phase 15 5-row detached-console smoke gate | N/A | No Windows execution-path edits |
| 7. `wfp_port_integration` test suite | Skip-documented | No WFP surface touched |
| 8. `learn_windows_integration` test suite | Skip-documented | No Windows learn surface touched |

## Plan-Specific Verification

| Check | Result |
|-------|--------|
| PSV-1: No `Upstream-commit:` trailer in Plan 35-03 commits | PASS (0 found) |
| PSV-2: `test_policy_show_json_no_rust_debug_syntax` passes | PASS (1 passed) |
| PSV-2: `test_policy_diff_json_no_rust_debug_syntax` passes | PASS (1 passed) |
| PSV-3: `test_query_path_denied` passes on Windows | PASS (1 passed) |
| PSV-4: `cmd_diff` body stdout printers preserved | PASS (diff_scalar_option still present) |
| PSV-5: Phase 35 closure section appended | PASS (grep count = 1) |
| PSV-6: No `*_windows.rs` edits | PASS (0 files) |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] test_query_path_reports_near_miss_with_source_and_fix asserting buggy UNC form**
- **Found during:** Task 1, running full query_ext test suite after production fix
- **Issue:** Test asserted `Some("--write-file \\?\C:\...")` (UNC-prefixed buggy output) — the production fix made the test fail in the other direction
- **Fix:** Updated expected_flag to apply `strip_verbatim_prefix` to `test_file_canon`, matching what production code now emits
- **Files modified:** `crates/nono-cli/src/query_ext.rs`
- **Commit:** `d8cb250b` (included with Task 1 fix)

**2. [Rule 1 - Bug] test_query_path_denied asserting POSIX path string on cross-platform code**
- **Found during:** Task 1, after initial fix
- **Issue:** Test asserted `Some("--read /some/random")` (POSIX literal) which fails on Windows where `/some/random/path` canonicalizes to a Windows drive-rooted path
- **Fix:** Rewrote assertion to compute expected_flag via `strip_verbatim_prefix(&canonical)` + `suggested_flag_parts` — same helpers as production code, making the test platform-agnostic without a `#[cfg]` gate
- **Files modified:** `crates/nono-cli/src/query_ext.rs`
- **Commit:** `d8cb250b` (included with Task 1 fix)

## Ticket Closures

| Ticket | Status | Plan | Commit |
|--------|--------|------|--------|
| P34-DEFER-01-1 | closed-by-Phase-35-03 | 35-03 Task 1 | `d8cb250b` |
| P34-DEFER-09-3 | closed-by-Phase-35-03 (transitive) | 35-03 Task 1 | `d8cb250b` |
| P34-DEFER-10-1 | closed-by-Phase-35-03 | 35-03 Task 2 | `66d7a386` |

See `.planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/deferred-items.md § Phase 35 closure` for the consolidated cross-plan ledger (D-35-D4).

## Self-Check

Files created/modified:
- `crates/nono-cli/src/query_ext.rs` — exists
- `crates/nono-cli/src/profile_cmd.rs` — exists
- `.planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/deferred-items.md` — exists

Commits verified:
- `d8cb250b` — fix(35-03): strip UNC verbatim prefix in query_path suggested_flag emission
- `66d7a386` — fix(35-03): replace format!("{:?}") JSON-emission with serde_json::Map insertion
- `a444558c` — docs(35-03): append Phase 35 closure to Phase 34 deferred-items ledger

## Self-Check: PASSED
