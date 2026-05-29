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
                    NonoError::HookInstall(
                        "Write tool_input missing string file_path".to_string(),
                    )
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
                    NonoError::HookInstall(
                        "Edit tool_input missing string file_path".to_string(),
                    )
                })?;
            let old_string = tool_input
                .get("old_string")
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    NonoError::HookInstall(
                        "Edit tool_input missing string old_string".to_string(),
                    )
                })?;
            let new_string = tool_input
                .get("new_string")
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    NonoError::HookInstall(
                        "Edit tool_input missing string new_string".to_string(),
                    )
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
            let tool_input = event
                .get("tool_input")
                .cloned()
                .ok_or_else(|| {
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
            let edits = tool_input
                .get("edits")
                .cloned()
                .ok_or_else(|| {
                    NonoError::HookInstall(
                        "MultiEdit tool_input missing edits array".to_string(),
                    )
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

    if let Some(home) = home_dir().and_then(|path| canonical_existing_path(&path).ok()) {
        if cwd_covers_home_claude_state(&cwd, &home) {
            return Ok(Some(self_disable_reason(&cwd)));
        }
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
fn path_covers(parent: &Path, child: &Path) -> bool {
    let child = canonical_existing_path(child).unwrap_or_else(|_| child.to_path_buf());
    child.starts_with(parent)
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
    let nono_exe = shlex::try_quote(&nono_exe.display().to_string())
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

/// Builds a bare inner PowerShell expression that writes `content` to `file_path`.
///
/// The content is base64-encoded (raw bytes, not UTF-16LE) to safely handle embedded
/// single quotes, newlines, and PowerShell metacharacters. The returned string is the
/// INNER PS expression — callers must NOT call `wrapped_bash_command` on the result;
/// the Bash arm does exactly one wrap when Claude retries via the Bash tool.
#[cfg(target_os = "windows")]
#[must_use]
fn build_confined_write_cmd(file_path: &str, content: &str) -> Result<String> {
    let b64_content = nono::trust::base64::base64_encode(content.as_bytes());
    let path_quoted = powershell_single_quoted(file_path);
    let ps_inner = format!(
        "$b = [Convert]::FromBase64String('{b64_content}'); \
         [System.IO.File]::WriteAllText({path_quoted}, \
         [System.Text.Encoding]::UTF8.GetString($b))"
    );
    Ok(ps_inner)
}

/// Builds a bare inner PowerShell expression that edits `file_path` by replacing
/// the literal `old_string` with `new_string` using `String.Replace()` (not `-replace`
/// regex) to avoid regex metacharacter injection.
///
/// The returned string is the INNER PS expression — callers must NOT call
/// `wrapped_bash_command` on the result.
#[cfg(target_os = "windows")]
#[must_use]
fn build_confined_edit_cmd(file_path: &str, old_string: &str, new_string: &str) -> Result<String> {
    let path_quoted = powershell_single_quoted(file_path);
    let old_quoted = powershell_single_quoted(old_string);
    let new_quoted = powershell_single_quoted(new_string);
    let ps_inner = format!(
        "$content = [System.IO.File]::ReadAllText({path_quoted}); \
         $content = $content.Replace({old_quoted}, {new_quoted}); \
         [System.IO.File]::WriteAllText({path_quoted}, $content)"
    );
    Ok(ps_inner)
}

/// Builds a bare inner PowerShell expression that applies all edits in `edits` to
/// `file_path` in-memory (reading once, applying all replacements, writing once) to
/// avoid partially-edited state on failure.
///
/// `edits` must be a JSON array of `{"old_string": "...", "new_string": "..."}` objects.
/// The returned string is the INNER PS expression — callers must NOT call
/// `wrapped_bash_command` on the result.
#[cfg(target_os = "windows")]
#[must_use]
fn build_confined_multiedit_cmd(file_path: &str, edits: &Value) -> Result<String> {
    let edits_array = edits.as_array().ok_or_else(|| {
        NonoError::HookInstall("MultiEdit tool_input.edits must be an array".to_string())
    })?;

    let path_quoted = powershell_single_quoted(file_path);
    let mut ps_inner = format!("$content = [System.IO.File]::ReadAllText({path_quoted}); ");

    for (i, edit) in edits_array.iter().enumerate() {
        let old_string = edit
            .get("old_string")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                NonoError::HookInstall(format!(
                    "MultiEdit edits[{i}] missing string old_string"
                ))
            })?;
        let new_string = edit
            .get("new_string")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                NonoError::HookInstall(format!(
                    "MultiEdit edits[{i}] missing string new_string"
                ))
            })?;
        let old_quoted = powershell_single_quoted(old_string);
        let new_quoted = powershell_single_quoted(new_string);
        ps_inner.push_str(&format!(
            "$content = $content.Replace({old_quoted}, {new_quoted}); "
        ));
    }

    ps_inner.push_str(&format!(
        "[System.IO.File]::WriteAllText({path_quoted}, $content)"
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
}
