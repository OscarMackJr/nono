# Windows Sandbox-The-Tools POC

Status: prototype slice

## Verdict

Running the Claude Code TUI at Medium IL and wrapping individual tools can be a
useful Windows POC, but it is not equivalent to sandboxing the agent process.
The hardened prototype uses a `PreToolUse` hook with `matcher: "*"` so every
Claude Code tool event reaches the handler before execution. The handler wraps
`Bash`, allows only read-only tools, and denies everything else.

The security posture is therefore:

- `Bash`: rewritten to
  `nono run --profile claude-code-tools-windows-runner --allow-cwd -- ...`. On
  Windows this is a PowerShell-backed runner, not Git Bash.
- Read-only tools (`Read`, `Glob`, `Grep`): explicitly allowed.
- In-process file tools (`Write`, `Edit`, `MultiEdit`, `NotebookEdit`): denied
  because they cannot be wrapped into a Low-IL subprocess.
- Network and orchestration tools (`WebFetch`, `WebSearch`, `Task`): denied.
- MCP tools: must be treated as separate side-effecting surfaces. A Claude hook
  can see and deny a tool event such as `mcp__server__tool`, but a Medium-IL MCP
  server process is outside the `nono run` boundary unless it is launched
  through nono or proxied.

This means the POC should be described as defense-in-depth tool mediation, not
complete agent isolation. A complete boundary still requires launching Claude
Code and all MCP servers inside an enforceable sandbox or moving each
side-effecting tool behind a broker.

## Implemented Slice

The prototype adds a hidden native hook handler:

```text
nono claude-code-hook
```

It reads Claude Code hook JSON from stdin and:

- silently ignores non-`PreToolUse` events,
- allows `Read`, `Glob`, and `Grep`,
- rewrites `PreToolUse` for `Bash` by replacing `tool_input.command`,
- denies all other `PreToolUse` events fail-closed,
- preserves non-command Bash fields such as timeouts.

On Windows the rewritten command deliberately uses a native PowerShell child:

```powershell
powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -EncodedCommand <outer-trampoline>
```

The outer trampoline runs
`nono run --profile claude-code-tools-windows-runner --allow-cwd -- powershell.exe
-NoProfile -NonInteractive -EncodedCommand <tool-command>`. Both encoded layers
avoid fragile nested quoting.

The runner profile is intentionally separate from the normal `claude-code`
agent profile. It grants CWD access for the rewritten tool process, blocks
network, has no hook installation stanza, and does not grant write access to
`~/.claude`. It also declares deny entries for `$HOME/.claude` and
`$WORKDIR/.claude`, but those are not sufficient on Windows when a broader CWD
grant covers the same path. The enforceable self-disable fix is in the native
hook handler: before rewriting `Bash`, it canonicalizes the hook CWD and denies
the tool call if that CWD is equal to, contains, or is an ancestor of Claude Code
hook settings or agent state (`~/.claude`, `~/.claude.json`, or project-local
`.claude` settings).

The read-only allow-list is a usability compromise, not read confinement:
`Read`, `Glob`, and `Grep` still execute inside the Medium-IL Claude Code
process and can read whatever that process can read. Likewise, network is
blocked for rewritten `Bash`, so network-using commands fail until a future
per-call capability mapping can grant specific domains.

Git Bash/MSYS2 is not a viable Low-IL child for this slice. It starts under
the broker, then fails during runtime initialization when it tries to create
objects in the medium-integrity `\BaseNamedObjects` namespace:

```text
fatal error - NtCreateDirectoryObject(\BaseNamedObjects\msys-2.0...): 0xC0000022
```

That preserves the security boundary but means Windows `Bash` tool calls must
use native PowerShell syntax unless a separate shell provider is built. Do not
try to fix this as a quoting issue or by adding `C:\Program Files\Git\usr\bin`
path grants; those only get as far as the MSYS2 runtime initialization failure.

## Hook Installation Mechanics

A Windows trampoline is bundled at:

```text
crates/nono-cli/data/hooks/nono-tool-hook.ps1
packages/claude-code/hooks/nono-tool-hook.ps1
```

It passes stdin to `nono claude-code-hook`. The hook installer now embeds this
script and emits a PowerShell hook handler when a profile references a `.ps1`
script.

The current built-in `claude-code` profile still installs the existing
diagnostic `PostToolUseFailure` hook. A follow-up profile or installer mode
should opt into the tool-wrapping POC with:

```json
"hooks": {
  "claude-code": {
    "event": "PreToolUse",
    "matcher": "*",
    "script": "nono-tool-hook.ps1"
  }
}
```

Using `matcher: "*"` is intentional: Claude Code documents `*` as the match-all
tool pattern for `PreToolUse` hooks. The same hook rewrites `Bash`, allows
read-only tools, and returns `permissionDecision: deny` for every other tool
class so side-effecting tools such as `NotebookEdit`, `WebFetch`, `Task`, and
`mcp__*` do not bypass the handler.

## Test Evidence

Targeted tests:

```text
cargo test -p nono-cli claude_code_hook
cargo test -p nono-cli hooks::tests::test_embedded_script_exists
```

Manual Windows probes:

- `nono claude-code-hook` rewrote sample `PreToolUse:Bash` JSON into a `nono run`
  command.
- The generated wrapper executed `Write-Output hi` successfully under
  `nono run --profile claude-code-tools-windows-runner --allow-cwd`.
- A generated wrapper attempting to write
  `C:\Users\OMack\NonoDebug\outside-hook-denied.txt` failed with
  `UnauthorizedAccessException`, confirming the write happened inside the
  sandboxed Low-IL tool process rather than in the Medium-IL parent.

### Live UAT: 2026-05-29

Environment:

- Repo: `C:\Users\OMack\NonoDebug\nono`
- Hook: `PreToolUse` / matcher `*` / `nono-tool-hook.ps1`
- Claude Code process: Medium IL in a normal terminal
- Tool process: launched through `nono run` by `nono claude-code-hook`

Allowed PowerShell-backed command:

```text
Run a Bash tool command using PowerShell syntax: Get-Location; Get-ChildItem -Force
```

Observed result:

- Claude invoked the `Bash` tool.
- The tool output included the `nono v0.57.4` capability banner.
- The Windows broker spawned a Low-IL child.
- The command completed with exit code 0.
- Output showed the current location as
  `C:\Users\OMack\NonoDebug\nono` and listed the repository contents.

Conclusion: the hook is installed and active, and a side-effecting tool call can
execute successfully through `nono run` while the Claude Code TUI stays outside
the Low-IL sandbox.

Denied outside-CWD write:

```text
Run a Bash tool command using PowerShell syntax that tries to write "blocked" to C:\Users\OMack\NonoDebug\outside-hook-denied-live.txt.
```

Observed result:

```text
Set-Content : Access to the path
'C:\Users\OMack\NonoDebug\outside-hook-denied-live.txt' is denied.
UnauthorizedAccessException
```

The child exited with code 1 and no file was created.

Conclusion: the write happened inside the Low-IL sandboxed tool process. The
parent directory `C:\Users\OMack\NonoDebug` is outside the granted CWD
(`C:\Users\OMack\NonoDebug\nono`), so Windows denied the write at the OS
boundary.

Git Bash/MSYS2 negative probe:

```text
Run a Bash command that prints the current directory and lists the files in it using: pwd && ls -la
```

Earlier wrapper variants tried to preserve Bash syntax by launching Git Bash
inside `nono run`. After adding the needed executable-path grants, Git Bash
started under the broker but failed during MSYS2 runtime initialization:

```text
fatal error - NtCreateDirectoryObject(\BaseNamedObjects\msys-2.0...): 0xC0000022
```

Conclusion: Git Bash/MSYS2 is not a viable Low-IL tool runner for this POC.
The failure is a Windows integrity-boundary issue in MSYS2 startup, not a
quoting problem and not an executable-path grant problem.

Denied in-process file tool:

```text
Use the Write tool to create a file named should-not-be-created-by-write-tool.txt in this repo with the text "blocked".
```

Observed result:

```text
nono tool sandbox prototype denies Write; only Bash rewriting and read-only tools are allowed
```

Conclusion: the match-all hook reaches in-process file tools before execution
and denies them. The file was not created.

Denied CWD self-disable edge case:

```powershell
Push-Location $HOME\.claude
# Then ask Claude Code to run any Bash tool command.
```

Observed result:

```text
refusing to wrap Bash: CWD 'C:\Users\OMack\.claude' covers Claude Code hook
settings or agent state; would allow the tool jail to disable its own hooks
```

Conclusion: the hook denies before producing a `nono run --allow-cwd` wrapper.
This is deliberately implemented at the hook level because Windows
`add_deny_access` is not yet a backend-enforced deny-within-allow primitive.
The profile-level deny entries remain as policy documentation and for platforms
that can enforce them, but the Windows security edge depends on the hook guard.
The same caveat applies to any Windows attempt to carve `.env`, `.git`, or a
credential directory out of a broader allowed parent: use narrower grants or a
feature-specific guard until Windows deny labels are wired through the sandbox
backend.

Network-blocked runner tradeoff:

- `network.block=true` means network-using Bash commands fail by design.
- On Windows hosts without the nono WFP driver installed, a live run with
  blocked network fails closed before the child command starts. Dry-run profile
  validation still works, but executable smoke tests need WFP installed or an
  explicit test override.

## Next Slices

1. Add live UAT for `NotebookEdit`, `WebFetch`, `Task`, and an MCP tool name to
   verify that the match-all policy denies each surface in Claude Code.
2. Add argument-derived grants for the hook handler. Static `--allow-cwd` is
   enough for the POC; per-tool path/domain grants are the later hardening step.
3. Decide whether the product wants to expose this as "Bash" on Windows or as a
   separate "PowerShell tool runner" mode, since the command syntax is
   PowerShell-backed.
4. Decide the non-Bash side-effect policy. The secure default is deny. A useful
   product mode probably needs a separate file-operation broker instead of
   trying to wrap `Write` and `Edit`.
5. Audit MCP launch paths. MCP servers must either run under `nono run` or be
   considered unconfined Medium-IL side effects.
