# Phase 60: Sandbox-the-Tools — Confined Coding Loop - Research

**Researched:** 2026-05-29
**Domain:** Windows Claude Code hook protocol; Rust PreToolUse handler extension; Low-IL file-op confinement mechanism; PowerShell steering via profile schema
**Confidence:** HIGH (code verified in repo; hook JSON contract confirmed via official Claude Code docs)

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01:** Confined file edits (`Write`/`Edit`/`MultiEdit`/`NotebookEdit`) are granted the **whole project CWD (`--allow-cwd`)**, consistent with the Bash runner — not per-file.
- **D-02:** Ship the Windows shell as a **PowerShell tool runner** and **steer Claude to emit PowerShell syntax** on Windows (via the profile tool-description / a system-prompt note — exact wiring is a planning detail). Do NOT attempt a confined real-bash path for the POC.
- **D-03:** Accept unconfined Medium-IL agent reads (`Read`/`Glob`/`Grep`) and document it clearly. Read-confinement deferred.
- **D-04:** Keep the **fail-closed deny with a clear reason** for unwrappable tools (`WebFetch`/`WebSearch`/`Task`/`mcp__*`), and ship a short **"what works / what's denied" tool-support matrix** for POC users.
- **D-05:** The per-call CWD grant for confined edits **must never make `~/.claude` hook state writable**. The hook CWD guard (`claude_code_hook.rs::cwd_self_disable_risk_reason`) already refuses to wrap when the resolved CWD covers `~/.claude`; the file-op confinement runs under the **same** guard. Do **NOT** rely on profile `add_deny_access` (Windows no-op for the allow-overlap case).

### Claude's Discretion

- The file-op **mechanism**: rewrite `Write`/`Edit` into a `nono run --allow-cwd -- <PowerShell write op>` subprocess **vs.** a `nono`-native Low-IL file broker. Pick per research.
- The exact **PowerShell-steering wiring** (profile tool-description field vs. appended system-prompt note vs. `CLAUDE.md` note).
- `Edit`'s read-before-write and new-file-in-subdir cases — covered by the CWD r+w grant (D-01); confirm no extra handling needed.

### Deferred Ideas (OUT OF SCOPE)

- Network / `allow_domain` per-call grants (confined `WebFetch`/`WebSearch`)
- MCP-under-`nono` or MCP-deny policy + audit of MCP launch paths
- `Task`/subagent confinement
- Read-confinement
- Agent-process FS confinement
- Wire deny→label in the Windows backend so `add_deny_access` becomes a real Windows deny-within-allow primitive

</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| REQ-STW-01 | Confined file ops: `Write`/`Edit`/`MultiEdit`/`NotebookEdit` produce Low-IL confined writes (not mere denies) scoped to the project CWD, with out-of-scope writes denied at the OS boundary | Mechanism: deny → subprocess rewrite via `wrapped_bash_command` pattern; `nono run --profile claude-code-tools-windows-runner --allow-cwd -- powershell.exe <write op>` |
| REQ-STW-02 | Usable shell story: PowerShell-backed Bash runner works AND Claude is steered to emit PowerShell syntax automatically, removing the manual "use PowerShell syntax" instruction | Wiring: CLAUDE.md file in the project or `packages/claude-code/CLAUDE.md` package file (confirmed path; already exists); profile `meta.description` is display-only and not model-visible |

</phase_requirements>

---

## Summary

Phase 60 extends the merged PR #4 (`7488dbba`) hook handler by changing four deny arms
(`Write`, `Edit`, `MultiEdit`, `NotebookEdit`) into confined subprocess rewrites, and wires
PowerShell-syntax steering so Claude doesn't require manual prompting to use PS syntax on Windows.

**The decisive research finding:** `updatedInput` in a PreToolUse hook is confirmed to rewrite
tool inputs (e.g., `command` for Bash — the entire PR #4 is proof). However, rewriting a
`Write` tool's `file_path`/`content` via `updatedInput` would still execute the write
**in-process at Medium-IL** — the confinement goal is not achievable by input mutation alone.
The `claude_code_hook.rs` module comment (line 7) states this directly: "In-process tools such
as `Write` and `Edit` cannot be made Low-IL by command rewriting." The correct approach is to
**deny the in-process tool call and replace it with a confined subprocess** — the same pattern
used for Bash but applied to the file tools. This turns each `Write`/`Edit` into a PowerShell
`Set-Content` / `Select-String`-based equivalent running under `nono run --allow-cwd`.

**PowerShell steering:** The official Claude Code `settings.json` has no per-tool description
field and no `disabledTools` array. The only model-visible instruction surface that is
practical for this project is: a `CLAUDE.md` file (already present at
`packages/claude-code/CLAUDE.md` as the nono-sandbox `CLAUDE.md`; this file is injected into
the session as instructions). Adding a short PS-syntax steering note there is the correct wiring.
Profile `meta.description` is human-readable metadata only, not model-visible.

**Primary recommendation:** Extend `claude_code_hook.rs::pre_tool_use_response` by adding four
new match arms (one per file tool) that (a) extract `tool_input.file_path` and
`tool_input.content` / `old_string` / `new_string`, (b) gate through `cwd_self_disable_risk_reason`,
(c) build a PowerShell file-write command from the tool arguments, and (d) return
`permissionDecision: deny` — not allow — with a `hookSpecificOutput` that tells Claude to
re-issue the operation as a Bash tool call with the PS command. OR use the confirmed-working
`deny` path and instruct Claude via `additionalContext` to retry as Bash. The simplest
end-to-end shape: emit `deny` with a clear re-try hint — Claude will then call the `Bash` tool
with the PS file-write command, which the existing Bash arm wraps through `nono run` Low-IL.

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| PreToolUse hook intercept (deny/rewrite) | CLI binary (`claude_code_hook.rs`) | Profile JSON (matcher `*`) | Hook handler lives in the CLI; profile only configures which script runs |
| Low-IL file write confinement | Windows MIC backend (`sandbox/windows.rs`) | CLI runner profile | Mandatory-label enforcement is kernel-side; CLI constructs the grant |
| PowerShell steering | Project CLAUDE.md | — | Only model-visible instruction surface available |
| Tool-support matrix (UX) | Package profile README / CLAUDE.md | — | Documentation artifact, no code change |
| CWD self-disable guard | CLI hook handler (`cwd_self_disable_risk_reason`) | — | Already exists; new arms must route through it |

---

## Standard Stack

No new external packages are introduced in this phase. All functionality is within the existing
workspace crates.

### Existing Crates Extended

| Crate | File | Change |
|-------|------|--------|
| `nono-cli` | `src/claude_code_hook.rs` | Add Write/Edit/MultiEdit/NotebookEdit confined arms |
| `nono-cli` | (tests inside `claude_code_hook.rs`) | New unit tests for each new arm |
| Package files | `packages/claude-code/CLAUDE.md` | Add PowerShell-steering note |
| Package files | `packages/claude-code/claude-code-tools-windows-runner.profile.json` | Verify no changes needed (CWD grant already correct) |

**Installation:** No `cargo add` required. No new dependencies.

---

## Package Legitimacy Audit

> No new external packages are installed in this phase. Section not applicable.

---

## Architecture Patterns

### System Architecture Diagram

```
Claude Code TUI (Medium-IL, real terminal)
  │
  │  Tool call event (JSON on stdin)
  ▼
nono-tool-hook.ps1 (trampoline)
  │  pipes JSON to
  ▼
nono claude-code-hook (Medium-IL, in the nono binary)
  │
  │  reads tool_name + tool_input from JSON
  │
  ├─ tool_name ∈ {Read, Glob, Grep}
  │    └─ return permissionDecision: allow  (unconfined, D-03)
  │
  ├─ tool_name == Bash
  │    ├─ cwd_self_disable_risk_reason?  → deny
  │    └─ wrap with nono run --profile runner --allow-cwd -- powershell.exe -EncodedCommand <cmd>
  │         └─ Low-IL PowerShell process (kernel-confined by mandatory label)
  │
  ├─ tool_name ∈ {Write, Edit, MultiEdit, NotebookEdit}  ← NEW
  │    ├─ cwd_self_disable_risk_reason?  → deny
  │    ├─ extract tool_input.file_path (+ content / old_string / new_string)
  │    ├─ build PowerShell file-write command
  │    └─ return deny + additionalContext("retry as Bash: <PS cmd>")
  │         Claude re-issues as Bash tool → Bash arm wraps → Low-IL
  │         OR: directly spawn subprocess and return allow (simpler, explored below)
  │
  └─ tool_name ∈ {WebFetch, WebSearch, Task, mcp__*}
       └─ return permissionDecision: deny  (D-04, unchanged)
```

### Recommended Project Structure

No new directories. All changes are confined to:

```
crates/nono-cli/src/
└── claude_code_hook.rs     # Add Write/Edit/MultiEdit/NotebookEdit arms + tests

packages/claude-code/
├── CLAUDE.md               # Add PowerShell-steering paragraph
└── claude-code-tools-windows-runner.profile.json  # No changes expected
```

---

## Pattern 1: File-Op Mechanism — Deny + additionalContext (Recommended)

**What:** When `Write`/`Edit`/`MultiEdit`/`NotebookEdit` arrive, the hook:
1. Extracts `file_path`, `content`, `old_string`, `new_string` from `tool_input`.
2. Constructs an equivalent PowerShell command (e.g. `Set-Content -Path '<path>' -Value '<content>'` for Write; `(Get-Content '<path>') -replace [regex]::Escape('<old>'), '<new>' | Set-Content '<path>'` for Edit).
3. Returns `permissionDecision: deny` with an `additionalContext` field containing the PS command, instructing Claude to call the `Bash` tool with that exact command.
4. Claude calls the `Bash` tool, which the existing Bash arm wraps via `nono run --allow-cwd` at Low-IL.

**Why this shape:** `permissionDecision: deny` is the only confirmed-working output shape for stopping an in-process write. Returning `allow` + `updatedInput` with a new `file_path` would still execute the write at Medium-IL — the confinement guarantee is lost. The `deny` + retry pattern forces the file write through the Low-IL Bash subprocess, which is the actual enforcement path.

**Why not `additionalContext` alone?** `additionalContext` injects text into Claude's context at the tool result point; combined with a `deny`, Claude receives the context explaining what to do next. This is the correct pairing — confirmed in the official hooks protocol.

**Confirmed JSON shape (from existing hook protocol):** [VERIFIED: code.claude.com/docs/en/hooks]

```rust
// Source: claude_code_hook.rs existing pattern + official hook protocol
fn confined_file_op_response(
    tool_name: &str,
    tool_input: &Value,
) -> Result<Option<Value>> {
    // Gate: D-05 self-disable guard
    #[cfg(target_os = "windows")]
    if let Some(reason) = cwd_self_disable_risk_reason()? {
        return Ok(Some(deny_response(reason)));
    }

    let file_path = tool_input
        .get("file_path")
        .and_then(Value::as_str)
        .ok_or_else(|| NonoError::HookInstall(
            format!("{tool_name} tool_input missing string file_path")
        ))?;

    let ps_cmd = build_ps_file_write_cmd(tool_name, tool_input, file_path)?;

    Ok(Some(json!({
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "permissionDecision": "deny",
            "permissionDecisionReason": format!(
                "{tool_name} is an in-process tool that cannot be confined directly; \
                 retry as a Bash tool call with the PowerShell command provided in additionalContext"
            ),
            "additionalContext": format!(
                "Use the Bash tool with this PowerShell command to perform the confined file operation: {ps_cmd}"
            )
        }
    })))
}
```

**Confidence:** MEDIUM — `deny` + `additionalContext` pattern confirmed as valid JSON fields by
official docs [CITED: code.claude.com/docs/en/hooks]. Behavioral effect (Claude re-issues as
Bash) requires UAT confirmation; no existing published example of this exact deny+retry loop.

### Pattern 2: PowerShell File-Write Command Construction

**What:** Translate each file tool's semantics into a safe PS command string.

| Tool | tool_input fields | PS equivalent |
|------|------------------|---------------|
| `Write` | `file_path`, `content` | `Set-Content -Path '<path>' -Encoding UTF8 -Value @'<content>'@` |
| `Edit` | `file_path`, `old_string`, `new_string` | Read, replace, write-back via `(Get-Content) -replace [regex]::Escape('<old>'), '<new>'` |
| `MultiEdit` | `file_path`, `edits` (array of {old_string, new_string}) | Same as Edit, iterated |
| `NotebookEdit` | `notebook_path`, `cell_index`, `new_source` | Write JSON cell back (more complex; may need fallback) |

**Critical constraint on quoting:** The double-base64 `-EncodedCommand` trampoline in
`wrapped_bash_command` / `windows_bash_wrapper_script` avoids nested-quoting fragility.
The PS command string constructed in the hook handler must be passed through
`powershell_encoded_command()` — the existing inner-encoding function — not as a literal
string, to handle embedded quotes and newlines in file content safely.

**Edit's read-before-write:** Edit reads the existing file content, finds `old_string`, replaces
with `new_string`, and writes back. Under a CWD r+w grant (D-01) the file at `file_path` is
already readable and writable if it is inside the project CWD. No extra permission handling is
needed — confirmed by the existing Bash runner profile which grants CWD ReadWrite including all
files in it. [VERIFIED: packages/claude-code/claude-code-tools-windows-runner.profile.json]

**New file in subdirectory:** `Write` to a new file at `<CWD>/subdir/newfile.txt` is covered
by the CWD r+w directory grant — Landlock/Windows MIC grants apply recursively to subdirectories.
No extra handling needed. [ASSUMED — recursive grant behavior; consistent with how Bash
file creation in CWD subdirs works in the existing runner.]

### Pattern 3: PowerShell-Steering Wiring

**Decision: CLAUDE.md is the correct wiring.** [VERIFIED: code.claude.com/docs/en/settings]

Official Claude Code `settings.json` has:
- NO `disabledTools` array [VERIFIED: settings docs]
- NO per-tool description field that is model-visible [VERIFIED: settings docs]
- YES `permissions.deny` array — but this blocks the tool entirely (Claude receives an error,
  not a steering note). Blocking `Write` via permissions.deny would conflict with the hook's
  deny+retry strategy (Claude would never attempt Write, so the retry loop is moot; but also
  Claude would not know to use Bash+PS instead without a steering note).

The practical options ranked:

| Option | Model-visible? | Scope | Verdict |
|--------|---------------|-------|---------|
| `packages/claude-code/CLAUDE.md` | YES — loaded as instructions at session start | Project-scoped when user runs from that dir | RECOMMENDED |
| `~/.claude/CLAUDE.md` | YES — user-level persistent | All claude sessions | Too broad; don't pollute user-global |
| `settings.json` `claudeMd` | YES — but managed/policy settings only | Managed deployments | Not available in standalone setup |
| Profile `meta.description` | NO — display-only metadata | N/A | Ruled out |
| `additionalContext` on every hook response | YES — but only per-call | Per-tool-call | Redundant and noisy |

**Steering note to add to `packages/claude-code/CLAUDE.md`:**

```markdown
## Windows Shell Syntax

On Windows, the Bash tool is backed by native PowerShell (not Git Bash/MSYS2).
Always use PowerShell syntax for Bash commands on Windows:
- Path separator: `\` (or `/` in most PowerShell cmdlets)
- List files: `Get-ChildItem` not `ls`
- Read file: `Get-Content` not `cat`
- Write file: use the Bash tool with `Set-Content -Path '<file>' -Value '<content>'`
- Edit file: use the Bash tool with the PowerShell replace pattern

This ensures commands run in the Low-IL confined shell.
```

The existing `packages/claude-code/CLAUDE.md` already has an "you are inside a nono security
sandbox" framing — this adds a Windows-specific shell note. [VERIFIED: file read at
packages/claude-code/CLAUDE.md]

### Pattern 4: Tool-Support Matrix (D-04)

**What:** A short matrix shipped as part of the profile README or CLAUDE.md so POC users know
what works and what's denied without hitting denies unexpectedly.

| Tool | Status | Notes |
|------|--------|-------|
| `Read`, `Glob`, `Grep` | Allowed (unconfined) | Agent reads are Medium-IL; D-03 accepted |
| `Bash` | Confined Low-IL | PowerShell-backed; use PS syntax |
| `Write`, `Edit`, `MultiEdit`, `NotebookEdit` | Confined Low-IL (via Bash retry) | Hook converts to Bash+PS; files inside CWD only |
| `WebFetch`, `WebSearch` | Denied | Network is blocked; out of POC scope |
| `Task` | Denied | Subagent spawn is unconfined; out of POC scope |
| `mcp__*` | Denied | MCP server runs at Medium-IL; out of POC scope |

### Anti-Patterns to Avoid

- **`updatedInput` for Write confinement:** Returning `permissionDecision: allow` +
  `updatedInput` with a modified `file_path` or `content` still executes the write at
  Medium-IL in-process. This provides no confinement. Never use this for security.
  [VERIFIED: code comment in claude_code_hook.rs line 7 + analysis of hook protocol]
- **`add_deny_access` for `~/.claude` carve-out:** This is a Windows no-op for the allow-overlap
  case. The only reliable guard on Windows is the hook-level CWD check. [VERIFIED: policy.rs
  line 1038-1047 + todo file 2026-05-29]
- **Git Bash / MSYS2 as the confined shell:** `0xC0000022` / `NtCreateDirectoryObject` failure.
  PowerShell is the only confirmed working shell for Low-IL children on Windows.
  [VERIFIED: SPEC.md Q4 answer + debug session evidence]
- **Literal string concatenation for PS commands:** File content may contain single quotes,
  newlines, or PowerShell metacharacters. Always use the double-base64 encoded command
  (`powershell_encoded_command()`) as the inner encoding layer. [VERIFIED: existing pattern
  in claude_code_hook.rs::windows_bash_wrapper_script]
- **`permissions.deny` for Write/Edit in settings.json:** This would block the tool outright
  so Claude never attempts it — making the deny+retry loop impossible. The hook handler's deny
  is already enforced at the tool-call level.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| PowerShell base64 encoding | Custom encoder | `powershell_encoded_command()` in `claude_code_hook.rs` | Already handles UTF-16LE encoding; already tested |
| CWD self-disable check | New guard | `cwd_self_disable_risk_reason()` — already exists | Covers all three cases (home `.claude`, `~/.claude.json`, project `.claude/`) |
| Low-IL process launch | New subprocess code | `nono run --profile claude-code-tools-windows-runner --allow-cwd` | Already tested and field-verified (PR #4 UAT) |
| PS single-quote escaping | Ad-hoc | `powershell_single_quoted()` in `claude_code_hook.rs` | Already exists, handles embedded single-quotes |
| Hook JSON response shape | New JSON construction | `deny_response()` / `json!({...})` patterns from existing arms | Matches the established hook contract |

---

## Common Pitfalls

### Pitfall 1: Edit's old_string contains regex metacharacters

**What goes wrong:** If `old_string` is passed as-is to PowerShell's `-replace` operator, any
regex metacharacter (`.`, `*`, `(`, `)`, `[`, `]`, `$`, `^`, `|`, `?`, `+`, `\`) in the
literal string causes wrong matches or errors.

**Why it happens:** PS `-replace` treats the left operand as a regex pattern, not a literal string.

**How to avoid:** Wrap `old_string` with `[regex]::Escape('<old_string>')` in the PS command
before passing to `-replace`. Or use the `.Replace()` string method which does literal matching.

**Warning signs:** Edit produces unexpected results on strings containing dots or parens.

### Pitfall 2: File content with heredoc/quote conflicts in encoded command

**What goes wrong:** File content passed as a string literal inside a PS command breaks when
content contains single or double quotes, newlines, or `@` followed by `'`.

**How to avoid:** Always route file content through the double-base64 layer. For Write,
encode content separately and pass it via a Base64-decode-then-write pattern inside the PS
command:

```powershell
# Inner PS command (then encoded again for -EncodedCommand)
[System.IO.File]::WriteAllText(
    '<file_path>',
    [System.Text.Encoding]::UTF8.GetString([Convert]::FromBase64String('<b64_content>'))
)
```

**Warning signs:** Content with `'` or newlines silently writes wrong data or fails.

### Pitfall 3: Edit on a file outside the CWD

**What goes wrong:** Edit is called with `file_path` pointing outside the CWD. Under the CWD
r+w grant, the write is denied at the OS boundary but the error message may be confusing.

**How to avoid:** The deny at the OS boundary IS the correct outcome (D-01 intent). No additional
check needed. Ensure the deny message is surfaced clearly to Claude so it can inform the user.

### Pitfall 4: MultiEdit with partially applied edits on failure

**What goes wrong:** `MultiEdit` applies multiple edits to one file. If the PS command fails
after applying some edits but before others, the file is in a partially-edited state.

**How to avoid:** Apply all edits to an in-memory string (reading once, applying all replacements,
writing once). Never interleave read/write operations. The single `Set-Content` at the end is atomic
at the PS level (though not TOCTOU-safe — but that is a separate deferred concern per REQUIREMENTS.md).

### Pitfall 5: Confusing the deny mechanism with `permissions.deny`

**What goes wrong:** Adding `"Write"` to `settings.json` `permissions.deny` blocks the tool
before the hook runs — Claude never attempts Write, the deny+retry loop never fires, and Claude
has no instructions to use Bash+PS instead.

**How to avoid:** Do NOT add file tools to `permissions.deny`. Let the hook handler issue the deny
with the retry instruction. The hook's deny is the enforcement point; permissions.deny would
short-circuit it.

---

## Code Examples

### Existing Bash arm (the pattern to extend)

```rust
// Source: crates/nono-cli/src/claude_code_hook.rs (PR #4, commit 7488dbba)
// This is the EXACT pattern that Write/Edit arms should mirror for the CWD gate.
if tool_name != "Bash" {
    return Ok(Some(deny_response(format!(
        "nono tool sandbox prototype denies {tool_name}; ..."
    ))));
}

#[cfg(target_os = "windows")]
if let Some(reason) = cwd_self_disable_risk_reason()? {
    return Ok(Some(deny_response(reason)));
}

// ... extract command, build wrapped_bash_command, return allow + updatedInput
```

### New Write arm (recommended shape)

```rust
// Source: research synthesis — pattern derived from existing arms
"Write" => {
    #[cfg(target_os = "windows")]
    if let Some(reason) = cwd_self_disable_risk_reason()? {
        return Ok(Some(deny_response(reason)));
    }
    let tool_input = event.get("tool_input").cloned().unwrap_or(Value::Null);
    let file_path = tool_input.get("file_path").and_then(Value::as_str)
        .ok_or_else(|| NonoError::HookInstall(
            "Write tool_input missing string file_path".to_string()
        ))?;
    let content = tool_input.get("content").and_then(Value::as_str)
        .unwrap_or("");
    let ps_cmd = build_confined_write_cmd(file_path, content)?;
    Ok(Some(json!({
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "permissionDecision": "deny",
            "permissionDecisionReason":
                "Write is an in-process tool; use the Bash tool with the PowerShell command in additionalContext",
            "additionalContext": format!("Confined file write command: {ps_cmd}")
        }
    })))
}
```

### PowerShell write command (base64-content approach)

```rust
// Source: research synthesis — follows double-base64 pattern from wrapped_bash_command
#[cfg(target_os = "windows")]
fn build_confined_write_cmd(file_path: &str, content: &str) -> Result<String> {
    let b64_content = nono::trust::base64::base64_encode(content.as_bytes());
    let ps_inner = format!(
        "[System.IO.File]::WriteAllText(\
         {path_quoted}, \
         [System.Text.Encoding]::UTF8.GetString(\
           [Convert]::FromBase64String('{b64}')\
         )\
        )",
        path_quoted = powershell_single_quoted(file_path),
        b64 = b64_content,
    );
    // Wrap with nono run --allow-cwd via the existing double-base64 trampoline
    wrapped_bash_command(&ps_inner)
    // NOTE: wrapped_bash_command already adds the outer nono run invocation;
    // the returned string is the full Bash tool command to suggest via additionalContext.
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Deny Write/Edit outright (PR #4 merged) | Deny + retry-as-Bash (Phase 60 target) | Phase 60 | File edits become usable for POC users |
| Manual "use PowerShell syntax" instruction | CLAUDE.md steering note (Phase 60) | Phase 60 | Removes friction for POC users |
| Per-tool description field (not available) | CLAUDE.md project instructions | N/A | Settled design |

**Deprecated/outdated:**
- `add_deny_access` in runner profile for `~/.claude`: This was a well-intentioned attempt in PR #4 but is a Windows no-op for the allow-overlap case. Keep the entries (still valuable on macOS) but do not rely on them for Windows security. The hook CWD guard is the enforcement path.

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `deny` + `additionalContext` causes Claude to re-issue the operation as a Bash tool call | Pattern 1 (File-Op Mechanism) | If Claude ignores the additionalContext and stops, file edits fail silently; needs UAT gate |
| A2 | New-file-in-subdir write is covered by the CWD r+w directory grant (recursive) | Pattern 2 (Edit's read-before-write) | If Windows MIC grants are not recursive for subdirectory creates, new files in subdirs fail; low risk given Bash creates files in subdirs fine in the existing runner |
| A3 | `[System.IO.File]::WriteAllText` and Base64-content approach is the right shape for Write; `(Get-Content) -replace` for Edit | Pattern 2 (PS command construction) | If these PS cmdlets are unavailable in the Low-IL PowerShell child, file ops fail; low risk — both are core .NET/PS built-ins present on Windows 10/11 |
| A4 | Claude respects `additionalContext` and uses the PS command provided | Pattern 1 | Behavioral — requires UAT |

---

## Open Questions (RESOLVED)

1. **Behavioral: does deny+additionalContext reliably trigger a Bash+PS retry?**
   - **RESOLVED: UAT-gated.** The mechanism is shipped as designed (deny + additionalContext
     with the inner PS expression). Whether Claude reliably re-issues the Bash retry is
     verified in the manual UAT gate (VALIDATION.md step c). Fallback disposition is Q2 below.

2. **Alternative mechanism: direct subprocess from hook handler**
   - **RESOLVED: Fallback if Q1 UAT fails.** If deny+additionalContext does not cause Claude
     to retry as Bash, the fallback is a direct nono subprocess launched from within the hook
     handler: spawn `nono run --allow-cwd -- powershell.exe <write>`, wait for completion,
     then return `deny` with a reason stating "write was completed by the sandbox." This avoids
     Claude's action loop entirely. Document the fallback approach and file a follow-on todo
     before closing the phase if Q1 UAT fails (see VALIDATION.md note).

3. **NotebookEdit complexity**
   - **RESOLVED: Informative deny, no PS command, by design decision.** NotebookEdit is kept
     as a simple `deny` with reason "NotebookEdit is not yet supported in the Windows POC sandbox"
     for this phase. No PS command is constructed. This is a deliberate design choice (see
     CONTEXT.md D-04 and THREAT-60-06): the complexity of JSON cell manipulation is deferred.
     Add to follow-on backlog if users request it.

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `powershell.exe` | Confined file-write subprocess | Yes | Windows 10/11 built-in (PS 5.1+) | None — required |
| `nono run` CLI | Subprocess confinement | Yes | v0.57.5 (target\release\nono.exe) | None — required |
| `nono claude-code-hook` subcommand | Hook handler dispatch | Yes | Part of nono binary | None — required |
| Claude Code with PreToolUse support | Hook execution | Yes (confirmed PR #4 field UAT) | 2.1.156+ | None — required |
| `[Convert]::FromBase64String` PS built-in | Content encoding | Yes | .NET Framework / .NET Core (all Win 10/11) | None — standard |

**Missing dependencies with no fallback:** None.
**Missing dependencies with fallback:** None.

---

## Validation Architecture

> `workflow.nyquist_validation: true` — section required.

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in test runner (`cargo test`) |
| Config file | none (uses Cargo.toml `[dev-dependencies]`) |
| Quick run command | `cargo test --package nono-cli --bin nono claude_code_hook` |
| Full suite command | `cargo test --package nono-cli --bin nono` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| REQ-STW-01 | Write tool returns deny + additionalContext with PS cmd | unit | `cargo test --bin nono claude_code_hook::tests::pre_tool_use_write_returns_deny_with_ps_cmd -x` | ❌ Wave 0 |
| REQ-STW-01 | Edit tool returns deny + additionalContext with PS cmd | unit | `cargo test --bin nono claude_code_hook::tests::pre_tool_use_edit_returns_deny_with_ps_cmd -x` | ❌ Wave 0 |
| REQ-STW-01 | MultiEdit tool returns deny + additionalContext | unit | `cargo test --bin nono claude_code_hook::tests::pre_tool_use_multiedit_returns_deny_with_ps_cmd -x` | ❌ Wave 0 |
| REQ-STW-01 | Write arm runs through CWD self-disable guard (Windows) | unit | `cargo test --bin nono claude_code_hook::tests::windows_write_cwd_guard_denies_home_claude -x` | ❌ Wave 0 |
| REQ-STW-01 | Out-of-scope write denied at OS boundary (Bash+PS confined) | HUMAN-UAT | Manual: claude attempts to write outside CWD; verify OS deny | — |
| REQ-STW-01 | In-scope write succeeds via Bash retry (Low-IL, file lands) | HUMAN-UAT | Manual: ask claude "write 'hello' to test.txt in project" | — |
| REQ-STW-02 | CLAUDE.md steering note present and model-visible | manual-check | `grep -r "PowerShell" packages/claude-code/CLAUDE.md` | ❌ Wave 0 |
| REQ-STW-02 | Bash shell runs PS syntax (Get-ChildItem works) | HUMAN-UAT | Manual: ask claude "list files using Get-ChildItem" without explicit PS instruction | — |

### Sampling Rate

- **Per task commit:** `cargo test --bin nono claude_code_hook`
- **Per wave merge:** `cargo test --package nono-cli --bin nono`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps

- [ ] `crates/nono-cli/src/claude_code_hook.rs` — add unit tests for Write/Edit/MultiEdit confined arms (tests are inline in the module, add to existing `#[cfg(test)] mod tests` block)
- [ ] `packages/claude-code/CLAUDE.md` — add Windows PS-steering section

*(No new test files; all new tests are inline in `claude_code_hook.rs` per established pattern.)*

---

## Security Domain

> `security_enforcement` is not explicitly disabled — section required.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | No | N/A — tool mediation, no auth |
| V3 Session Management | No | N/A |
| V4 Access Control | Yes | Mandatory Integrity Control (Windows MIC) + hook CWD guard |
| V5 Input Validation | Yes | `file_path` must not escape CWD; `old_string` must be regex-escaped; content must be base64-encoded |
| V6 Cryptography | No | N/A — base64 is encoding, not encryption |

### Known Threat Patterns for This Stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Path traversal via `file_path` (`../../etc/passwd`) | Tampering | The CWD grant is the OS-level enforcement; `cwd_self_disable_risk_reason` guards the hook state. An out-of-CWD path is denied by the Windows MIC grant (not by the hook). |
| Self-disable via `~/.claude` write | Tampering | `cwd_self_disable_risk_reason` guard — already implemented and tested in 8 existing tests. Must also gate new Write/Edit arms. |
| Regex injection in Edit `old_string` | Tampering | Use `[regex]::Escape()` or `String.Replace()` (literal) in the PS command. |
| Content injection via heredoc | Tampering | Base64-encode content before embedding in PS command (Pitfall 2 above). |
| Bypass via `permissions.deny` conflict | Tampering | Do NOT add file tools to settings.json deny — leave hook as the interception point. |
| MCP server side effects (Medium-IL) | Elevation of Privilege | Out of scope (D-04/deferred); deny all `mcp__*` tool calls in hook (already done). |

**Security note:** The `cwd_self_disable_risk_reason` guard is the single most important
security invariant in this phase. It MUST be called before any file-op arm emits a confined
write command. Omitting it for the new arms would allow an agent running from `~/.claude` to
rewrite hook configuration via a Write tool call. [VERIFIED: existing test coverage in 3 of
8 existing tests in claude_code_hook.rs]

---

## Project Constraints (from CLAUDE.md)

Directives that apply directly to this phase's implementation:

1. **No `.unwrap()` / `.expect()`** — use `?` and `NonoError` variants. Clippy enforces this.
2. **`#[must_use]` on critical Results** — the new `confined_file_op_response` helper must be `#[must_use]`.
3. **Path security** — validate and canonicalize paths; use `Path::starts_with()` not string `starts_with()`. The existing `canonical_existing_path()` and `path_covers()` helpers must be reused.
4. **Cross-target clippy** — the new arms are Windows-only (`#[cfg(target_os = "windows")]`). The planner must include a `PARTIAL` cross-target verification note per the CLAUDE.md cross-target requirement. A Windows-host-only `cargo check` is NOT a substitute.
5. **DCO sign-off** — all commits must include `Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>`.
6. **GSD workflow** — use `/gsd:execute-phase` for implementation. No direct edits outside a GSD workflow.
7. **`#[allow(dead_code)]` prohibited** — all new functions must be called (either from production code or from tests).
8. **Libraries should almost never panic** — use `Result`, not `panic!`.

---

## Sources

### Primary (HIGH confidence)
- `crates/nono-cli/src/claude_code_hook.rs` — direct code read; PR #4 squash commit `7488dbba`
- `packages/claude-code/claude-code-tools-windows-runner.profile.json` — direct code read
- `packages/claude-code/claude-code-tools-windows.profile.json` — direct code read
- `packages/claude-code/CLAUDE.md` — direct read (the PS-steering target)
- `crates/nono-cli/src/policy.rs:1038-1047` — `validate_deny_overlaps` Windows no-op confirm
- `crates/nono-cli/src/policy.rs:674` — `add_deny_access_rules` macOS-only emit confirm
- `.planning/quick/260528-sch-spec-the-sandbox-the-tools-windows-tool-/260528-sch-SPEC.md` — §7 answered (full Q1–Q6)
- `.planning/phases/60-sandbox-the-tools-confined-coding-loop-v2-9/60-CONTEXT.md` — D-01..D-05 locked decisions
- `.planning/todos/pending/2026-05-29-claude-tools-runner-deny-dotclaude-regardless-of-cwd.md` — folded as D-05

### Secondary (MEDIUM confidence)
- [CITED: code.claude.com/docs/en/hooks] — PreToolUse hook protocol: `permissionDecision`, `deny`, `additionalContext` fields confirmed; `updatedInput` exists but is Bash-command-centric in examples
- [CITED: code.claude.com/docs/en/settings] — `disabledTools` does not exist; `permissions.deny` blocks tools before hook; no per-tool description field
- [CITED: code.claude.com/docs/en/hooks-guide] — `tool_input.file_path` confirmed present for Write/Edit; `Edit|Write` matcher confirmed; PostToolUse shows these fields are accessible

### Tertiary (LOW confidence)
- WebSearch: `updatedInput` confirmed working since v2.0.10 for Bash command rewrites; no published examples of Write/Edit input rewrite confirming it bypasses Medium-IL execution

---

## Metadata

**Confidence breakdown:**
- Standard stack (existing crates): HIGH — code read directly
- Hook JSON protocol: MEDIUM-HIGH — official docs confirmed but truncated; existing PR #4 field UAT is the best evidence
- File-op mechanism (deny+retry): MEDIUM — confirmed `deny` path works; `additionalContext` retry behavior needs UAT gate (A1)
- PS command construction: MEDIUM — core PS cmdlets are standard; edge cases (regex escaping, heredoc safety) flagged as pitfalls
- PowerShell-steering wiring: HIGH — CLAUDE.md confirmed model-visible via settings docs

**Research date:** 2026-05-29
**Valid until:** 2026-06-29 (30 days — stable Claude Code hook protocol; PS built-ins stable)
