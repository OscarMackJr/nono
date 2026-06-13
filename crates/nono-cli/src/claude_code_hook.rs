//! Claude Code hook handler for tool-level sandboxing prototypes.
//!
//! This is intentionally narrow: the first enforceable slice rewrites Claude
//! Code's `Bash` tool into a `nono run` invocation, allows read-only tools, and
//! confines or denies everything else. On Windows the rewritten `Bash` tool is
//! backed by native PowerShell because Git Bash/MSYS2 cannot initialize as a
//! Low-IL child. Write, Edit, and MultiEdit are confined via deny+additionalContext
//! — the hook denies the in-process call and provides a bare inner PowerShell
//! command for Claude to retry as a Bash tool call (the Bash arm calls
//! `wrapped_bash_command` on it exactly once).

use nono::{NonoError, Result};
use serde_json::{json, Value};
use std::io::Read;
#[cfg(target_os = "windows")]
use std::path::{Path, PathBuf};

const DEFAULT_TOOL_RUNNER_PROFILE: &str = "claude-code-tools-windows-runner";

pub(crate) fn run() -> Result<()> {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).map_err(|e| {
        NonoError::HookInstall(format!("failed to read Claude Code hook stdin: {e}"))
    })?;

    if let Some(output) = pre_tool_use_response(&input)? {
        let serialized = serde_json::to_string(&output).map_err(|e| {
            NonoError::HookInstall(format!(
                "failed to serialize Claude Code hook response: {e}"
            ))
        })?;
        println!("{serialized}");
    }

    Ok(())
}

fn pre_tool_use_response(input: &str) -> Result<Option<Value>> {
    let event: Value = serde_json::from_str(input).map_err(|e| {
        NonoError::HookInstall(format!("invalid Claude Code hook JSON on stdin: {e}"))
    })?;

    if event.get("hook_event_name").and_then(Value::as_str) != Some("PreToolUse") {
        return Ok(None);
    }

    let Some(tool_name) = event.get("tool_name").and_then(Value::as_str) else {
        return Ok(Some(deny_response(
            "nono tool sandbox prototype requires a Claude Code tool name",
        )));
    };

    if is_read_only_tool(tool_name) {
        return Ok(Some(allow_response(
            "read-only Claude Code tool allowed by nono tool sandbox prototype",
        )));
    }

    match tool_name {
        "Write" => {
            #[cfg(target_os = "windows")]
            if let Some(reason) = cwd_self_disable_risk_reason()? {
                return Ok(Some(deny_response(reason)));
            }
            let tool_input = event
                .get("tool_input")
                .cloned()
                .ok_or_else(|| NonoError::HookInstall("Write tool_input missing".to_string()))?;
            let file_path = tool_input
                .get("file_path")
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    NonoError::HookInstall("Write tool_input missing string file_path".to_string())
                })?;
            let content = tool_input
                .get("content")
                .and_then(Value::as_str)
                .unwrap_or("");

            #[cfg(target_os = "windows")]
            {
                let ps_inner = build_confined_write_cmd(file_path, content)?;
                return Ok(Some(json!({
                    "hookSpecificOutput": {
                        "hookEventName": "PreToolUse",
                        "permissionDecision": "deny",
                        "permissionDecisionReason": "Write is an in-process tool that cannot be confined directly; use the Bash tool with the PowerShell command in additionalContext to perform the confined file operation",
                        "additionalContext": format!("Use the Bash tool with this PowerShell command: {ps_inner}")
                    }
                })));
            }
            #[cfg(not(target_os = "windows"))]
            {
                let _ = (file_path, content);
                return Ok(Some(deny_response(
                    "Write confinement via Bash retry is Windows-only",
                )));
            }
        }
        "Edit" => {
            #[cfg(target_os = "windows")]
            if let Some(reason) = cwd_self_disable_risk_reason()? {
                return Ok(Some(deny_response(reason)));
            }
            let tool_input = event
                .get("tool_input")
                .cloned()
                .ok_or_else(|| NonoError::HookInstall("Edit tool_input missing".to_string()))?;
            let file_path = tool_input
                .get("file_path")
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    NonoError::HookInstall("Edit tool_input missing string file_path".to_string())
                })?;
            let old_string = tool_input
                .get("old_string")
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    NonoError::HookInstall("Edit tool_input missing string old_string".to_string())
                })?;
            let new_string = tool_input
                .get("new_string")
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    NonoError::HookInstall("Edit tool_input missing string new_string".to_string())
                })?;

            #[cfg(target_os = "windows")]
            {
                let ps_inner = build_confined_edit_cmd(file_path, old_string, new_string)?;
                return Ok(Some(json!({
                    "hookSpecificOutput": {
                        "hookEventName": "PreToolUse",
                        "permissionDecision": "deny",
                        "permissionDecisionReason": "Edit is an in-process tool that cannot be confined directly; use the Bash tool with the PowerShell command in additionalContext to perform the confined file operation",
                        "additionalContext": format!("Use the Bash tool with this PowerShell command: {ps_inner}")
                    }
                })));
            }
            #[cfg(not(target_os = "windows"))]
            {
                let _ = (file_path, old_string, new_string);
                return Ok(Some(deny_response(
                    "Edit confinement via Bash retry is Windows-only",
                )));
            }
        }
        "MultiEdit" => {
            #[cfg(target_os = "windows")]
            if let Some(reason) = cwd_self_disable_risk_reason()? {
                return Ok(Some(deny_response(reason)));
            }
            let tool_input = event.get("tool_input").cloned().ok_or_else(|| {
                NonoError::HookInstall("MultiEdit tool_input missing".to_string())
            })?;
            let file_path = tool_input
                .get("file_path")
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    NonoError::HookInstall(
                        "MultiEdit tool_input missing string file_path".to_string(),
                    )
                })?;
            let edits = tool_input.get("edits").cloned().ok_or_else(|| {
                NonoError::HookInstall("MultiEdit tool_input missing edits array".to_string())
            })?;

            #[cfg(target_os = "windows")]
            {
                let ps_inner = build_confined_multiedit_cmd(file_path, &edits)?;
                return Ok(Some(json!({
                    "hookSpecificOutput": {
                        "hookEventName": "PreToolUse",
                        "permissionDecision": "deny",
                        "permissionDecisionReason": "MultiEdit is an in-process tool that cannot be confined directly; use the Bash tool with the PowerShell command in additionalContext to perform the confined file operation",
                        "additionalContext": format!("Use the Bash tool with this PowerShell command: {ps_inner}")
                    }
                })));
            }
            #[cfg(not(target_os = "windows"))]
            {
                let _ = (file_path, edits);
                return Ok(Some(deny_response(
                    "MultiEdit confinement via Bash retry is Windows-only",
                )));
            }
        }
        "NotebookEdit" => {
            return Ok(Some(deny_response(
                "NotebookEdit is not yet supported in the Windows POC sandbox; use the Bash tool with PowerShell to read and write the notebook JSON directly",
            )));
        }
        "Bash" => {
            // fall through to the Bash arm below
        }
        _ => {
            return Ok(Some(deny_response(format!(
                "nono tool sandbox prototype denies {tool_name}; only Bash rewriting and read-only tools are allowed"
            ))));
        }
    }

    #[cfg(target_os = "windows")]
    if let Some(reason) = cwd_self_disable_risk_reason()? {
        return Ok(Some(deny_response(reason)));
    }

    let mut updated_input = event
        .get("tool_input")
        .cloned()
        .ok_or_else(|| NonoError::HookInstall("PreToolUse input missing tool_input".to_string()))?;
    let Some(tool_input) = updated_input.as_object_mut() else {
        return Err(NonoError::HookInstall(
            "PreToolUse tool_input must be a JSON object".to_string(),
        ));
    };
    let command = tool_input
        .get("command")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            NonoError::HookInstall("Bash tool_input missing string command".to_string())
        })?;

    if command_contains_nono_wrapper(command) {
        return Ok(None);
    }

    tool_input.insert(
        "command".to_string(),
        Value::String(wrapped_bash_command(command)?),
    );

    Ok(Some(json!({
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "permissionDecision": "allow",
            "permissionDecisionReason": "Bash command rewritten through nono tool sandbox",
            "updatedInput": updated_input
        }
    })))
}

fn is_read_only_tool(tool_name: &str) -> bool {
    matches!(tool_name, "Read" | "Glob" | "Grep")
}

fn allow_response(reason: impl Into<String>) -> Value {
    json!({
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "permissionDecision": "allow",
            "permissionDecisionReason": reason.into()
        }
    })
}

fn deny_response(reason: impl Into<String>) -> Value {
    json!({
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "permissionDecision": "deny",
            "permissionDecisionReason": reason.into()
        }
    })
}

fn command_contains_nono_wrapper(command: &str) -> bool {
    command.contains(" nono run ") || command.starts_with("nono run ")
}

#[cfg(target_os = "windows")]
fn cwd_self_disable_risk_reason() -> Result<Option<String>> {
    let cwd = std::env::current_dir()
        .map_err(|e| NonoError::HookInstall(format!("failed to resolve hook CWD: {e}")))?;
    cwd_self_disable_risk_reason_for(&cwd)
}

#[cfg(target_os = "windows")]
fn cwd_self_disable_risk_reason_for(cwd: &Path) -> Result<Option<String>> {
    let cwd = canonical_existing_path(cwd)?;

    let home = match home_dir() {
        None => {
            return Ok(Some(self_disable_reason_no_home()));
        }
        Some(path) => canonical_existing_path(&path)?,
    };
    if cwd_covers_home_claude_state(&cwd, &home) {
        return Ok(Some(self_disable_reason(&cwd)));
    }

    let project_claude = cwd.join(".claude");
    if project_claude.exists() {
        return Ok(Some(self_disable_reason(&cwd)));
    }

    Ok(None)
}

#[cfg(target_os = "windows")]
fn cwd_covers_home_claude_state(cwd: &Path, home: &Path) -> bool {
    let home_claude = home.join(".claude");
    if path_covers(cwd, &home_claude) {
        return true;
    }
    if cwd.starts_with(&home_claude) {
        return true;
    }

    [".claude.json", ".claude.json.lock"]
        .iter()
        .map(|state_path| home.join(state_path))
        .any(|target| path_covers(cwd, &target))
}

#[cfg(target_os = "windows")]
fn self_disable_reason(cwd: &Path) -> String {
    format!(
        "refusing to wrap Bash: CWD '{}' covers Claude Code hook settings or agent state; would allow the tool jail to disable its own hooks",
        cwd.display()
    )
}

#[cfg(target_os = "windows")]
fn self_disable_reason_no_home() -> String {
    "refusing to wrap Bash: home directory cannot be resolved; cannot verify CWD does not cover Claude Code hook state".to_string()
}

#[cfg(target_os = "windows")]
fn path_covers(parent: &Path, child: &Path) -> bool {
    let child = canonicalize_with_existing_prefix(child);
    child.starts_with(parent)
}

#[cfg(target_os = "windows")]
fn canonicalize_with_existing_prefix(path: &Path) -> PathBuf {
    let components_count = path.components().count();
    for (i, ancestor) in path.ancestors().enumerate() {
        if ancestor.exists() {
            if let Ok(canonical_base) = ancestor.canonicalize() {
                // Re-append the tail components that were stripped off.
                // path.ancestors() yields path, then parent, then grandparent...
                // so `i` is the number of components we stripped from the end.
                let tail_count = i;
                if tail_count == 0 {
                    return canonical_base;
                }
                // Collect the tail components (the last `tail_count` components of path).
                let all_components: Vec<_> = path.components().collect();
                let tail_start = components_count - tail_count;
                let result = all_components[tail_start..]
                    .iter()
                    .fold(canonical_base, |acc, c| acc.join(c));
                return result;
            }
            // If canonicalize failed despite exists() being true, continue to the next ancestor.
        }
    }
    // Degenerate case: no ancestor could be canonicalized (e.g. in-memory path with no real root).
    // Return the path as-is to remain conservative.
    path.to_path_buf()
}

#[cfg(target_os = "windows")]
fn canonical_existing_path(path: &Path) -> Result<PathBuf> {
    path.canonicalize().map_err(|e| {
        NonoError::HookInstall(format!(
            "failed to canonicalize path '{}': {e}",
            path.display()
        ))
    })
}

#[cfg(target_os = "windows")]
fn home_dir() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
}

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

#[cfg(target_os = "windows")]
fn windows_bash_wrapper_script(nono_exe: &str, command: &str) -> String {
    format!(
        "& {nono_exe} run --profile {profile} --allow-cwd -- powershell.exe -NoProfile -NonInteractive -EncodedCommand {encoded_command}",
        nono_exe = powershell_single_quoted(nono_exe),
        profile = powershell_single_quoted(DEFAULT_TOOL_RUNNER_PROFILE),
        encoded_command = powershell_encoded_command(command),
    )
}

#[cfg(not(target_os = "windows"))]
fn wrapped_bash_command(command: &str) -> Result<String> {
    let nono_exe = std::env::current_exe().map_err(|e| {
        NonoError::HookInstall(format!("failed to resolve current nono executable: {e}"))
    })?;
    let nono_exe_display = nono_exe.display().to_string();
    let nono_exe = shlex::try_quote(&nono_exe_display)
        .map_err(|e| NonoError::HookInstall(format!("failed to quote nono executable: {e}")))?;
    let command = shlex::try_quote(command)
        .map_err(|e| NonoError::HookInstall(format!("failed to quote shell command: {e}")))?;
    Ok(format!(
        "{nono_exe} run --profile {DEFAULT_TOOL_RUNNER_PROFILE} --allow-cwd -- sh -lc {command}"
    ))
}

#[cfg(target_os = "windows")]
fn powershell_single_quoted(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

#[cfg(target_os = "windows")]
fn powershell_encoded_command(command: &str) -> String {
    let utf16le = command
        .encode_utf16()
        .flat_map(u16::to_le_bytes)
        .collect::<Vec<_>>();
    nono::trust::base64::base64_encode(&utf16le)
}

/// Renders raw bytes as a PowerShell `[byte[]]` array literal, e.g.
/// `[byte[]]@(104,101,108,108,111)`. Empty input renders `[byte[]]@()`.
///
/// This is Constrained-Language-Mode (CLM) safe: `[byte[]]` is a primitive array
/// type permitted in CLM, integer literals are permitted, and the `@(...)` array
/// subexpression operator is a core language element. No .NET method is called,
/// so it runs inside an AppContainer (where PowerShell is forced into CLM).
///
/// Byte-faithful by construction: the exact bytes handed in are reproduced on disk
/// when combined with `Set-Content -Encoding Byte` (no encoding transform, no BOM,
/// no trailing newline).
#[cfg(target_os = "windows")]
fn powershell_byte_array_literal(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 4 + 12);
    out.push_str("[byte[]]@(");
    for (i, b) in bytes.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(&b.to_string());
    }
    out.push(')');
    out
}

/// Emits one CLM-safe PowerShell literal BYTE-sequence find-and-replace statement
/// operating on `$src` (a `[byte[]]`) and producing a new `[byte[]]` left in `$src`.
///
/// The emitted code does a left-to-right, non-overlapping literal replacement of the
/// byte sequence `old` with `new`, using ONLY core PowerShell language elements that
/// are permitted in Constrained Language Mode (CLM):
///   - `[byte[]]@(...)` literals (primitive array type, integer literals)
///   - `while` loops, comparison operators (`-lt`, `-le`, `-gt`, `-ne`), arithmetic
///   - array indexing (`$a[$i]`), the `.Length` property, `+=` array concatenation
///   - the array-subexpression operator `@(...)`
///
/// No .NET method is ever called (no `[Convert]`, `[System.IO.File]`,
/// `[System.Text.Encoding]`, `[regex]`, `String.Replace`, `::` static methods), so
/// this runs inside the per-run AppContainer where PowerShell is forced into CLM.
///
/// Byte-faithful: old/new are carried as exact UTF-8 byte literals (pre-computed by
/// the Medium-IL hook), so there is no encoding round-trip and no BOM. An empty `old`
/// is a no-op (matches PowerShell `-replace` semantics for an empty pattern would
/// differ, but for the Edit tool an empty `old_string` is not a meaningful literal
/// search and is left unchanged).
///
/// `idx` disambiguates the loop variables so multiple statements can be chained
/// (MultiEdit) without variable collisions.
#[cfg(target_os = "windows")]
fn byte_replace_statement(idx: usize, old: &[u8], new: &[u8]) -> String {
    let old_literal = powershell_byte_array_literal(old);
    let new_literal = powershell_byte_array_literal(new);
    // Variable names are suffixed with `idx` to keep each chained edit independent.
    format!(
        "$old{idx} = {old_literal}; \
         $new{idx} = {new_literal}; \
         $out{idx} = [byte[]]@(); \
         $i{idx} = 0; \
         $n{idx} = $src.Length; \
         $m{idx} = $old{idx}.Length; \
         while ($i{idx} -lt $n{idx}) {{ \
           $match{idx} = $false; \
           if ($m{idx} -gt 0 -and ($i{idx} + $m{idx}) -le $n{idx}) {{ \
             $match{idx} = $true; \
             $k{idx} = 0; \
             while ($k{idx} -lt $m{idx}) {{ \
               if ($src[$i{idx} + $k{idx}] -ne $old{idx}[$k{idx}]) {{ $match{idx} = $false; break }} \
               $k{idx} = $k{idx} + 1 \
             }} \
           }} \
           if ($match{idx}) {{ \
             if ($new{idx}.Length -gt 0) {{ $out{idx} += $new{idx} }} \
             $i{idx} = $i{idx} + $m{idx} \
           }} else {{ \
             $out{idx} += $src[$i{idx}]; \
             $i{idx} = $i{idx} + 1 \
           }} \
         }} \
         $src = [byte[]]$out{idx}; "
    )
}

/// Builds a bare inner PowerShell expression that writes `content` to `file_path`.
///
/// CLM-safe: the content is emitted as a `[byte[]]` literal (UTF-8 bytes) and
/// written with `Set-Content -Encoding Byte`. No .NET method calls (`[Convert]`,
/// `[System.IO.File]`, `[System.Text.Encoding]`) are used, so it runs inside the
/// per-run AppContainer (where PowerShell is in Constrained Language Mode).
///
/// Byte-faithful: `Set-Content -Encoding Byte` writes the exact bytes with no
/// added BOM and no trailing newline; arbitrary content (quotes, newlines,
/// non-ASCII) is carried as raw bytes, never interpolated into the script text.
///
/// The returned string is the INNER PS expression — callers must NOT call
/// `wrapped_bash_command` on the result; the Bash arm does exactly one wrap when
/// Claude retries via the Bash tool.
#[cfg(target_os = "windows")]
fn build_confined_write_cmd(file_path: &str, content: &str) -> Result<String> {
    let bytes_literal = powershell_byte_array_literal(content.as_bytes());
    let path_quoted = powershell_single_quoted(file_path);
    let ps_inner = format!(
        "$bytes = {bytes_literal}; \
         Set-Content -LiteralPath {path_quoted} -Value $bytes -Encoding Byte"
    );
    Ok(ps_inner)
}

/// Builds a bare inner PowerShell expression that edits `file_path` by replacing
/// the literal `old_string` with `new_string`.
///
/// CLM-safe AND byte-faithful (NO BOM): reads the target file as a `[byte[]]` via
/// `Get-Content -Encoding Byte -Raw` (returns a byte array in PS 5.1, no .NET call),
/// performs a pure-PowerShell literal BYTE-sequence find-and-replace (see
/// `byte_replace_statement`), and writes the result with `Set-Content -Encoding Byte`.
///
/// This replaces the earlier `-Encoding UTF8` string round-trip, which under PowerShell
/// 5.1 always prepends a UTF-8 BOM (`EF BB BF`) on `Set-Content`/`Out-File` (no
/// utf8NoBOM option exists in 5.1). The byte vehicle has no encoding transform, so
/// there is no BOM and content is reproduced byte-for-byte. The old/new strings are
/// carried as `[byte[]]` literals (their exact UTF-8 bytes), never interpolated into
/// the script text, so quotes/newlines/non-ASCII are all safe.
///
/// Matching is literal and case-sensitive by construction (byte equality), matching the
/// Edit tool's semantics. No .NET method calls.
///
/// The returned string is the INNER PS expression — callers must NOT call
/// `wrapped_bash_command` on the result.
#[cfg(target_os = "windows")]
fn build_confined_edit_cmd(file_path: &str, old_string: &str, new_string: &str) -> Result<String> {
    let path_quoted = powershell_single_quoted(file_path);
    let replace_stmt = byte_replace_statement(0, old_string.as_bytes(), new_string.as_bytes());
    let ps_inner = format!(
        "$src = [byte[]](Get-Content -LiteralPath {path_quoted} -Encoding Byte -Raw); \
         {replace_stmt}\
         Set-Content -LiteralPath {path_quoted} -Value $src -Encoding Byte"
    );
    Ok(ps_inner)
}

/// Builds a bare inner PowerShell expression that applies all edits in `edits` to
/// `file_path` in-memory (reading once, applying all replacements in sequence, writing
/// once) to avoid partially-edited state on failure.
///
/// CLM-safe AND byte-faithful (NO BOM): same byte vehicle as `build_confined_edit_cmd`
/// — single `Get-Content -Encoding Byte -Raw`, a chain of pure-PowerShell byte
/// find-and-replace blocks (one per edit, each feeding `$src` to the next), and a single
/// `Set-Content -Encoding Byte`. No `-Encoding UTF8` string write (the PS 5.1 BOM source)
/// and no .NET method calls.
///
/// `edits` must be a JSON array of `{"old_string": "...", "new_string": "..."}` objects.
/// The returned string is the INNER PS expression — callers must NOT call
/// `wrapped_bash_command` on the result.
#[cfg(target_os = "windows")]
fn build_confined_multiedit_cmd(file_path: &str, edits: &Value) -> Result<String> {
    let edits_array = edits.as_array().ok_or_else(|| {
        NonoError::HookInstall("MultiEdit tool_input.edits must be an array".to_string())
    })?;

    let path_quoted = powershell_single_quoted(file_path);
    let mut ps_inner =
        format!("$src = [byte[]](Get-Content -LiteralPath {path_quoted} -Encoding Byte -Raw); ");

    for (i, edit) in edits_array.iter().enumerate() {
        let old_string = edit
            .get("old_string")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                NonoError::HookInstall(format!("MultiEdit edits[{i}] missing string old_string"))
            })?;
        let new_string = edit
            .get("new_string")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                NonoError::HookInstall(format!("MultiEdit edits[{i}] missing string new_string"))
            })?;
        ps_inner.push_str(&byte_replace_statement(
            i,
            old_string.as_bytes(),
            new_string.as_bytes(),
        ));
    }

    ps_inner.push_str(&format!(
        "Set-Content -LiteralPath {path_quoted} -Value $src -Encoding Byte"
    ));

    Ok(ps_inner)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn pre_tool_use_bash_rewrites_command() -> std::result::Result<(), Box<dyn Error>> {
        let response = pre_tool_use_response(
            r#"{
              "hook_event_name": "PreToolUse",
              "tool_name": "Bash",
              "tool_input": {"command": "Get-ChildItem", "timeout": 120000},
              "tool_use_id": "toolu_1"
            }"#,
        )?
        .ok_or("Bash tool should be rewritten")?;

        let updated_command = response["hookSpecificOutput"]["updatedInput"]["command"]
            .as_str()
            .ok_or("updated command must be a string")?;

        #[cfg(target_os = "windows")]
        assert!(
            updated_command.contains("powershell.exe")
                && updated_command.contains("-EncodedCommand"),
            "Windows wrapper should encode the outer PowerShell trampoline: {updated_command}"
        );
        #[cfg(not(target_os = "windows"))]
        assert!(
            updated_command.contains("nono") && updated_command.contains(" run "),
            "updated command should invoke nono run: {updated_command}"
        );
        assert_eq!(
            response["hookSpecificOutput"]["updatedInput"]["timeout"],
            json!(120000),
            "non-command Bash tool fields must be preserved"
        );
        Ok(())
    }

    #[test]
    fn pre_tool_use_file_tools_deny() -> std::result::Result<(), Box<dyn Error>> {
        // Each tool needs tool_input that satisfies the arm's field extraction so
        // we get a deny response rather than a propagated Err from a missing field.
        let tool_inputs: &[(&str, serde_json::Value)] = &[
            ("Write", json!({"file_path": "x.txt", "content": "hello"})),
            (
                "Edit",
                json!({"file_path": "x.txt", "old_string": "foo", "new_string": "bar"}),
            ),
            (
                "MultiEdit",
                json!({"file_path": "x.txt", "edits": [{"old_string": "a", "new_string": "b"}]}),
            ),
            (
                "NotebookEdit",
                json!({"notebook_path": "nb.ipynb", "cell_index": 0, "new_source": "print(1)"}),
            ),
            ("WebFetch", json!({"url": "https://example.com"})),
            ("WebSearch", json!({"query": "test"})),
            ("Task", json!({"description": "do something"})),
            (
                "mcp__filesystem__write_file",
                json!({"path": "x.txt", "content": "x"}),
            ),
        ];
        for (tool_name, tool_input_val) in tool_inputs {
            let input = json!({
                "hook_event_name": "PreToolUse",
                "tool_name": tool_name,
                "tool_input": tool_input_val,
                "tool_use_id": "toolu_2"
            })
            .to_string();

            let response = pre_tool_use_response(&input)
                .map_err(|e| format!("valid hook response for {tool_name}: {e}"))?
                .ok_or_else(|| format!("{tool_name} should be denied"))?;

            assert_eq!(
                response["hookSpecificOutput"]["permissionDecision"],
                json!("deny"),
                "{tool_name} must be denied by the tool-sandbox hook"
            );

            // On Windows, Write/Edit/MultiEdit must include a non-empty additionalContext
            // PS command. NotebookEdit must NOT include one (informative deny only).
            match *tool_name {
                "Write" | "Edit" | "MultiEdit" => {
                    #[cfg(target_os = "windows")]
                    assert!(
                        response["hookSpecificOutput"]["additionalContext"]
                            .as_str()
                            .map(|s| !s.is_empty())
                            .unwrap_or(false),
                        "{tool_name} must include a non-empty additionalContext PS command on Windows"
                    );
                }
                "NotebookEdit" => {
                    let ctx = &response["hookSpecificOutput"]["additionalContext"];
                    assert!(
                        ctx.is_null() || ctx.as_str().map(|s| s.is_empty()).unwrap_or(true),
                        "NotebookEdit must NOT include an additionalContext PS command"
                    );
                }
                _ => {}
            }
        }
        Ok(())
    }

    #[test]
    fn pre_tool_use_read_only_tools_allow() -> std::result::Result<(), Box<dyn Error>> {
        for tool_name in ["Read", "Glob", "Grep"] {
            let input = json!({
                "hook_event_name": "PreToolUse",
                "tool_name": tool_name,
                "tool_input": {"path": "src"},
                "tool_use_id": "toolu_3"
            })
            .to_string();

            let response = pre_tool_use_response(&input)
                .map_err(|e| format!("valid hook response for {tool_name}: {e}"))?
                .ok_or_else(|| format!("{tool_name} should be explicitly allowed"))?;

            assert_eq!(
                response["hookSpecificOutput"]["permissionDecision"],
                json!("allow"),
                "{tool_name} must be allowed by the read-only tool policy"
            );
        }
        Ok(())
    }

    #[test]
    fn non_pre_tool_use_event_is_silent() -> std::result::Result<(), Box<dyn Error>> {
        let response = pre_tool_use_response(
            r#"{
              "hook_event_name": "PostToolUse",
              "tool_name": "Bash",
              "tool_input": {"command": "echo ok"}
            }"#,
        )?;

        assert!(response.is_none());
        Ok(())
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_wrapper_uses_native_powershell_child() {
        let script = windows_bash_wrapper_script(
            "C:\\repo\\target\\debug\\nono.exe",
            "Get-Location; Get-ChildItem -Force",
        );

        assert!(script.contains("-- powershell.exe -NoProfile -NonInteractive -EncodedCommand"));
        assert!(script.contains("claude-code-tools-windows-runner"));
        assert!(
            !script.contains("claude-code'"),
            "tool runner must not use the Claude Code agent profile that grants ~/.claude writes"
        );
        assert!(
            !script.contains("bash.exe") && !script.contains("Git\\usr\\bin"),
            "Git Bash/MSYS2 must not be used for the Windows Low-IL tool runner"
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_cwd_guard_denies_home_claude_ancestor() -> std::result::Result<(), Box<dyn Error>> {
        let root = tempfile::tempdir()?;
        let home = root.path().join("home");
        std::fs::create_dir_all(home.join(".claude"))?;
        std::fs::write(home.join(".claude.json"), "{}")?;
        let cwd = home.canonicalize()?;

        assert!(
            cwd_covers_home_claude_state(&cwd, &home.canonicalize()?),
            "home CWD covers ~/.claude and ~/.claude.json state"
        );
        Ok(())
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_cwd_guard_uses_path_components() -> std::result::Result<(), Box<dyn Error>> {
        let root = tempfile::tempdir()?;
        let home = root.path().join("home");
        std::fs::create_dir_all(home.join(".claude"))?;
        std::fs::create_dir_all(home.join(".claudefoo"))?;
        let cwd = home.join(".claudefoo").canonicalize()?;

        assert!(
            !cwd_covers_home_claude_state(&cwd, &home.canonicalize()?),
            "component comparison must not treat .claudefoo as .claude"
        );
        Ok(())
    }

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

    #[test]
    fn pre_tool_use_write_returns_deny_with_ps_cmd() -> std::result::Result<(), Box<dyn Error>> {
        let input = json!({
            "hook_event_name": "PreToolUse",
            "tool_name": "Write",
            "tool_input": {"file_path": "test.txt", "content": "hello world"},
            "tool_use_id": "t1"
        })
        .to_string();

        let response = pre_tool_use_response(&input)
            .map_err(|e| format!("Write hook response failed: {e}"))?
            .ok_or("Write should produce a response")?;

        assert_eq!(
            response["hookSpecificOutput"]["permissionDecision"],
            json!("deny"),
            "Write must be denied"
        );

        #[cfg(target_os = "windows")]
        assert!(
            response["hookSpecificOutput"]["additionalContext"]
                .as_str()
                .map(|s| !s.is_empty())
                .unwrap_or(false),
            "Write must include a non-empty additionalContext PS command on Windows"
        );

        Ok(())
    }

    #[test]
    fn pre_tool_use_edit_returns_deny_with_ps_cmd() -> std::result::Result<(), Box<dyn Error>> {
        let input = json!({
            "hook_event_name": "PreToolUse",
            "tool_name": "Edit",
            "tool_input": {"file_path": "src/main.rs", "old_string": "fn main()", "new_string": "fn run()"},
            "tool_use_id": "t2"
        })
        .to_string();

        let response = pre_tool_use_response(&input)
            .map_err(|e| format!("Edit hook response failed: {e}"))?
            .ok_or("Edit should produce a response")?;

        assert_eq!(
            response["hookSpecificOutput"]["permissionDecision"],
            json!("deny"),
            "Edit must be denied"
        );

        #[cfg(target_os = "windows")]
        assert!(
            response["hookSpecificOutput"]["additionalContext"]
                .as_str()
                .map(|s| !s.is_empty())
                .unwrap_or(false),
            "Edit must include a non-empty additionalContext PS command on Windows"
        );

        Ok(())
    }

    #[test]
    fn pre_tool_use_multiedit_returns_deny_with_ps_cmd() -> std::result::Result<(), Box<dyn Error>>
    {
        let input = json!({
            "hook_event_name": "PreToolUse",
            "tool_name": "MultiEdit",
            "tool_input": {
                "file_path": "src/lib.rs",
                "edits": [
                    {"old_string": "a", "new_string": "b"},
                    {"old_string": "c", "new_string": "d"}
                ]
            },
            "tool_use_id": "t3"
        })
        .to_string();

        let response = pre_tool_use_response(&input)
            .map_err(|e| format!("MultiEdit hook response failed: {e}"))?
            .ok_or("MultiEdit should produce a response")?;

        assert_eq!(
            response["hookSpecificOutput"]["permissionDecision"],
            json!("deny"),
            "MultiEdit must be denied"
        );

        #[cfg(target_os = "windows")]
        assert!(
            response["hookSpecificOutput"]["additionalContext"]
                .as_str()
                .map(|s| !s.is_empty())
                .unwrap_or(false),
            "MultiEdit must include a non-empty additionalContext PS command on Windows"
        );

        Ok(())
    }

    #[test]
    fn pre_tool_use_notebookedit_deny_no_ps_cmd() -> std::result::Result<(), Box<dyn Error>> {
        let input = json!({
            "hook_event_name": "PreToolUse",
            "tool_name": "NotebookEdit",
            "tool_input": {"notebook_path": "nb.ipynb", "cell_index": 0, "new_source": "print('x')"},
            "tool_use_id": "t4"
        })
        .to_string();

        let response = pre_tool_use_response(&input)
            .map_err(|e| format!("NotebookEdit hook response failed: {e}"))?
            .ok_or("NotebookEdit should produce a response")?;

        assert_eq!(
            response["hookSpecificOutput"]["permissionDecision"],
            json!("deny"),
            "NotebookEdit must be denied"
        );

        let ctx = &response["hookSpecificOutput"]["additionalContext"];
        assert!(
            ctx.is_null() || ctx.as_str().map(|s| s.is_empty()).unwrap_or(true),
            "NotebookEdit must NOT include an additionalContext PS command (informative deny only)"
        );

        Ok(())
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn pre_tool_use_write_content_with_special_chars() -> std::result::Result<(), Box<dyn Error>> {
        let input = json!({
            "hook_event_name": "PreToolUse",
            "tool_name": "Write",
            "tool_input": {"file_path": "test.txt", "content": "it's a test\nwith newlines"},
            "tool_use_id": "t5"
        })
        .to_string();

        let response = pre_tool_use_response(&input)
            .map_err(|e| format!("Write with special chars failed: {e}"))?
            .ok_or("Write should produce a response")?;

        assert_eq!(
            response["hookSpecificOutput"]["permissionDecision"],
            json!("deny"),
            "Write with special chars must be denied"
        );
        assert!(
            response["hookSpecificOutput"]["additionalContext"]
                .as_str()
                .map(|s| !s.is_empty())
                .unwrap_or(false),
            "Write with special chars must produce a non-empty additionalContext PS command"
        );

        Ok(())
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_write_arm_cwd_guard_fires_before_ps_cmd() -> std::result::Result<(), Box<dyn Error>>
    {
        // Mirror of windows_cwd_guard_denies_project_claude_child (lines 672-686):
        // directly call cwd_self_disable_risk_reason_for with a CWD that contains
        // a .claude/ child, assert the guard fires. This confirms the guard logic
        // that is also the first call in the Write/Edit/MultiEdit arms.
        // Do NOT call pre_tool_use_response here (would require set_current_dir).
        let root = tempfile::tempdir()?;
        let repo = root.path().join("repo");
        std::fs::create_dir_all(repo.join(".claude"))?;

        let reason = cwd_self_disable_risk_reason_for(&repo)?;
        assert!(
            reason.is_some(),
            "repo CWD with .claude child must trigger guard before any PS command is constructed"
        );

        Ok(())
    }

    /// Regression test for CR-01: guard must fire when CWD is an ancestor of ~/.claude.json
    /// but the file does NOT exist on disk.
    ///
    /// The pre-fix path_covers() returned false in this case because the raw C:\... child
    /// path did not match the \\?\-canonical parent CWD.
    #[cfg(target_os = "windows")]
    #[test]
    fn windows_cwd_guard_denies_home_claude_ancestor_absent_file(
    ) -> std::result::Result<(), Box<dyn Error>> {
        let root = tempfile::tempdir()?;
        let home = root.path().join("home");
        // Create ONLY the home directory itself — no .claude.json, .claude.json.lock, or .claude/
        std::fs::create_dir_all(&home)?;
        let cwd = home.canonicalize()?;

        assert!(
            cwd_covers_home_claude_state(&cwd, &home.canonicalize()?),
            "guard must fire when CWD is home dir and .claude.json does not yet exist on disk"
        );
        Ok(())
    }

    /// Regression test for WR-01: guard must fire when CWD is nested INSIDE ~/.claude,
    /// not only when it is an ancestor or equal.
    #[cfg(target_os = "windows")]
    #[test]
    fn windows_cwd_guard_denies_inside_home_claude() -> std::result::Result<(), Box<dyn Error>> {
        let root = tempfile::tempdir()?;
        let home = root.path().join("home");
        std::fs::create_dir_all(home.join(".claude").join("projects").join("myrepo"))?;
        let cwd = home
            .join(".claude")
            .join("projects")
            .join("myrepo")
            .canonicalize()?;
        let canonical_home = home.canonicalize()?;

        assert!(
            cwd_covers_home_claude_state(&cwd, &canonical_home),
            "guard must fire when CWD is nested inside ~/.claude subtree"
        );
        Ok(())
    }

    // --- CLM-safety regression tests (debug confined-write-clm-blocked) ---
    //
    // The confined Write/Edit/MultiEdit payloads run inside a per-run AppContainer
    // (BrokerLaunchNoPty arm, Plan 62-12), where PowerShell is forced into
    // Constrained Language Mode. CLM blocks ALL .NET type/method calls. These tests
    // assert the emitted payloads contain NONE of the blocked constructs and use the
    // CLM-safe vehicle ([byte[]] literal + Set-Content -Encoding Byte for Write;
    // Get-Content/-replace/Set-Content for Edit/MultiEdit).

    #[cfg(target_os = "windows")]
    fn assert_no_dotnet_method_calls(payload: &str, label: &str) {
        for blocked in [
            "[Convert]",
            "[System.IO.File]",
            "[System.Text.Encoding]",
            "[regex]",
            ".Replace(",
            "FromBase64String",
            "WriteAllText",
            "ReadAllText",
        ] {
            assert!(
                !payload.contains(blocked),
                "{label} payload must not contain CLM-blocked construct {blocked:?}: {payload}"
            );
        }
    }

    /// The PS 5.1 BOM source is `-Encoding UTF8` on a STRING write. No confined
    /// file-mutating payload may contain it — Write/Edit/MultiEdit must all use the
    /// byte vehicle (`-Encoding Byte`) so the file is written byte-faithfully (no BOM).
    #[cfg(target_os = "windows")]
    fn assert_no_utf8_string_encoding(payload: &str, label: &str) {
        assert!(
            !payload.contains("-Encoding UTF8"),
            "{label} payload must not use `-Encoding UTF8` (PS 5.1 prepends a BOM); \
             use the byte vehicle (`-Encoding Byte`) instead: {payload}"
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn byte_array_literal_is_clm_safe_and_byte_faithful() {
        assert_eq!(powershell_byte_array_literal(b""), "[byte[]]@()");
        assert_eq!(powershell_byte_array_literal(b"hi"), "[byte[]]@(104,105)");
        // Non-ASCII (UTF-8 'é' = 0xC3 0xA9) survives as raw bytes.
        assert_eq!(
            powershell_byte_array_literal("é".as_bytes()),
            "[byte[]]@(195,169)"
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn confined_write_cmd_is_clm_safe() -> std::result::Result<(), Box<dyn Error>> {
        // Content with single quotes, newlines, and non-ASCII — none of which may
        // appear literally in the payload (carried as bytes instead).
        let content = "it's a test\nwith \"newlines\" and é";
        let payload = build_confined_write_cmd("C:\\proj\\out.txt", content)?;

        assert_no_dotnet_method_calls(&payload, "Write");
        assert_no_utf8_string_encoding(&payload, "Write");
        assert!(
            payload.contains("[byte[]]@(") && payload.contains("Set-Content")
                && payload.contains("-Encoding Byte"),
            "Write payload must use the byte-array + Set-Content -Encoding Byte vehicle: {payload}"
        );
        // The byte literal must encode the exact UTF-8 bytes of the content.
        let expected_literal = powershell_byte_array_literal(content.as_bytes());
        assert!(
            payload.contains(&expected_literal),
            "Write payload must embed the exact UTF-8 bytes of the content"
        );
        Ok(())
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn confined_write_empty_content_emits_empty_byte_array(
    ) -> std::result::Result<(), Box<dyn Error>> {
        let payload = build_confined_write_cmd("C:\\proj\\empty.txt", "")?;
        assert!(
            payload.contains("[byte[]]@()"),
            "empty content must emit an empty byte array (0-byte file): {payload}"
        );
        Ok(())
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn confined_edit_cmd_is_clm_safe_byte_vehicle() -> std::result::Result<(), Box<dyn Error>> {
        // old/new contain regex metacharacters, quotes, '$', and non-ASCII — none of
        // which may appear literally in the payload (carried as byte literals instead).
        let old_string = "fn main() { foo.bar(1+2); }";
        let new_string = "fn run() { $env:X = 'é' }";
        let payload = build_confined_edit_cmd("C:\\proj\\main.rs", old_string, new_string)?;

        assert_no_dotnet_method_calls(&payload, "Edit");
        // BOM regression: no `-Encoding UTF8` string write may remain on the Edit path.
        assert_no_utf8_string_encoding(&payload, "Edit");
        // Byte vehicle: read as bytes, write as bytes.
        assert!(
            payload.contains("Get-Content") && payload.contains("-Encoding Byte"),
            "Edit payload must read/write the file as bytes (`-Encoding Byte`): {payload}"
        );
        assert!(
            payload.contains("Set-Content -LiteralPath 'C:\\proj\\main.rs' -Value $src -Encoding Byte"),
            "Edit payload must write $src with Set-Content -Encoding Byte: {payload}"
        );
        // The old/new strings must be embedded as their exact UTF-8 byte literals,
        // NOT interpolated as raw text (so quotes/newlines/'$'/non-ASCII are safe and
        // there is no regex/`$`-substitution surface).
        assert!(
            payload.contains(&powershell_byte_array_literal(old_string.as_bytes())),
            "Edit payload must embed old_string as its exact UTF-8 byte literal: {payload}"
        );
        assert!(
            payload.contains(&powershell_byte_array_literal(new_string.as_bytes())),
            "Edit payload must embed new_string as its exact UTF-8 byte literal: {payload}"
        );
        // No `-replace`/`-creplace` regex surface remains (the source of the prior
        // escaping complexity and BOM round-trip).
        assert!(
            !payload.contains("-creplace") && !payload.contains("-replace"),
            "Edit payload must not use the -replace/-creplace regex vehicle: {payload}"
        );
        Ok(())
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn confined_multiedit_cmd_is_clm_safe_single_read_write(
    ) -> std::result::Result<(), Box<dyn Error>> {
        let edits = json!([
            {"old_string": "a.b", "new_string": "x"},
            {"old_string": "c*d", "new_string": "y"}
        ]);
        let payload = build_confined_multiedit_cmd("C:\\proj\\lib.rs", &edits)?;

        assert_no_dotnet_method_calls(&payload, "MultiEdit");
        assert_no_utf8_string_encoding(&payload, "MultiEdit");
        // Single read + single write for atomicity.
        assert_eq!(
            payload.matches("Get-Content").count(),
            1,
            "MultiEdit must read the file exactly once: {payload}"
        );
        assert_eq!(
            payload.matches("Set-Content").count(),
            1,
            "MultiEdit must write the file exactly once: {payload}"
        );
        // The single read and single write must both be byte vehicles.
        assert!(
            payload.contains("Get-Content -LiteralPath 'C:\\proj\\lib.rs' -Encoding Byte -Raw"),
            "MultiEdit must read as bytes: {payload}"
        );
        assert!(
            payload.contains("Set-Content -LiteralPath 'C:\\proj\\lib.rs' -Value $src -Encoding Byte"),
            "MultiEdit must write $src as bytes: {payload}"
        );
        // One byte-replace block per edit, each writing back into $src so the next
        // edit operates on the cumulative result (sequential, in-memory).
        assert_eq!(
            payload.matches("$src = [byte[]]$out").count(),
            2,
            "MultiEdit must emit one byte-replace block per edit, chained via $src: {payload}"
        );
        Ok(())
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn byte_replace_statement_is_clm_safe_and_embeds_byte_literals() {
        // 'a.b' -> 'x': old/new embedded as exact UTF-8 byte literals; no .NET, no
        // -replace, no UTF8 string encoding.
        let stmt = byte_replace_statement(0, b"a.b", b"x");
        assert_no_dotnet_method_calls(&stmt, "byte_replace[0]");
        assert!(
            stmt.contains("[byte[]]@(97,46,98)"),
            "old bytes for 'a.b' must be embedded literally: {stmt}"
        );
        assert!(
            stmt.contains("[byte[]]@(120)"),
            "new bytes for 'x' must be embedded literally: {stmt}"
        );
        assert!(
            !stmt.contains("-replace") && !stmt.contains("-creplace"),
            "byte replace must not use the regex -replace vehicle: {stmt}"
        );
        // Idx suffix isolates loop variables for chaining.
        let stmt1 = byte_replace_statement(1, b"c", b"d");
        assert!(
            stmt1.contains("$old1") && stmt1.contains("$out1") && stmt1.contains("$i1"),
            "idx must suffix loop variables for chained edits: {stmt1}"
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn byte_replace_statement_empty_new_is_a_deletion() {
        // Empty new_string => deletion of old_string occurrences; no new bytes appended.
        let stmt = byte_replace_statement(0, b"x", b"");
        assert!(
            stmt.contains("$new0 = [byte[]]@()"),
            "empty new_string must emit an empty byte array (deletion): {stmt}"
        );
        // The append of $new0 is guarded by a length check so an empty replacement
        // appends nothing.
        assert!(
            stmt.contains("if ($new0.Length -gt 0) { $out0 += $new0 }"),
            "empty replacement must append nothing (guarded by length check): {stmt}"
        );
    }
}
