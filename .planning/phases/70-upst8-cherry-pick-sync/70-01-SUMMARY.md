---
phase: 70-upst8-cherry-pick-sync
plan: "01"
subsystem: upst-sync
tags: [cherry-pick, diagnostic, profile, suppress-system-services, registry-ref, upstream-sync]

# Dependency graph
requires:
  - phase: 69-upst8-prep
    provides: "DIVERGENCE-LEDGER.md with C3 commit list + v0.62.0 gap analysis"
provides:
  - "diagnostics.suppress_system_services profile field (cc21229f / C3)"
  - "DiagnosticFormatter.with_suppressed_system_service_operations() builder method"
  - "suppressed_system_service_operations field on PreparedSandbox / ExecConfig / ExecutionFlags / PreparedProfile (C2 prerequisite)"
  - "Registry ref preservation in profile extends (20cc5df9 / C3)"
  - "UPST8-01 acceptance language updated to v0.62.0 upper bound"
affects: [70-02, 70-03]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "D-19 trailer format: Upstream-commit + Upstream-tag + Upstream-author + Co-Authored-By + two Signed-off-by lines"
    - "Inline cfg-gated adaptation replacing upstream helpers absent from fork (collect_ignored_denial_paths)"

key-files:
  created: []
  modified:
    - crates/nono/src/diagnostic.rs
    - crates/nono-cli/src/sandbox_prepare.rs
    - crates/nono-cli/src/exec_strategy.rs
    - crates/nono-cli/src/execution_runtime.rs
    - crates/nono-cli/src/launch_runtime.rs
    - crates/nono-cli/src/command_runtime.rs
    - crates/nono-cli/src/profile_runtime.rs
    - crates/nono-cli/src/profile_save_runtime.rs
    - crates/nono-cli/src/proxy_runtime.rs
    - crates/nono-cli/src/main.rs
    - crates/nono-cli/data/nono-profile.schema.json
    - .planning/REQUIREMENTS.md
    - .planning/ROADMAP.md

key-decisions:
  - "D-70-01: UPST8-01 acceptance criteria extended to v0.62.0 upper bound per Phase 69 DIVERGENCE-LEDGER D-01"
  - "cc21229f conflict resolution: took HEAD side for diagnostic.rs builder method absence (fork already had methods later in file); added missing with_suppressed_system_service_operations() builder"
  - "collect_ignored_denial_paths adapted inline with cfg-gate: upstream helper absent from fork; Windows path returns Vec::new() (UX-only field, no security impact)"
  - "20cc5df9 sandbox_state.rs conflict: HEAD side taken for domain_endpoint_state_tests module (Phase 56 addition not in upstream); profile_save_runtime.rs registry-ref feature auto-merged cleanly"
  - "D-70-E1 invariant honored: zero _windows.rs / exec_strategy_windows/ / nono-shell-broker/ files touched"

patterns-established:
  - "PreparedSandbox pipeline: suppressed_system_service_operations flows PreparedProfile -> PreparedSandbox -> ExecutionFlags/ExecConfig"

requirements-completed: [UPST8-02]

# Metrics
duration: 90min
completed: 2026-06-12
---

# Phase 70 Plan 01: Profile Diagnostic Features Summary

**Cherry-picked cc21229f (suppress_system_services) + 20cc5df9 (registry-ref extends) from upstream v0.61.0 into fork; suppression-only diagnostic filtering wired end-to-end through PreparedSandbox pipeline**

## Performance

- **Duration:** ~90 min
- **Started:** 2026-06-12T21:00:00Z
- **Completed:** 2026-06-12T23:30:00Z
- **Tasks:** 2 (Task 1: D-70-01 docs amendment; Task 2: two cherry-picks)
- **Files modified:** 15

## Accomplishments

- Task 1: Updated UPST8-01 acceptance criteria (REQUIREMENTS.md + ROADMAP.md) to reflect v0.62.0 upper bound as specified in D-70-01
- Task 2a: Cherry-picked cc21229f — `diagnostics.suppress_system_services` profile field, suppression-only diagnostic filtering through DiagnosticFormatter, `suppressed_system_service_operations` field wired through PreparedProfile -> PreparedSandbox -> ExecutionFlags -> ExecConfig, prerequisite for Plan 70-03 C2
- Task 2b: Cherry-picked 20cc5df9 — registry references preserved in profile extends when saving (e.g. `always-further/claude@1.2.0` no longer silently dropped)
- Both cherry-picks carry verbatim D-19 trailers; zero Windows-only files touched (D-70-E1)

## Task Commits

Each task was committed atomically:

1. **Task 1: D-70-01 docs amendment** - `8f0e54e1` (docs)
2. **Task 2 C3a: cc21229f cherry-pick** - `e80a7c45` (feat)
3. **Task 2 C3b: 20cc5df9 cherry-pick** - `497101ae` (feat)

## Files Created/Modified

- `crates/nono/src/diagnostic.rs` - Added `with_suppressed_system_service_operations()` builder method; struct field `suppressed_system_service_operations` was already present in fork
- `crates/nono-cli/src/sandbox_prepare.rs` - Added `ignored_denial_paths: Vec<PathBuf>` and `suppressed_system_service_operations: Vec<String>` to `PreparedSandbox` struct
- `crates/nono-cli/src/exec_strategy.rs` - Added fields to `ExecConfig`; updated `should_offer_profile_save` signature with `sandbox_violations` param
- `crates/nono-cli/src/execution_runtime.rs` - Removed duplicate `ignored_denial_paths` local computation; now uses `flags.ignored_denial_paths` from PreparedSandbox pipeline
- `crates/nono-cli/src/launch_runtime.rs` - Added fields to `ExecutionFlags` struct and `defaults()` init
- `crates/nono-cli/src/command_runtime.rs` - Added new fields to PreparedSandbox struct literals in run_shell/run_wrap
- `crates/nono-cli/src/profile_runtime.rs` - Added `ignored_denial_paths` (cfg-gated inline adaptation) and `suppressed_system_service_operations` to PreparedProfile init
- `crates/nono-cli/src/profile_save_runtime.rs` - Registry ref preservation in `prepare_profile_save_from_patch` (auto-merged from 20cc5df9)
- `crates/nono-cli/src/proxy_runtime.rs` - Added missing `ignored_denial_paths` and `suppressed_system_service_operations` to PreparedSandbox literal
- `crates/nono-cli/src/main.rs` - Added new fields to test fixture PreparedSandbox literals
- `crates/nono-cli/data/nono-profile.schema.json` - diagnostics.suppress_system_services schema field
- `.planning/REQUIREMENTS.md` - UPST8-01 updated to v0.62.0 upper bound
- `.planning/ROADMAP.md` - Phase 70 SC #1 updated with D-70-01 amendment note

## Decisions Made

- **collect_ignored_denial_paths adaptation**: The upstream helper function `collect_ignored_denial_paths` and the corresponding `SandboxArgs::suppress_save_prompt` CLI field are not yet in the fork. Resolution: inlined the profile-only path in `profile_runtime.rs` using the existing `canonicalize_suppress_path` helper, cfg-gated to non-Windows (Windows returns `Vec::new()` as this field only affects UX diagnostics, not enforcement).
- **with_suppressed_system_service_operations() missing**: The fork's conflict resolution took HEAD side for `diagnostic.rs` (fork already had the struct field and usage at line 1181, just not the builder method). Added the missing builder method to complete the API surface.
- **sandbox_state.rs conflict**: The fork's Phase 56 `domain_endpoint_state_tests` module didn't exist in upstream; git placed the upstream `cap_file_validation_tests` content at the fork module boundary. Resolution: HEAD side taken for all three conflict hunks, preserving fork's module structure. `profile_save_runtime.rs` (the core feature) auto-merged cleanly.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Missing `with_suppressed_system_service_operations()` builder method in DiagnosticFormatter**
- **Found during:** Task 2, C3a cherry-pick (cc21229f) — compile error after conflict resolution
- **Issue:** The fork's conflict resolution took HEAD side for `diagnostic.rs` (fork already had the struct field and method usage). But the builder method itself was absent, causing E0599 on test code that called `.with_suppressed_system_service_operations(&suppressed)`
- **Fix:** Added `pub fn with_suppressed_system_service_operations(mut self, ops: &'a [String]) -> Self` builder method to DiagnosticFormatter
- **Files modified:** `crates/nono/src/diagnostic.rs`
- **Verification:** `cargo build -p nono` and `cargo test -p nono` pass
- **Committed in:** `e80a7c45` (part of C3a task commit)

**2. [Rule 3 - Blocking] Duplicate `ignored_denial_paths` field in ExecConfig struct literal (execution_runtime.rs)**
- **Found during:** Task 2, C3a cherry-pick — identified pre-commit as duplicate struct field compile error
- **Issue:** Cherry-pick inserted `ignored_denial_paths: &flags.ignored_denial_paths` at line 457 while the fork already had `ignored_denial_paths: &ignored_denial_paths` (local variable) at line 474
- **Fix:** Removed the local `ignored_denial_paths` computation block and the duplicate struct field entry; now uses `flags.ignored_denial_paths` from PreparedSandbox pipeline
- **Files modified:** `crates/nono-cli/src/execution_runtime.rs`
- **Verification:** `cargo build -p nono-cli` passes
- **Committed in:** `e80a7c45` (part of C3a task commit)

**3. [Rule 3 - Blocking] Missing `ignored_denial_paths`/`suppressed_system_service_operations` in proxy_runtime.rs PreparedSandbox literal**
- **Found during:** Task 2, C3a — compile error during `cargo test -p nono-cli`
- **Issue:** `proxy_runtime.rs` had a PreparedSandbox struct literal missing the two new required fields
- **Fix:** Added `ignored_denial_paths: Vec::new()` and `suppressed_system_service_operations: Vec::new()` to the literal
- **Files modified:** `crates/nono-cli/src/proxy_runtime.rs`
- **Verification:** `cargo test -p nono-cli` reaches the same pre-existing baseline failures (no new failures)
- **Committed in:** `e80a7c45` (part of C3a task commit)

**4. [Rule 1 - Bug] `collect_ignored_denial_paths` upstream helper absent from fork**
- **Found during:** Task 2, C3a — compile error E0425 in profile_runtime.rs
- **Issue:** Upstream commit references `collect_ignored_denial_paths(args, &args.suppress_save_prompt, workdir)` where `SandboxArgs::suppress_save_prompt` is also absent from fork
- **Fix:** Replaced the function call with an inline expression using existing `profile_save_runtime::canonicalize_suppress_path`, cfg-gated to non-Windows (Windows: `Vec::new()`). No security impact — this field only affects diagnostic UX, not sandbox enforcement.
- **Files modified:** `crates/nono-cli/src/profile_runtime.rs`
- **Verification:** `cargo build -p nono-cli` passes on Windows host
- **Committed in:** `e80a7c45` (part of C3a task commit)

---

**Total deviations:** 4 auto-fixed (2 Rule 1 bugs, 2 Rule 3 blocking)
**Impact on plan:** All auto-fixes necessary for the cherry-pick adaptations. The upstream commits were authored against intermediate commits the fork lacks (ProfileSaveOffer struct, collect_ignored_denial_paths helper, SandboxArgs::suppress_save_prompt). No scope creep.

## Issues Encountered

- **8-file conflict for cc21229f**: The upstream commit was authored against a significantly different baseline that included `ProfileSaveOffer` struct, `has_saveable_system_service_rules`, and `collect_ignored_denial_paths`. The fork is missing these intermediate upstream commits (they arrive in Plans 70-02 and 70-03). Resolution required adapting intent without the missing APIs.
- **Cross-target clippy**: PARTIAL — Windows host cannot cross-compile to Linux/macOS. The cfg-gated Unix code (diagnostic.rs, exec_strategy.rs, execution_runtime.rs, profile_runtime.rs) must be verified in CI against `x86_64-unknown-linux-gnu` and `x86_64-apple-darwin` targets.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Plan 70-02 (C2 cherry-pick cluster: bd4c469a) can proceed — its prerequisite `suppressed_system_service_operations` field on PreparedSandbox is now wired
- Cross-target clippy verification deferred to live CI per CLAUDE.md PARTIAL policy

---

## Known Stubs

None — all new fields are fully wired through the data pipeline. The `ignored_denial_paths` on Windows returns `Vec::new()` by design (cfg-gated; this is the correct cross-platform behavior, not a stub).

## Threat Flags

None — suppression only affects diagnostic reporting. The sandbox continues to enforce all denials; `suppressed_system_service_operations` never reaches the Landlock or Seatbelt enforcement path.

## Self-Check: PASSED

- FOUND: crates/nono/src/diagnostic.rs
- FOUND: crates/nono-cli/src/sandbox_prepare.rs
- FOUND: crates/nono-cli/src/profile_save_runtime.rs
- FOUND: .planning/REQUIREMENTS.md
- FOUND commit: 8f0e54e1 (Task 1)
- FOUND commit: e80a7c45 (Task 2 C3a)
- FOUND commit: 497101ae (Task 2 C3b)

---
*Phase: 70-upst8-cherry-pick-sync*
*Completed: 2026-06-12*
