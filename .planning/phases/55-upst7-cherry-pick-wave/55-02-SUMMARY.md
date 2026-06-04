---
phase: 55-upst7-cherry-pick-wave
plan: "02"
subsystem: profile
tags: [cherry-pick, upst7, jsonc, target_binary, opencode, profile, c7]
requirements: [REQ-UPST7-02]

dependency_graph:
  requires:
    - phase: 55-upst7-cherry-pick-wave
      plan: "01"
      provides: "Wave 1 (C4 proxy 502 hardening) — must precede Wave 2"
  provides:
    - "JSONC profile parsing (jsonc-parser 0.32, .jsonc extension support)"
    - "Profile target_binary field (user profiles can declare binary to exec)"
    - "is_file_path_ref() helper function"
    - "opencode removed from built-in policy.json (now always-further/opencode OfficialPack)"
    - "resolve_user_profile_path() preferring .jsonc over .json"
    - "SC3 schema-collision check artifact (55-02-SC3-SCHEMA-COLLISION-CHECK.md)"
  affects:
    - "Phase 55 Plan 55-06 (C12 policy ENV_LOCK test — touches policy.rs)"
    - "Phase 55 Plan 55-07 (C13 sigstore bump — touches Cargo.lock)"

tech_stack:
  added:
    - "jsonc-parser 0.32 (crates.io, David Sherret / dprint — JSONC parser with serde integration)"
  patterns:
    - "D-19 6-line trailer on all 5 cherry-pick commits"
    - "Fork-specific conflict resolution: WR-01 security steps retained in parse_profile_bytes"
    - "Edition 2021 compatibility: chained if-let patterns rewritten as nested if/else"
    - "Platform-conditional env var guards in tests (APPDATA on Windows, XDG_CONFIG_HOME on Unix)"

key_files:
  created:
    - .planning/phases/55-upst7-cherry-pick-wave/55-02-SC3-SCHEMA-COLLISION-CHECK.md
  modified:
    - crates/nono-cli/Cargo.toml
    - Cargo.lock
    - crates/nono-cli/src/profile/mod.rs
    - crates/nono-cli/src/profile/builtin.rs
    - crates/nono-cli/src/profile_cmd.rs
    - crates/nono-cli/src/profile_save_runtime.rs
    - crates/nono-cli/src/learn_runtime.rs
    - crates/nono-cli/src/command_runtime.rs
    - crates/nono-cli/src/cli.rs
    - crates/nono-cli/src/policy.rs
    - crates/nono-cli/data/policy.json

decisions:
  - "SC3 pre-flight check produced before any cherry-picks — all 4 items CLEAR"
  - "jsonc-parser crate legitimacy verified (David Sherret / dprint provenance)"
  - "WR-01 security steps (detect_legacy_override_deny_key + raw_profile_has_both_bypass_and_override_keys) retained in parse_profile_bytes around the JSONC parser substitution"
  - "Edition 2021 incompatibility: upstream's chained if-let (cfa24f3d sandbox_prepare.rs refactoring) not ported — fork has a structurally divergent sandbox_prepare.rs and Edition 2021 does not support the pattern"
  - "migration.rs not present in fork: opencode OfficialPack entry already tracked in package_status.rs (OPENCODE_PACK)"
  - "D-55-E3 PARTIAL: cross-toolchain (x86_64-linux-gnu-gcc / macOS cc) not installed on Windows host — deferred to live CI"

metrics:
  duration: "~4 hours"
  completed: "2026-06-04"
  tasks: 2
  files_modified: 12
---

# Phase 55 Plan 02: PROFILE-JSONC-TARGET-BINARY Summary

**C7 cluster cherry-picks: JSONC parsing + target_binary field + is_file_path_ref + chained-if-let test isolation + opencode OfficialPack relocation. 5 upstream commits (53a0c521, 9398a139, e15aa53c, cfa24f3d, 2bd9b4d5) landed as 6 commits (5 cherry-picks + 1 auto-fix) on the worktree-agent branch.**

## Performance

- **Duration:** ~4 hours
- **Started:** 2026-06-04
- **Completed:** 2026-06-04
- **Tasks:** 2 (SC3 pre-flight + 5 cherry-picks)

## SC3 Schema-Collision Check Verdict

| Item | Upstream Commits | Verdict |
|------|-----------------|---------|
| target_binary field vs nono-profile.schema.json | 9398a139 | **CLEAR** |
| JSONC deserialization vs From\<ProfileDeserialize\>/WR-01 | 53a0c521 | **CLEAR** (fork-specific conflict resolution applied) |
| opencode removal vs canonical sections | 2bd9b4d5 | **CLEAR** |
| From\<ProfileDeserialize\> enumeration preserved | 9398a139 | **CLEAR** |
| jsonc-parser crate legitimacy | 53a0c521 | **CLEAR** (David Sherret / dprint provenance) |

All items CLEAR. Cherry-picks proceeded without blocking issues.

## C7 Cherry-pick Log

| # | Upstream SHA | Tag | Subject | Fork Commit | Conflicts |
|---|-------------|-----|---------|-------------|-----------|
| 1 | 53a0c521 | v0.58.0 | feat(profile): add JSONC support for profile files | fc80c036 | profile/mod.rs (WR-01 around JSONC parser), profile_cmd.rs (resolve_validate_target), profile_save_runtime.rs |
| 2 | 9398a139 | v0.58.0 | feat(profile): allow profiles to specify a target binary | 5be4355d | command_runtime.rs (fork's Phase 41 WFP wiring), cli.rs, profile/mod.rs |
| 3 | e15aa53c | v0.58.0 | fix: review fixes | 00fbf624 | profile_cmd.rs N/A (fork has no resolve_validate_target) |
| 4 | cfa24f3d | v0.58.0 | refactor: use chained if let for conditional statements | c19705d1 | sandbox_prepare.rs SKIPPED (structural divergence + Edition 2021) |
| 5 | 2bd9b4d5 | v0.59.0 | refactor(profile): extract opencode profile from built-ins | 2652e256 | migration.rs N/A (fork uses package_status.rs), builtin.rs, policy.json, profile/mod.rs, profile_cmd.rs, profile_save_runtime.rs |
| fix | — | — | Windows env var fix for jsonc_resolve test | 0940ead5 | Auto-fix (Rule 1) |

## Conflict-File Inventory

### profile/mod.rs (commit 53a0c521)

- **WR-01 security wrapper**: The upstream replaces `serde_json::from_slice` with `jsonc_parser::parse_to_serde_value` in `parse_profile_bytes` and `parse_profile_file`. The fork has an additional 3-step security wrapper (detect_legacy_override_deny_key + raw_profile_has_both_bypass_and_override_keys + fail-closed dual-key check). **Resolution**: Retained fork's security steps 2+3, replaced only the deserialize step (step 4) with JSONC parser. Both `parse_profile_bytes` and `parse_profile_file` updated.
- **list_profiles**: Upstream uses chained `if let ... && ...` (Edition 2024). Rewritten as nested `if { if { } }` for Edition 2021 compatibility.
- **`resolve_user_profile_path` / `user_profile_dir`**: New functions added cleanly alongside existing `get_user_profile_path`.

### command_runtime.rs (commit 9398a139)

- **Fork's Phase 41 WFP wiring**: `dangerous_force_wfp_ready` flag + `ResolveContext` threading is fork-specific. Applied upstream `resolve_profile_binary` + `resolve_program_from_profile_or_cli` + `command_args` expansion around the fork's WFP wiring. Changed `args = run_args.sandbox.clone()` to remain after profile loading.

### sandbox_prepare.rs (commit cfa24f3d) — SKIPPED

The upstream refactors `resolved_workdir` and a `prepare_sandbox` macOS block using chained `if let ... && let ...` (Edition 2024). The fork's sandbox_prepare.rs has a structurally different workdir resolution path (uses `launch_runtime::resolve_requested_workdir` instead) and a different macOS capability block. **Decision**: Skip the sandbox_prepare.rs portion entirely. Only the `test_list_profiles` isolation change (test-only, safe) was applied.

### migration.rs (commit 2bd9b4d5) — N/A

`migration.rs` does not exist in the fork. The opencode OfficialPack entry is already tracked in `crates/nono-cli/src/package_status.rs` as `OPENCODE_PACK`. The upstream's `migration.rs` change is superseded by the existing fork structure.

## Baseline-Aware CI Gate (D-55-E4)

Phase 54 baseline SHA: `43b27dde`

**Pre-existing failures (red→red carry-forward, NOT introduced by C7):**

| Test | Category |
|------|----------|
| `exec_strategy::launch::broker_dispatch_tests::broker_launch_assigns_child_to_job_object` | red→red (pre-existing) |
| `exec_strategy::launch::write_deny_low_il_broker_no_pty_tests::write_deny_low_il_broker_no_pty_prevents_child_write_to_medium_il_file` | red→red (pre-existing) |
| `profile_cmd::tests::test_init_allowed_when_pack_has_same_short_name` | red→red (pre-existing) |
| `protected_paths::tests::blocks_child_directory_capability` | red→red (pre-existing) |
| `protected_paths::tests::blocks_parent_directory_capability` | red→red (pre-existing) |
| `protected_paths::tests::requested_path_blocks_nonexistent_child_under_protected_root` | red→red (pre-existing) |

**New failures introduced and fixed:**

| Test | Category |
|------|----------|
| `profile::windows_low_il_broker_tests::jsonc_resolve_prefers_jsonc_extension` | red→green (introduced by C7-01, fixed by auto-fix commit 0940ead5 — Windows APPDATA env var) |

**Improvements:**

| Test | Category |
|------|----------|
| 5 new JSONC/target_binary tests | green (new tests added) |
| `test_list_profiles` | green (ENV_LOCK + TempDir isolation improved) |

**Final test result: 1136 passed / 6 failed (all 6 pre-existing)**

## Cross-Target Clippy Status (D-55-E3)

**PARTIAL** — cross-toolchain unavailable on Windows host.

- `cargo clippy --workspace --target x86_64-unknown-linux-gnu`: SKIPPED — `x86_64-linux-gnu-gcc` not found on Windows host
- `cargo clippy --workspace --target x86_64-apple-darwin`: SKIPPED — macOS `cc` not found on Windows host
- Windows-host `cargo build -p nono-cli`: PASS (clean, no warnings)
- **Disposition**: `skipped_gates_environmental` — same documented skip as Phases 36-01a/36-01b/36-01c/43/48. Deferred to live CI per `.planning/templates/cross-target-verify-checklist.md`.

## D-55-E1 Windows-Invariant Status

**PASS** — zero windows-specific files touched.

```
git diff --name-only HEAD~6 HEAD | grep -E "_windows\.rs|exec_strategy_windows|nono-shell-broker"
(no output)
```

## Held-Branch Status (D-55-03)

Phase 55 code accumulates on `worktree-agent-a9fb8616748f4eba9` (per-agent worktree branch). NOT merged to `main`. Merge-to-main gate remains the v0.58.0 tag+signed release per D-55-03.

## Acceptance Criteria Verification

| Criterion | Status |
|-----------|--------|
| SC3 artifact exists with ≥3 CLEAR verdicts | PASS — 4 CLEAR items |
| 5 cherry-pick commits with D-19 trailers | PASS — `git log --format="%B" HEAD~5..HEAD | grep -c "^Upstream-commit:"` = 5 |
| D-55-E1 Windows files: 0 | PASS |
| policy.rs: exactly 1 line added | PASS — `git diff HEAD~6..HEAD -- policy.rs | grep "^+" | wc -l` = 1 |
| data/policy.json opencode removed | PASS — 39 lines removed, 0 added |
| JSONC: .jsonc profile extension supported | PASS — resolve_user_profile_path + parse_profile_file use jsonc_parser |
| target_binary: user profiles can declare binary field | PASS — Profile.binary: Option\<String\> |
| `cargo build -p nono-cli` exits 0 | PASS |
| Cross-target clippy | PARTIAL — deferred to CI (skipped_gates_environmental) |
| Baseline CI gate | PASS — 6 pre-existing failures, no new load-bearing regressions |
| Feature branch NOT merged to main | PASS |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Windows env var in jsonc_resolve_prefers_jsonc_extension test**
- **Found during:** Task 2 test run (D-55-E4)
- **Issue:** The upstream test uses `XDG_CONFIG_HOME` exclusively for config dir isolation. On Windows, `resolve_user_config_dir()` uses `APPDATA` (not `XDG_CONFIG_HOME`), so the test was writing profiles to a temp dir but reading from the real `APPDATA` user profile dir.
- **Fix:** Added `#[cfg(target_os = "windows")]` / `#[cfg(not(target_os = "windows"))]` env var guards matching the `user_config_dir_guard` pattern in `builtin.rs`.
- **Files modified:** `crates/nono-cli/src/profile/mod.rs`
- **Commit:** `0940ead5`

**2. [Rule 3 - Blocking] Edition 2021 incompatibility with upstream's chained if-let syntax**
- **Found during:** Task 2 (build after cherry-pick attempt)
- **Issue:** Upstream code uses `if let ... && bool_expr` and `if let ... && let ...` chains (Rust 2024 feature). Fork workspace is Edition 2021 which does not support these.
- **Fix:** Rewrote all chained if-let patterns as nested if/else structures throughout profile/mod.rs and profile_cmd.rs.
- **Files modified:** `crates/nono-cli/src/profile/mod.rs`, `crates/nono-cli/src/profile_cmd.rs`
- **Committed in:** fc80c036 (C7-01)

**3. [Rule 3 - Structural Divergence] sandbox_prepare.rs chained-if-let refactoring skipped**
- **Found during:** Task 2 (cfa24f3d commit analysis)
- **Issue:** The upstream's `resolved_workdir` and `prepare_sandbox` macOS block don't have corresponding code in the fork's divergent `sandbox_prepare.rs`. The refactoring is purely stylistic.
- **Fix:** Applied only the test isolation portion (test_list_profiles ENV_LOCK + TempDir). Skipped the sandbox_prepare.rs functional changes.
- **Files modified:** None (skip)
- **Documentation:** Commit c19705d1 body

**4. [Rule 3 - Structural Divergence] migration.rs does not exist in fork**
- **Found during:** Task 2 (2bd9b4d5 cherry-pick)
- **Issue:** Upstream adds an `OfficialPack` entry for opencode to `migration.rs`. The fork doesn't have `migration.rs` — it uses `package_status.rs` with `OPENCODE_PACK`.
- **Fix:** Skipped `migration.rs` creation; the equivalent functionality already exists in the fork.
- **Files modified:** None (existing package_status.rs covers this)
- **Documentation:** Commit 2652e256 body

**5. [Rule 1 - Bug] profile_cmd.rs resolve_validate_target function absent in fork**
- **Found during:** Task 2 (e15aa53c review fixes)
- **Issue:** The upstream's e15aa53c fixes `resolve_validate_target` in `profile_cmd.rs` to use `profile::is_file_path_ref`. The fork's `cmd_validate` handles path resolution inline and doesn't have this helper function.
- **Fix:** Applied the `is_file_path_ref` helper addition and usage in profile/mod.rs and command_runtime.rs (the intended change). The profile_cmd.rs fix is N/A for the fork.
- **Files modified:** `crates/nono-cli/src/profile/mod.rs`, `crates/nono-cli/src/command_runtime.rs`
- **Committed in:** 00fbf624 (C7-03)

## Known Stubs

None — all changes are wired production code. The `binary` field, JSONC parsing, and opencode removal are fully functional.

## Threat Flags

| Flag | File | Description |
|------|------|-------------|
| threat_flag: new_dependency | crates/nono-cli/Cargo.toml | jsonc-parser 0.32 added as new dependency — verified legitimate (David Sherret / dprint; widely used in Rust JSONC ecosystem) |
| threat_flag: user_controlled_binary | crates/nono-cli/src/command_runtime.rs | target_binary field from user profiles is restricted to user profiles only (is_user_override || is_file_path_ref guards) — T-55-02-02 mitigated |

## Self-Check: PASSED

### Files exist:
- [x] `.planning/phases/55-upst7-cherry-pick-wave/55-02-SC3-SCHEMA-COLLISION-CHECK.md` — FOUND
- [x] `crates/nono-cli/src/profile/mod.rs` — FOUND (resolve_user_profile_path at line 3111)
- [x] `crates/nono-cli/src/command_runtime.rs` — FOUND (resolve_profile_binary + is_file_path_ref usage)
- [x] `crates/nono-cli/data/policy.json` — FOUND (opencode profile removed)

### Commits exist:
- [x] ba4cba10 — docs(55-02): SC3 schema-collision check artifact
- [x] fc80c036 — feat(profile): add JSONC support for profile files
- [x] 5be4355d — feat(profile): allow profiles to specify a target binary
- [x] 00fbf624 — fix: review fixes
- [x] c19705d1 — refactor: use chained if let for conditional statements
- [x] 2652e256 — refactor(profile): extract opencode profile from built-ins
- [x] 0940ead5 — fix(profile): add Windows env var compatibility for jsonc_resolve test

### Acceptance criteria:
- [x] `git log --format="%B" HEAD~5..HEAD | grep -c "^Upstream-commit:"` = 5
- [x] `git diff --name-only HEAD~6 HEAD | grep -E "_windows\.rs|exec_strategy_windows|nono-shell-broker"` = 0 lines
- [x] `git diff HEAD~6..HEAD -- crates/nono-cli/src/policy.rs | grep "^+" | wc -l` = 1
- [x] `test -f ".planning/phases/55-upst7-cherry-pick-wave/55-02-SC3-SCHEMA-COLLISION-CHECK.md"` exits 0
- [x] `cargo build -p nono-cli` exits 0
- [x] `cargo test -p nono-cli` = 1136 passed / 6 failed (all pre-existing)
- [x] Feature branch NOT merged to main
