# Feature Research

**Domain:** Engine-agnostic AI-agent confinement (nono v2.12 "AI Agent Abstraction") — Windows
**Researched:** 2026-06-13
**Confidence:** HIGH (engine launch models verified against vendor docs + DeepWiki; confinement primitive already proven by spike 003)

---

## Framing: what the abstraction boundary actually is

nono confines at the **OS process boundary**. `nono run -- <exe>` applies a Low-IL broker token + AppContainer/WFP to a child process, and everything that process (and its descendants) does is OS-enforced. Spike 003 proved this is engine-neutral (cmd / powershell / python confined identically).

Therefore the ONE question that determines how an engine fits is:

> **When the engine performs a risky operation (file write, shell, network), does that operation cross a process boundary nono can wrap, or does it happen in-process inside the already-confined engine process?**

Two consequences fall out of this:

1. **If nono launches the engine process itself**, then the engine + all its risky ops are *already inside* the sandbox. In-process file writes, in-process Python `exec()`, and subprocess shells are ALL confined transitively — because the confinement is on the engine process and inherited by descendants. This is the **launch-and-confine** model.

2. **If something else launches the engine** (an IDE, a pre-existing agent process, a CI runner), nono must either (a) adopt/relaunch it, or (b) sit between the engine and its tool calls via a **hook/shim** (the current Claude Code PreToolUse model, which rewrites each tool call to `nono run`).

So "what every engine must expose" is small and concrete (below), but the *delivery mechanism* splits engines into two camps. The roadmap-relevant per-engine table is in the section after the feature landscape.

---

## What every engine MUST expose (the abstraction boundary)

This is the contract every engine has to satisfy for nono to mediate it. Drawn from the banked spike-003 contract and confirmed against each engine's launch behavior.

| # | Exposed thing | Why nono needs it | Verified against |
|---|---------------|-------------------|------------------|
| **E1** | **Engine executable + interpreter path(s)** | The launch policy MUST cover the engine binary (and its interpreter — `python.exe`, `node.exe`) or nono fail-secure refuses to launch. `--allow <exe-dir>`. | All engines. Aider→`python.exe`; Copilot/Cursor/Claude→`node.exe`; LangChain→`python.exe`. |
| **E2** | **A launch command nono can own** (argv + env) | Launch-and-confine needs to be the parent. If a third party owns the spawn, fall back to adopt or hook. | cmd/ps/python proven (003). |
| **E3** | **Intended writable workspace as an ABSOLUTE path** | Granted + relabeled Low-writable. Engines do NOT uniformly inherit launcher CWD (PowerShell resolved a relative write to `C:\`). | 003 (PowerShell CWD divergence). |
| **E4** | **A network identity for WFP scoping** (AppContainer package SID, broker no-PTY arm) | Per-agent kernel network enforcement. | Phase 62 WFP + AppContainer. |
| **E5** | **(hook-camp only) A pre-execution interception point** | For engines nono cannot be the parent of, the engine must offer a hook/callback that fires *before* a tool runs and can redirect it (Claude PreToolUse; Copilot hooks; Cursor permissions/MCP). | Claude (built); Copilot (JSON-RPC hooks); Cursor (permission gate). |

E1–E4 are the launch-and-confine contract (already validated). **E5 is the new contract this milestone must define** for engines nono can't parent — and it is the part with real per-engine variance.

---

## Feature Landscape

### Table Stakes (Users Expect These)

Features without which "engine-agnostic" is a false claim.

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| **Generic launch-and-confine** (`nono run --profile <runner> -- <engine.exe> <args>` for any engine) | This is the headline promise. Spike 003 already proved it works for cmd/ps/python. | LOW | Productionize the validated path; honor E1 exe-coverage + E3 absolute grants. The primitive exists. |
| **Per-engine launch profile** (exe/interpreter coverage list + writable-workspace grant baked in) | Users won't hand-enumerate `node.exe`/`python.exe` paths per engine. Profiles must ship the E1/E3 facts. | MEDIUM | One profile per engine: `aider`, `copilot-cli`, `cursor-agent`, `langchain-python`. Mirrors existing `claude-code` profile shape (`target_binary`, `windows_low_il_broker`). |
| **Fail-secure on uncovered exe/interpreter** | Core nono invariant — never silently degrade. Already the behavior; must hold for every engine. | LOW | python was refused until `--allow`'d (003). Keep that gate; just make the profile supply the allow. |
| **Engine descendant confinement** (subprocess shells the engine spawns stay confined) | Aider's `/run`, Copilot's child-process shell, Cursor's `$SHELL` commands all spawn children. Users expect those confined too. | LOW | Free with launch-and-confine — Low-IL token + Job Object are inherited by descendants. No per-engine work. |
| **In-process operation confinement** (engine's own file writes / Python `exec()`) | Aider writes files in-process; LangChain `PythonREPLTool` runs `exec()` in-process. These have NO child boundary to wrap. | LOW (transitive) | Confined *because the engine process itself is sandboxed*. This is the killer argument for launch-and-confine over hooks. |
| **Workspace ownership / relabel handling** | Confined writes fail if the workspace dir is owned by `BUILTIN\Administrators` (no `WRITE_DAC`). Cross-engine, same trap. | MEDIUM | `takeown` / non-elevated create. R-B3. Surface a clear diagnostic, not a silent deny. |
| **Per-engine "fit" documentation** (launch-and-confine vs hook/shim) | Users need to know Cursor-on-Windows-is-WSL-only *before* they try it. | LOW | Doc + a `nono setup`-style capability probe. |

### Differentiators (Competitive Advantage)

Where nono out-competes per-engine ad-hoc sandboxing (Docker, devcontainers, vendor "trusted folder" prompts).

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| **Persistent multi-tenant daemon** (one long-running launcher adopts/launches many agents, marks each `AI_AGENT` job/SID, serves per-agent capability requests over one multi-client pipe) | Run several confined agents at once with zero per-launch startup cost; the marquee v2.12 capability. Generalizes the supervisor IPC. | **HIGH** (riskiest; former spike 004) | Not yet spiked. Start point: `crates/nono/src/supervisor/socket_windows.rs`. Token/job *reuse* across agents is the unproven part. |
| **`nono-py` engine binding** (confine a real Python/LangChain agent with NO Claude hook, via the library binding) | Proves "engine is a variable" in code, not just on the CLI; lets framework authors embed confinement. | HIGH (former spike 005) | The formal proof of the abstraction boundary. Targets the in-process-exec case directly. |
| **Uniform kernel network enforcement across engines** (WFP per-agent, deny-by-default) | OS-enforced egress control regardless of engine — Docker-grade isolation without Docker, and it works for in-process network calls too. | MEDIUM (primitive exists) | Reuse Phase 62 AppContainer+WFP; per-engine E4 SID. Needs `nono-wfp-service` (elevated). |
| **Post-hoc demote control** (drop the IL of a misbehaving/escaped agent — spike 002) | Incident-response lever: clamp an agent that's already running. | MEDIUM (supplementary) | **Demote-only.** Explicitly NOT a standalone confinement boundary (leaky/unsound as primary — see anti-features). |
| **Cross-platform parity of the abstraction** | Same engine-agnostic model on Linux (Landlock) / macOS (Seatbelt), not just Windows. | MEDIUM | The launcher contract is platform-neutral; backends already exist. Lower priority for this Windows-focused milestone. |

### Anti-Features (Commonly Requested, Often Problematic)

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|-----------------|-------------|
| **"Detect-and-confine any running AI_AGENT process" as the primary model** | Feels magical — confine agents you didn't launch. | Post-hoc IL-drop is leaky/unsound (kernel objects already opened, handles already held, threads already running at higher IL). Banked finding: use launch-time confinement; post-hoc only to demote. | Launch-and-confine (E2) as primary; post-hoc IL-drop strictly as a demote/IR lever. |
| **Per-engine deep API integration / plugin per engine** | "Native" feel; intercept each engine's own tool dispatcher. | Couples nono to each vendor's internal tool API (churns constantly — Copilot subagents, Cursor MCP, Aider coders). Re-creates the Claude-hook-specificity the milestone is trying to delete. | Confine at the OS process boundary (engine-neutral). Only use the engine's hook (E5) when nono *cannot* be the parent. |
| **In-engine "ask the engine nicely to restrict itself"** (rely on the agent's own permission prompts / trusted-folder feature) | Copilot, Cursor, Claude all ship approval prompts — "just use those." | Not OS-enforced; the agent (or a prompt-injected tool call) can bypass app-level gating. Defeats nono's entire structural-impossibility thesis. | OS-enforced Low-IL + WFP underneath, regardless of the engine's own prompts. The engine prompt is UX; nono is the wall. |
| **Wrap each individual tool call as its own `nono run`** (generalize the Claude PreToolUse hook to all engines) | It's what the Claude path does today; "just do that for everyone." | Per-call process spawn = startup cost × N tool calls; requires every engine to expose a rewrite-capable pre-hook (most don't, or expose a weaker one); doesn't confine in-process ops. It was a workaround for not owning the Claude spawn. | Own the *engine* spawn once (launch-and-confine). Keep per-tool hooking only for engines you can't parent (e.g. IDE-embedded). |
| **Auto-elevate to fix workspace-ownership failures** | "Confined write failed because dir is admin-owned — just elevate and fix it." | Running the agent workspace from an elevated context is exactly the R-B3 trap (admin-owned dir, no WRITE_DAC, fail-secure deny). Elevating makes it worse, not better. | Require user-owned workspace; `takeown`/non-elevated create; clear diagnostic. |
| **Confining Cursor agent natively on Windows (no WSL)** | Cursor CLI is popular; users will ask for native Windows confinement. | Official `cursor-agent` CLI is **Linux/macOS-only**; Windows runs it under WSL (a separate Linux namespace nono's Windows backend doesn't reach). | Confine Cursor via the Linux/Landlock backend inside WSL, or document it as WSL-scoped. Don't fake native Windows support. |

---

## Per-Engine Launch Model & Fit

The decisive table for the roadmap. "Fit" = does nono own the spawn (launch-and-confine, the sound model) or must it hook/shim.

| Engine | Process / interpreter (E1) | How it runs risky ops | nono fit | What it must expose beyond E1–E4 | Confidence |
|--------|---------------------------|------------------------|----------|----------------------------------|------------|
| **Claude Code** | `node` (bundled `claude.exe`, 234 MB self-contained native on Win) | Tool calls dispatched in-process; shell tool spawns children | **Hook today; launch-and-confine target.** Already mediated via PreToolUse hook→`nono run`. v2.12 can also launch `claude.exe` directly under the broker (Phase 51/52 proved `nono run -- claude` works). | E5 already exposed (PreToolUse). For launch-and-confine, nothing new — exe is known, broker arm default-on. | HIGH (built) |
| **Aider** | **`python.exe`** (`pip install aider-chat`; runs as a Python process) | **File edits = in-process Python file I/O** (no child boundary). Shell via `/run`, `/test`, auto-lint/auto-test = subprocess spawned from the python process. | **Launch-and-confine — ideal.** nono parents `python.exe -m aider` (or the `aider` console-script shim). In-process writes + subprocess shells both confined transitively. | E1: cover `python.exe` + the aider script dir. E3: repo dir as absolute writable grant (aider is git-repo-rooted). No engine hook needed. | HIGH |
| **GitHub Copilot CLI** | `node` (`copilot` / `@github/copilot`) | All tool calls (read/write file, shell, MCP) flow through one permission/validation pipeline; **shell = child processes**; extensions = separate child processes over JSON-RPC/stdio | **Launch-and-confine — ideal.** Parent the `copilot` node process; children + MCP servers inherit confinement. | E1: cover `node.exe` + copilot install dir. Optionally E5 (its JSON-RPC hook system) if per-call policy is wanted, but not required. | HIGH |
| **Cursor agent CLI** | `agent` / `cursor-agent` binary at `~/.local/bin` (node under the hood) | Commands run in the **login `$SHELL`** with the CLI's CWD/env; file ops + web + MCP tools | **Launch-and-confine on Linux/macOS; WSL-only on Windows.** Official CLI does not run natively on Windows. | E1 + E3 as usual *but on the Linux backend* (inside WSL). Windows-native = not supported by the engine. | HIGH |
| **Custom Python / LangChain** | **`python.exe`** | **`PythonREPLTool` = in-process `exec()`** (no child boundary at all); `ShellTool` = subprocess; file writes = in-process Python I/O | **Launch-and-confine — and the strongest proof case.** nono parents the python entrypoint; the in-process `exec()` is confined because the whole interpreter is sandboxed. This is exactly what `nono-py` (differentiator) must demonstrate. | E1: cover `python.exe` + site-packages launcher. E3: workspace absolute grant. The `nono-py` binding can apply confinement from *inside* the process at startup (alternative to external launch). | HIGH |

**Reading of the table:** every engine except Windows-native Cursor fits the **launch-and-confine** model cleanly. The hook/shim path is needed only when nono cannot be the parent (IDE-embedded Cursor/Copilot, or an already-running agent) — and there it depends on E5, which only Claude (built) and Copilot (JSON-RPC) expose robustly. **The milestone's center of gravity is launch-and-confine + profiles, not generalizing the hook.**

---

## Feature Dependencies

```
Generic launch-and-confine (table stakes)
    └──requires──> Engine launch profile (exe/interpreter coverage + workspace grant)
                       └──requires──> Fail-secure exe-coverage gate (exists)
                       └──requires──> Workspace-ownership/relabel handling (R-B3)

Persistent multi-tenant daemon (differentiator, riskiest)
    └──requires──> Generic launch-and-confine
    └──requires──> Per-agent AI_AGENT marker (job/SID)
    └──requires──> Multi-client capability pipe  ──generalizes──> supervisor/socket_windows.rs

nono-py engine binding (differentiator)
    └──requires──> Generic launch-and-confine semantics expressed in the library
    └──enables──> proving the abstraction on in-process LangChain exec()

Uniform WFP network enforcement
    └──requires──> per-engine network identity (E4 / AppContainer SID)
    └──requires──> nono-wfp-service running (elevated)

Post-hoc demote control (supplementary)
    └──enhances──> Persistent daemon (demote a marked AI_AGENT)
    └──conflicts──> "detect-and-confine as primary model" (anti-feature)
```

### Dependency Notes

- **Launch profiles gate everything:** the whole milestone is "make `nono run -- <engine>` Just Work per engine," which means each engine needs a profile carrying its E1 (exe + interpreter) and E3 (workspace) facts. Build these before the daemon.
- **Daemon depends on single-launch working first:** the multi-tenant daemon is launch-and-confine + reuse + a marker + a multi-client pipe. Don't attempt it until the single-engine launch path is productionized.
- **`nono-py` and the daemon are independent** — `nono-py` proves the boundary in-library; the daemon proves it at scale. Either can ship without the other.
- **Post-hoc demote conflicts with the detect-and-confine anti-feature** — they share the IL-drop mechanism, but one is a sound IR lever and the other is an unsound primary model. Keep the framing explicit.

---

## MVP Definition

### Launch With (v1)

- [ ] **Generic launch-and-confine productionized** — the validated 003 path as a first-class, non-spike code path. Essential: it IS the abstraction.
- [ ] **Engine launch profiles for Aider + LangChain-Python** (both `python.exe`) — proves a non-Claude engine end-to-end; LangChain is the in-process-exec proof.
- [ ] **Fail-secure exe/interpreter coverage gate per engine** — non-negotiable nono invariant.
- [ ] **Workspace-ownership/relabel handling + clear diagnostic** (R-B3) — without it confined writes silently fail per engine.
- [ ] **Per-engine fit documentation** (launch-and-confine vs hook vs WSL-only) — sets correct expectations, esp. Cursor-on-Windows.

### Add After Validation (v1.x)

- [ ] **Copilot CLI profile** — once the node-engine launch shape is proven via Claude, Copilot is a second node engine; low marginal cost.
- [ ] **`nono-py` engine binding** — trigger: launch-and-confine CLI path is solid; then prove it in-library on a real LangChain agent (former spike 005). The formal abstraction proof.
- [ ] **Uniform WFP network enforcement per engine** — trigger: file-confinement stable; layer per-agent egress control (needs elevated service).

### Future Consideration (v2+)

- [ ] **Persistent multi-tenant daemon** — riskiest; token/job reuse across agents + AI_AGENT marker + multi-client pipe is unspiked. Defer until single-launch + binding are solid OR scope as the milestone's hero feature with its own spike. (former spike 004)
- [ ] **Post-hoc demote control** — supplementary IR lever; only after launch-time confinement is the proven default.
- [ ] **Native Windows Cursor** — blocked on the engine (Cursor CLI is Linux/macOS-only); revisit if/when Cursor ships a native Windows CLI.
- [ ] **Cross-platform parity of the abstraction** (Landlock/Seatbelt) — the launcher contract is platform-neutral; lower priority for a Windows milestone.

---

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| Generic launch-and-confine (productionized) | HIGH | LOW (primitive exists) | P1 |
| Engine launch profiles (Aider, LangChain-Python) | HIGH | MEDIUM | P1 |
| Fail-secure exe/interpreter coverage gate | HIGH | LOW | P1 |
| Workspace-ownership/relabel handling (R-B3) | HIGH | MEDIUM | P1 |
| Per-engine fit documentation | MEDIUM | LOW | P1 |
| Copilot CLI profile | MEDIUM | LOW | P2 |
| `nono-py` engine binding (abstraction proof) | HIGH | HIGH | P2 |
| Uniform WFP network enforcement per engine | MEDIUM | MEDIUM | P2 |
| Persistent multi-tenant daemon | HIGH | HIGH (riskiest, unspiked) | P2/P3 |
| Post-hoc demote control | LOW | MEDIUM | P3 |
| Native Windows Cursor | LOW | (engine-blocked) | P3 |

**Priority key:** P1 = must have for launch · P2 = should have, add when possible · P3 = nice to have / future.

---

## Competitor / Comparable Feature Analysis

| Approach | Docker / devcontainer | Vendor in-app "trusted folder" prompts | nono engine-agnostic |
|----------|----------------------|----------------------------------------|----------------------|
| Confines in-process engine ops (Aider writes, LangChain `exec()`) | Yes (whole container) | No (app-level only) | **Yes** (whole engine process is OS-sandboxed) |
| OS-enforced (not bypassable by prompt injection) | Yes | **No** | **Yes** (Low-IL + WFP, kernel-enforced) |
| Engine-neutral (one mechanism, many engines) | Yes but heavyweight | No (per-vendor) | **Yes** (one `nono run` path) |
| Startup cost | High (container boot) | None | **Low** (zero-startup-latency goal preserved) |
| Per-agent network egress control | Coarse (network mode) | No | **Yes** (per-agent WFP/AppContainer SID) |
| Multi-agent on one host | Heavy (N containers) | N/A | **Daemon (differentiator)** — one launcher, N marked agents |

nono's wedge: OS-enforced, engine-neutral, low-startup, and confines *in-process* operations that app-level prompts can't — without the weight of a container per agent.

---

## Sources

- Banked spike-003 contract: `.claude/skills/spike-findings-nono/references/engine-agnostic-confinement.md` (VALIDATED Win11 26200.8390 — cmd/powershell/python confined identically) — HIGH
- Milestone scope: `.planning/PROJECT.md` "Current Milestone: v2.12 AI Agent Abstraction" — HIGH
- [GitHub Copilot CLI — Tool Execution & Permissions (DeepWiki)](https://deepwiki.com/github/copilot-cli/3.5-tool-execution-and-permissions) — all tool calls through one validation pipeline; shell + extensions as child processes — HIGH
- [GitHub Copilot CLI — Agent Modes & Subagents (DeepWiki)](https://deepwiki.com/github/copilot-cli/3.6-agent-modes-and-subagents) — MEDIUM
- [About GitHub Copilot CLI — GitHub Docs](https://docs.github.com/copilot/concepts/agents/about-copilot-cli) — MEDIUM
- [Aider Documentation](https://aider.chat/docs/) and [Aider lint/test](https://aider.chat/docs/usage/lint-test.html) — python process, in-place file edits, `/run`/auto-test shell — HIGH (architecture well-established)
- [Cursor CLI — Shell Mode](https://cursor.com/docs/cli/shell-mode) — commands run in login `$SHELL` — HIGH
- [Cursor CLI — Installation](https://cursor.com/docs/cli/installation) — `~/.local/bin`, **Linux/macOS only; WSL on Windows** — HIGH
- [Cursor Agent CLI announcement](https://cursor.com/blog/cli) — MEDIUM
- [LangChain Deep Agents / tools overview](https://docs.langchain.com/oss/python/deepagents/overview) and PythonREPLTool behavior (in-process `exec()`, subprocess ShellTool) — HIGH
- [cursor-agent-cli-windows community patch (evidence official CLI is non-native on Windows)](https://github.com/gitcnd/cursor-agent-cli-windows) — MEDIUM

---
*Feature research for: engine-agnostic AI-agent confinement (Windows)*
*Researched: 2026-06-13*
