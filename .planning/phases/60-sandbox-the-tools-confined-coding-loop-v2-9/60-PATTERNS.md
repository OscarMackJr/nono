# Phase 60: Sandbox-the-Tools — Confined Coding Loop - Pattern Map

**Mapped:** 2026-05-29
**Files analyzed:** 3 (1 primary, 2 supporting)
**Analogs found:** 3 / 3 (all changes are extensions of existing patterns within the same files)

---

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `crates/nono-cli/src/claude_code_hook.rs` | middleware / hook handler | request-response (tool intercept) | Same file — existing `Bash` arm + `cwd_self_disable_risk_reason` guard | exact |
| `packages/claude-code/CLAUDE.md` | config / model-visible instruction | N/A (documentation) | Same file — existing "Constraints" section heading pattern | exact |
| `packages/claude-code/claude-code-tools-windows-runner.profile.json` | config | N/A (JSON profile) | Same file — verified no changes needed; runner profile already supplies CWD r+w and network block | verification-only |

---

## Pattern Assignments

### `crates/nono-cli/src/claude_code_hook.rs` — Write/Edit/MultiEdit arms (middleware, request-response)

**Change site:** Replace the single `if tool_name != "Bash"` guard (lines 56–60) with a match or extended if-chain that carves out `Write`, `Edit`, `MultiEdit`, and `NotebookEdit` as distinct arms before the fall-through deny.

**Analog within same file:** The existing `Bash` arm — lines 56–100.

#### Pattern 1: The guard gate (analog: lines 62–65)

The Windows CWD self-disable guard is the first call in the `Bash` arm body. The new Write/Edit arms must call it first in exactly the same position:

```rust
// Source: claude_code_hook.rs lines 62-65 — COPY THIS GATE INTO EACH NEW ARM
#[cfg(target_os = "windows")]
if let Some(reason) = cwd_self_disable_risk_reason()? {
    return Ok(Some(deny_response(reason)));
}
```

#### Pattern 2: tool_input extraction (analog: lines 67–81)

The `Bash` arm extracts `tool_input` from the event and then a field from it, returning a typed `Err` when the field is absent:

```rust
// Source: claude_code_hook.rs lines 67-81 — the field-extraction shape to mirror for file_path/content
let mut updated_input = event
    .get("tool_input")
    .cloned()
    .ok_or_else(|| NonoError::HookInstall("PreToolUse input missing tool_input".to_string()))?;
// ... then field from updated_input:
let command = tool_input
    .get("command")
    .and_then(Value::as_str)
    .ok_or_else(|| {
        NonoError::HookInstall("Bash tool_input missing string command".to_string())
    })?;
```

New arms extract `file_path` and `content` / `old_string` / `new_string` using the same `.get(field).and_then(Value::as_str).ok_or_else(|| NonoError::HookInstall(...))` chain.

#### Pattern 3: deny_response with reason (analog: lines 56–59 + lines 116–124)

The generic deny arm and the `deny_response` helper show the response shape. New arms emit `deny` with `additionalContext` added:

```rust
// Source: claude_code_hook.rs lines 116-124 — existing deny_response helper
fn deny_response(reason: impl Into<String>) -> Value {
    json!({
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "permissionDecision": "deny",
            "permissionDecisionReason": reason.into()
        }
    })
}
```

The new confined-file-op deny adds `"additionalContext"` alongside `"permissionDecisionReason"` in the same `hookSpecificOutput` object. Use `json!({...})` inline (same as the `allow` response at lines 92–99) rather than calling `deny_response()` so the extra field can be included:

```rust
// Source: claude_code_hook.rs lines 92-99 — the allow arm's json!({}) shape is the model
Ok(Some(json!({
    "hookSpecificOutput": {
        "hookEventName": "PreToolUse",
        "permissionDecision": "deny",
        "permissionDecisionReason": "<reason>",
        "additionalContext": format!("Use the Bash tool with this PowerShell command: {ps_inner}")
    }
})))
```

#### Pattern 4: PowerShell base64 encoding (analog: lines 200–219, 237–248)

All PowerShell command construction MUST route through the existing helpers — never hand-roll:

```rust
// Source: claude_code_hook.rs lines 200-219 — wrapped_bash_command (Windows path)
#[cfg(target_os = "windows")]
fn wrapped_bash_command(command: &str) -> Result<String> {
    let nono_exe = std::env::current_exe().map_err(|e| {
        NonoError::HookInstall(format!("failed to resolve current nono executable: {e}"))
    })?;
    let outer_script = windows_bash_wrapper_script(&nono_exe.display().to_string(), command);
    let encoded = powershell_encoded_command(&outer_script);
    Ok(format!(
        "powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -EncodedCommand {encoded}"
    ))
}

// Source: claude_code_hook.rs lines 212-219 — windows_bash_wrapper_script
#[cfg(target_os = "windows")]
fn windows_bash_wrapper_script(nono_exe: &str, command: &str) -> String {
    format!(
        "& {nono_exe} run --profile {profile} --allow-cwd -- powershell.exe -NoProfile -NonInteractive -EncodedCommand {encoded_command}",
        nono_exe = powershell_single_quoted(nono_exe),
        profile = powershell_single_quoted(DEFAULT_TOOL_RUNNER_PROFILE),
        encoded_command = powershell_encoded_command(command),
    )
}

// Source: claude_code_hook.rs lines 241-248 — powershell_encoded_command (UTF-16LE base64)
#[cfg(target_os = "windows")]
fn powershell_encoded_command(command: &str) -> String {
    let utf16le = command
        .encode_utf16()
        .flat_map(u16::to_le_bytes)
        .collect::<Vec<_>>();
    nono::trust::base64::base64_encode(&utf16le)
}

// Source: claude_code_hook.rs lines 236-239 — powershell_single_quoted (embedded-quote safe)
#[cfg(target_os = "windows")]
fn powershell_single_quoted(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}
```

The new `build_confined_write_cmd` / `build_confined_edit_cmd` / `build_confined_multiedit_cmd`
helpers return the BARE INNER PS EXPRESSION — they MUST NOT call `wrapped_bash_command`.
The `additionalContext` instructs Claude: "Use the Bash tool with this PowerShell command: {ps_inner}".
When Claude retries via the Bash arm, the Bash arm calls `wrapped_bash_command(ps_inner)` exactly once.

Rationale: `command_contains_nono_wrapper` (lines 126-128) checks for the literal string
` nono run ` in the command text. If build_* called `wrapped_bash_command`, the outer
`powershell.exe -EncodedCommand <base64>` result has ` nono run ` hidden inside the
base64 blob — the check misses it and the Bash arm wraps a second time. The inner PS
expression (e.g. `[System.IO.File]::WriteAllText(...)`) contains no ` nono run ` literal,
so the single Bash-arm wrap is the only wrap.

#### Pattern 5: The `is_read_only_tool` dispatch point (analog: lines 51–60)

The current control flow branches at `is_read_only_tool` then falls through to `if tool_name != "Bash"`. The change site is this second guard. Replace it with a match arm or extended if-chain:

```rust
// Source: claude_code_hook.rs lines 56-60 — CURRENT code; this is the exact change site
if tool_name != "Bash" {
    return Ok(Some(deny_response(format!(
        "nono tool sandbox prototype denies {tool_name}; only Bash rewriting and read-only tools are allowed"
    ))));
}
```

New shape (illustrative — planner resolves exact syntax):

```rust
// Confined file-op arms go here, before the Bash gate
match tool_name {
    "Write" | "Edit" | "MultiEdit" => {
        // Pattern 1: D-05 guard
        // Pattern 2: extract file_path / content / old_string / new_string
        // call build_*_cmd(...) -> ps_inner  (bare inner PS expression, NO wrapped_bash_command)
        // Pattern 3: return deny + additionalContext("Use the Bash tool with this PowerShell command: {ps_inner}")
    }
    "NotebookEdit" => {
        // Research Q3 says keep as simple deny for initial phase
        return Ok(Some(deny_response(
            "NotebookEdit is not supported in the Windows POC; use the Bash tool with PowerShell to edit notebook JSON directly"
        )));
    }
    _ => {} // fall through to Bash gate
}
```

---

### `crates/nono-cli/src/claude_code_hook.rs` — New unit tests (test, N/A)

**Change site:** The `#[cfg(test)] mod tests` block — lines 251–429. New tests are appended inline.

**Analog within same file:** The existing `pre_tool_use_file_tools_deny` test (lines 291–321) and the `windows_cwd_guard_denies_project_claude_child` test (lines 413–428).

#### Pattern 6: Deny-result assertion test (analog: lines 291–321)

```rust
// Source: claude_code_hook.rs lines 291-321 — loop over tool names, assert permissionDecision==deny
#[test]
fn pre_tool_use_file_tools_deny() -> std::result::Result<(), Box<dyn Error>> {
    for tool_name in ["Write", "Edit", "MultiEdit", "NotebookEdit", ...] {
        let input = json!({
            "hook_event_name": "PreToolUse",
            "tool_name": tool_name,
            "tool_input": {"file_path": "x.txt", "content": "x"},
            "tool_use_id": "toolu_2"
        }).to_string();

        let response = pre_tool_use_response(&input)
            .map_err(|e| format!("valid hook response for {tool_name}: {e}"))?
            .ok_or_else(|| format!("{tool_name} should be denied"))?;

        assert_eq!(
            response["hookSpecificOutput"]["permissionDecision"],
            json!("deny"),
            "{tool_name} must be denied by the tool-sandbox hook"
        );
    }
    Ok(())
}
```

New tests must check `permissionDecision == "deny"` AND that `additionalContext` is present and non-empty for `Write`, `Edit`, `MultiEdit`. Use the same `json!({...}).to_string()` input construction and `response["hookSpecificOutput"]["additionalContext"]` path assertion.

#### Pattern 7: Windows-gated guard test (analog: lines 413–428)

```rust
// Source: claude_code_hook.rs lines 413-428 — Windows-gated test using tempfile + cwd_self_disable_risk_reason_for
#[cfg(target_os = "windows")]
#[test]
fn windows_cwd_guard_denies_project_claude_child() -> std::result::Result<(), Box<dyn Error>> {
    let root = tempfile::tempdir()?;
    let home = root.path().join("home");
    let repo = root.path().join("repo");
    std::fs::create_dir_all(&home)?;
    std::fs::create_dir_all(repo.join(".claude"))?;

    let reason = cwd_self_disable_risk_reason_for(&repo)?;
    assert!(
        reason.is_some(),
        "repo CWD with project-local .claude child must be denied"
    );
    Ok(())
}
```

New Windows guard test for the Write arm must set up a tempdir with `.claude/`, call `pre_tool_use_response` with a `Write` tool_name and a CWD that covers `~/.claude`, and assert `permissionDecision == "deny"` WITHOUT an `additionalContext` PS command (the guard fires before any PS command is built).

---

### `packages/claude-code/CLAUDE.md` (config / model-visible instruction, N/A)

**Change site:** Append a new `## Windows Shell Syntax` section after the existing `## Working directory` section (line 28 — end of file).

**Analog within same file:** The existing `## Constraints` and `## When an operation is denied` heading+body sections (lines 5–22). Match their heading level (`##`), plain prose body, and bullet-list style.

```markdown
<!-- Source: packages/claude-code/CLAUDE.md lines 1-28 — existing heading/body pattern -->
## Constraints

- **Filesystem**: ...
- **Network**: ...
- **No escalation**: ...

## When an operation is denied

If a file read, write, or command fails due to a permission error:

1. Do NOT retry...
```

New section to append:

```markdown
## Windows Shell Syntax

On Windows, the Bash tool is backed by native PowerShell (not Git Bash or MSYS2).
Always use PowerShell syntax for Bash tool commands:

- List files: `Get-ChildItem` not `ls`
- Read file: `Get-Content` not `cat`
- Write file: use the Bash tool with `Set-Content -Path '<file>' -Value '<content>'`
- Edit file: use the Bash tool with the PowerShell string-replace pattern

If the Write or Edit tool is blocked, re-issue the operation as a Bash tool call using
the PowerShell command provided in the denial context. This routes the file write through
the Low-IL confined shell.
```

---

### `packages/claude-code/claude-code-tools-windows-runner.profile.json` (config, N/A)

**Change site:** None expected. Verification only.

**Current state** (lines 1–30): Profile already grants `"workdir": {"access": "readwrite"}` (CWD r+w, the D-01 scope), `"network": {"block": true}`, and `"add_deny_access": ["$HOME/.claude", "$WORKDIR/.claude"]` (macOS-effective carve-outs, Windows no-op per D-05).

The planner should include a verification task confirming no profile change is needed for the Write/Edit confinement path — the runner profile's CWD grant already covers files in subdirectories, so new file creation and Edit's read-before-write are both covered.

---

## Shared Patterns

### CWD Self-Disable Guard (D-05)

**Source:** `crates/nono-cli/src/claude_code_hook.rs` lines 130–197 (`cwd_self_disable_risk_reason`, `cwd_self_disable_risk_reason_for`, `cwd_covers_home_claude_state`, `path_covers`, `canonical_existing_path`, `home_dir`)

**Apply to:** Every new file-tool arm (`Write`, `Edit`, `MultiEdit`, `NotebookEdit`) — the guard call MUST precede any PS command construction.

```rust
// Source: claude_code_hook.rs lines 131-135
#[cfg(target_os = "windows")]
fn cwd_self_disable_risk_reason() -> Result<Option<String>> {
    let cwd = std::env::current_dir()
        .map_err(|e| NonoError::HookInstall(format!("failed to resolve hook CWD: {e}")))?;
    cwd_self_disable_risk_reason_for(&cwd)
}
```

The guard checks three conditions (home `.claude`, `~/.claude.json[.lock]`, project `.claude/`). No new conditions are added in Phase 60.

### deny_response JSON Shape

**Source:** `crates/nono-cli/src/claude_code_hook.rs` lines 116–124

**Apply to:** All deny paths. New arms that emit `additionalContext` must construct the JSON with `json!({...})` directly (not via `deny_response()`) to add the extra field.

### NonoError::HookInstall for all errors

**Source:** `crates/nono-cli/src/claude_code_hook.rs` — every `Err(...)` in the file uses `NonoError::HookInstall(format!(...))`.

**Apply to:** All new helper functions (`build_confined_write_cmd`, etc.). No other error variant is used in this module.

### powershell_encoded_command + powershell_single_quoted

**Source:** `crates/nono-cli/src/claude_code_hook.rs` lines 236–248

**Apply to:** All PS command string construction. File content MUST be encoded before embedding — pass the inner PS command through `powershell_encoded_command`, which handles UTF-16LE + base64. For file content with embedded quotes or newlines, encode the content as base64 separately and decode it inside the PS command string.

### #[cfg(target_os = "windows")] gating

**Source:** `crates/nono-cli/src/claude_code_hook.rs` lines 130, 137, 155, 168, 176, 182, 192, 199, 212, 222, 236, 241 — every Windows-specific function carries this attribute.

**Apply to:** All new PS command builders and any new guard helpers. The corresponding non-Windows stub (if needed) follows the `#[cfg(not(target_os = "windows"))]` pattern at lines 222–234.

### Test boilerplate

**Source:** `crates/nono-cli/src/claude_code_hook.rs` lines 251–253

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;
    // Windows-only tests also use tempfile::tempdir()
```

All new tests go in the existing `mod tests` block — no new test file. Windows-gated tests use `#[cfg(target_os = "windows")]` on the `#[test]` fn directly (see line 361 pattern).

---

## No Analog Found

None. All three modified files have strong in-file analogs. No new files are created. The planner should reference RESEARCH.md for the PS command construction specifics (`build_confined_write_cmd`, regex-escape for Edit, base64-content approach for Write) because those are PS-language patterns with no Rust analog in the codebase.

---

## Metadata

**Analog search scope:** `crates/nono-cli/src/claude_code_hook.rs` (primary, read in full), `packages/claude-code/CLAUDE.md` (read in full), `packages/claude-code/claude-code-tools-windows-runner.profile.json` (read in full), `crates/nono-cli/src/hooks.rs` (read in full for cross-reference)
**Files scanned:** 4
**Pattern extraction date:** 2026-05-29
