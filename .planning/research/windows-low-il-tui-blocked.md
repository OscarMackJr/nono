# Research seed: interactive TUI inside the Low-IL Windows `nono shell`

**Status:** open research item (seed for a future `/gsd:plan-phase`). Created 2026-05-28 from the
resolved debug session `nono-shell-claude-hang` (see `.planning/debug/resolved/nono-shell-claude-hang.md`).

## Problem

Inside `nono shell --profile claude-code --allow-cwd` (a Low-IL mandatory-label sandbox), the
**interactive** `claude` Code TUI does not launch. Non-interactive use works (`claude --version`,
`claude -p "..."`), and all grandchildren run, after the v0.57.4 fixes — but the full-screen Ink/raw-mode
TUI never renders.

## Definitive finding (why it's hard)

A **Low-IL child process cannot register with the Windows console subsystem.** Console-client
registration (`ConClntInitialize` → `NtAlpcConnectPort` to conhost) is denied/blocked across the
integrity boundary. Three independent shapes were tested and all fail:

| Approach | Mechanism | Result |
|---|---|---|
| Original `nono shell` | grandchildren on nono.exe's real console (Medium-IL conhost) | silent **HANG** (cross-IL ALPC never completes) |
| **D′** (shipped fix) | anonymous-pipe stdio, no console at all | works, but **no TTY** → TUI won't engage |
| **B′** PoC | Medium-IL parent creates ConPTY, Low-IL child attached | **0xC0000142** STATUS_DLL_INIT_FAILED |
| **B′′** PoC | Low-IL helper creates ConPTY (intended Low-IL conhost), Low-IL child attached | **0xC0000142** |

The only thing that ever worked for the **direct** child is the Phase-31 A1 inherited-console skip
(the child inherits an already-attached console and skips registration) — but that gives no relay-able
TTY and does not extend to grandchildren. This matches the Phase-30 root cause
(`.planning/debug/resolved/nono-shell-status-dll-init-failed.md`).

**Conclusion:** an interactive TUI is not reachable via the console subsystem while the agent runs at
Low IL. PoCs proving B′/B′′ dead: `.planning/quick/260508-m99-broker-process-poc-minimal-rust-binary-t/poc-broker`
(`--conpty`, `--conpty-lowil`), commits `8416ff60` + `efe638d4`.

## Constraint

The Low-IL mandatory-label model (NO_WRITE_UP) is the core structural-isolation guarantee. Any TUI fix
must NOT weaken it (no running the agent at Medium IL, no dropping the label).

## Research directions for a future phase

1. ~~**AppContainer** instead of the Low-IL mandatory label.~~ **TESTED + DEAD (spike 001, 2026-05-28).**
   An AppContainer-confined `cmd.exe` attached to a ConPTY dies with the same `0xC0000142`. Root reason: an
   AppContainer token is itself **Low integrity** (Low IL + AppContainer SID + capability SIDs), so its
   console-subsystem client registration is denied exactly as for a mandatory-label Low-IL child. AppContainer
   changes the capability/ACL model, not the integrity level → it does NOT escape the console wall. PoC:
   `.planning/spikes/001-appcontainer-conpty-tui`. **Do not re-attempt.**
2. **Medium-IL console-broker proxy**: a cooperating Medium-IL helper hosts the real console/ConPTY and
   proxies console API calls on behalf of the Low-IL tree. Beyond what ConPTY already does (ConPTY's conhost
   is the failing component); would amount to re-implementing the console subsystem. Large, uncertain,
   low priority.
3. **★ RECOMMENDED — sandbox-the-tools, not sandbox-the-TUI (architecture pivot, no console work).** The
   structural answer is to stop trying to confine the interactive process. Run `claude` itself at Medium IL
   in a real terminal (full TUI, no console wall), and confine the *operations it spawns* — Bash/file/network
   tool calls — by wrapping them with `nono run` via Claude Code hooks. Dangerous operations stay
   kernel-isolated; the TUI works. This sidesteps the console subsystem entirely and matches how agent
   sandboxing is practically deployed.

   **Existing foundation (NOT greenfield):** nono already ships Claude Code hook integration —
   `crates/nono-cli/src/hooks.rs` (hook installation) + `crates/nono-cli/data/hooks/nono-hook.sh`. BUT the
   current script is **bash/Unix-oriented** and its example wraps *claude itself* (`nono run --allow … -- claude`),
   not the individual tool calls. **Gap for a pivot phase:** (a) a `PreToolUse`-style hook that wraps each
   Bash/tool invocation with `nono run` (intercept + rewrite the tool command), rather than wrapping the whole
   session; (b) a **Windows** hook script (the current one is `.sh`); (c) per-tool → capability mapping (which
   grants each tool class needs). Open design questions → this phase wants a `discuss-phase`/research pass
   before planning. It is a new capability (a second Windows sandboxing mode) — milestone-worthy, NOT a fit
   for the v2.8 drain+release scope.
4. **Accept non-interactive as a supported mode** (status quo D′): `nono run` / `claude -p` for fully-confined
   non-interactive use. Already shipped; complements (3).

## Conclusion (post-spike)

The interactive TUI is **OS-blocked for every Windows confinement primitive that lowers integrity** — raw
Low-IL (hang / 0xC0000142) and AppContainer (0xC0000142). There is no console-subsystem path for a
Low-integrity client; a TUI requires Medium+ integrity, which abandons in-process structural isolation. The
productive direction is the architecture pivot (#3): sandbox the tool invocations, not the agent's terminal.

## What's already done (do not redo)

- D′ pipe-stdio fix, cwd `\\?\` strip fix, granted-RO-dirs-on-PATH fix — all shipped + field-verified.
- B′ (ConPTY-attach) and B′′ (Low-IL conhost) — empirically proven dead (0xC0000142). Do not re-attempt
  these exact shapes; any new attempt must change the sandbox primitive, not just who creates the ConPTY.
