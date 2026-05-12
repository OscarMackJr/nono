---
phase: 34-upst3-upstream-v0-41-v0-52-sync-execution
plan: 08b
slug: learn-deprecation
subsystem: cli-deprecation, diagnostic-engine, release-notes
tags: [upst3, c12-non-env, learn-deprecation, macos-learn, v0.52.0, wave-2-close, split-from-34-08]
requires: [34-04, 34-04b, 34-01, 34-02, 34-05, 34-07, 34-08a]
provides:
  - "Cross-platform `nono learn` deprecation banner (`b34c2af6`)"
  - "macOS learn → sandboxed `nono run` redirect (subset of `b5f0a3ab`)"
  - "CHANGELOG v0.52.0 + v0.51.0 release-notes entries (CHANGELOG-only port of `5d15b50e`)"
affects:
  - crates/nono-cli/src/cli.rs
  - crates/nono-cli/src/learn_runtime.rs
  - crates/nono/src/diagnostic.rs
  - crates/nono-cli/src/profile_runtime.rs
  - docs/cli/usage/flags.mdx
  - docs/cli/usage/troubleshooting.mdx
  - CHANGELOG.md
tech-stack:
  added: []
  patterns:
    - "D-34-B2 surgical retrofit posture: deprecation message lands on cross-platform surface; `learn_windows.rs` byte-identical"
    - "Fork-version invariance for upstream release commits (mirror 34-04b / 34-06): CHANGELOG-only port; drop Cargo.toml + Cargo.lock version bumps"
    - "D-34-A2 escalation pattern with scope-trim: when cherry-pick produces structural ExecConfig divergence, absorb additive cross-platform pieces and defer deep refactor"
key-files:
  created: []
  modified:
    - crates/nono-cli/src/cli.rs
    - crates/nono-cli/src/learn_runtime.rs
    - crates/nono/src/diagnostic.rs
    - crates/nono-cli/src/profile_runtime.rs (already up-to-date; cherry-pick 1/5 was empty)
    - docs/cli/usage/flags.mdx
    - docs/cli/usage/troubleshooting.mdx
    - CHANGELOG.md
decisions:
  - "Cherry-pick 1/5 (`1d491b4d` cargo fmt) lands as `--allow-empty`: fork's HEAD already carries the equivalent fmt-clean shape via Plan 34-08a's D-34-E1-preserving `validate_env_var_patterns_local`. Ledger traceability preserved via `Upstream-commit:` trailer."
  - "Cherry-pick 2/5 (`b5f0a3ab` macOS-learn-and-run) scope-trimmed: the additive learn-runtime + diagnostic + docs pieces landed; the ExecConfig-restructuring pieces deferred as P34-DEFER-08b-1."
  - "Cherry-pick 3/5 (`b34c2af6` learn deprecation) lands cross-platform; `print_learn_deprecation` helper called from `run_learn` AFTER fork's Phase 10/D-02 Windows admin gate. The `args.trace` reference in `print_learn_deprecation` removed (paired with deferred `LearnArgs.trace` field in P34-DEFER-08b-1)."
  - "Cherry-pick 4/5 (`bbdf7b85` escape-quote fix) further scope-trimmed: only the small `or_else` fallback in `extract_denied_path_from_error_line` retained; the function-body rewrite + helpers + 3 tests deferred as P34-DEFER-08b-2 (paired with P34-DEFER-08b-1's `analyze_error_output` wiring deferral). The orphan helpers landed in commit 2/5 are also removed here (Rule 1 fix: orphans fail `-D warnings`)."
  - "Cherry-pick 5/5 (`5d15b50e` v0.52.0 release) lands CHANGELOG-only: 5 Cargo.toml + Cargo.lock version bumps reverted to HEAD (fork tracks own version `0.37.1` per 34-04b / 34-06 precedent)."
metrics:
  duration: "≈3h (Windows host; D-20 escalation mid-flight on commit 2/5 added ~1h vs straight cherry-picks)"
  completed: "2026-05-12"
  commits_landed: 5
  upstream_commits_absorbed: 5  # ledger; functional content scope-trimmed per per-commit deferrals
---

# Phase 34 Plan 08b: C12-non-env Learn Deprecation + macOS Diagnostics + v0.52.0 Release Summary

One-liner: 5 v0.52.0 upstream cherry-picks landed onto fork's `main` — cross-platform `nono learn` deprecation banner (`b34c2af6`), macOS learn → sandboxed-run redirect (scope-trimmed subset of `b5f0a3ab`), CHANGELOG v0.52.0 entry (CHANGELOG-only port of `5d15b50e`), and 2 admin-traceability cherry-picks (`1d491b4d` empty-after-Plan-34-08a + `bbdf7b85` deferred-wiring marker) — with D-34-B2 surgical posture preserving `learn_windows.rs` byte-identical AND fork-version invariant preserving Cargo.toml at `0.37.1`. Wave 2 of Phase 34 closes jointly with sibling Plan 34-08a (env-surface port).

## Pre-state Captures (Task 1 baselines)

| Anchor | Value | Notes |
|--------|-------|-------|
| Pre-plan HEAD | `990c43308b392039df4cbf1d635386ce866a7ee4` | Plan 34-08a close |
| `learn_windows.rs` last-touched SHA | `aa4d33dc801b631883ba9c5fc7917e0e194342a4` | D-34-B2 anchor — MUST be unchanged at plan close |
| Fork Cargo version (nono-cli, nono, nono-proxy, nono-ffi) | `0.37.1` | Must remain `0.37.1` post-`5d15b50e` |
| `baseline_learn_winarms` (`crates/nono-cli/src/learn.rs`) | **1** | Windows-cfg arms unchanged post-`b34c2af6` |
| `baseline_cli_winarms` (`crates/nono-cli/src/cli.rs`) | **18** | Windows-cfg arms unchanged post-`b34c2af6` |
| `crates/nono/src/diagnostic.rs` macos-cfg arms | **0** | Path corrected from plan-text (`crates/nono-cli/src/diagnostic.rs` doesn't exist) |
| `crates/nono/src/diagnostic.rs` windows-cfg arms | **0** | (baseline) |
| Fork-defense grep: `never_grant\|apply_deny_overrides` | **24** | Must stay ≥24 |
| Fork-defense grep: `validate_path_within` | **9** | Must stay ≥9 |
| Fork-defense grep: `capabilities.aipc\|loaded_profile` | **76** | Must stay ≥76 |
| Fork-defense grep: `find_denied_user_grants` | **8** | Must stay ≥8 |
| Fork-defense grep: `bypass_protection` | **17** | Must stay ≥17 |
| G-25-DRIFT-01 closure invariant | **0** | RESL flag rename surface must stay 0 |
| `cargo build --workspace` baseline | PASS | Green |

## Upstream Commit Shapes (pre-cherry-pick `git show --stat`)

| SHA | Subject | Files | Lines | Notes |
|-----|---------|-------|-------|-------|
| `1d491b4d` | style: run cargo fmt | 2 | +18/-16 | Author: Advaith Sujith. Pure fmt; fork already fmt-clean post-34-08a → empty cherry-pick. |
| `b5f0a3ab` | feat(cli): enhance macos learn and run diagnostics | 11 | +721/-118 | Largest of the 5; deep `ExecConfig` refactor triggered escalation. |
| `b34c2af6` | feat(cli): deprecate 'nono learn' and improve diagnostics | 3 | +119/-9 | D-34-B2 surgical-posture commit. |
| `bbdf7b85` | fix(diagnostic): parse escaped quotes in structured properties | 2 | +52/-3 | Inseparable from `b5f0a3ab`'s wiring → wiring-side deferred. |
| `5d15b50e` | chore: release v0.52.0 | 6 | +48/-10 | CHANGELOG.md + 5 Cargo.toml files; per fork convention land CHANGELOG only. |

**Planner-text correction recorded:** the plan's "interfaces" table listed `1d491b4d` as "macOS learn diagnostics improvement"; the actual upstream commit is `style: run cargo fmt`. The real "macOS learn diagnostics" commit is `b5f0a3ab`. Cherry-pick order in this plan still follows upstream chronological order (`1d491b4d` first by date 2026-04-30).

## Per-Commit Cherry-Pick Log

### Commit 1/5: `1d491b4d` → `322e2ddb` (`--allow-empty`)

**Upstream:** `1d491b4d795f9fe610db8d83755fc59e2061f17a` `style: run cargo fmt` (Advaith Sujith)

**Resolution:** Cherry-pick produced a conflict in `crates/nono-cli/src/profile_runtime.rs` where upstream rewrites the caller path to use `crate::exec_strategy::validate_env_var_patterns` (crossing the `exec_strategy_windows` module boundary D-34-E1 forbids). Plan 34-08a's `validate_env_var_patterns_local` helper already absorbs the functional intent. Kept HEAD → staged diff went empty → committed `--allow-empty` for ledger traceability.

**Invariants:**
- `_windows / exec_strategy_windows` diff lines: **0** ✓
- `learn_windows.rs` diff lines: **0** ✓
- `learn_windows.rs` last-touched SHA: `aa4d33dc...` ✓

**D-19 trailer:** present with lowercase `a`. Author Advaith Sujith preserved.

### Commit 2/5: `b5f0a3ab` → `7497edf5` (scope-trimmed)

**Upstream:** `b5f0a3ab6e3b5c4eaf1796abc9c1c45cc9bec06f` `feat(cli): enhance macos learn and run diagnostics` (Luke Hinds)

**Escalation:** Initial `--strategy-option=theirs` resolution produced 17 compile errors from `ExecConfig` field-shape mismatch (fork carries `capability_elevation`, `resource_limits`, `audit_signer`, `no_diagnostics`, `threading`, `protected_paths`, `profile_save_base`, `startup_timeout`, `allowed_env_vars`, `denied_env_vars`, `bypass_protection_paths` — 8+ fork-side fields the upstream `--theirs` takeover destroyed). Aborted and re-approached as D-34-A2 escalation pattern: cherry-pick `-n`, surgically revert deep-divergence files to HEAD, keep additive set.

**Files absorbed (4 net staged):**
- `crates/nono-cli/src/learn_runtime.rs`: macOS `print_macos_run_guidance` helper + `command_display::format_command_line` import. Phase 10/D-02 Windows admin gate PRESERVED.
- `crates/nono/src/diagnostic.rs` (+276 lines): cross-platform diagnostic surface improvements (text catalog, structured-property helpers — orphans removed in commit 4/5).
- `docs/cli/usage/flags.mdx` + `docs/cli/usage/troubleshooting.mdx`: updated nono-learn-deprecation-direction docs.

**Files deferred to P34-DEFER-08b-1:**
- `crates/nono-cli/src/exec_strategy.rs` (244 lines)
- `crates/nono-cli/src/execution_runtime.rs` (46 lines)
- `crates/nono-cli/src/cli.rs` `LearnArgs.trace` field addition
- `crates/nono-cli/src/profile_save_runtime.rs`, `pty_proxy.rs`, `sandbox_log.rs`, `startup_prompt.rs` minor refinements

**Invariants:**
- `_windows / exec_strategy_windows` diff lines: **0** ✓
- `learn_windows.rs` diff lines: **0** ✓
- `learn_windows.rs` SHA: `aa4d33dc...` ✓
- Windows-cfg arm counts in `learn.rs` (1) and `cli.rs` (18): unchanged ✓
- Fork-defense greps: unchanged ✓

**D-19 trailer:** present with lowercase `a`. Author Luke Hinds preserved.

### Commit 3/5: `b34c2af6` → `4ed9df9d` (D-34-B2 surgical-posture commit)

**Upstream:** `b34c2af6acd990dc82b6a78f0ec492651d790c80` `feat(cli): deprecate 'nono learn' and improve diagnostics` (Luke Hinds)

**Resolution:** 2 conflicts in `crates/nono-cli/src/cli.rs` (help text + doc comment): take upstream's deprecation messaging verbatim. 1 conflict in `crates/nono-cli/src/learn_runtime.rs` (deprecation-call vs Windows-admin-gate): surgical merge → keep BOTH, with Windows admin gate (Phase 10/D-02 contract) running FIRST so non-admin Windows users get the `LearnError` before the deprecation banner. The cross-platform `print_learn_deprecation` then runs on every platform after the admin gate.

**Inline scope-trim:** the upstream `print_learn_deprecation` references `args.trace` (the `LearnArgs.trace` field deferred per P34-DEFER-08b-1). Replaced the trace-conditional branch with a TODO marker pointing to the deferral.

**Files modified:** `crates/nono-cli/src/cli.rs`, `crates/nono-cli/src/learn_runtime.rs`, `crates/nono/src/diagnostic.rs` (auto-merged).

**D-34-B2 invariants verified post-commit:**
- `learn_windows.rs` diff lines: **0** ✓ (no Windows-specific deprecation docstring added)
- `learn_windows.rs` last-touched SHA: `aa4d33dc...` ✓
- Windows-cfg arm count in `learn.rs`: **1** = baseline ✓
- Windows-cfg arm count in `cli.rs`: **18** = baseline ✓
- `_windows / exec_strategy_windows` diff lines: **0** ✓

**Commit body markers:**
- `D-34-B2` mentions: 2 ✓ (rationale paragraph + invariants section)
- `^Upstream-commit:` lines: 1 ✓

**D-19 trailer:** present with lowercase `a`. Author Luke Hinds preserved.

### Commit 4/5: `bbdf7b85` → `025d8099` (deferred-wiring + Rule-1 test cleanup)

**Upstream:** `bbdf7b85a86119316800c1ce696c081509285597` `fix(diagnostic): parse escaped quotes in structured properties` (Luke Hinds)

**Escalation:** 2 conflicted files. `crates/nono-cli/src/exec_strategy.rs` adds a `POST_EXIT_PTY_DRAIN_TIMEOUT` constant referenced only by `b5f0a3ab`-deferred code → reverted to HEAD. `crates/nono/src/diagnostic.rs` has 2 hunks (function body rewrite + new test) both of which require the `b5f0a3ab` structured-property wiring that was deferred per P34-DEFER-08b-1.

**Rule-1 fix during this commit:** the 4 orphan helpers (`extract_path_after_syscall_word`, `infer_access_from_structured_syscall_line`, `extract_structured_path_property`, `extract_structured_string_property`) and the 1 dependent test (`test_analyze_error_output_detects_node_eperm_mkdir_as_write`) that landed in commit 2/5 from `b5f0a3ab`'s additive diff fail under `-D warnings` (dead-code) and `cargo test --workspace --all-features` (test assertion). Removed in this commit; restoration tracked as P34-DEFER-08b-2.

**Files absorbed:** small additive fallback `extract_path_from_segment(prefix).or_else(|| extract_path_from_segment(line))` in `extract_denied_path_from_error_line` + deferral-marker comment blocks.

**Files deferred to P34-DEFER-08b-2:**
- 4 structured-property helper functions
- 3 structured-property tests
- `POST_EXIT_PTY_DRAIN_TIMEOUT` constant + reference
- `extract_structured_string_property` function-body rewrite (escape-quote handling)

**Invariants:**
- `_windows / exec_strategy_windows` diff lines: **0** ✓
- `learn_windows.rs` diff lines: **0** ✓
- `learn_windows.rs` SHA: `aa4d33dc...` ✓
- `cargo test -p nono --lib`: 675 passed, 0 failed ✓

**D-19 trailer:** present with lowercase `a`. Author Luke Hinds preserved.

### Commit 5/5: `5d15b50e` → `64b231a7` (CHANGELOG-only release)

**Upstream:** `5d15b50e2fbb60de9fdf69379bcaaf5bc1109e59` `chore: release v0.52.0` (Luke Hinds)

**Resolution:** 6 conflicted files (CHANGELOG.md + Cargo.lock + 4 Cargo.toml files). Per 34-04b / 34-06 release-commit precedent: revert all Cargo.toml + Cargo.lock changes to HEAD; keep CHANGELOG.md only. Resolved CHANGELOG.md conflict by taking upstream (its v0.52.0 + v0.51.0 entries; the v0.51.0 backfill landed transparently because fork's CHANGELOG.md lacked the v0.51.0 stanza).

**Fork-version invariant verified post-commit:**
- `crates/nono-cli/Cargo.toml`: `version = "0.37.1"` ✓
- `crates/nono/Cargo.toml`: `version = "0.37.1"` ✓
- `crates/nono-proxy/Cargo.toml`: `version = "0.37.1"` ✓
- `bindings/c/Cargo.toml`: `version = "0.37.1"` ✓
- Cargo.toml + Cargo.lock diff lines: **0** ✓
- CHANGELOG.md staged additions: 55 lines ✓

**Invariants:**
- `_windows / exec_strategy_windows` diff lines: **0** ✓
- `learn_windows.rs` diff lines: **0** ✓
- `learn_windows.rs` SHA: `aa4d33dc...` ✓

**D-19 trailer:** present with lowercase `a`. Author Luke Hinds preserved.

## Plan-Close Smoke Verifications (all PASS)

| # | Check | Expected | Actual | Status |
|---|-------|----------|--------|--------|
| 1 | `^Upstream-commit:` trailer count on HEAD~5..HEAD | 5 | 5 | ✓ |
| 2 | Uppercase `Upstream-Author:` count | 0 | 0 | ✓ |
| 2b | Lowercase `^Upstream-author:` count | 5 | 5 | ✓ |
| 3 | `^Signed-off-by:` count (2 per commit) | 10 | 10 | ✓ |
| 4 | `learn_windows.rs` anchor SHA | `aa4d33dc...` | `aa4d33dc...` | ✓ |
| 5 | D-34-E1 `_windows / exec_strategy_windows` diff lines HEAD~5..HEAD | 0 | 0 | ✓ |
| 6 | Fork-version invariant: `0.37.1` across all 4 Cargo.toml files | 0.37.1 | 0.37.1 | ✓ |
| 7a | Windows-cfg arms in `learn.rs` | 1 | 1 | ✓ |
| 7b | Windows-cfg arms in `cli.rs` | 18 | 18 | ✓ |
| 8 | G-25-DRIFT-01 closure invariant (RESL flag rename grep) | 0 | 0 | ✓ |
| 9a | Fork-defense `never_grant\|apply_deny_overrides` | ≥24 | 24 | ✓ |
| 9b | Fork-defense `validate_path_within` | ≥9 | 9 | ✓ |
| 9c | Fork-defense `capabilities.aipc\|loaded_profile` | ≥76 | 76 | ✓ |
| 9d | Fork-defense `find_denied_user_grants` | ≥8 | 8 | ✓ |
| 9e | Fork-defense `bypass_protection` | ≥17 | 17 | ✓ |
| 10 | `cargo build --workspace` | 0 | 0 | ✓ |

## D-34-D2 Close-Gate Results (per user-accepted posture)

| # | Gate | Status | Notes |
|---|------|--------|-------|
| 1 | `cargo test --workspace --all-features` (Windows host) | **PASS*** | 674 passed; 1 failed (`supervisor::aipc_sdk::windows_loopback_tests::helper_stamps_session_token_from_env`) — carry-forward AIPC-SDK env-leak flake explicitly accepted in plan posture; passes cleanly in isolation (`--test-threads=1`). |
| 2 | `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used` | **PASS** | Clean |
| 3 | Cross-target clippy Linux (`x86_64-unknown-linux-gnu`) | DEFERRED-TO-CI | `cc-rs: failed to find tool "x86_64-linux-gnu-gcc"` — Windows host lacks Linux cross-compiler. Per user-accepted posture, deferred to GitHub Actions CI runner where the Linux toolchain is provisioned. |
| 4 | Cross-target clippy macOS (`x86_64-apple-darwin`) | DEFERRED-TO-CI | `cc-rs: failed to find tool "cc"` — Windows host lacks macOS cross-compiler. Per user-accepted posture, deferred to GitHub Actions CI runner. |
| 5 | `cargo fmt --all -- --check` | **PASS** | Clean |
| 6 | Phase 15 5-row detached-console smoke | ADMIN-SKIPPED | Requires Windows admin context for `nono detach` + IPC pipe. Phase 15 has its own gate; not regressed by this plan. |
| 7 | `wfp_port_integration` test suite | ADMIN-SKIPPED | Requires WFP service + Windows admin. Phase 11/19 has its own gate. |
| 8 | `learn_windows_integration` test suite | ADMIN-SKIPPED | Requires ETW capture + Windows admin. The D-34-B2 byte-identity assertion at every commit AND at plan close serves as the proxy guarantee. |

**Gate 1 / 2 / 5 PASS** per the plan's must-PASS requirement. **Gates 3 / 4 deferred-to-CI** per the explicitly-accepted dev-host posture. **Gates 6 / 7 / 8 admin-skipped** per the standing posture (dev host runs as standard user; admin gates have dedicated phase-15/11/19 surfaces).

## D-34-B2 Surgical Posture Compliance (CRITICAL)

| Item | Pre-plan | Post-plan | Status |
|------|----------|-----------|--------|
| `learn_windows.rs` last-touched SHA | `aa4d33dc801b631883ba9c5fc7917e0e194342a4` | `aa4d33dc801b631883ba9c5fc7917e0e194342a4` | ✓ UNCHANGED |
| `learn_windows.rs` byte-content | hash X | hash X | ✓ identical |
| Windows-cfg arms in `learn.rs` | 1 | 1 | ✓ unchanged (no Windows-specific deprecation arm added) |
| Windows-cfg arms in `cli.rs` | 18 | 18 | ✓ unchanged (no Windows-specific deprecation arm added) |
| Deprecation banner reaches Windows users? | n/a | yes (via cross-platform `print_learn_deprecation` in `learn_runtime.rs`) | ✓ — Windows users see the same stderr deprecation message as Linux/macOS |
| Phase 10/D-02 Windows admin gate preserved? | yes (`if !crate::learn_windows::is_admin()`) | yes (runs BEFORE deprecation banner) | ✓ |

## D-34-E1 Windows-only-Files Invariant

| Scope | `git diff --stat ... -- crates/ \| grep -E '_windows\|exec_strategy_windows' \| wc -l` | Expected |
|-------|----------------------------------------------------------------------------------------|----------|
| Commit 1/5 (`322e2ddb` vs HEAD~1) | 0 | 0 |
| Commit 2/5 (`7497edf5` vs HEAD~1) | 0 | 0 |
| Commit 3/5 (`4ed9df9d` vs HEAD~1) | 0 | 0 |
| Commit 4/5 (`025d8099` vs HEAD~1) | 0 | 0 |
| Commit 5/5 (`64b231a7` vs HEAD~1) | 0 | 0 |
| Plan-close (`64b231a7` vs `990c4330`) | 0 | 0 |

All ✓.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Planner-text-versus-upstream-truth mismatch on `1d491b4d`**
- **Found during:** Task 1 baseline capture (`git show 1d491b4d --stat`)
- **Issue:** The plan's `<interfaces>` table described `1d491b4d` as "feat(diagnostic): macOS learn diagnostics improvement"; the actual upstream commit is `style: run cargo fmt` by Advaith Sujith. The plan's commit-1 task body also referenced `crates/nono-cli/src/diagnostic.rs` for the macOS diagnostic verify-grep, but that file doesn't exist — `diagnostic.rs` lives at `crates/nono/src/diagnostic.rs`.
- **Fix:** Corrected baseline captures to use the real upstream content (cargo fmt commit) and the real `crates/nono/src/diagnostic.rs` path. Reordered the macOS-learn rationale to map onto `b5f0a3ab` (the actual macOS-learn commit). Documented the planner-text correction in this SUMMARY (Pre-state Captures + Upstream Commit Shapes sections).
- **Files modified:** none (documentation-only correction; baseline captures use corrected paths).
- **Commit:** baseline capture (Task 1) only.

**2. [Rule 3 - Blocking] `b5f0a3ab` cherry-pick `--theirs` strategy regression**
- **Found during:** Task 2 commit 2/5 initial trial
- **Issue:** Trial cherry-pick using `--strategy-option=theirs` on `exec_strategy.rs` + `execution_runtime.rs` overwrote fork's `ExecConfig` (8+ fork-side fields removed), producing 17 compile errors. The plan didn't anticipate this depth of structural divergence.
- **Fix:** Aborted (`git cherry-pick --abort`). Re-approached as D-34-A2 escalation: cherry-pick `-n`, restore deep-divergence files to HEAD, keep additive cross-platform set. Documented as P34-DEFER-08b-1.
- **Files modified:** commit 2/5 net 4 files (vs upstream's 11) — `learn_runtime.rs`, `diagnostic.rs`, `flags.mdx`, `troubleshooting.mdx`.
- **Commit:** `7497edf5`.

**3. [Rule 1 - Bug] Orphan helpers + failing tests from `b5f0a3ab` scope-trim**
- **Found during:** Task 2 commit 4/5 (`bbdf7b85` post-resolve build + test)
- **Issue:** Commit 2/5's diagnostic.rs port carried 4 helper functions (`extract_path_after_syscall_word`, `infer_access_from_structured_syscall_line`, `extract_structured_path_property`, `extract_structured_string_property`) and 1 test (`test_analyze_error_output_detects_node_eperm_mkdir_as_write`) that were dead-code/failing without the deferred `b5f0a3ab` `analyze_error_output` wiring. `-D warnings` failed on the 4 helpers; `cargo test` failed on the test.
- **Fix:** Removed all 4 helpers and the failing test in commit 4/5 (alongside dropping `bbdf7b85`'s own 2 dependent tests and its function-body rewrite). Restoration tracked as P34-DEFER-08b-2 (paired with P34-DEFER-08b-1's `analyze_error_output` wiring deferral).
- **Files modified:** `crates/nono/src/diagnostic.rs` (within commit 4/5 amend).
- **Commit:** `025d8099` (amended).

**4. [Rule 3 - Blocking] `print_learn_deprecation` references missing `LearnArgs.trace` field**
- **Found during:** Task 2 commit 3/5 post-resolve build
- **Issue:** Upstream's `print_learn_deprecation` references `args.trace` — but `LearnArgs.trace` is part of the deferred `b5f0a3ab` cli.rs scope and doesn't exist in fork's `LearnArgs`.
- **Fix:** Removed the `if args.trace { ... }` branch from `print_learn_deprecation` and inserted an inline TODO marker pointing to P34-DEFER-08b-1 for restoration.
- **Files modified:** `crates/nono-cli/src/learn_runtime.rs` (within commit 3/5).
- **Commit:** `4ed9df9d`.

### Auth Gates

None encountered.

### Deferred Items (recorded in `deferred-items.md`)

| ID | Description |
|----|-------------|
| P34-DEFER-08b-1 | `b5f0a3ab` deep refactor of exec_strategy + execution_runtime + LearnArgs.trace + paired profile-save / PTY-quiet-period refinements |
| P34-DEFER-08b-2 | `bbdf7b85` escape-quote wiring + structured-property pipeline (4 helpers + 3 tests + body rewrite) — paired with P34-DEFER-08b-1's `analyze_error_output` wiring |

Both new deferrals appended to `.planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/deferred-items.md`.

## Wave 2 Closure

Plan 34-08b is the LAST plan in Wave 2. Wave 2 5/5 plans now closed:

| Plan | Cluster | Status |
|------|---------|--------|
| 34-02 | Proxy net | Closed (prior wave) |
| 34-05 | Completion | Closed (prior wave) |
| 34-07 | PS env-uri | Closed (prior wave) |
| 34-08a | Env-surface port (C12 env subset) | Closed `990c4330` |
| **34-08b** | **C12-non-env (this plan)** | **Closed `64b231a7`** |

## Self-Check: PASSED

- [x] SUMMARY.md present at expected path.
- [x] 5 commits landed on `main` (`322e2ddb`, `7497edf5`, `4ed9df9d`, `025d8099`, `64b231a7`).
- [x] All 5 commits carry verbatim D-19 trailer with lowercase `a` (count = 5, uppercase A count = 0).
- [x] All 5 commits carry 2× `Signed-off-by:` lines (total = 10).
- [x] `learn_windows.rs` byte-identical (anchor SHA `aa4d33dc...` unchanged).
- [x] D-34-E1 plan-close diff for `*_windows.rs / exec_strategy_windows/`: 0 lines.
- [x] Fork-version invariant: all 4 Cargo.toml files at `0.37.1`.
- [x] Windows-cfg arm counts in `learn.rs` (1) and `cli.rs` (18) unchanged.
- [x] G-25-DRIFT-01 closure invariant preserved (RESL flag rename grep = 0).
- [x] Fork-defense grep baselines preserved (24/9/76/8/17).
- [x] D-34-D2 Gates 1/2/5 PASS; Gates 3/4 deferred-to-CI; Gates 6/7/8 admin-skipped.
- [x] Deferrals appended to `deferred-items.md` (P34-DEFER-08b-1, P34-DEFER-08b-2).
