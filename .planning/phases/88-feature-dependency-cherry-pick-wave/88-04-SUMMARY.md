---
phase: 88-feature-dependency-cherry-pick-wave
plan: "04"
subsystem: nono-cli
tags: [cherry-pick, upstream-sync, pty, update-check, profile-namespace, aliases, feat, deps]
dependency_graph:
  requires: [88-03]
  provides: [DEPS-01-pty-ctrl-z, FEAT-06a-ci-provider, FEAT-06b-profile-namespace, D-07-aliases, D-08-fork-profile-namespace]
  affects: [88-05-env-clear-removal, 88-06-m-cluster-misc]
tech_stack:
  added: []
  patterns:
    - "profile_aliases map in policy.json: namespace-form -> bare-name, one-hop resolution only (T-88-14 mitigated)"
    - "get_policy_profile() alias fallback: canonical lookup first, alias map second, no chaining"
    - "detect_ci_provider() pure env-var lookup returning Option<&'static str>"
key_files:
  created: []
  modified:
    - crates/nono-cli/src/exec_strategy.rs
    - crates/nono-cli/src/pty_proxy.rs
    - crates/nono-cli/src/update_check.rs
    - crates/nono-cli/src/cli.rs
    - crates/nono-cli/data/policy.json
    - crates/nono-cli/src/profile/builtin.rs
    - crates/nono-cli/src/profile/mod.rs
    - crates/nono-cli/src/policy.rs
    - crates/nono-cli/src/setup.rs
    - crates/nono-cli/README.md
    - .planning/phases/88-feature-dependency-cherry-pick-wave/88-PARTIAL-CI.md
key-decisions:
  - "D-07 implemented: always-further/claude -> claude-code alias in policy.json; bare name claude-code remains canonical internal key"
  - "D-08 implemented: swival/default, nono-ts/wfp-test-open, nono-ts/wfp-test-blocked, nono-ts/default aliases for fork-only profiles"
  - "exec_strategy.rs module-level cfg gate (main.rs line 34) is sufficient for DEPS-01 nix:: symbols - no per-function guards needed"
  - "DEPS-01 conflict resolution: pty_proxy.rs had suspension_requested+master_fd already; resolved by taking upstream's 4 new methods"
  - "6d88638e conflict resolution: fork deletions of migration.rs, pull_ui.rs, claude-clear-nono.sh, codex-clear-nono.sh preserved via git rm"
requirements-completed:
  - DEPS-01
  - FEAT-06

duration: "~1.5 hours"
completed: "2026-06-20"
---

# Phase 88 Plan 04: DEPS-01 (PTY ctrl-z) + FEAT-06a (CI discovery) + FEAT-06b (Profile namespace + D-07/D-08 aliases) Summary

**PTY ctrl-z hang fix (4179ce03), CI provider detection (cc11b389), and profile namespace rename (6d88638e) absorbed with fork-specific D-07/D-08 alias map wiring (`always-further/claude` -> `claude-code`, plus fork-only profiles namespaced consistently).**

## Performance

- **Duration:** ~1.5 hours
- **Started:** 2026-06-20T15:15:00Z
- **Completed:** 2026-06-20T16:45:48Z
- **Tasks:** 4
- **Files modified:** 11

## Accomplishments

- DEPS-01 (`4179ce03`): PTY ctrl-z hang fix landed — `signal_pty_foreground_group()` and `handle_pty_suspension()` in `exec_strategy.rs`, plus `in_alt_screen()`, `leave_screen_for_suspension()`, `reenter_screen_for_resume()`, `shutdown_attach_listener()`, `take_suspension_request()` in `pty_proxy.rs`. Module-level `cfg(not(target_os="windows"))` gate in main.rs provides sufficient isolation.
- FEAT-06a (`cc11b389`): `detect_ci_provider()` in `update_check.rs` — pure env-var lookup, clean apply, 16 tests pass.
- FEAT-06b (`6d88638e`): Profile namespace rename absorbed in docs/CLI examples/comments. Fork-specific D-07/D-08 alias work added: `profile_aliases` in `policy.json`, alias resolution in `policy.rs::get_policy_profile()`, and 6 new alias tests in `builtin.rs`.
- PARTIAL→CI deferral recorded for DEPS-01 (nix:: on Unix-only module path).

## Task Commits

1. **Task 1: Cherry-pick 4179ce03 (DEPS-01 PTY ctrl-z fix)** - `1f4fd335` (fix)
2. **Task 2: Cherry-pick cc11b389 (FEAT-06a CI provider discovery)** - `a4fb72df` (feat)
3. **Task 3: Cherry-pick 6d88638e (FEAT-06b profile namespace) + D-07/D-08 aliases** - `d80b2b19` (refactor)
4. **Chore: cargo fmt pty_proxy.rs** - `8ee56d80` (chore)
5. **Task 4: PARTIAL→CI record** - `4ade9de1` (docs)

## Files Created/Modified

- `crates/nono-cli/src/exec_strategy.rs` - Added signal_pty_foreground_group(), handle_pty_suspension() (DEPS-01)
- `crates/nono-cli/src/pty_proxy.rs` - Added in_alt_screen(), leave_screen_for_suspension(), reenter_screen_for_resume(), shutdown_attach_listener(), take_suspension_request() (DEPS-01)
- `crates/nono-cli/src/update_check.rs` - Added detect_ci_provider() returning Option<&'static str> (FEAT-06a)
- `crates/nono-cli/src/cli.rs` - Updated RUN_AFTER_HELP/SHELL_AFTER_HELP constants to always-further/claude (FEAT-06b)
- `crates/nono-cli/data/policy.json` - Added profile_aliases section (D-07/D-08)
- `crates/nono-cli/src/policy.rs` - Added profile_aliases to Policy struct; alias fallback in get_policy_profile() (D-07/D-08)
- `crates/nono-cli/src/profile/builtin.rs` - Added 6 alias resolution tests (D-07/D-08)
- `crates/nono-cli/src/profile/mod.rs` - Updated doc comments to always-further/claude (FEAT-06b)
- `crates/nono-cli/src/setup.rs` - Updated shell integration examples to always-further/claude (FEAT-06b)
- `crates/nono-cli/README.md` - Updated profile table to namespace forms (FEAT-06b)
- `.planning/phases/88-feature-dependency-cherry-pick-wave/88-PARTIAL-CI.md` - Plan 88-04 DEPS-01 deferral entries

## Decisions Made

- **D-07 alias implementation**: `policy.json::profile_aliases` is the source of truth. `policy.rs::get_policy_profile()` does canonical lookup first, then alias lookup with one-hop resolution only (aliases cannot point to aliases). Bare name `claude-code` remains the stable internal key; `always-further/claude` is the public-facing alias.
- **D-08 fork-only profiles**: `swival/default`, `nono-ts/wfp-test-open`, `nono-ts/wfp-test-blocked`, `nono-ts/default` aliases added. All bare names (swival, nono-ts-wfp-test-open, etc.) remain canonical.
- **exec_strategy.rs cfg guards**: The module is already gated with `#[cfg(not(target_os = "windows"))]` in main.rs (line 34), making per-function guards for nix:: references redundant. Marked PARTIAL→CI per CLAUDE.md MUST/NEVER rule.
- **6d88638e conflict resolution**: Fork had already deleted migration.rs, pull_ui.rs, claude-clear-nono.sh, codex-clear-nono.sh — kept fork deletions via `git rm`. For cli.rs conflicts, kept the fork's constant-based approach (RUN_AFTER_HELP/SHELL_AFTER_HELP) and updated constants to use always-further/claude.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] pty_proxy.rs cherry-pick conflict**
- **Found during:** Task 1 (DEPS-01 cherry-pick)
- **Issue:** `git cherry-pick -x 4179ce03` conflicted in pty_proxy.rs on two conflict regions: (a) missing new method group after `detach()`, (b) `take_suspension_request()` placement.
- **Fix:** Manually resolved conflicts by taking upstream's new methods (`in_alt_screen`, `leave_screen_for_suspension`, `reenter_screen_for_resume`, `shutdown_attach_listener`, `take_suspension_request`). exec_strategy.rs applied cleanly.
- **Files modified:** `crates/nono-cli/src/pty_proxy.rs`
- **Commit:** `1f4fd335`

**2. [Rule 3 - Blocking] 6d88638e multi-file conflicts and delete/modify conflicts**
- **Found during:** Task 3 (FEAT-06b cherry-pick)
- **Issue:** Many conflicts: (a) cli.rs uses constants vs upstream inline strings; (b) package_cmd.rs references pull_ui.rs which fork deleted; (c) migration.rs, pull_ui.rs, claude-clear-nono.sh, codex-clear-nono.sh had modify/delete conflicts; (d) doc files had expanded content.
- **Fix:** For cli.rs: kept constants, updated them to use always-further/claude. For package_cmd.rs: kept fork's print_pull_summary() since pull_ui.rs is deleted. For DU conflicts: `git rm` to preserve fork deletions. For docs: took upstream's expanded content.
- **Files modified:** Multiple doc/script/CLI files
- **Commit:** `d80b2b19`

**3. [Rule 1 - Bug] pty_proxy.rs extra blank line from conflict resolution**
- **Found during:** Task 4 (`cargo fmt --all -- --check` failure)
- **Issue:** Double blank line after `shutdown_attach_listener()` introduced during conflict resolution.
- **Fix:** Removed extra blank line.
- **Files modified:** `crates/nono-cli/src/pty_proxy.rs`
- **Commit:** `8ee56d80`

---

**Total deviations:** 3 auto-fixed (2 blocking, 1 bug)
**Impact on plan:** All auto-fixes necessary for the cherry-picks to apply. The conflict resolutions preserved the fork's structural decisions (constants, deleted files) while adopting upstream's content updates.

## CI Gate Results (Windows Host)

- **clippy:** PASS (0 warnings, 0 errors)
- **fmt:** PASS
- **cargo test -p nono-cli:** 1341+ passed, 4 failed (all 4 are pre-existing Windows baseline failures per `nono_cli_windows_baseline_test_failures` memory note)
- **PARTIAL→CI:** exec_strategy.rs and pty_proxy.rs use nix:: symbols; module-level gated but not verifiable on Windows host → deferred to GH Actions Linux/macOS CI lanes (see 88-PARTIAL-CI.md Plan 88-04 rows)

## D-Constraint Verification

| Constraint | Status | Evidence |
|-----------|--------|---------|
| D-07: always-further/claude alias | PASS | policy.json profile_aliases + policy.rs resolver + test_get_builtin_claude_code_by_namespace_alias passes |
| D-08: fork-only profiles namespaced | PASS | swival/default, nono-ts/* aliases in policy.json; 4 alias tests pass |
| D-09: CI provider discovery absorbed | PASS | detect_ci_provider() in update_check.rs (cc11b389); independent of rename |
| D-12: cherry-pick -x + DCO | PASS | All 3 cherries have (cherry picked from commit ...) + Signed-off-by |
| PARTIAL→CI for DEPS-01 | PASS | 88-PARTIAL-CI.md Plan 88-04 rows recorded |

## Known Stubs

None.

## Threat Flags

None — no new network endpoints, auth paths, file access patterns, or schema changes at trust boundaries beyond what's already covered in the threat model. The profile alias resolver is static (build-time embedded) and one-hop only per T-88-14 mitigation.

## Self-Check: PASSED

Files exist:
- `crates/nono-cli/src/update_check.rs` — detect_ci_provider() present (line 284)
- `crates/nono-cli/data/policy.json` — profile_aliases section present (line 1106)
- `crates/nono-cli/src/profile/builtin.rs` — always-further/claude alias test present
- `crates/nono-cli/src/exec_strategy.rs` — signal_pty_foreground_group() present (line 2615)
- `crates/nono-cli/src/policy.rs` — profile_aliases field in Policy struct; alias fallback in get_policy_profile()
- `.planning/phases/88-feature-dependency-cherry-pick-wave/88-PARTIAL-CI.md` — Plan 88-04 rows present

Commits exist:
- `1f4fd335` — fix(pty): ctrl-z hangs when running with a PTY (DEPS-01)
- `a4fb72df` — feat(update-check): discover ci environments on update (FEAT-06a)
- `d80b2b19` — refactor(profiles): standardize profile names with namespace (FEAT-06b + D-07/D-08)
- `8ee56d80` — chore(88-04): cargo fmt fix double blank line in pty_proxy.rs
- `4ade9de1` — docs(88-04): add Plan 88-04 PARTIAL→CI deferral entries
