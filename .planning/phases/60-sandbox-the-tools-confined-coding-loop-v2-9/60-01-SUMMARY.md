---
phase: "60"
plan: "01"
subsystem: nono-cli/claude_code_hook
tags: [hook, windows, confinement, file-ops, powershell, sandbox]
dependency_graph:
  requires: []
  provides: [confined-write-arm, confined-edit-arm, confined-multiedit-arm, notebookedit-informative-deny]
  affects: [crates/nono-cli/src/claude_code_hook.rs]
tech_stack:
  added: []
  patterns: [deny+additionalContext, bare-inner-PS-expression, match-dispatch, base64-content-encoding]
key_files:
  modified:
    - crates/nono-cli/src/claude_code_hook.rs
decisions:
  - "Replaced blanket `if tool_name != Bash` deny with match dispatch; Write/Edit/MultiEdit get confined arms with D-05 guard first"
  - "Removed explicit #[must_use] from build_confined_* helpers — Result<T> is already must_use, explicit attr triggered clippy::double_must_use"
  - "Updated pre_tool_use_file_tools_deny test to supply per-tool tool_input (Edit needs old_string/new_string, MultiEdit needs edits array) — required for the new arms to return deny instead of Err on missing fields"
  - "Test 6 (windows_write_arm_cwd_guard_fires_before_ps_cmd) uses option (b): calls cwd_self_disable_risk_reason_for() directly, not pre_tool_use_response — avoids set_current_dir flakiness per CLAUDE.md"
metrics:
  duration_minutes: 55
  completed_date: "2026-05-29"
  tasks_completed: 2
  tasks_total: 2
  files_changed: 1
---

# Phase 60 Plan 01: Confined Write/Edit/MultiEdit Arms — Summary

## One-Liner

Write/Edit/MultiEdit hook arms that deny in-process tool calls and emit a bare inner PowerShell command via additionalContext, enabling Claude to retry as a Low-IL confined Bash subprocess; NotebookEdit kept as informative deny.

## What Was Built

Extended `crates/nono-cli/src/claude_code_hook.rs` to implement the core REQ-STW-01 mechanism:

### Dispatch Restructure (Task 1)

Replaced the blanket `if tool_name != "Bash"` deny guard with a `match tool_name` block that dispatches Write/Edit/MultiEdit/NotebookEdit/Bash/everything-else as distinct arms.

Each confined arm (Write, Edit, MultiEdit) follows the D-05 protocol:
1. CWD self-disable guard fires first (`cwd_self_disable_risk_reason()`) — if the process CWD covers `~/.claude` or a project `.claude/` child, returns deny without any PS command
2. Extracts `tool_input` fields via fail-secure `ok_or_else(|| NonoError::HookInstall(...))` chains (no unwrap)
3. Calls the appropriate builder to get the bare inner PS expression
4. Returns deny + `additionalContext` via `json!({...})` inline (not via `deny_response()` helper which cannot include extra fields)

### Three New Windows-Gated Helper Functions (Task 1)

**`build_confined_write_cmd(file_path, content) -> Result<String>`**
- Base64-encodes `content` (raw bytes, not UTF-16LE) to safely handle single quotes, newlines, and PS metacharacters
- Returns inner PS: `$b = [Convert]::FromBase64String('...'); [System.IO.File]::WriteAllText('path', [System.Text.Encoding]::UTF8.GetString($b))`
- Does NOT call `wrapped_bash_command` — returns bare inner expression only

**`build_confined_edit_cmd(file_path, old_string, new_string) -> Result<String>`**
- Uses `String.Replace()` method (literal, not `-replace` regex) to avoid regex metacharacter injection (RESEARCH.md Pitfall 1)
- Returns inner PS: read-replace-write-back in a single statement chain

**`build_confined_multiedit_cmd(file_path, edits: &Value) -> Result<String>`**
- Reads file once into `$content`, applies all `old_string.Replace(new_string)` calls in-memory in sequence, writes once via `WriteAllText` (RESEARCH.md Pitfall 4 avoided)
- Validates each edit element's `old_string`/`new_string` with fail-secure errors

### Non-Windows Stubs

Each confined arm has a `#[cfg(not(target_os = "windows"))]` path that returns a deny without additionalContext (`"<Tool> confinement via Bash retry is Windows-only"`). This preserves the deny behavior on Linux/macOS without requiring the PS helpers to be compiled.

### Module Doc Comment Update (Task 1)

Removed the stale sentence "In-process tools such as Write and Edit cannot be made Low-IL by command rewriting." Added the correct description of the new confined-arm mechanism.

### 6 New Unit Tests + Updated Existing Test (Task 2)

Added to the existing `#[cfg(test)] mod tests` block:

| Test | Platform | What It Verifies |
|------|----------|-----------------|
| `pre_tool_use_write_returns_deny_with_ps_cmd` | cross-platform | Write returns deny + non-empty additionalContext (Windows) |
| `pre_tool_use_edit_returns_deny_with_ps_cmd` | cross-platform | Edit returns deny + non-empty additionalContext (Windows) |
| `pre_tool_use_multiedit_returns_deny_with_ps_cmd` | cross-platform | MultiEdit returns deny + non-empty additionalContext (Windows) |
| `pre_tool_use_notebookedit_deny_no_ps_cmd` | cross-platform | NotebookEdit returns deny, additionalContext absent/empty |
| `pre_tool_use_write_content_with_special_chars` | Windows-gated | Write with `'` + `\n` in content encodes without error |
| `windows_write_arm_cwd_guard_fires_before_ps_cmd` | Windows-gated | `cwd_self_disable_risk_reason_for()` fires for a CWD with `.claude/` child |

Updated `pre_tool_use_file_tools_deny` to:
- Use per-tool `tool_input` (Edit needs `old_string`/`new_string`, MultiEdit needs `edits` array) so arms return deny instead of propagating `Err`
- Add Windows-gated assertion that Write/Edit/MultiEdit have non-empty `additionalContext`
- Add cross-platform assertion that NotebookEdit has absent/empty `additionalContext`

## Test Results

```
cargo test --bin nono claude_code_hook: 14 passed; 0 failed
clippy --workspace -D warnings -D clippy::unwrap_used: clean (0 errors)
No .unwrap() or .expect() in any new production code
```

Pre-existing full-suite failures (6 tests, unchanged from baseline):
- `broker_dispatch_tests::broker_launch_assigns_child_to_job_object` (requires elevated privileges)
- `write_deny_low_il_broker_no_pty_tests::...` (requires elevated privileges)
- `profile_cmd::tests::test_init_allowed_when_pack_has_same_short_name` (pre-existing)
- `protected_paths::tests::*` (3 tests, pre-existing — require specific test environment)

These are not related to Phase 60 changes.

## Verification Against Plan Success Criteria

| Criterion | Status |
|-----------|--------|
| Write arm: permissionDecision deny AND non-empty additionalContext | PASS — test confirms |
| Edit arm: same shape | PASS — test confirms |
| MultiEdit arm: same shape | PASS — test confirms |
| NotebookEdit arm: permissionDecision deny AND additionalContext absent | PASS — test confirms |
| D-05 guard fires first in Write/Edit/MultiEdit arms | PASS — code order verified + test 6 covers guard logic |
| Three new #[cfg(target_os = "windows")] helpers exist | PASS |
| cargo test --bin nono claude_code_hook exits 0 | PASS (14/14) |
| No .unwrap() or .expect() in new code | PASS (grep confirms) |
| Commits include DCO Signed-off-by | PASS (both commits) |
| clippy -D warnings -D clippy::unwrap_used: clean | PASS |
| Cross-target clippy | PARTIAL — Windows host only; deferred to CI (see below) |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed pre_tool_use_file_tools_deny test to supply per-tool tool_input**
- **Found during:** Task 1 verification (`cargo test --bin nono claude_code_hook`)
- **Issue:** The existing test used `{"file_path": "x.txt", "content": "x"}` for ALL tools. After the dispatch restructure, the `Edit` arm requires `old_string` and `new_string` fields; their absence causes `Err(NonoError::HookInstall(...))` which the test propagates rather than catching as a deny response.
- **Fix:** Replaced the single shared `tool_input` with per-tool inputs in the test loop. `Edit` gets `{"file_path": ..., "old_string": "foo", "new_string": "bar"}`, `MultiEdit` gets a proper `edits` array, etc. This is the correct behavior — the test now validates that well-formed tool calls are denied, not that malformed calls error.
- **Files modified:** `crates/nono-cli/src/claude_code_hook.rs` (test block)
- **Committed in:** `1c4de489`

**2. [Rule 1 - Bug] Removed #[must_use] from build_confined_* helpers**
- **Found during:** Task 2 clippy run
- **Issue:** Explicit `#[must_use]` on functions returning `Result<String>` triggers `clippy::double_must_use` (implied by `-D warnings`) since `Result<T>` is already annotated `#[must_use]` in std.
- **Fix:** Removed the `#[must_use]` attribute from all three helper functions. The `must_use` invariant is still enforced by `Result<T>`'s own attribute.
- **Files modified:** `crates/nono-cli/src/claude_code_hook.rs`
- **Committed in:** `b0e9ff7e`

## Cross-Target Clippy: PARTIAL

Per CLAUDE.md cross-target requirement: the new Write/Edit/MultiEdit arms and the three `build_*` helpers are all gated `#[cfg(target_os = "windows")]`. The non-Windows path returns deny without PS command. Windows-host clippy confirmed clean. Linux/macOS cross-target clippy is deferred to CI:

- `cargo clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` — deferred to CI linux runner
- `cargo clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used` — deferred to CI macOS runner

## Known Stubs

None. The confined-arm mechanism is fully wired — `build_confined_write_cmd`, `build_confined_edit_cmd`, and `build_confined_multiedit_cmd` produce real PS expressions. The `deny+additionalContext` path is complete. UAT (whether Claude reliably re-issues the Bash retry) is a human verification gate documented in VALIDATION.md — this is by design, not a stub.

## Threat Surface Scan

No new network endpoints, auth paths, or file access patterns introduced. The three helper functions accept `&str` parameters (file_path, content, old_string, new_string) from the hook's JSON input and produce PS command strings. All threat mitigations from the plan's threat register are implemented:

| Threat ID | Status |
|-----------|--------|
| T-60-01 (file_path tampering) | Mitigated — path passed through `powershell_single_quoted()` + OS CWD grant is the enforcement boundary |
| T-60-02 (content tampering) | Mitigated — content base64-encoded, decoded inside PS command |
| T-60-03 (old_string/new_string tampering) | Mitigated — `powershell_single_quoted()` + `String.Replace()` (literal, not regex) |
| T-60-04 (self-disable via Write) | Mitigated — `cwd_self_disable_risk_reason()` is first call in each confined arm |
| T-60-05 (path escape via ../..) | Mitigated — OS MIC CWD grant is the enforcement boundary (not hook-level string check) |
| T-60-06 (NotebookEdit complexity) | Accepted — informative deny, no PS command |
| T-60-07 (MultiEdit partial-edit state) | Mitigated — read once, apply all in-memory, write once |

## Commits

| Task | Commit | Message |
|------|--------|---------|
| Task 1 | `1c4de489` | feat(60-01): add confined Write/Edit/MultiEdit arms + NotebookEdit deny |
| Task 2 | `b0e9ff7e` | test(60-01): add 6 new unit tests for confined arms + update deny test |

## Self-Check: PASSED

- FOUND: `crates/nono-cli/src/claude_code_hook.rs` — modified file exists
- FOUND: `.planning/phases/60-sandbox-the-tools-confined-coding-loop-v2-9/60-01-SUMMARY.md` — this file
- FOUND commit `1c4de489` — Task 1 feat commit
- FOUND commit `b0e9ff7e` — Task 2 test commit
- `cargo test --bin nono claude_code_hook`: 14/14 pass (verified)
- `cargo clippy --workspace -D warnings -D clippy::unwrap_used`: clean (verified)
