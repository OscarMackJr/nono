# nono Sandbox Instructions

You are running inside a nono security sandbox. The sandbox enforces OS-level capability restrictions that cannot be bypassed from within this session.

## Constraints

- **Filesystem**: You can only read and write paths explicitly granted by the active profile. All other paths are blocked at the kernel level.
- **Network**: Network access may be blocked or filtered depending on the profile configuration.
- **No escalation**: There is no sudo, no permission changes, and no workaround that can expand the sandbox from within.

## When an operation is denied

If a file read, write, or command fails due to a permission error:

1. Do NOT retry with alternative paths or workarounds.
2. Do NOT attempt to copy files into allowed locations.
3. Tell the user to exit this session and restart with the required path:

```
nono run --allow /path/to/needed -- claude
```

This is the only way to expand the sandbox.

## Working directory

The current working directory is granted read-write access. You can freely create, edit, and delete files here.

## Windows Shell Syntax

On Windows, the Bash tool is backed by native PowerShell (not Git Bash or MSYS2).
Always use PowerShell syntax for Bash tool commands:

- List files: `Get-ChildItem` not `ls`
- Read file: `Get-Content` not `cat`
- Check a path exists: `Test-Path` not `[ -f ... ]`
- Copy: `Copy-Item`, move: `Move-Item`, delete: `Remove-Item`
- Write to a file: use the Bash tool with `Set-Content` (see "File edits" below)

## File edits

If the Write or Edit tool call is blocked (you receive a denial with a PowerShell command
in the denial context), re-issue the operation as a Bash tool call using the PowerShell
command provided. This routes the file write through the Low-IL confined shell instead of
executing it in-process at Medium IL.

Do NOT retry Write/Edit directly after a denial. Do NOT use alternative paths or workarounds.
Use the Bash tool with the exact PowerShell command from the denial context.

## Tool support matrix

What works in this sandbox:

| Tool | Status | Notes |
|------|--------|-------|
| Read, Glob, Grep | Allowed | Unconfined reads; accepted posture |
| Bash | Confined (Low-IL) | PowerShell-backed; use PS syntax |
| Write, Edit, MultiEdit | Confined (Low-IL via Bash retry) | Hook converts to Bash+PS; files inside CWD only |
| NotebookEdit | Denied | Not yet supported in the Windows POC |
| WebFetch, WebSearch | Denied | Network is blocked |
| Task | Denied | Subagent spawn is unconfined |
| mcp__* | Denied | MCP runs at Medium-IL outside the confinement boundary |
