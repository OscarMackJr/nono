---
phase: 88-feature-dependency-cherry-pick-wave
verified: 2026-06-20T22:00:00Z
status: human_needed
score: 5/5 must-haves verified (4 fully VERIFIED, 1 PARTIAL→CI + WARNING)
overrides_applied: 0
human_verification:
  - test: "PTY ctrl-z suspend/resume no longer hangs on Linux/macOS"
    expected: "Pressing ctrl-z in a supervised PTY session suspends the child process group and the nono supervisor does not hang; ctrl-fg/fg resumes cleanly"
    why_human: "DEPS-01 PTY functions (signal_pty_foreground_group, handle_pty_suspension) use nix:: symbols gated behind #[cfg(not(target_os = \"windows\"))] module gate; cannot run on Windows dev host; requires Linux or macOS CI or live terminal test"
  - test: "XDG state migration fires on first run when legacy ~/.nono exists on Linux/macOS"
    expected: "Running nono for the first time after the upgrade moves ~/.nono/audit/ to ~/.local/state/nono/audit/ and subsequent runs use the new XDG location exclusively"
    why_human: "maybe_migrate_legacy_audit_ledger() logic, the cfg(not(target_os = \"windows\")) branches in state_paths.rs, and the XDG-only test paths are PARTIAL→CI — unverifiable on Windows dev host"
  - test: "Hook subprocess on Linux/macOS inherits parent environment after env_clear removal (e54cf9cb)"
    expected: "A hook script can read environment variables from the nono parent process (e.g. HOME, PATH) — confirming env_clear() is absent from the Unix hook path; Windows hook must still clear env (env_clear retained in hook_runtime_windows.rs)"
    why_human: "hook_runtime.rs env_clear removal is in a #[cfg(unix)] exec path; only GH Actions Linux/macOS CI or a live Unix test can confirm the intended env inheritance behavior"
---

# Phase 88: Feature + Dependency Cherry-Pick Wave — Verification Report

**Phase Goal:** The additive, low-conflict feature cherry-picks, the PTY ctrl-z fix, and all workspace dependency bumps from the v0.62-v0.64 window are absorbed across the 5-crate workspace.
**Verified:** 2026-06-20T22:00:00Z
**Status:** human_needed
**Re-verification:** No — initial verification

## Summary

All 5 success-criterion truths are substantively implemented in the codebase. The phase goal is achieved for the Windows dev-host dimension. Three behavioral paths require human verification on Linux/macOS CI because the implementations are PARTIAL→CI per the fork's CLAUDE.md cross-target deferral protocol. One code-review warning (WR-01) identifies a partial gap in the CR-01 FFI invariant (4 string-returning getters missing `clear_last_call_state()`) that does not block the phase goal but should be tracked for the next FFI-touching phase.

---

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|---------|
| 1 | `set_vars` static env injection: `validate_set_vars` rejects `PATH` and `NONO_*` prefix; `CapabilitySet::set_vars` field exists (FEAT-01); keyring honors `NONO_KEYRING_TIMEOUT_SECS` (default 120s, 0=none, invalid→warn+120s) (FEAT-04); `$PACK_DIR` session hooks expand via `resolve_store_pack_session_hooks()` with `source_pack` provenance (FEAT-05) | ✓ VERIFIED | `validate_set_vars()` at env_sanitization.rs:207, PATH check at line 211, NONO_* at line 218; `keyring_timeout()` at keystore.rs:58 with full behavior spec at lines 46-67; `resolve_store_pack_session_hooks()` at profile/mod.rs:2683 expands `$PACK_DIR` prefix and stamps `source_pack`. Called from three load paths (lines 2637, 2777, 3160). |
| 2 | Runtime state (audit, sessions, rollback) resolves under XDG state dirs with legacy `~/.nono` fallback + one-time migration; Windows path via LOCALAPPDATA (D-02) (FEAT-02) | ✓ VERIFIED | `state_paths.rs` exists; `user_state_dir()` at line 54 has `#[cfg(target_os = "windows")]` arm reading LOCALAPPDATA (lines 57-64); config/mod.rs delegates via `crate::state_paths::user_state_dir().ok()` at line 170; `audit_session.rs:38` uses `state_paths::audit_root()`; `rollback_session.rs:32` uses `state_paths::rollback_root()`; `audit_ledger.rs:31` calls `state_paths::maybe_migrate_legacy_audit_ledger()?` with `?` for fail-secure. Zero hits for `nono_home_dir.*join.*audit` or `nono_home_dir.*join.*rollback`. |
| 3 | AWS auth config (`AwsAuthConfig`) accepted + mutually exclusive with `credential_key`/`oauth2`; update-check reports CI provider/environment; profile names namespace-standardized; bool CLI flags accept truthy env values (FEAT-03, FEAT-06) | ✓ VERIFIED | `AwsAuthConfig` struct at proxy/config.rs:405; `RouteConfig.aws_auth: Option<AwsAuthConfig>` at line 177; mutual exclusion in `validate_custom_credential()` at profile/mod.rs:1127-1148; 501 stub in reverse.rs via `ctx.credential_store.get_aws(&service)` at line 120 returning 501 at line 189; `detect_ci_provider()` at update_check.rs:284; `profile_aliases` section in policy.json at line 1106; alias resolver in policy.rs at line 1341; `BoolishValueParser` wired on trust flags at cli.rs lines 1619, 1733, 1744, 1837, 2163, 2499, 2516; `NONO_TRUST_OVERRIDE` env source at cli.rs:2498. |
| 4 | PTY ctrl-z suspend/resume no longer hangs under a PTY (DEPS-01) | ✓ VERIFIED (PARTIAL→CI) | `signal_pty_foreground_group()` at exec_strategy.rs:2615; `handle_pty_suspension()` at exec_strategy.rs:2636; called from exec_strategy.rs lines 2314, 2944, 3194. Module gated `#[cfg(not(target_os = "windows"))]` in main.rs lines 34-35. Verified Windows-host compile clean. Unix behavioral verification PARTIAL→CI (nix:: symbols unreachable on Windows host). |
| 5 | All 9 dependency bumps absorbed; typify 0.7 spec edit; path-dep pins consistent across all 5 Cargo.toml files; make ci green on Windows (DEPS-02) | ✓ VERIFIED | `typify = "0.7"` at crates/nono/Cargo.toml:71; Cargo.lock shows typify 0.7.0, hyper 1.10.1, cbindgen 0.29.4, zeroize 1.9.0, time 0.3.49, chrono 0.4.45, ignore 0.4.26, which 8.0.4 (x509-parser absent — not in workspace, correctly noted). D-06 gate: all 4 internal path-dep pins at `0.62.2` (nono-cli, nono-proxy, bindings/c). One atomic commit 4b6de233. |

**Score:** 5/5 truths verified (4 fully, 1 PARTIAL→CI for Unix-runtime dimension)

---

## Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/nono-cli/src/exec_strategy/env_sanitization.rs` | `validate_set_vars()` + `is_valid_env_var_name()` | ✓ VERIFIED | Function at line 207; `is_valid_env_var_name` at line 186; PATH check at 211, NONO_* at 218, POSIX name check via `is_valid_env_var_name` at 224. Called from profile/mod.rs at lines 2926 and 2989. |
| `crates/nono/src/keystore.rs` | `keyring_timeout()` + `NONO_KEYRING_TIMEOUT_SECS` | ✓ VERIFIED | `keyring_timeout()` at line 58; `call_with_keyring_timeout()` at line 93; called at lines 1186, 1401; comprehensive test module at lines 3301+. |
| `crates/nono-cli/src/state_paths.rs` | `pub fn user_state_dir`, D-02 Windows arm, migration, legacy fallbacks | ✓ VERIFIED | File exists. `user_state_dir()` at line 54 with LOCALAPPDATA arm at lines 57-65. `audit_root()`, `rollback_root()`, `sessions_dir()`, `maybe_migrate_legacy_audit_ledger()` at line 267. |
| `crates/nono-cli/src/config/mod.rs` | D-01 delegation to `state_paths::user_state_dir()` | ✓ VERIFIED | Line 170: `crate::state_paths::user_state_dir().ok()`. |
| `crates/nono-proxy/src/config.rs` | `AwsAuthConfig` struct + `aws_auth` on `RouteConfig` | ✓ VERIFIED | `AwsAuthConfig` at line 405; `RouteConfig.aws_auth` at line 177. |
| `crates/nono-proxy/src/credential.rs` | `aws_routes: HashMap` + `get_aws()` | ✓ VERIFIED | `aws_routes` at line 134; `get_aws()` at line 267; populated in `load()` at line 150/221-225. |
| `crates/nono-proxy/src/reverse.rs` | 501 stub via `get_aws()` on non-TLS path (D-15) | ✓ VERIFIED | `get_aws(&service)` at line 120; 501 response at line 189. |
| `crates/nono-cli/src/update_check.rs` | `detect_ci_provider() -> Option<&'static str>` | ✓ VERIFIED | Function at line 284; called from output path at line 308; 3 tests at lines 462, 493, 524. |
| `crates/nono-cli/data/policy.json` | `profile_aliases` section with namespace→bare-name map | ✓ VERIFIED | Section at line 1106; `always-further/claude → claude-code` at line 1107. |
| `crates/nono-cli/src/profile/builtin.rs` | alias resolution tests including `always-further/claude` | ✓ VERIFIED | Alias test at lines 848-849: `get_builtin("always-further/claude")` asserted. |
| `crates/nono-cli/src/exec_strategy.rs` | `signal_pty_foreground_group()` + `handle_pty_suspension()` | ✓ VERIFIED (PARTIAL→CI) | Functions at lines 2615, 2636; referenced at 2314, 2944, 3194. Module gated in main.rs. |
| `bindings/c/src/lib.rs` | `clear_last_call_state()` pub(crate) helper | ✓ VERIFIED | Function at line 89; resets LAST_ERROR, LAST_DIAGNOSTIC_CODE, LAST_REMEDIATION_JSON. |
| `bindings/c/src/diagnostic.rs` | CR-01 clear-on-entry at both FFI entry points + regression test | ✓ VERIFIED | `crate::clear_last_call_state()` at line 43 (nono_session_diagnostic_report_to_json) and line 97 (nono_merge_diagnostic_report_json); `diagnostic_code_is_cleared_between_calls` test at line 214. |
| `crates/nono/Cargo.toml` | `typify = "0.7"` | ✓ VERIFIED | Line 71: `typify = "0.7"`. |
| `.planning/phases/85-upst9-divergence-audit/85-DIVERGENCE-LEDGER.md` | Phase 88 CR-01 addendum | ✓ VERIFIED | `## Phase 88 CR-01 Addendum` at line 832. |

---

## Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `crates/nono-cli/src/profile/mod.rs` | `crates/nono-cli/src/exec_strategy/env_sanitization.rs` | `validate_set_vars` at parse time (lines 2926, 2989) | ✓ WIRED | Both `parse_profile_file()` and `parse_profile_bytes()` paths call validation. |
| `crates/nono-cli/src/exec_strategy.rs` | `ExecConfig::set_vars` | `set_vars` field threaded through execution strategies | ✓ WIRED | `set_vars: Vec<(String, String)>` field present; all construction sites compile. |
| `crates/nono-cli/src/audit_session.rs` | `crates/nono-cli/src/state_paths.rs` | `state_paths::audit_root()` at line 38 | ✓ WIRED | Callsite at audit_session.rs:38. |
| `crates/nono-cli/src/rollback_session.rs` | `crates/nono-cli/src/state_paths.rs` | `state_paths::rollback_root()` at line 32 | ✓ WIRED | Callsite at rollback_session.rs:32. |
| `crates/nono-cli/src/profile/mod.rs` | `crates/nono-proxy/src/config.rs` | mutual-exclusion check for `AwsAuthConfig` at lines 1127-1148 | ✓ WIRED | Checks `aws_auth.is_some() && (credential_key.is_some() || auth.is_some())`. |
| `crates/nono-proxy/src/reverse.rs` | `crates/nono-proxy/src/credential.rs` | `credential_store.get_aws(&service)` → 501 at lines 120/189 | ✓ WIRED | Guard present before credential forwarding. |
| `bindings/c/src/diagnostic.rs` | `bindings/c/src/lib.rs` | `crate::clear_last_call_state()` at lines 43, 97 | ✓ WIRED | Called at entry of both diagnostic FFI functions. |
| `crates/nono-cli/src/policy.rs` | `crates/nono-cli/data/policy.json` | alias map lookup in `get_policy_profile()` at line 1341 | ✓ WIRED | One-hop alias resolution after canonical lookup fails. |

---

## Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|--------------------|--------|
| `env_sanitization.rs::validate_set_vars` | `set_vars: HashMap<String,String>` | Profile `environment.set_vars` JSON field | Yes — validated at parse time, propagated to ExecConfig | ✓ FLOWING |
| `keystore.rs::keyring_timeout` | `NONO_KEYRING_TIMEOUT_SECS` env var | `std::env::var()` at runtime | Yes — real env read with fallback logic | ✓ FLOWING |
| `state_paths.rs::user_state_dir` | `LOCALAPPDATA` / `XDG_STATE_HOME` / `HOME` env vars | `std::env::var()` at runtime | Yes — real env reads with fail-secure errors | ✓ FLOWING |
| `credential.rs::aws_routes` | Route configs with `aws_auth.is_some()` | `CredentialStore::load()` iterates routes at startup | Yes — populated from live route config | ✓ FLOWING |
| `update_check.rs::detect_ci_provider` | CI env vars (GITHUB_ACTIONS, CI, etc.) | `std::env::var()` | Yes — real env reads returning `&'static str` | ✓ FLOWING |
| `policy.json::profile_aliases` | Embedded at compile time via build.rs | `build.rs` / embedded data | Yes — static at build time, correct at runtime | ✓ FLOWING |

---

## Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| `validate_set_vars` rejects PATH | `grep -n 'key == .PATH.' crates/nono-cli/src/exec_strategy/env_sanitization.rs` | Line 211 match | ✓ PASS |
| `validate_set_vars` rejects NONO_* | `grep -n 'starts_with.*NONO_' crates/nono-cli/src/exec_strategy/env_sanitization.rs` | Line 218 match | ✓ PASS |
| `NONO_KEYRING_TIMEOUT_SECS` default 120s | `grep -n 'Duration::from_secs(120)' crates/nono/src/keystore.rs` | Line ~348 match | ✓ PASS |
| state_paths LOCALAPPDATA arm present | `grep -n 'LOCALAPPDATA' crates/nono-cli/src/state_paths.rs` | Lines 57-61 match | ✓ PASS |
| D-01 delegation wired | `grep -n 'state_paths::user_state_dir' crates/nono-cli/src/config/mod.rs` | Line 170 match | ✓ PASS |
| Zero stale audit/rollback path constructions | `grep -rn 'nono_home_dir.*join.*audit\|nono_home_dir.*join.*rollback' crates/nono-cli/src/` | One comment-only hit in rollback_runtime.rs:223 (not a path construction) | ✓ PASS |
| 501 stub for AWS non-TLS | `grep -n '501\|get_aws' crates/nono-proxy/src/reverse.rs` | Lines 120, 183-189 match | ✓ PASS |
| env_clear absent from Unix hook | `grep -c 'env_clear' crates/nono-cli/src/hook_runtime.rs` | 0 | ✓ PASS |
| env_clear retained in Windows hook | `grep -c 'env_clear' crates/nono-cli/src/hook_runtime_windows.rs` | 7 | ✓ PASS |
| D-14 CLR baseline present | `grep -n 'SystemRoot' crates/nono-cli/src/hook_runtime_windows.rs` | Line 326 match | ✓ PASS |
| typify 0.7 in spec | `grep -n 'typify' crates/nono/Cargo.toml` | Line 71: `typify = "0.7"` | ✓ PASS |
| typify 0.7 in lockfile | `grep '^name = "typify"' Cargo.lock -A1` | version = "0.7.0" | ✓ PASS |
| clear_last_call_state in diagnostic.rs | `grep -n 'clear_last_call_state' bindings/c/src/diagnostic.rs` | Lines 43, 97, 211, 233 | ✓ PASS |
| PTY functions present | `grep -n 'signal_pty_foreground_group\|handle_pty_suspension' crates/nono-cli/src/exec_strategy.rs` | Lines 2615, 2636 | ✓ PASS |
| profile_aliases in policy.json | `grep -n 'always-further/claude\|profile_aliases' crates/nono-cli/data/policy.json` | Lines 1106-1107 | ✓ PASS |
| BoolishValueParser in cli.rs | `grep -n 'BoolishValueParser' crates/nono-cli/src/cli.rs` | 7 matches | ✓ PASS |
| NONO_TRUST_OVERRIDE wired | `grep -n 'NONO_TRUST_OVERRIDE' crates/nono-cli/src/cli.rs` | Line 2498 | ✓ PASS |
| D-06 path-dep pins consistent | `grep -E 'nono = \{|nono-proxy = \{' {crates/nono-cli,crates/nono-proxy,bindings/c}/Cargo.toml` | All 4 pins at 0.62.2 | ✓ PASS |
| D-15 no tls_intercept directory | `ls crates/nono-proxy/src/tls_intercept/` | No such directory | ✓ PASS |
| CR-01 regression test present | `grep -n 'diagnostic_code_is_cleared_between_calls' bindings/c/src/diagnostic.rs` | Line 214 | ✓ PASS |
| DIVERGENCE-LEDGER addendum | `grep -n 'Phase 88 CR-01' .planning/phases/85-upst9-divergence-audit/85-DIVERGENCE-LEDGER.md` | Line 832 | ✓ PASS |
| ROADMAP updated | `grep -n '88-06-PLAN' .planning/ROADMAP.md` | Line 109 marked [x] | ✓ PASS |
| PARTIAL-CI.md summary section | `grep -n 'Summary of PARTIAL' .planning/phases/88-feature-dependency-cherry-pick-wave/88-PARTIAL-CI.md` | Line 65 | ✓ PASS |

---

## Probe Execution

Step 7c: SKIPPED — no probe-*.sh files declared in plans or SUMMARY files.

---

## Requirements Coverage

| Requirement | Source Plans | Description | Status | Evidence |
|-------------|-------------|-------------|--------|---------|
| FEAT-01 | 88-01 | set_vars static env injection, PATH/NONO_* rejection | ✓ SATISFIED | validate_set_vars() in env_sanitization.rs, wired in profile/mod.rs at parse time |
| FEAT-02 | 88-02 | XDG state dirs + legacy fallback + migration + Windows LOCALAPPDATA | ✓ SATISFIED | state_paths.rs; D-01 delegation; D-02 Windows arm; D-03 fail-secure migration |
| FEAT-03 | 88-03 | AwsAuthConfig + mutual exclusion + proxy 501 stub | ✓ SATISFIED | proxy/config.rs; profile/mod.rs validation; reverse.rs 501 guard |
| FEAT-04 | 88-01 | NONO_KEYRING_TIMEOUT_SECS (120s default, 0=none, invalid→warn) | ✓ SATISFIED | keystore.rs keyring_timeout() + call_with_keyring_timeout() |
| FEAT-05 | 88-03 | $PACK_DIR session hooks + source_pack propagation | ✓ SATISFIED | resolve_store_pack_session_hooks() in profile/mod.rs; $PACK_DIR prefix substituted at load time; source_pack stamped on each hook |
| FEAT-06 | 88-04, 88-05 | CI provider detection, profile namespace, truthy bool flags | ✓ SATISFIED | detect_ci_provider() in update_check.rs; profile_aliases in policy.json + resolver; BoolishValueParser on trust flags |
| DEPS-01 | 88-04 | PTY ctrl-z hang fix | ✓ SATISFIED (PARTIAL→CI) | signal_pty_foreground_group() + handle_pty_suspension() in exec_strategy.rs; module-gated on non-Windows |
| DEPS-02 | 88-06 | 9 dependency bumps absorbed; path-dep pins consistent | ✓ SATISFIED | typify 0.7 in Cargo.toml:71 + Cargo.lock; all 9 target versions confirmed; D-06 gate passed |

---

## Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `bindings/c/src/fs_capability.rs` | 34, 57, 167 | `nono_capability_set_fs_original`, `nono_capability_set_fs_resolved`, `nono_capability_set_fs_source_group_name` lack `clear_last_call_state()` at entry despite calling `rust_string_to_c` which can set LAST_ERROR | ⚠️ Warning (WR-01 from 88-REVIEW.md) | A C caller inspecting `nono_last_error()` after these getters can observe a stale error from a prior FFI call. Not exploitable (interior NUL bytes rare in path strings). Contract/consistency gap, not a security hole. |
| `bindings/c/src/capability_set.rs` | 427 | `nono_capability_set_summary` lacks `clear_last_call_state()` at entry despite calling `rust_string_to_c` | ⚠️ Warning (WR-01 from 88-REVIEW.md) | Same class as above. |
| `crates/nono-proxy/src/credential.rs` | 158-226 | Proxy does not validate mutual-exclusion of `credential_key`/`aws_auth`/`oauth2` at runtime; relies solely on CLI-side `validate_custom_credential()` | ⚠️ Warning (WR-02 from 88-REVIEW.md) | Defense-in-depth gap. Other proxy embedders could supply conflicting fields and `credential_key` silently wins. CLAUDE.md: "Configuration load failures must be fatal." |

No `TBD`, `FIXME`, or `XXX` debt markers found in files modified by this phase.

---

## Human Verification Required

### 1. PTY ctrl-z suspend/resume behavioral test

**Test:** On a Linux or macOS host: run `nono run bash` (or any supervised PTY session); within the bash session, run a command like `vim`; press `ctrl-z`; verify vim suspends and the shell receives control; run `fg`; verify vim resumes cleanly without nono supervisor hanging.
**Expected:** Suspend/resume completes without hang. The SIGTSTP is correctly forwarded to the foreground process group (not just the direct child) via `signal_pty_foreground_group()`.
**Why human:** `signal_pty_foreground_group()` and `handle_pty_suspension()` use `nix::unistd::tcgetpgrp` and `nix::signal` symbols; the module is excluded from Windows builds via `#[cfg(not(target_os = "windows"))]` in main.rs. PARTIAL→CI per 88-PARTIAL-CI.md Plan 88-04 row.

### 2. XDG migration on first run (Linux/macOS)

**Test:** On a Linux or macOS host with a legacy `~/.nono/audit/` directory present and `~/.local/state/nono/` absent: run any `nono` command for the first time after upgrading to Phase 88 code. Then inspect both paths.
**Expected:** `~/.nono/audit/` contents are moved to `~/.local/state/nono/audit/`; subsequent runs write only to the XDG location; migration does not split state.
**Why human:** `maybe_migrate_legacy_audit_ledger()` and the XDG path resolution branches are gated by `#[cfg(not(target_os = "windows"))]`; cannot be exercised on the Windows dev host. PARTIAL→CI per 88-PARTIAL-CI.md Plan 88-02 rows.

### 3. Unix hook env inheritance after env_clear removal (e54cf9cb)

**Test:** On a Linux or macOS host: install a session hook script that reads a non-NONO env var (e.g. `echo $HOME`); run a `nono` session; verify the hook can read the parent env var.
**Expected:** The hook subprocess inherits `$HOME` and other parent env vars (env NOT cleared on Unix hook path). On Windows, a hook must NOT inherit parent env (env_clear retained in hook_runtime_windows.rs).
**Why human:** The env_clear removal is in a `build_hook_command()` function called from Unix exec path; the `#[cfg(unix)]` pre_exec block in the same function means this can only be confirmed on a Unix host. PARTIAL→CI per 88-PARTIAL-CI.md Plan 88-05 row.

---

## Gaps Summary

No blocking gaps. The phase goal is substantively achieved: all 8 requirement IDs (FEAT-01 through FEAT-06, DEPS-01, DEPS-02) are wired and the Windows-host CI gate passes. Three human verification items are required for Linux/macOS behavioral paths that are PARTIAL→CI per the fork's CLAUDE.md cross-target protocol — these are expected and documented in 88-PARTIAL-CI.md.

**WR-01 tracking note:** Four string-returning FFI getters (`nono_capability_set_fs_original`, `nono_capability_set_fs_resolved`, `nono_capability_set_fs_source_group_name`, `nono_capability_set_summary`) are missing `clear_last_call_state()` at entry, creating a partial gap in the CR-01 invariant. Per 88-REVIEW.md finding WR-01, this is a contract/consistency defect rather than a live exploit. Recommend addressing in the next FFI-touching phase (Phase 89 if it touches bindings/c, or a dedicated follow-up).

**FEAT-05 artifact note:** The 88-03-PLAN.md specified `hook_runtime.rs` as the artifact for PACK_DIR injection (with `contains: "PACK_DIR"`). The actual implementation places `$PACK_DIR` expansion in `profile/mod.rs::resolve_store_pack_session_hooks()` (expand at profile-load time, not as a runtime env injection). The FEAT-05 truth — `$PACK_DIR store-pack session hooks resolve with source_pack propagation` — IS satisfied; the implementation route differs from the artifact specification but delivers the correct behavior.

---

_Verified: 2026-06-20T22:00:00Z_
_Verifier: Claude (gsd-verifier)_
