---
phase: 55-upst7-cherry-pick-wave
plan: "03"
subsystem: pack-update-hints
tags: [upstream-sync, pack-hints, detached-process, atomic-write, nono-cli]

requires:
  - phase: 55-02-PROFILE-JSONC-TARGET-BINARY
    provides: cli.rs, startup_runtime.rs, main.rs in post-C7 state (dependency order)

provides:
  - "pack-update-hint refresh via detached child process (std::process::Command, not thread)"
  - "NONO_NO_PACK_UPDATE_HINTS env var for independent pack-hint opt-out"
  - "pack-update-hint-helper hidden subcommand with recursion guard"
  - "Atomic state file writes with pid-scoped temp name (concurrent-helper safe)"

affects:
  - 55-05-TIMEOUT-CONSTANTS
  - 55-WAVE4

tech-stack:
  added: []
  patterns:
    - "Detached-process refresh (std::process::Command) instead of background thread for pre-fork safety"
    - "Pid-scoped atomic temp file write (.{name}.{pid}.tmp + rename + cleanup on error)"

key-files:
  created: []
  modified:
    - crates/nono-cli/src/pack_update_hint.rs
    - crates/nono-cli/src/cli.rs
    - crates/nono-cli/src/cli_bootstrap.rs
    - crates/nono-cli/src/main.rs
    - crates/nono-cli/src/app_runtime.rs
    - crates/nono-cli/src/startup_runtime.rs
    - docs/cli/usage/flags.mdx

key-decisions:
  - "D-20 manual replay (not git cherry-pick) used for both C9 commits due to significant fork divergence in pack_update_hint.rs from Phase 44 WR-05"
  - "Phase 44 D-44-B2 option b preserved: no synchronous first-run path; ALL stale entries go to detached process regardless of cache existence"
  - "save_state upgraded to pid-scoped temp name (b1a650a3) — supersedes Phase 44 IN-01 path.with_extension pattern"
  - "docs/cli/features/managing-packs.mdx: not tracked in fork (not in HEAD); docs/cli/usage/flags.mdx updated instead"

patterns-established:
  - "pack-update-hint-helper: hidden subcommand for out-of-process refresh, registered in cli.rs / app_runtime.rs / cli_bootstrap.rs / startup_runtime.rs"

requirements-completed: [REQ-UPST7-02]

duration: 35min
completed: 2026-06-04
---

# Phase 55 Plan 03: PACK-HINT-ROBUSTNESS Summary

**C9 cluster absorbed: pack-update-hint refresh switched from background thread to detached std::process::Command child; state file writes upgraded to pid-scoped atomic temp+rename.**

## Performance

- **Duration:** ~35 min
- **Started:** 2026-06-04T22:35:00Z
- **Completed:** 2026-06-04T23:10:00Z
- **Tasks:** 1 (2 commits)
- **Files modified:** 7

## Accomplishments

- Absorbed upstream commits 74fbbf12 and b1a650a3 (C9, v0.58.0) as D-20 manual replay commits with D-19 trailers
- pack-update-hint now uses a detached `std::process::Command` child process instead of a background thread, preventing thread creation in the supervised parent before fork
- `NONO_NO_PACK_UPDATE_HINTS` env var added for independent pack-hint opt-out (separate from `NONO_NO_UPDATE_CHECK`)
- `pack-update-hint-helper` hidden subcommand wired through cli.rs, app_runtime.rs, cli_bootstrap.rs, startup_runtime.rs, and main.rs (with recursion guard via `allows_pre_exec_update_check`)
- Atomic state file writes upgraded to pid-scoped temp filename for concurrent-helper safety
- 6 new tests (4 pack_update_hint module tests + test_pre_exec_update_check_disabled_for_pack_update_hint_helper already in main.rs tests)

## Task Commits

1. **Task 1a: C9-commit-1 (74fbbf12 D-20 replay)** — `d999abcc` (refactor)
   - pack_update_hint.rs, cli.rs, cli_bootstrap.rs, main.rs, app_runtime.rs, startup_runtime.rs, docs/cli/usage/flags.mdx
2. **Task 1b: C9-commit-2 (b1a650a3 D-20 replay)** — `c4f0c652` (fix)
   - pack_update_hint.rs only (save_state pid-scoped atomic write + debug log)

## Files Created/Modified

- `crates/nono-cli/src/pack_update_hint.rs` — Replaced background thread with `refresh_in_background_process` (std::process::Command); added `run_refresh_helper`, `refresh_synchronous`, `refresh_helper_args`, `parse_refresh_helper_args`; NONO_NO_PACK_UPDATE_HINTS env var; pid-scoped atomic write in save_state; 6 new unit tests
- `crates/nono-cli/src/cli.rs` — Added `PackUpdateHintHelper(PackUpdateHintHelperArgs)` variant + `PackUpdateHintHelperArgs` struct
- `crates/nono-cli/src/cli_bootstrap.rs` — Added `PackUpdateHintHelper(_)` to verbosity=0 list
- `crates/nono-cli/src/startup_runtime.rs` — Added `PackUpdateHintHelper(_)` to `allows_pre_exec_update_check` exclusion
- `crates/nono-cli/src/app_runtime.rs` — Added `PackUpdateHintHelper(args) => pack_update_hint::run_refresh_helper(args)` dispatch
- `crates/nono-cli/src/main.rs` — Added `test_pre_exec_update_check_disabled_for_pack_update_hint_helper` test
- `docs/cli/usage/flags.mdx` — Added `NONO_NO_PACK_UPDATE_HINTS` row to env var table

## C9 Cherry-pick Log

| Commit | Upstream SHA | Trailer verified | Files |
|--------|-------------|-----------------|-------|
| `d999abcc` | 74fbbf12 (Upstream-commit: 74fbbf125cc8b7672f879b395b667f0ba7ccbc84) | yes | 7 |
| `c4f0c652` | b1a650a3 (Upstream-commit: b1a650a3f1ec074af9b4b1b3edbc96185e5a51a6) | yes | 1 |

**Trailer verification:** `git log --format="%B" HEAD~2..HEAD | grep -c "^Upstream-commit:"` = 2 (PASS)

## Platform Spawn Verification (T-55-03-01)

`grep -n "Command\|process::" crates/nono-cli/src/pack_update_hint.rs` shows:
- Line 16: `use std::process::{Command, Stdio};` — cross-platform std spawn
- Line 199: `Command::new(exe)` — standard process spawn

**No `fork()` or `libc::fork` present.** Confirmed via `grep -c "fork()\|libc::fork" pack_update_hint.rs` = 0.

On Windows, `std::process::Command` uses `CreateProcessW` — no elevated token or WFP filter handle inheritance. T-55-03-01: PASS.

## Conflict File Inventory

| File | Conflict | Resolution |
|------|----------|-----------|
| `pack_update_hint.rs` | Major — Phase 44 WR-05 removed synchronous first-run path; upstream still had it | Kept Phase 44 D-44-B2 option b (no synchronous path); took upstream's detached-process structure + NONO_NO_PACK_UPDATE_HINTS + run_refresh_helper |
| `cli.rs` | Minor — upstream inserted PackUpdateHintHelper between OpenUrlHelper and nothing; fork has ClaudeCodeHook between them | Inserted between OpenUrlHelper and ClaudeCodeHook |
| `cli_bootstrap.rs` | Minor — upstream didn't have ClaudeCodeHook | Inserted PackUpdateHintHelper before ClaudeCodeHook |
| `startup_runtime.rs` | Minor — upstream had different exclusion list | Added PackUpdateHintHelper to matches! |
| `app_runtime.rs` | Minor — upstream didn't have ClaudeCodeHook dispatch | Inserted before ClaudeCodeHook |
| `main.rs` | None — test added cleanly | N/A |
| `docs/cli/features/managing-packs.mdx` | Delete/modify — file not tracked in fork | Skipped; updated flags.mdx instead |

**Note on managing-packs.mdx:** This file exists in upstream but is NOT tracked in the fork (confirmed: `git show HEAD:docs/cli/features/managing-packs.mdx` → fatal: not in HEAD). The upstream content of the change was not applied; `flags.mdx` updated with `NONO_NO_PACK_UPDATE_HINTS` row instead.

## D-55-E1 Windows-Invariant Status

**PASS** — `git diff --name-only HEAD~2 HEAD | grep -E "_windows\.rs|exec_strategy_windows|nono-shell-broker"` returns 0 lines.

Zero windows-touch. C9 ledger entry: `windows-touch: no`. Structurally correct.

## D-55-E3 Cross-Target Clippy Status

**PARTIAL** — cross-toolchain unavailable on Windows host.

- `cargo clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used`: SKIPPED — `x86_64-linux-gnu-gcc` not found
- `cargo clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used`: SKIPPED — macOS cc not found
- Windows-host `cargo build -p nono-cli`: PASS (clean, no errors)

**Assessment:** New code in `pack_update_hint.rs` has ZERO cfg-gated Unix blocks — only uses `std::process::Command` and `std::process::id()` (both cross-platform). The cfg-gated Unix code in cli.rs, main.rs, and startup_runtime.rs is pre-existing and was not modified by this plan.

**Disposition:** `skipped_gates_environmental` — same as Phases 36-01a/b/c, 43, 48, 55-02. Deferred to live CI per `.planning/templates/cross-target-verify-checklist.md`.

## D-55-E4 Baseline-Aware CI Gate

**Phase 54 baseline:** `fd28ed55` (includes Phase 55 waves 1+2).

`cargo test -p nono-cli` result:

| Test result | Count | Category |
|-------------|-------|----------|
| Passing | 1140 | +4 vs 55-02 baseline (new pack_update_hint tests) |
| Failing | 6 | red→red carry-forward (pre-existing, not introduced by C9) |

Pre-existing failures (all 6 documented in 55-02-SUMMARY):
- `exec_strategy::launch::broker_dispatch_tests::broker_launch_assigns_child_to_job_object`
- `exec_strategy::launch::write_deny_low_il_broker_no_pty_tests::write_deny_low_il_broker_no_pty_prevents_child_write_to_medium_il_file`
- `profile_cmd::tests::test_init_allowed_when_pack_has_same_short_name`
- `protected_paths::tests::blocks_child_directory_capability`
- `protected_paths::tests::blocks_parent_directory_capability`
- `protected_paths::tests::requested_path_blocks_nonexistent_child_under_protected_root`

**No green→red transitions** — D-55-E4 PASS.

## Decisions Made

1. **D-20 manual replay** — Both C9 commits applied as manual replay (not git cherry-pick) because pack_update_hint.rs diverged significantly from upstream via Phase 44 WR-05 changes. The cherry-pick attempt produced 5 conflict markers; manual resolution produced cleaner, well-annotated code.

2. **Phase 44 D-44-B2 preserved** — Upstream's 74fbbf12 included a `cache_existed` variable + synchronous first-run fallback (restoring behavior Phase 44 explicitly removed). Fork preserves the Phase 44 decision: all stale entries go to the detached process; no synchronous first-run path.

3. **save_state upgrade to pid-scoped temp** — Phase 44's `path.with_extension("json.tmp")` was safe for single-process use but could collide if multiple helper processes ran simultaneously. Upstream's b1a650a3 approach (`.{name}.{pid}.tmp`) is strictly better; adopted.

4. **docs/cli/features/managing-packs.mdx skipped** — File not tracked in fork (not in HEAD). The `flags.mdx` update covers the user-visible `NONO_NO_PACK_UPDATE_HINTS` documentation.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 — Missing Critical] D-20 manual replay instead of cherry-pick**

- **Found during:** Task 1 (cherry-pick attempt)
- **Issue:** `git cherry-pick 74fbbf12` produced 5 conflict markers in pack_update_hint.rs because the fork's Phase 44 WR-05 changes (thread-based background refresh, atomic write) diverged significantly from upstream's state. The upstream commit also reintroduced the synchronous first-run path that Phase 44 explicitly removed.
- **Fix:** Applied changes as D-20 manual replay with `Upstream-commit:` trailers; preserved Phase 44 decisions; annotated all fork-divergence points.
- **Files modified:** All 7 C9 files
- **Verification:** Build clean, 6 new tests pass, trailer count = 2
- **Committed in:** d999abcc + c4f0c652

---

**Total deviations:** 1 (D-20 replay due to Phase 44 fork divergence)
**Impact on plan:** D-20 is the documented procedure for this type of divergence (per 55-CONTEXT D-55-02 and upstream-sync-quick.md). No scope creep.

## Issues Encountered

- Cherry-pick conflict on `docs/cli/features/managing-packs.mdx` (DU — deleted in HEAD, modified upstream): file is not tracked in fork. Resolved by `git rm` to clean the working tree; `flags.mdx` covers the user-visible documentation.

## Known Stubs

None — no placeholder values, hardcoded empty values, or TODO/FIXME patterns introduced by this plan.

## Threat Surface Scan

No new network endpoints, auth paths, or trust boundary schema changes introduced. The detached child process (`std::process::Command`) inherits no sandbox token or WFP filter handles (T-55-03-01). No new cargo dependencies. T-55-03-SC: ACCEPT (no new deps).

## D-55-03 Held-Branch Status

**Feature branch NOT merged to main** — per D-55-03 (hold Phase 55 off main until v0.58.0 tagged + signed). Commits `d999abcc` and `c4f0c652` are on `worktree-agent-a93ee40425a4206ed` branch.

## Next Phase Readiness

- C9 (pack-hint robustness) complete; held on feature branch per D-55-03
- Wave 3 parallel plan: 55-04 (C10, diagnostic polish) — separate executor
- Wave 4 plan: 55-05 (C11, timeout constants) depends on Wave 3 completion; touches startup_runtime.rs + cli.rs (ensure no conflict with C9 changes before wave 4 runs)
- startup_runtime.rs change: only `allows_pre_exec_update_check` modified (PackUpdateHintHelper added to exclusion). C11 plan 55-05 touches same file — confirm no conflict at merge.

## Self-Check: PASSED

Files verified:
- `crates/nono-cli/src/pack_update_hint.rs` — FOUND
- `crates/nono-cli/src/cli.rs` — FOUND
- `crates/nono-cli/src/app_runtime.rs` — FOUND
- `crates/nono-cli/src/startup_runtime.rs` — FOUND

Commits verified:
- `d999abcc` — FOUND (`git log --oneline` confirms)
- `c4f0c652` — FOUND (`git log --oneline` confirms)

---
*Phase: 55-upst7-cherry-pick-wave*
*Plan: 03 — PACK-HINT-ROBUSTNESS*
*Completed: 2026-06-04*
