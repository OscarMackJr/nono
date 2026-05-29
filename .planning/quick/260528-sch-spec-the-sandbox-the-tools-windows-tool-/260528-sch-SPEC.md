# SPEC — Windows "sandbox-the-tools" mode (tool-wrapping via Claude Code hooks)

**Status:** DRAFT FOR EXTERNAL REVIEW · **Author:** Oscar Mack Jr (+ Claude) · **Date:** 2026-05-29
**Purpose:** Hand this to a reviewer along with the testing/failures below; have them answer the
**§7 Feedback Requested** questions so the answers can be consumed back into a planned phase.

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

**Q2 (tool coverage).** Enumerate the tool types that need wrapping and which can/can't be expressed as
`nono run -- <cmd>`. Bash is obvious; what about in-process file Write/Edit (no subprocess to wrap)? MCP?

**Q3 (capability mapping).** For each wrappable tool, how should the per-call `nono run` capability set be
derived? Static per-profile, or computed from the tool's arguments (e.g. the file path being written)?
What's the failure mode when the grant is too narrow (tool fails) vs too broad (isolation leak)?

**Q4 (Windows hook mechanics).** What's the right Windows hook implementation — a `.ps1`/`.cmd` script, or
a `nono`-native hook handler? How does the hook receive the tool command + rewrite it on Windows?

**Q5 (security verdict).** Given §6, is "Medium-IL agent + per-tool Low-IL wrapping" an acceptable security
posture for nono's threat model (untrusted agent), or a regression vs the current confined-but-non-TUI
`nono run` path? What would make it acceptable (e.g. also confining the agent process's FS)?

**Q6 (scope/sizing).** Is this one phase or several? What's the thinnest end-to-end slice that proves the
model (e.g. PreToolUse hook wrapping only `Bash` on Windows)?

## 8. References (in this repo)

- `.planning/research/windows-low-il-tui-blocked.md` — full dead-end matrix + pivot rationale + foundation/gap.
- `.planning/debug/resolved/nono-shell-claude-hang.md` — resolved debug session (D′ fix, full evidence trail).
- `.planning/spikes/001-appcontainer-conpty-tui/` — AppContainer spike (INVALIDATED, `0xC0000142`).
- `.planning/quick/260508-m99-.../poc-broker` — B′ (`--conpty`) and B′′ (`--conpty-lowil`) PoCs.
- `crates/nono-cli/src/hooks.rs`, `crates/nono-cli/data/hooks/nono-hook.sh` — existing hook foundation.
- Commits: `40c11831` (D′), `2fb0fad2` (cwd), `738690a5` (PATH), `8416ff60`/`efe638d4` (PoCs).
