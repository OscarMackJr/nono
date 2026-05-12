---
phase: 35-upst3-closure-quick-wins
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - crates/nono-cli/src/exec_strategy_windows/mod.rs
  - crates/nono-cli/src/exec_strategy_windows/launch.rs
  - crates/nono-cli/src/exec_strategy/env_sanitization.rs
autonomous: true
requirements:
  - REQ-PORT-CLOSURE-01
tags:
  - phase-35
  - port-closure
  - windows
  - env-filter
  - p34-defer-08a-1
  - d-20-manual-replay

must_haves:
  truths:
    - "On Windows, `nono run --env-deny SECRET_PREFIX_* -- <cmd>` strips matching env vars from the child process environment before launch."
    - "On Windows, `nono run --env-allow KEY1,KEY2 -- <cmd>` passes only matching env vars (plus nono-injected credentials and the static Windows runtime allowlist) through to the child."
    - "On Windows, an empty allow-list with no deny-list (`allow_vars: []`, `deny_vars: None`) DENIES ALL user environment variables in the child (fail-closed empty-allow invariant from upstream `780965d7`)."
    - "Nono-injected credentials (`config.env_vars` appended in launch.rs:656-658) always bypass both the allow-list and the deny-list, matching the documented Unix precedence."
    - "Removing the two `#[allow(dead_code)]` attributes from `is_env_var_allowed` (line 113) and `is_env_var_denied` (line 153) in `env_sanitization.rs` does not produce dead-code or unused-import warnings under `clippy -D warnings` on Windows host."
  artifacts:
    - path: "crates/nono-cli/src/exec_strategy_windows/mod.rs"
      provides: "Two new `pub allowed_env_vars: Option<Vec<String>>` and `pub denied_env_vars: Option<Vec<String>>` fields on `ExecConfig` (lines 130-145 struct), each carrying the same D-20 doc-comment block copied from `exec_strategy.rs:314-325`. NO `#[cfg_attr(target_os = \"windows\", allow(dead_code))]` attribute."
      contains: "pub allowed_env_vars: Option<Vec<String>>"
    - path: "crates/nono-cli/src/exec_strategy_windows/launch.rs"
      provides: "Modified `build_child_env` (starts at line 551) — after the existing `if !should_skip_env_var(...)` block returns true (line 554), two new filter arms run BEFORE `env_pairs.push((key, value))` on line 645. Deny-list check precedes allow-list check, matching Unix `exec_strategy.rs:443-456` precedence."
      contains: "is_env_var_denied"
    - path: "crates/nono-cli/src/exec_strategy_windows/launch.rs"
      provides: "New `#[cfg(all(test, target_os = \"windows\"))]` test module at the bottom of the file (modeled on `pty_token_gate_tests` lines 1869-1961). Contains `test_windows_empty_allow_denies_all_env_vars` and at least one cross-platform invariant test."
      contains: "fn test_windows_empty_allow_denies_all_env_vars"
    - path: "crates/nono-cli/src/exec_strategy/env_sanitization.rs"
      provides: "`#[allow(dead_code)]` attributes at lines 113 and 153 REMOVED (both helpers are now live on Windows via the new wiring)."
      contains: "pub(crate) fn is_env_var_allowed"
  key_links:
    - from: "crates/nono-cli/src/exec_strategy_windows/launch.rs::build_child_env"
      to: "crates/nono-cli/src/exec_strategy/env_sanitization.rs::is_env_var_denied"
      via: "direct function call inside the `for (key, value) in std::env::vars()` loop"
      pattern: "is_env_var_denied\\(&key, denied\\)"
    - from: "crates/nono-cli/src/exec_strategy_windows/launch.rs::build_child_env"
      to: "crates/nono-cli/src/exec_strategy/env_sanitization.rs::is_env_var_allowed"
      via: "direct function call after the deny check"
      pattern: "is_env_var_allowed\\(&key, allowed\\)"
    - from: "crates/nono-cli/src/exec_strategy_windows/mod.rs::ExecConfig"
      to: "crates/nono-cli/src/exec_strategy_windows/launch.rs::build_child_env"
      via: "field access on `&ExecConfig<'_>` parameter — `config.allowed_env_vars` / `config.denied_env_vars`"
      pattern: "config\\.(allowed|denied)_env_vars"
---

<objective>
Wire `allowed_env_vars` and `denied_env_vars` consumption into the Windows execution path so Windows enforces the env filter that Plan 34-08a added to the cross-platform Unix path. Mirrors the Unix call-site shape at `exec_strategy.rs:435-457` (precedence: dangerous-var blocklist > deny_vars > allow_vars; nono-injected credentials always bypass both). Closes P34-DEFER-08a-1. D-20 manual-replay shape — commit body cites Plan 34-08a + upstream `1b412a7` (v0.37.0 env-filter surface introduction) + `780965d7` (empty-allow fail-closed invariant) as design-source citations; NO D-19 trailer block (no direct upstream commit lineage).

**Purpose:** Operator-controlled `--env-deny SECRET_*` and `--env-allow KEY1,KEY2` flags must produce identical observable behavior on Windows as on Linux/macOS. Currently the fields are forwarded cross-platform but no-op on Windows (`#[cfg_attr(target_os = "windows", allow(dead_code))]`-class dead surface).

**Output:** Two new fields on Windows `ExecConfig`, two new filter arms inside `build_child_env`, two `#[allow(dead_code)]` attributes removed from `env_sanitization.rs`, one new Windows-gated test module locking the empty-allow fail-closed invariant.

**Scope ceiling (D-35-A1 / D-34-B2):** ONLY the env-filter wiring. No audit-event emission, no WFP composition, no `run_nono` integration tests (host-blocked per `dirs::home_dir()`). No other `*_windows.rs` edits.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@CLAUDE.md
@.planning/STATE.md
@.planning/ROADMAP.md
@.planning/REQUIREMENTS.md
@.planning/phases/35-upst3-closure-quick-wins/35-CONTEXT.md
@.planning/phases/35-upst3-closure-quick-wins/35-PATTERNS.md
@.planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/34-08a-ENV-SURFACE-PORT-SUMMARY.md

<interfaces>
<!-- Unix ExecConfig env-filter fields with D-20 doc comments (mirror verbatim into Windows ExecConfig). -->

From `crates/nono-cli/src/exec_strategy.rs` lines 314-325:
```rust
/// Plan 34-08a Task 3 (D-20 manual replay of upstream `1b412a7`):
/// allow-list of environment variable names. When `Some`, only
/// variables matching an exact name or prefix pattern (e.g. `"AWS_*"`)
/// are passed to the child. `None` means inherit-all (default).
/// Nono-injected credentials (`config.env_vars`) always bypass this list.
pub allowed_env_vars: Option<Vec<String>>,
/// Plan 34-08a Task 4 (D-20 manual-replay-by-escalation of upstream
/// v0.52.0 `3657c935`): operator-controlled deny-list of environment
/// variable names. Variables matching an exact name or prefix pattern
/// (e.g. `"GITHUB_*"`) are stripped even if they also appear in
/// `allowed_env_vars`. Nono-injected credentials bypass this list.
pub denied_env_vars: Option<Vec<String>>,
```

From `crates/nono-cli/src/exec_strategy.rs` lines 435-457 (the Unix call-site shape to mirror):
```rust
for (key, value) in std::env::vars() {
    if should_skip_env_var(&key, &config.env_vars, &["NONO_CAP_FILE"]) {
        continue;
    }
    if let Some(ref denied) = config.denied_env_vars {
        if env_sanitization::is_env_var_denied(&key, denied) {
            continue;
        }
    }
    if let Some(ref allowed) = config.allowed_env_vars {
        if !env_sanitization::is_env_var_allowed(&key, allowed) {
            continue;
        }
    }
    cmd.env(&key, &value);
}
```

From `crates/nono-cli/src/exec_strategy/env_sanitization.rs` (the two helpers; both `pub(crate)` and reachable via the `#[path = "../exec_strategy/env_sanitization.rs"]` re-export at `exec_strategy_windows/mod.rs:20-21`):
```rust
#[allow(dead_code)] // Windows execution path uses exec_strategy_windows; allow-list wiring there ships separately.
pub(crate) fn is_env_var_allowed(key: &str, allowed_env_vars: &[String]) -> bool;
#[allow(dead_code)] // Windows execution path uses exec_strategy_windows; deny-list wiring there ships separately.
pub(crate) fn is_env_var_denied(key: &str, denied_env_vars: &[String]) -> bool;
```

From `crates/nono-cli/src/exec_strategy_windows/mod.rs` lines 130-145 (current Windows ExecConfig — does NOT yet have env-filter fields):
```rust
pub struct ExecConfig<'a> {
    pub command: &'a [String],
    pub resolved_program: &'a Path,
    pub caps: &'a CapabilitySet,
    pub env_vars: Vec<(&'a str, &'a str)>,
    pub cap_file: Option<&'a Path>,
    pub current_dir: &'a Path,
    pub session_sid: Option<String>,
    pub interactive_shell: bool,
    pub session_token: Option<String>,
    pub cap_pipe_rendezvous_path: Option<PathBuf>,
}
```
</interfaces>

</context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: Extend Windows ExecConfig with allowed_env_vars + denied_env_vars fields</name>
  <files>crates/nono-cli/src/exec_strategy_windows/mod.rs</files>
  <read_first>
    - crates/nono-cli/src/exec_strategy_windows/mod.rs (lines 1-160, especially struct ExecConfig at 130-145 and the env_sanitization re-export at 20-21 / 77-78)
    - crates/nono-cli/src/exec_strategy.rs (lines 270-326 to see the Unix ExecConfig with the D-20 doc-comment block; mirror this comment shape verbatim)
    - .planning/phases/35-upst3-closure-quick-wins/35-PATTERNS.md (Pattern Assignments § "Analog 1 — Unix ExecConfig field shape")
    - .planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/34-08a-ENV-SURFACE-PORT-SUMMARY.md (Plan 34-08a Task 3 + Task 4 record what `1b412a7` and `3657c935` introduced)
  </read_first>
  <behavior>
    - On compile, `ExecConfig` in `exec_strategy_windows/mod.rs` exposes two new `pub` fields: `allowed_env_vars: Option<Vec<String>>` and `denied_env_vars: Option<Vec<String>>`.
    - Both fields have the verbatim D-20 doc-comment block copied from `exec_strategy.rs:314-325` (Plan 34-08a Task 3 / Task 4 references; explains the upstream lineage to `1b412a7` + `3657c935`).
    - Neither field carries `#[cfg_attr(target_os = "windows", allow(dead_code))]` (per D-35-A1, the Windows-only-files invariant from D-34-E1 is explicitly inverted for this Phase 35 surface).
    - Adding the fields does not break any existing `ExecConfig { ... }` literal construction in the codebase — every existing call site must be updated to pass `allowed_env_vars: None, denied_env_vars: None` (or wire from `ExecutionFlags.allowed_env_vars` / `.denied_env_vars` per CONTEXT integration-point note).
  </behavior>
  <action>
    1. Open `crates/nono-cli/src/exec_strategy_windows/mod.rs`. Locate the `pub struct ExecConfig<'a> { ... }` at line 130.
    2. At the bottom of the struct, BEFORE the closing `}` (line 145), insert the two new fields with verbatim D-20 doc comments copied from `exec_strategy.rs` lines 314-325. The exact insertion text:
       ```rust
           /// Plan 34-08a Task 3 (D-20 manual replay of upstream `1b412a7`):
           /// allow-list of environment variable names. When `Some`, only
           /// variables matching an exact name or prefix pattern (e.g. `"AWS_*"`)
           /// are passed to the child. `None` means inherit-all (default).
           /// Nono-injected credentials (`config.env_vars`) always bypass this list.
           ///
           /// Plan 35-01 (REQ-PORT-CLOSURE-01, P34-DEFER-08a-1 closure): wired into
           /// Windows execution path via `launch::build_child_env`. Mirrors the
           /// Unix consumption site at `exec_strategy.rs:435-457` (Plan 34-08a
           /// Wave 2).
           pub allowed_env_vars: Option<Vec<String>>,
           /// Plan 34-08a Task 4 (D-20 manual-replay-by-escalation of upstream
           /// v0.52.0 `3657c935`): operator-controlled deny-list of environment
           /// variable names. Variables matching an exact name or prefix pattern
           /// (e.g. `"GITHUB_*"`) are stripped even if they also appear in
           /// `allowed_env_vars`. Nono-injected credentials bypass this list.
           ///
           /// Plan 35-01 (REQ-PORT-CLOSURE-01): wired into Windows execution path
           /// per Plan 35-01 — empty-allow fail-closed invariant from upstream
           /// `780965d7` is locked by the `test_windows_empty_allow_denies_all_env_vars`
           /// Windows-gated unit test.
           pub denied_env_vars: Option<Vec<String>>,
       ```
    3. Run `cargo build -p nono-cli --target x86_64-pc-windows-msvc 2>&1 | tee build.log` (or on Windows host: `cargo build -p nono-cli`). Compiler will flag every existing `ExecConfig { ... }` literal that does not name the new fields. Update each site by appending `, allowed_env_vars: None, denied_env_vars: None` (literal None — wiring from `ExecutionFlags` is task 2's call-site responsibility for the production launch path; test fixtures and supervised-launch fixtures should pass `None`).
    4. If a production call site (e.g., `command_runtime.rs`, `launch_runtime.rs`, `execution_runtime.rs`) constructs the Windows `ExecConfig` and already has access to `ExecutionFlags.allowed_env_vars` / `.denied_env_vars` (mirroring the Unix wiring in `crates/nono-cli/src/execution_runtime.rs`), thread those values through instead of hard-coding `None`. Search with `grep -rn 'ExecConfig {' crates/nono-cli/src/ --include='*.rs'` to find all sites; cross-reference each against the Unix `execution_runtime.rs` shape to decide threading vs `None`.
    5. NO removal of `#[cfg_attr(target_os = "windows", allow(dead_code))]` is required on the new fields themselves (they were never gated). DO NOT add such a gate.
  </action>
  <acceptance_criteria>
    - `cargo build -p nono-cli` on Windows host exits 0 (or on Linux host: `cargo build -p nono-cli --target x86_64-pc-windows-msvc` exits 0).
    - `grep -c 'pub allowed_env_vars: Option<Vec<String>>' crates/nono-cli/src/exec_strategy_windows/mod.rs` returns 1.
    - `grep -c 'pub denied_env_vars: Option<Vec<String>>' crates/nono-cli/src/exec_strategy_windows/mod.rs` returns 1.
    - `grep -c 'cfg_attr.*target_os = "windows".*allow.dead_code.*allowed_env_vars' crates/nono-cli/src/exec_strategy_windows/mod.rs` returns 0 (no dead_code gate added).
    - `grep -c 'Plan 34-08a Task 3' crates/nono-cli/src/exec_strategy_windows/mod.rs` returns 1 (the D-20 doc-comment provenance line is present verbatim).
    - `grep -c 'Plan 35-01 (REQ-PORT-CLOSURE-01' crates/nono-cli/src/exec_strategy_windows/mod.rs` returns at least 1 (the Plan 35-01 closure citation).
    - <automated>cargo build -p nono-cli 2>&1 | grep -E 'error\[|warning:' | grep -v 'unused import\|dead_code' | wc -l returns 0</automated>
  </acceptance_criteria>
  <verify>
    <automated>cargo build -p nono-cli</automated>
  </verify>
  <done>Windows `ExecConfig` carries the two env-filter fields with verbatim D-20 doc-comment blocks; all existing call sites updated; workspace compiles clean on Windows.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: Wire env-filter into Windows build_child_env + remove dead_code attributes</name>
  <files>crates/nono-cli/src/exec_strategy_windows/launch.rs, crates/nono-cli/src/exec_strategy/env_sanitization.rs</files>
  <read_first>
    - crates/nono-cli/src/exec_strategy_windows/launch.rs (lines 551-663 — the entire `build_child_env` function and the immediately following `append_windows_runtime_env`)
    - crates/nono-cli/src/exec_strategy.rs (lines 431-458 — the Unix `execute_direct` env-vars loop with the deny/allow filter arms to mirror)
    - crates/nono-cli/src/exec_strategy/env_sanitization.rs (lines 100-156 — `is_env_var_allowed`, `validate_env_var_patterns`, `is_env_var_denied`; note the two `#[allow(dead_code)]` attributes at lines 113 and 153 that must be removed)
    - crates/nono-cli/src/exec_strategy_windows/mod.rs (lines 20-21 + 77-78 — the existing `#[path = "../exec_strategy/env_sanitization.rs"] mod env_sanitization;` re-export and the `use env_sanitization::should_skip_env_var;` bring-in)
    - .planning/phases/35-upst3-closure-quick-wins/35-PATTERNS.md (Pattern Assignments § "Analog 2 — Unix filter call-site shape" and § "Analog 3 — Re-exports")
  </read_first>
  <behavior>
    - On any call to `build_child_env(&config)` where `config.denied_env_vars` is `Some(patterns)` and a process env-var key matches any pattern, that key is NOT pushed to `env_pairs`.
    - On any call to `build_child_env(&config)` where `config.allowed_env_vars` is `Some(patterns)` and a process env-var key does NOT match any pattern, that key is NOT pushed to `env_pairs`.
    - Precedence: dangerous-var check (`should_skip_env_var` returning true) > deny-list match > allow-list miss. All three skip the env_pairs push.
    - Nono-injected credentials (`config.env_vars` appended at lines 656-658) and the runtime block (`append_windows_runtime_env` at line 660) are NOT subject to the deny/allow filters — they are appended unconditionally AFTER the filter loop.
    - The two `#[allow(dead_code)]` attributes at `env_sanitization.rs:113` and `env_sanitization.rs:153` are removed; clippy passes cleanly under `-D warnings -D clippy::unwrap_used` on Windows + cross-target Linux + macOS.
  </behavior>
  <action>
    1. Open `crates/nono-cli/src/exec_strategy_windows/mod.rs`. At line 78, the existing `use env_sanitization::should_skip_env_var;` brings in only one helper. Extend the import to also bring in `is_env_var_allowed` and `is_env_var_denied`:
       ```rust
       use env_sanitization::{is_env_var_allowed, is_env_var_denied, should_skip_env_var};
       ```
       (Alphabetical order matches the existing rustfmt convention in this file.)
    2. Open `crates/nono-cli/src/exec_strategy_windows/launch.rs`. Locate the `pub(super) fn build_child_env(config: &ExecConfig<'_>) -> Vec<(String, String)>` at line 551. The current loop body (lines 553-647) reads:
       ```rust
       for (key, value) in std::env::vars() {
           if !should_skip_env_var(
               &key,
               &config.env_vars,
               &[ /* 60+ Windows runtime entries */ ],
           ) {
               env_pairs.push((key, value));
           }
       }
       ```
    3. Insert the two new filter arms INSIDE the existing `if !should_skip_env_var(...)` block, BEFORE `env_pairs.push((key, value));` on line 645. The deny check precedes the allow check (matches Unix precedence at `exec_strategy.rs:443-456`). Use the cross-crate `super::is_env_var_denied` / `super::is_env_var_allowed` imports added in step 1 (or qualify directly via the local `use` in launch.rs if more idiomatic). Exact insertion shape (insert immediately after line 643 `]`-closing of the blocked_extra list, but still inside the `if !should_skip_env_var(...)` body, just before the existing `env_pairs.push(...)`):
       ```rust
           ) {
               // Plan 35-01 (REQ-PORT-CLOSURE-01 / P34-DEFER-08a-1 closure):
               // mirror the Unix env-filter precedence from
               // exec_strategy.rs:443-456 (Plan 34-08a Wave 2 / D-20 replay
               // of upstream 1b412a7 + 3657c935). Deny-list checked BEFORE
               // allow-list; both bypassed by nono-injected credentials
               // (config.env_vars appended unconditionally below).
               if let Some(ref denied) = config.denied_env_vars {
                   if super::is_env_var_denied(&key, denied) {
                       continue;
                   }
               }
               if let Some(ref allowed) = config.allowed_env_vars {
                   if !super::is_env_var_allowed(&key, allowed) {
                       continue;
                   }
               }
               env_pairs.push((key, value));
           }
       ```
       Note: `continue` inside the `for` loop skips the push; the outer `if !should_skip_env_var(...)` already gates dangerous vars, so the new arms only run for non-dangerous keys (matches Unix shape).
    4. Open `crates/nono-cli/src/exec_strategy/env_sanitization.rs`. Remove the `#[allow(dead_code)]` attribute at line 113 (immediately above `pub(crate) fn is_env_var_allowed`) and the matching `#[allow(dead_code)]` at line 153 (above `pub(crate) fn is_env_var_denied`). Leave the doc-comment block above each function untouched. Update the doc comments to remove the "Windows execution path uses exec_strategy_windows; allow-list/deny-list wiring there ships separately." sentence since that sentence is now stale. Replace with: "Wired into Unix (`exec_strategy.rs:435-457`) AND Windows (`exec_strategy_windows/launch.rs::build_child_env`) execution paths."
    5. Verify the existing test `cargo test -p nono-cli --lib exec_strategy::env_sanitization::tests` still passes (the helpers themselves are unchanged; only the dead-code gate is removed).
  </action>
  <acceptance_criteria>
    - `grep -c '#\[allow(dead_code)\]' crates/nono-cli/src/exec_strategy/env_sanitization.rs` returns 0 (BOTH attributes removed).
    - `grep -c 'is_env_var_denied(&key, denied)' crates/nono-cli/src/exec_strategy_windows/launch.rs` returns 1.
    - `grep -c 'is_env_var_allowed(&key, allowed)' crates/nono-cli/src/exec_strategy_windows/launch.rs` returns 1.
    - `grep -c 'Plan 35-01 (REQ-PORT-CLOSURE-01' crates/nono-cli/src/exec_strategy_windows/launch.rs` returns at least 1.
    - The deny check appears BEFORE the allow check in source order (mirrors Unix precedence). Verify with `grep -n 'is_env_var_denied\|is_env_var_allowed' crates/nono-cli/src/exec_strategy_windows/launch.rs` — the denied line number is less than the allowed line number.
    - `cargo clippy -p nono-cli --all-targets -- -D warnings -D clippy::unwrap_used` (Windows host) exits 0.
    - `cargo clippy -p nono-cli --all-targets --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` exits 0 (cross-target Linux gate per Phase 25 CR-A lesson; D-35-D2 step 3).
    - `cargo test -p nono-cli --lib exec_strategy::env_sanitization::tests` passes (existing 19 tests from Plan 34-08a still green).
    - <automated>cargo test -p nono-cli --lib exec_strategy::env_sanitization::tests 2>&1 | grep -c 'test result: ok' returns 1</automated>
  </acceptance_criteria>
  <verify>
    <automated>cargo test -p nono-cli --lib exec_strategy::env_sanitization::tests &amp;&amp; cargo clippy -p nono-cli --all-targets -- -D warnings -D clippy::unwrap_used</automated>
  </verify>
  <done>Windows `build_child_env` consumes deny + allow filters with correct precedence; `env_sanitization.rs` dead-code gates removed; existing tests still pass; clippy clean on Windows + cross-target Linux.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 3: Add Windows-gated regression tests locking empty-allow fail-closed invariant</name>
  <files>crates/nono-cli/src/exec_strategy_windows/launch.rs</files>
  <read_first>
    - crates/nono-cli/src/exec_strategy_windows/launch.rs (lines 1869-1961 — `pty_token_gate_tests` for the test-module shape; lines 1963+ for the `low_integrity_primary_token_tests` if it exists for cross-platform-gate examples)
    - crates/nono-cli/src/exec_strategy_windows/launch.rs (lines 551-663 — the `build_child_env` function under test)
    - crates/nono-cli/src/exec_strategy/env_sanitization.rs (the `is_env_var_allowed` / `is_env_var_denied` invariant tests for design reference; lines 158+ contain the existing `tests` module)
    - CLAUDE.md § "Environment variables in tests" — the save/restore guard pattern for `std::env::set_var` / `std::env::remove_var` (mandatory to avoid flaky parallel-test failures)
    - .planning/phases/35-upst3-closure-quick-wins/35-PATTERNS.md § "Analog 4 — Windows-gated unit-test shape"
  </read_first>
  <behavior>
    - `test_windows_empty_allow_denies_all_env_vars`: constructs an `ExecConfig` with `allowed_env_vars: Some(vec![])` (empty allow-list) and `denied_env_vars: None`, then asserts that `build_child_env(&config)` returns a `Vec<(String, String)>` whose keys, after removing the known Windows runtime entries appended by `append_windows_runtime_env` and the keys in `config.env_vars`, contains ZERO inherited user env vars. (i.e., empty-allow fails closed — strips all inherited variables.)
    - `test_windows_deny_strips_matching_env_vars`: seeds a fixture env var (via the save/restore guard from CLAUDE.md), constructs `ExecConfig` with `denied_env_vars: Some(vec!["NONO_TEST_FIXTURE_*".to_string()])` and `allowed_env_vars: None`, asserts the fixture key is NOT present in the returned `env_pairs`.
    - `test_windows_allow_passes_only_matching_env_vars`: seeds two fixture env vars; `allowed_env_vars: Some(vec!["NONO_TEST_FIXTURE_KEEP".to_string()])`; asserts the KEEP key passes and the other does not.
    - `test_windows_nono_injected_credentials_bypass_both`: `config.env_vars = vec![("NONO_INJECTED_CRED", "secret")]`, `allowed_env_vars: Some(vec![])`, `denied_env_vars: Some(vec!["NONO_INJECTED_CRED".to_string()])`. Asserts `NONO_INJECTED_CRED` IS in the returned env_pairs (injected credentials bypass both filters per the documented invariant).
    - All four tests gated `#[cfg(all(test, target_os = "windows"))]` and saved/restored env vars per CLAUDE.md.
  </behavior>
  <action>
    1. At the bottom of `crates/nono-cli/src/exec_strategy_windows/launch.rs` (after the existing test modules `pty_token_gate_tests` and any subsequent `#[cfg(all(test, target_os = "windows"))]` modules), insert a new `#[cfg(all(test, target_os = "windows"))]` test module named `env_filter_tests`. The module imports `super::build_child_env` and `super::super::ExecConfig` (or whatever path resolves from inside the test module — likely `super::{build_child_env, ExecConfig}`; verify by reading the existing `pty_token_gate_tests` import shape).
    2. Implement an env-var save/restore guard struct at the top of the test module per CLAUDE.md (`struct EnvGuard { key: String, prior: Option<OsString> }` with `Drop` impl that restores via `std::env::set_var` if `prior.is_some()` else `std::env::remove_var`). Constructor `EnvGuard::set(key, value)` saves prior via `std::env::var_os(&key)`, then sets the new value.
    3. Implement a small helper that constructs a minimal valid `ExecConfig` for testing — most fields can be defaulted to sentinel values (`command: &[]`, `resolved_program: Path::new(""), caps: &CapabilitySet::new(), ...` — match the existing pattern in `pty_token_gate_tests` if it exists, otherwise extract from a real call site).
    4. Implement the four tests as described in <behavior>. For each test:
       - **`test_windows_empty_allow_denies_all_env_vars`**:
         ```rust
         let _guard = EnvGuard::set("NONO_TEST_EMPTY_ALLOW_FIXTURE", "should_be_stripped");
         let config = make_minimal_exec_config(/* allow */ Some(vec![]), /* deny */ None, /* env_vars */ vec![]);
         let env_pairs = build_child_env(&config);
         // The runtime allowlist (PATH, SystemRoot, etc.) and append_windows_runtime_env
         // both bypass the new allow/deny filter, but the fixture key (which is NOT in
         // the runtime allowlist) MUST NOT appear.
         assert!(
             !env_pairs.iter().any(|(k, _)| k == "NONO_TEST_EMPTY_ALLOW_FIXTURE"),
             "Empty allow-list MUST strip non-runtime inherited env vars (fail-closed invariant from upstream 780965d7)"
         );
         ```
       - **`test_windows_deny_strips_matching_env_vars`**:
         ```rust
         let _guard = EnvGuard::set("NONO_TEST_DENY_FIXTURE_A", "should_be_stripped");
         let config = make_minimal_exec_config(
             /* allow */ None,
             /* deny */ Some(vec!["NONO_TEST_DENY_FIXTURE_*".to_string()]),
             /* env_vars */ vec![],
         );
         let env_pairs = build_child_env(&config);
         assert!(!env_pairs.iter().any(|(k, _)| k == "NONO_TEST_DENY_FIXTURE_A"));
         ```
       - **`test_windows_allow_passes_only_matching_env_vars`**:
         ```rust
         let _g1 = EnvGuard::set("NONO_TEST_ALLOW_FIXTURE_KEEP", "passes");
         let _g2 = EnvGuard::set("NONO_TEST_ALLOW_FIXTURE_DROP", "stripped");
         let config = make_minimal_exec_config(
             /* allow */ Some(vec!["NONO_TEST_ALLOW_FIXTURE_KEEP".to_string()]),
             /* deny */ None,
             /* env_vars */ vec![],
         );
         let env_pairs = build_child_env(&config);
         assert!(env_pairs.iter().any(|(k, v)| k == "NONO_TEST_ALLOW_FIXTURE_KEEP" && v == "passes"));
         assert!(!env_pairs.iter().any(|(k, _)| k == "NONO_TEST_ALLOW_FIXTURE_DROP"));
         ```
       - **`test_windows_nono_injected_credentials_bypass_both`**:
         ```rust
         let config = make_minimal_exec_config(
             /* allow */ Some(vec![]),
             /* deny */ Some(vec!["NONO_INJECTED_CRED".to_string()]),
             /* env_vars */ vec![("NONO_INJECTED_CRED", "secret")],
         );
         let env_pairs = build_child_env(&config);
         assert!(
             env_pairs.iter().any(|(k, v)| k == "NONO_INJECTED_CRED" && v == "secret"),
             "Nono-injected credentials MUST bypass both allow-list and deny-list filters"
         );
         ```
    5. Run the new tests on the Windows dev host. All four must exit 0.
  </action>
  <acceptance_criteria>
    - `grep -c 'fn test_windows_empty_allow_denies_all_env_vars' crates/nono-cli/src/exec_strategy_windows/launch.rs` returns 1.
    - `grep -c 'fn test_windows_deny_strips_matching_env_vars' crates/nono-cli/src/exec_strategy_windows/launch.rs` returns 1.
    - `grep -c 'fn test_windows_allow_passes_only_matching_env_vars' crates/nono-cli/src/exec_strategy_windows/launch.rs` returns 1.
    - `grep -c 'fn test_windows_nono_injected_credentials_bypass_both' crates/nono-cli/src/exec_strategy_windows/launch.rs` returns 1.
    - `grep -c 'cfg(all(test, target_os = "windows"))' crates/nono-cli/src/exec_strategy_windows/launch.rs` returns at least 2 (existing `pty_token_gate_tests` + new `env_filter_tests`; the second may match `#[cfg(all(test, target_os = "windows"))]` rather than the simpler form).
    - `grep -c 'struct EnvGuard' crates/nono-cli/src/exec_strategy_windows/launch.rs` returns 1 (the save/restore guard from CLAUDE.md is implemented; no bare `std::env::set_var` without a Drop guard).
    - `grep -c '\.unwrap()' crates/nono-cli/src/exec_strategy_windows/launch.rs` does NOT grow vs. the pre-Plan-35 baseline (CLAUDE.md no-unwrap policy).
    - On Windows host: `cargo test -p nono-cli --lib exec_strategy_windows::launch::env_filter_tests` exits 0 and the output line `test result: ok. 4 passed; 0 failed` is present.
    - <automated>cargo test -p nono-cli --lib exec_strategy_windows::launch::env_filter_tests 2>&amp;1 | grep -cE 'test_windows_empty_allow_denies_all_env_vars.*ok|test_windows_deny_strips_matching_env_vars.*ok|test_windows_allow_passes_only_matching_env_vars.*ok|test_windows_nono_injected_credentials_bypass_both.*ok' returns 4</automated>
  </acceptance_criteria>
  <verify>
    <automated>cargo test -p nono-cli --lib exec_strategy_windows::launch::env_filter_tests</automated>
  </verify>
  <done>Four Windows-gated regression tests lock the empty-allow fail-closed invariant + deny/allow precedence + nono-injected-credential bypass. All four pass deterministically on Windows host. No flaky env-var leakage across parallel tests.</done>
</task>

</tasks>

<threat_model>

## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| operator CLI → nono-cli supervisor | Operator passes `--env-deny`/`--env-allow` patterns from a trusted shell to the supervisor. |
| nono-cli supervisor → sandboxed child (Windows) | The supervisor constructs the child PEB env block in `build_child_env`. Variables crossing this boundary must respect the operator's filter intent. |
| Process env table → sandboxed child | Inherited host env vars are untrusted from the child's perspective (they may carry credentials, session tokens, paths the operator wants to block). The filter is the trust boundary. |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-35-01-01 | Information disclosure | `build_child_env` allow-list logic | mitigate | Empty-allow MUST fail closed (`allow_vars: []` strips all inherited vars). Locked by `test_windows_empty_allow_denies_all_env_vars` regression test. The `is_env_var_allowed` helper implements bare-`*` semantics in `env_sanitization.rs:114`; the new Windows wiring inherits the same fail-closed shape. |
| T-35-01-02 | Tampering | Filter precedence | mitigate | Deny-list MUST be checked BEFORE the allow-list (deny wins). Source order in `launch.rs::build_child_env` enforces this; `test_windows_deny_strips_matching_env_vars` regression test verifies that an env var present in BOTH lists is denied. |
| T-35-01-03 | Information disclosure | Pattern parsing accepting unsafe wildcards | accept | The `validate_env_var_patterns` helper (called at profile-load time by `profile_runtime.rs`) already rejects mid-string `*` patterns and leading `*` longer than 1 char. The filter consumers (`is_env_var_allowed`/`is_env_var_denied`) call `matches_env_var_patterns` which only honors trailing-suffix `*`. No new attack surface introduced by the Windows wiring; the validation gate lives upstream of the new code. |
| T-35-01-04 | Elevation of privilege | Nono-injected credential bypass | mitigate | Credentials in `config.env_vars` (nono's own KEY=secret injections) are appended UNCONDITIONALLY after the filter loop (launch.rs:656-658), bypassing both filters by construction. This is intentional and locked by `test_windows_nono_injected_credentials_bypass_both`. Operators relying on `--env-deny` to strip nono-injected secrets would be misled; the design choice is consistent with Unix (`exec_strategy.rs:465-467`). |
| T-35-01-05 | Information disclosure | Dead `#[allow(dead_code)]` removal | accept | Removing the two dead-code attributes from `env_sanitization.rs:113` and `:153` does not change runtime behavior — both helpers were already reachable via the `#[path]` re-export from `exec_strategy_windows/mod.rs`. The risk is purely lint-noise; verified by clippy passing post-removal. |

</threat_model>

<verification_criteria>

## Phase 34 Close-Gate (D-35-D2 inherited verbatim — all 8 steps)

1. `cargo test --workspace --all-features` (Windows host) exits 0.
2. `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used` (Windows host) exits 0.
3. `cargo clippy --workspace --all-targets --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` exits 0 (cross-target Linux gate — Phase 25 CR-A lesson; required because step 2 cannot lint inside `#[cfg(target_os = "linux")]` arms of any cross-platform file Plan 35-01 may have touched indirectly).
4. `cargo clippy --workspace --all-targets --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used` exits 0 (cross-target macOS gate; symmetric coverage).
5. `cargo fmt --all -- --check` exits 0.
6. Phase 15 5-row detached-console smoke gate (Plan 35-01 ONLY among Phase 35 plans): manually run `nono run --detached -- powershell -c "Start-Sleep -Seconds 10"`, then `nono ps`, then `nono attach <id>`, detach via the configured detach sequence, then `nono stop <id>`. All 5 transitions succeed. Document the run in the Plan 35-01 SUMMARY.
7. `wfp_port_integration` test suite passes OR documented-skipped with reason `admin/service-not-available`. Record the disposition in SUMMARY.
8. `learn_windows_integration` test suite passes OR documented-skipped with reason. Record disposition in SUMMARY.

## Plan-Specific Verification

- **PSV-1:** `git log --format='%B' -1 main | grep -c '^Upstream-commit: '` returns 0 for the Plan 35-01 commit (D-35-A4 — D-20 manual-replay shape, no D-19 trailer block; commit body cites Plan 34-08a + upstream `1b412a7` + `780965d7` as design-source citations only).
- **PSV-2:** `git log --format='%B' -1 main | grep -cE 'Plan 34-08a|1b412a7|780965d7'` returns at least 3 (commit body references the design sources per D-34-E3 manual-port shape).
- **PSV-3:** `git diff --stat HEAD~1 HEAD -- crates/nono-cli/src/ | grep -cE '_windows\.rs|exec_strategy_windows/' ` returns at least 1 (the Phase 35 D-35-A1 inversion explicitly permits this — the only Phase 35 plan that touches Windows-only files). `git diff --stat HEAD~1 HEAD -- crates/nono-cli/src/ | grep -cE 'learn_windows|pty_proxy_windows|session_commands_windows|win_runtime|windows_wfp|net_filter_windows|process_windows|sandbox_windows|svc_runtime|broker_runtime|network_runtime_windows'` returns 0 (no edits to other Windows-only files — D-34-E1 still holds for the unrelated Windows surfaces).
- **PSV-4:** The new Windows-gated tests `env_filter_tests::test_windows_empty_allow_denies_all_env_vars`, `::test_windows_deny_strips_matching_env_vars`, `::test_windows_allow_passes_only_matching_env_vars`, `::test_windows_nono_injected_credentials_bypass_both` all pass on Windows host. Verified via `cargo test -p nono-cli --lib exec_strategy_windows::launch::env_filter_tests 2>&1 | grep 'test result: ok. 4 passed'`.

## Acceptance Criteria Mapping (REQ-PORT-CLOSURE-01)

1. ✓ `nono run --env-deny KEY -- powershell -c '$env:KEY'` on Windows strips `KEY` from the child env — covered by Task 2 wiring + Task 3 `test_windows_deny_strips_matching_env_vars`.
2. ✓ `nono run --env-allow KEY1,KEY2 -- <cmd>` on Windows strips all env vars except KEY1, KEY2 — covered by Task 2 wiring + Task 3 `test_windows_allow_passes_only_matching_env_vars`.
3. ✓ Empty-allow fail-closed invariant holds — Task 3 `test_windows_empty_allow_denies_all_env_vars`.
4. ✓ New Windows-gated tests covering deny_vars precedence + empty-allow fail-closed — Task 3 (4 tests).

</verification_criteria>

<success_criteria>

- Plan 35-01 closes P34-DEFER-08a-1 (Plan SUMMARY appends closure-section ledger entry).
- Workspace compiles, tests pass, clippy clean, fmt clean on Windows + cross-target Linux + cross-target macOS.
- All four new `env_filter_tests` pass deterministically (no env-var leakage in parallel runs — save/restore guards present per CLAUDE.md).
- No `*_windows.rs` edits outside `exec_strategy_windows/mod.rs` + `launch.rs` (D-35-A1 scope ceiling).
- Commit lands on `main` with D-20 manual-replay shape (commit body cites Plan 34-08a + `1b412a7` + `780965d7`; NO D-19 trailer; DCO sign-off present).
- No `.unwrap()` introduced (CLAUDE.md § Coding Standards).
- Phase 35 D-35-D2 close-gate steps 1-8 all green or documented-skipped.

</success_criteria>

<output>
After completion, create `.planning/phases/35-upst3-closure-quick-wins/35-01-WIN-ENV-FILTER-SUMMARY.md` with:
- Frontmatter recording the commit SHA, fork-defense grep baselines (allowed_env_vars/denied_env_vars usage), test pass counts (`env_filter_tests` = 4 passed; `env_sanitization::tests` = 19+ pre-existing).
- Body documenting: D-20 manual-replay shape, design sources cited (Plan 34-08a, `1b412a7`, `780965d7`), the D-35-A1 explicit inversion of D-34-E1 for this plan only, close-gate disposition for each of the 8 D-35-D2 steps.
- Closure-section ledger entry: marks P34-DEFER-08a-1 as `closed-by-Phase-35-01` (the consolidated append to Phase 34's `deferred-items.md` is owned by Plan 35-03 per D-35-D4).
</output>
