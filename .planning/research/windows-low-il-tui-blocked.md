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

1. **AppContainer** instead of (or alongside) the Low-IL mandatory label. AppContainer has a different
   console/capability model; investigate whether an AppContainer-confined process can host a console/ConPTY
   while still structurally denying the filesystem/network the profile forbids. Biggest unknown: does
   AppContainer give equivalent or stronger isolation than the Low-IL label for nono's threat model?
2. **Medium-IL console-broker proxy**: a cooperating Medium-IL helper hosts the real console/ConPTY and
   proxies console API calls on behalf of the Low-IL tree (beyond what ConPTY already does — ConPTY's
   conhost is the failing component). Likely large; uncertain it sidesteps the ALPC denial.
3. **Accept non-interactive as the supported mode** (status quo D′): document `nono run` / `claude -p`
   as the Windows path; deprioritize the TUI. Cheapest; already shipped.

## What's already done (do not redo)

- D′ pipe-stdio fix, cwd `\\?\` strip fix, granted-RO-dirs-on-PATH fix — all shipped + field-verified.
- B′ (ConPTY-attach) and B′′ (Low-IL conhost) — empirically proven dead (0xC0000142). Do not re-attempt
  these exact shapes; any new attempt must change the sandbox primitive, not just who creates the ConPTY.
