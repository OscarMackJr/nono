---
phase: 88-feature-dependency-cherry-pick-wave
plan: "06"
subsystem: nono-ffi
tags: [fork-divergence, ffi, cr-01, deps-02, dep-bump, typify, divergence-ledger, partial-ci-closeout]
dependency_graph:
  requires: [88-05]
  provides: [CR-01-ffi-clear-on-entry, DEPS-02-dep-bumps, phase-88-complete]
  affects: [bindings/c, Cargo.lock]
tech_stack:
  added: []
  patterns:
    - "clear_last_call_state() pub(crate) helper resets all three FFI thread-locals atomically (CR-01)"
    - "clear_last_call_state() called at entry of every pub unsafe extern C fn that can set thread-locals (D-10 systematic fix)"
    - "typify 0.7 spec edit + 14 lockfile-only bumps in one atomic DEPS-02 commit (D-05)"
    - "D-06 path-dep pin gate: all 4 internal path-dep version pins verified at 0.62.2 pre- and post-DEPS-02"
key_files:
  created: []
  modified:
    - bindings/c/src/lib.rs
    - bindings/c/src/diagnostic.rs
    - bindings/c/src/capability_set.rs
    - bindings/c/src/sandbox.rs
    - bindings/c/src/state.rs
    - bindings/c/src/query.rs
    - bindings/c/src/fs_capability.rs
    - crates/nono/Cargo.toml
    - Cargo.lock
    - .planning/phases/85-upst9-divergence-audit/85-DIVERGENCE-LEDGER.md
    - .planning/phases/88-feature-dependency-cherry-pick-wave/88-PARTIAL-CI.md
    - .planning/ROADMAP.md
decisions:
  - "CR-01 committed as deliberate fork-divergence (NOT cherry-pick) per D-11 — clear_last_call_state() applied systematically to all 16 pub unsafe extern C entry points across all 6 bindings/c/src/ files, not just the two diagnostic.rs entry points (D-10 full-surface fix)"
  - "DEPS-02 committed as one atomic commit: typify 0.6->0.7 Cargo.toml spec edit + all 14 lockfile bumps together (D-05)"
  - "x509-parser not present in dependency graph (x509-cert already at latest); no Cargo.toml or Cargo.lock change needed for that dep"
  - "which bumped to 8.0.4 (not 8.0.3 as RESEARCH.md projected — latest compatible at time of execution)"
  - "fmt fix for diagnostic.rs test committed as separate style commit (rustfmt single-line unsafe preference)"
metrics:
  duration: "~11 minutes"
  completed: "2026-06-20"
  tasks_completed: 3
  files_changed: 12
---

# Phase 88 Plan 06: CR-01 FFI Fix + DEPS-02 Dep Bumps + Phase 88 Closeout Summary

CR-01 fork-divergence fix adding clear_last_call_state() at all 16 FFI entry points, DEPS-02 atomic dep bump (typify 0.6->0.7 + 14 lockfile bumps), DIVERGENCE-LEDGER Phase 88 addendum, PARTIAL->CI closeout summary, and ROADMAP Phase 88 completion.

## What Was Built

**CR-01 FFI clear-on-entry fix (deliberate fork-divergence, D-10/D-11):**

1. `bindings/c/src/lib.rs` — new `pub(crate) fn clear_last_call_state()` helper (line 89) that resets all three thread-locals (`LAST_ERROR`, `LAST_DIAGNOSTIC_CODE`, `LAST_REMEDIATION_JSON`) atomically. Mirrors `nono_clear_error()` but is internal (no `#[unsafe(no_mangle)]`).

2. `bindings/c/src/diagnostic.rs` — `crate::clear_last_call_state()` added at entry of:
   - `nono_session_diagnostic_report_to_json()` (line 43)
   - `nono_merge_diagnostic_report_json()` (line 97)
   - Regression test `diagnostic_code_is_cleared_between_calls` (line 214) verifies the stale LAST_DIAGNOSTIC_CODE from a prior `map_error()` call is reset to `Other` when `nono_merge_diagnostic_report_json(null, null)` is called next.

3. All other `pub unsafe extern "C"` entry points that can set thread-locals received `clear_last_call_state()` at entry (D-10 systematic fix):
   - `capability_set.rs`: 8 functions (allow_path, allow_file, set_network_blocked, set_network_mode, set_proxy_port, allow_command, block_command, add_platform_rule)
   - `sandbox.rs`: nono_sandbox_apply
   - `state.rs`: nono_sandbox_state_to_json, nono_sandbox_state_from_json, nono_sandbox_state_to_caps
   - `query.rs`: nono_query_context_query_path, nono_query_context_query_network
   - `fs_capability.rs`: nono_capability_set_fs_access, nono_capability_set_fs_source_tag

4. `85-DIVERGENCE-LEDGER.md` — Phase 88 CR-01 addendum added after the Phase 87 CR-02 addendum, mirroring the table format exactly. Records commit `db0f221d`, files, reason, fork behavior, and future sync note.

**DEPS-02 atomic dep bump:**

`crates/nono/Cargo.toml:71` — `typify = "0.6"` changed to `typify = "0.7"`. One atomic commit includes both the spec edit and all Cargo.lock updates:
- typify 0.6.2 -> 0.7.0 (Cargo.toml spec edit)
- typify-impl/typify-macro 0.6.2 -> 0.7.0 (transitive)
- cbindgen 0.29.2 -> 0.29.4
- hyper 1.9.0 -> 1.10.1 (v1 slot; v0.14.x legacy slot unchanged)
- h2 0.4.13 -> 0.4.15 (transitive of hyper)
- zeroize 1.8.2 -> 1.9.0 + zeroize_derive 1.4.3 -> 1.5.0
- time 0.3.47 -> 0.3.49 + time-core 0.1.8 -> 0.1.9 + num-conv 0.2.1 -> 0.2.2
- chrono 0.4.44 -> 0.4.45
- ignore 0.4.25 -> 0.4.26
- which 8.0.2 -> 8.0.4
- serde_json 1.0.149 -> 1.0.150 (transitive)

`cargo build -p nono` verified: typify 0.7 codegen is non-breaking for the capability-manifest schema.

**D-06 path-dep pin gate (PASS):**

Pre- and post-DEPS-02 verification confirms all 4 internal path-dep version pins at `0.62.2`:
- `crates/nono-cli/Cargo.toml`: nono = "0.62.2", nono-proxy = "0.62.2"
- `crates/nono-proxy/Cargo.toml`: nono = "0.62.2"
- `bindings/c/Cargo.toml`: nono = "0.62.2"

**make ci / Windows-host gate:**

- `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used`: PASS (0 warnings/errors)
- `cargo fmt --all -- --check`: PASS
- `cargo test -p nono-ffi`: 48 passed, 0 failed
- `cargo test -p nono`: 785 passed, 1 failed (pre-existing: `try_set_mandatory_label_surfaces_directive_when_user_owned_apply_fails`)
- `cargo test -p nono-cli`: 1346 passed, 4 failed (all 4 pre-existing Windows baseline failures per `nono_cli_windows_baseline_test_failures` memory note)

**PARTIAL->CI closeout:**

`88-PARTIAL-CI.md` finalized with a Summary table listing all 15 deferrals across Plans 88-01 through 88-05. Plan 88-06 has no new PARTIAL->CI deferrals (CR-01 files have no cfg-gated Unix blocks; DEPS-02 is Cargo files only).

**ROADMAP update:**

88-06-PLAN.md marked `[x]` complete. Phase 88 top-level checkbox marked `[x]` with completion date 2026-06-20.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] rustfmt style: unsafe block line width in diagnostic.rs test**
- **Found during:** Task 3 (`cargo fmt --all -- --check` gate)
- **Issue:** The multi-line unsafe block in `diagnostic_code_is_cleared_between_calls` did not match rustfmt's single-line preference (it fits on one line)
- **Fix:** Reformatted to single-line `let json_ptr = unsafe { ... };` form
- **Files modified:** `bindings/c/src/diagnostic.rs`
- **Commit:** `4652a3db`

**2. [Claude discretion] Systematic D-10 fix across all 6 bindings/c/src/ files (beyond the 2 diagnostic.rs entry points)**
- **Found during:** Task 1 analysis
- **Issue:** The plan focused on the two diagnostic.rs entry points; D-10 decision specifies "ALL pub unsafe extern C fns that can set thread-locals"
- **Fix:** Applied `clear_last_call_state()` to 16 entry points total (14 more than the 2 minimum) across capability_set.rs, fs_capability.rs, sandbox.rs, state.rs, query.rs
- **Files modified:** `bindings/c/src/capability_set.rs`, `bindings/c/src/fs_capability.rs`, `bindings/c/src/sandbox.rs`, `bindings/c/src/state.rs`, `bindings/c/src/query.rs`
- **Commit:** `db0f221d`

**3. [Observation] x509-parser not in dependency graph**
- **Found during:** Task 2 DEPS-02 cargo update
- **Issue:** `cargo update -p x509-parser` failed with "package ID specification did not match any packages". The actual dependency is `x509-cert` (not `x509-parser`); `x509-cert` is already at its latest compatible version.
- **Fix:** Skipped — x509-cert is already up to date; no action needed
- **Deviation impact:** None; the RESEARCH.md projected x509-parser which doesn't exist in this workspace's dependency graph

**4. [Observation] which bumped to 8.0.4 not 8.0.3**
- **Found during:** Task 2 DEPS-02 cargo update
- **Issue:** `cargo update -p which` resolved to 8.0.4 (not 8.0.3 as RESEARCH.md projected)
- **Fix:** Accepted — 8.0.4 is the latest compatible version; no source changes needed

## D-Constraint Verification

| Constraint | Status | Evidence |
|-----------|--------|---------|
| D-10: clear-on-entry across ALL FFI entry points | PASS | 16 entry points across 6 files; grep shows clear_last_call_state at every pub unsafe extern C fn that can set thread-locals |
| D-11: dedicated regression test + standalone fork-divergence commit | PASS | diagnostic_code_is_cleared_between_calls test passes; commit db0f221d has NO "(cherry picked from ..." line; DIVERGENCE-LEDGER addendum at cc52e1a4 |
| D-05: one atomic DEPS commit for all 9 bumps | PASS | Single commit 4b6de233 contains both Cargo.toml spec edit and all Cargo.lock changes |
| D-06: 5-crate path-dep pin gate | PASS | All 4 internal path-dep pins at 0.62.2 pre- and post-DEPS-02; verified via grep |

## CI Gate Results (Windows Host)

- **cargo clippy:** PASS (0 warnings, 0 errors; workspace + all-targets + -D warnings -D clippy::unwrap_used)
- **cargo fmt:** PASS (--check)
- **cargo test -p nono-ffi:** 48 passed, 0 failed
- **cargo test -p nono:** 785 passed, 1 failed (pre-existing Windows baseline)
- **cargo test -p nono-cli:** 1346 passed, 4 failed (pre-existing Windows baseline)
- **PARTIAL->CI:** No new deferrals in Plan 88-06; 15 total deferrals across Phase 88 documented in 88-PARTIAL-CI.md

## Task Commits

1. **Task 1a: CR-01 FFI clear-on-entry fix (all 16 entry points + regression test)** - `db0f221d`
2. **Task 1b: DIVERGENCE-LEDGER Phase 88 CR-01 addendum** - `cc52e1a4`
3. **Task 2: DEPS-02 atomic dep bump (typify 0.7 + 14 lockfile bumps)** - `4b6de233`
4. **Task 3a: rustfmt fix for diagnostic.rs test** - `4652a3db`
5. **Task 3b: PARTIAL-CI.md closeout + ROADMAP update** - `8afe0253`

## Known Stubs

None.

## Threat Flags

None. CR-01 fix introduces no new network endpoints, auth paths, file access patterns, or schema changes at trust boundaries. DEPS-02 is minor/patch bumps of existing approved packages.

## Self-Check: PASSED

Files exist:
- `bindings/c/src/lib.rs` — clear_last_call_state() at line 89
- `bindings/c/src/diagnostic.rs` — clear_last_call_state() at entry of both pub unsafe extern C fns; diagnostic_code_is_cleared_between_calls test at line 214
- `crates/nono/Cargo.toml` — typify = "0.7"
- `Cargo.lock` — typify 0.7.0 present
- `.planning/phases/85-upst9-divergence-audit/85-DIVERGENCE-LEDGER.md` — Phase 88 CR-01 addendum at line 832
- `.planning/phases/88-feature-dependency-cherry-pick-wave/88-PARTIAL-CI.md` — Summary section present
- `.planning/ROADMAP.md` — 88-06-PLAN.md marked [x]

Commits exist:
- `db0f221d` — fix(ffi): clear stale diagnostic state on every FFI entry (CR-01)
- `cc52e1a4` — docs(ledger): add Phase 88 CR-01 fork-divergence addendum
- `4b6de233` — chore(deps): absorb 9 dep bumps from v0.62..v0.64 window (DEPS-02)
- `4652a3db` — style(ffi): apply rustfmt to diagnostic.rs test (CR-01 fix)
- `8afe0253` — docs(88): finalize PARTIAL->CI deferral record + update ROADMAP plan list
