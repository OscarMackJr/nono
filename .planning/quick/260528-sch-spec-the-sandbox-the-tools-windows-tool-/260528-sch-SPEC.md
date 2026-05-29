# SPEC — Windows "sandbox-the-tools" mode (tool-wrapping via Claude Code hooks)

**Status:** REVIEWED — §7 answered 2026-05-29 (informed by PR #4 implementation + review; see https://github.com/oscarmackjr-twg/nono/pull/4) · **Author:** Oscar Mack Jr (+ Claude) · **Date:** 2026-05-29
**Purpose:** Hand this to a reviewer along with the testing/failures below; have them answer the
**§7 Feedback Requested** questions so the answers can be consumed back into a planned phase.
**§7 status:** Answered inline below (`> R:`). PR #4 ("Add experimental Windows Claude tool wrapping profile") implemented the thin slice and empirically settled most questions; the runner-profile `~/.claude`/`--allow-cwd` edge is tracked in todo `2026-05-29-claude-tools-runner-deny-dotclaude-regardless-of-cwd.md`.

> **How to use this doc (reviewer):** Read §1–§6. Then answer the numbered questions in **§7** —
> answer *inline directly under each question* (prefix your answer with `> R:`), or in a separate
> doc that references the question numbers. Be concrete; "it depends" answers should say *on what*.
> The single most important question is **§7 Q1 (enforceability)** — if hooks are not enforceable,
> the whole approach changes.

---

## 1. Problem

nono sandboxes untrusted AI agents with OS-enforced isolation (Low integrity level + mandatory-label
NO_WRITE_UP on Windows). The goal was: run the interactive **Claude Code TUI** *inside* that Low-IL
sandbox (`nono shell`). That goal is **OS-blocked** and proven dead — see §3.

The productive pivot: **sandbox the tools, not the TUI.** Run `claude` itself at **Medium IL** in a
real terminal (full TUI, no console wall), and confine the *operations it spawns* — Bash / file /
network tool calls — by wrapping each with `nono run` via Claude Code **hooks**. The dangerous
operations stay kernel-isolated; the TUI works; the console-subsystem wall is sidestepped entirely.

## 2. Why the TUI can't be sandboxed in-process (root cause, one paragraph)

A **Low-integrity** process cannot register with the Windows **console subsystem**. Console-client
registration (`ConClntInitialize` → `NtAlpcConnectPort` to conhost) is denied/blocked across the
integrity boundary. This is intrinsic to the Win32 console architecture, not a nono bug. Any TUI needs
a console/pseudoconsole; a Low-IL agent can't get one. The *only* thing that ever worked is a Low-IL
child **inheriting** an already-attached console (Phase-31 "A1"), which (a) gives no relay-able TTY and
(b) does **not** extend to grandchildren (the processes a shell actually spawns).

## 3. Testing & failures (the evidence to hand the reviewer)

All on Windows 11 build-26200. Commits are on `main` in this repo. Reproducers are throwaway
`windows-sys` binaries (not workspace members → zero production risk).

| # | Approach | Mechanism | Result | Evidence |
|---|----------|-----------|--------|----------|
| 0 | Original `nono shell` | grandchildren on the real console (Medium-IL conhost) | **silent HANG**, zero output | resolved debug `nono-shell-claude-hang` |
| 1 | **D′** (shipped fix) | anonymous-PIPE stdio for the whole Low-IL tree (no console) | **works, but no TTY** → interactive TUI never engages | commit `40c11831`; `claude --version` works, bare `claude` exits with no UI |
| 2 | **B′** | Medium-IL parent creates a ConPTY, Low-IL child attached (`PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE`) | **`0xC0000142`** STATUS_DLL_INIT_FAILED | PoC `poc-broker.exe --conpty`, commit `8416ff60` |
| 3 | **B′′** | a **Low-IL** helper creates the ConPTY (→ Low-IL conhost), Low-IL child attached | **`0xC0000142`** (same) | PoC `poc-broker.exe --conpty-lowil`, commit `efe638d4` |
| 4 | **AppContainer** | AppContainer-confined `cmd.exe` attached to a ConPTY | **`0xC0000142`** (AppContainer tokens are themselves Low IL) | spike `001-appcontainer-conpty-tui` (INVALIDATED) |

**Takeaway:** every confinement primitive that lowers integrity (raw Low-IL, AppContainer) re-trips the
same loader crash on a console/ConPTY. A real TUI requires Medium+ integrity. There is **no
console-subsystem path** for a Low-integrity client. Reproduce: build + run the PoCs above on a Win11
box (see each artifact's README).

**Status quo that DOES work:** `nono run --profile claude-code -- <cmd>` (non-interactive, pipe-stdio,
fully Low-IL-confined). `claude -p "..."` (print mode) works confined too. Only the *interactive TUI*
is the gap.

## 4. Proposed architecture (sandbox-the-tools)

```
┌─ Medium IL ─ real terminal ────────────────────────────────┐
│  claude (Claude Code TUI)   ← full TUI, unconfined process  │
│     │ wants to run a tool (Bash / Write / WebFetch / ...)   │
│     ▼                                                       │
│  Claude Code PreToolUse hook                                │
│     │ rewrites/wraps the tool invocation                    │
│     ▼                                                       │
│  nono run --profile <p> --allow <paths> -- <the tool cmd>   │
│     │ spawns                                                │
│     ▼                                                       │
│  ┌─ Low IL ─ kernel-confined (NO_WRITE_UP / WFP net) ─────┐ │
│  │  the actual Bash command / file op / network call      │ │
│  └────────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────┘
```

**Invariant:** the agent's *reasoning loop* (claude) is trusted-enough to run at Medium IL with a TTY,
but every *side-effecting operation it requests* is executed inside a Low-IL `nono run` jail. The
security boundary moves from "the agent process" to "each tool invocation."

## 5. Existing foundation (NOT greenfield) + the gap

nono already ships Claude Code hook integration:
- `crates/nono-cli/src/hooks.rs` — hook installation logic.
- `crates/nono-cli/data/hooks/nono-hook.sh` — the hook script (**bash/Unix**), whose example wraps
  *claude itself*: `nono run --allow /path/to/needed -- claude`.

**Gaps to close for this spec:**
1. **Per-tool-call wrapping**, not whole-session wrapping. A `PreToolUse` hook must intercept each
   tool call and run *that command* under `nono run`, rather than wrapping the outer `claude` process.
2. **A Windows hook script** — the current one is `.sh`; Windows needs a `.ps1`/`.cmd` equivalent (or a
   cross-platform invocation).
3. **Per-tool → capability mapping** — each tool class needs the right grants (e.g. `Bash` → cwd r+w +
   the dirs it touches; `WebFetch` → `allow_domain`; `Write` → the target path). How is the grant set
   derived per call?

## 6. Threat model (the crux — read before §7)

The whole approach rests on one assumption: **Claude Code hooks are an enforceable interception point,
not merely advisory.** If the agent (or a prompt-injected instruction) can perform a side effect
*without* going through a `nono run`-wrapped hook — e.g. a tool type with no hook, an MCP server, a
built-in capability, or by the model directly emitting bytes the harness executes unwrapped — then the
Medium-IL claude process is unconfined and the isolation guarantee is **weaker than the current Low-IL
`nono shell`**, not equivalent. This is the make-or-break question (§7 Q1).

Secondary threats: (a) a tool call whose capability set is computed too broadly re-opens write/network
the profile meant to deny; (b) the Medium-IL claude process can read/modify anything the user can
(no FS confinement on the agent itself) — is that acceptable for the threat model, or must the agent
process also be constrained?

## 7. Feedback requested (answer these — `> R:` inline)

**Q1 (BLOCKING — enforceability).** Are Claude Code hooks (PreToolUse) a *complete and enforceable*
interception point for ALL side-effecting operations claude can perform (Bash, file Write/Edit, WebFetch,
MCP tools, anything else)? Can any side effect bypass them? If bypass is possible, this approach does not
provide an isolation guarantee — say so plainly.

> R: **NO — hooks are an enforceable *mediation* point for the tools the matcher covers, but NOT a complete isolation boundary.** PreToolUse can return `deny`/rewrite, so for every tool routed to the handler the interception is real and binding. Three things break "complete":
> 1. **Matcher coverage.** Any tool the matcher doesn't list runs unhooked → unconfined. The fix is matcher `"*"` + a per-tool **deny-by-default** handler (allow read-only, rewrite Bash, deny all else incl. `NotebookEdit`/`WebFetch`/`WebSearch`/`Task`/`mcp__*`). PR #4's first cut shipped `matcher: "Bash|Write|Edit|MultiEdit"`, which silently left `NotebookEdit` (an in-process write!), `WebFetch`, MCP, etc. unconfined; fixed in `04bc6743`.
> 2. **Self-disable.** If the tool jail can write the hook config, it removes its own enforcement. The `claude-code` profile grants r+w to `~/.claude` (where `settings.json` lives), so a confined Bash call could rewrite it. Fixed by wrapping Bash under a runner profile that `extends: default` (no `~/.claude` grant). Residual: `--allow-cwd` re-grants `~/.claude` if `claude` is launched from there — tracked in todo `2026-05-29-claude-tools-runner-deny-dotclaude-regardless-of-cwd.md`.
> 3. **Out-of-band side effects.** The Medium-IL `claude` process itself, and any MCP **server** process, live entirely outside the `nono run` boundary. Hooks can deny an MCP *tool event*, but cannot confine an already-running Medium-IL MCP server.
>
> Verdict: usable as **defense-in-depth tool mediation**, not as agent isolation. State it that way (PR #4's docs now do).

**Q2 (tool coverage).** Enumerate the tool types that need wrapping and which can/can't be expressed as
`nono run -- <cmd>`. Bash is obvious; what about in-process file Write/Edit (no subprocess to wrap)? MCP?

> R: **Wrappable** (has a subprocess): **`Bash`** only. **Not wrappable** (no subprocess boundary): in-process file tools **`Write` / `Edit` / `MultiEdit` / `NotebookEdit`**, and **MCP** tools (the server is a separate Medium-IL process). **No wrap needed** (no side effect): read-only **`Read` / `Glob` / `Grep`** — but these are *unconfined reads* (see Q5). Correct handling for the unwrappable set is **deny** (PR #4 denies them), or, as a later option, route file ops through a dedicated low-IL file broker and launch MCP servers themselves under `nono run`. `Task` (subagent spawn) is also denied — it would otherwise spawn an unconfined reasoning loop.

**Q3 (capability mapping).** For each wrappable tool, how should the per-call `nono run` capability set be
derived? Static per-profile, or computed from the tool's arguments (e.g. the file path being written)?
What's the failure mode when the grant is too narrow (tool fails) vs too broad (isolation leak)?

> R: PR #4 uses a **static per-profile** grant: `--allow-cwd` + the `claude-code-tools-windows-runner` profile (CWD r+w, **network blocked**, no `~/.claude`). **Argument-derived** grants (Write→target path, WebFetch→`allow_domain` for the host, Bash→the specific dirs touched) are the later hardening step and are NOT built yet. Failure modes: **too narrow** → the tool fails at the OS boundary, which is the *correct fail-closed* outcome (observed in UAT: outside-CWD write → `UnauthorizedAccessException`, no file created); **too broad** → isolation leak (e.g. granting `~/.claude` reopens hook-config tampering — Q1.2). Until per-arg grants exist, keep the static set minimal and **deny** the unwrappable tools rather than guessing a broad grant.

**Q4 (Windows hook mechanics).** What's the right Windows hook implementation — a `.ps1`/`.cmd` script, or
a `nono`-native hook handler? How does the hook receive the tool command + rewrite it on Windows?

> R: A thin **`.ps1` trampoline → native `nono claude-code-hook` handler** (PR #4 ships exactly this, and it's the right shape). The `PreToolUse` hook pipes the tool-event JSON on stdin to `nono claude-code-hook`, which returns a `permissionDecision` — `allow` + `updatedInput` (Bash command rewritten to `nono run --profile <runner> --allow-cwd -- powershell …`), or `deny`. The `.ps1` is fail-closed: if the handler errors/non-zero, it emits `deny`. Windows "Bash" must be **native PowerShell-backed, not Git Bash/MSYS2** — MSYS2 can't initialize as a Low-IL child (`NtCreateDirectoryObject(\BaseNamedObjects\msys-2.0…) 0xC0000022`), an integrity-boundary failure, not a quoting/PATH issue. Use two base64 `-EncodedCommand` layers (outer trampoline + inner tool command) to avoid nested-quoting fragility.

**Q5 (security verdict).** Given §6, is "Medium-IL agent + per-tool Low-IL wrapping" an acceptable security
posture for nono's threat model (untrusted agent), or a regression vs the current confined-but-non-TUI
`nono run` path? What would make it acceptable (e.g. also confining the agent process's FS)?

> R: **Not acceptable as agent isolation; acceptable as explicitly-labeled defense-in-depth.** As-is the `claude` process runs Medium-IL with full user FS + network — strictly *weaker* for the agent itself than today's Low-IL `nono run` path. What makes the *tool-mediation* layer sound (PR #4 after review): matcher `"*"` + deny-by-default, Bash jailed under a minimal runner profile (no `~/.claude`, network blocked), all unwrappable tools denied. To approach real isolation you'd additionally confine the agent process's FS — or consciously **accept** that the agent can *read* anything the user can (writes/exec/network are mediated, reads are not). The make-or-break product decision: **is unconfined agent reading (of secrets/creds) acceptable for the untrusted-agent threat model?** If no, this mode is defense-in-depth only and the confined non-TUI `nono run` path remains the isolation story.

**Q6 (scope/sizing).** Is this one phase or several? What's the thinnest end-to-end slice that proves the
model (e.g. PreToolUse hook wrapping only `Bash` on Windows)?

> R: **Several phases.** The thinnest honest end-to-end slice — now realized by PR #4 — is: Windows `PreToolUse` matcher `"*"` + allow `Read`/`Glob`/`Grep` + rewrite `Bash` through a minimal runner profile + **deny everything else** + label defense-in-depth. Follow-on phases: (a) close the `--allow-cwd`/`~/.claude` edge (todo `2026-05-29-claude-tools-runner-deny-dotclaude-regardless-of-cwd.md`); (b) argument-derived capability grants (Q3); (c) MCP-under-`nono` or MCP-deny policy + audit MCP launch paths; (d) optional Low-IL file-operation broker so `Write`/`Edit` aren't merely denied; (e) decide the agent-process FS-confinement / unconfined-reads question (Q5). Promote this SPEC + these answers into a planned phase to sequence (a)–(e).

## 8. References (in this repo)

- `.planning/research/windows-low-il-tui-blocked.md` — full dead-end matrix + pivot rationale + foundation/gap.
- `.planning/debug/resolved/nono-shell-claude-hang.md` — resolved debug session (D′ fix, full evidence trail).
- `.planning/spikes/001-appcontainer-conpty-tui/` — AppContainer spike (INVALIDATED, `0xC0000142`).
- `.planning/quick/260508-m99-.../poc-broker` — B′ (`--conpty`) and B′′ (`--conpty-lowil`) PoCs.
- `crates/nono-cli/src/hooks.rs`, `crates/nono-cli/data/hooks/nono-hook.sh` — existing hook foundation.
- Commits: `40c11831` (D′), `2fb0fad2` (cwd), `738690a5` (PATH), `8416ff60`/`efe638d4` (PoCs).
