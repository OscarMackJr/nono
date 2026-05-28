---
spike: 001
name: appcontainer-conpty-tui
type: standard
validates: "Given a process confined in a Windows AppContainer attached to a ConPTY, when it runs as a console client (echo / interactive shell), then it renders output without re-tripping the Phase-30 0xC0000142 STATUS_DLL_INIT_FAILED"
verdict: INVALIDATED
related: []
tags: [windows, appcontainer, conpty, console, low-il, tui, security]
---

# Spike 001: appcontainer-conpty-tui

## What This Validates

**Given** a process confined in a Windows AppContainer (SECURITY_CAPABILITIES) attached to a ConPTY,
**when** it runs as a console client (`cmd.exe`, then `echo HI` / `powershell` / `claude`),
**then** it renders console output and returns to a prompt **without** the Phase-30
`0xC0000142 STATUS_DLL_INIT_FAILED` loader crash.

This is the **killer** question for the whole AppContainer-TUI idea. If it fails, spikes 002–004
(isolation parity) are moot.

## Research

**Prior art (do NOT re-test):** the raw Low-IL `nono shell` cannot host a TUI — a Low-IL child cannot
register with the Windows console subsystem. Proven dead: B′ (Medium-IL ConPTY → Low-IL child) and B′′
(Low-IL conhost → Low-IL child) both die `0xC0000142`; pipe-stdio (D′, shipped) has no TTY. Full analysis:
`.planning/research/windows-low-il-tui-blocked.md`; resolved debug session
`.planning/debug/resolved/nono-shell-claude-hang.md`; PoCs
`.planning/quick/260508-m99-broker-process-poc-minimal-rust-binary-t/poc-broker` (`--conpty`, `--conpty-lowil`).

**Key risk / why this is genuinely uncertain:** an AppContainer token **is itself Low integrity** (Low IL +
AppContainer SID + capability SIDs). So an AppContainer console client could hit the *exact same*
cross-IL console-subsystem wall and `0xC0000142`. The reason it's worth testing anyway: AppContainer is a
distinct confinement model (capability/ACL-based, with its own brokering for some subsystems), and its
console codepath is not necessarily identical to a bare mandatory-label Low-IL spawn. Empirical test settles it.

**Approach:** mirror the dead B′ PoC exactly (Medium-IL parent creates a ConPTY, spawns a child attached via
`PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE`, relays stdio), but place the child in an AppContainer via
`CreateAppContainerProfile` + `PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES` instead of a raw Low-IL token.
0 capabilities (bare AppContainer) — system DLLs in System32 are readable by ALL_APPLICATION_PACKAGES, so
`cmd.exe` should load. The diff vs B′ is *only* the confinement mechanism, isolating the AppContainer variable.

| Approach | Mechanism | Status |
|----------|-----------|--------|
| B′ (dead) | Medium-IL ConPTY + raw Low-IL token | 0xC0000142 |
| B′′ (dead) | Low-IL conhost + raw Low-IL token | 0xC0000142 |
| **001 (this)** | Medium-IL ConPTY + **AppContainer** (SECURITY_CAPABILITIES) | **testing** |

## How to Run

On the Win11 build-26200 box, from a normal PowerShell console:

```
cargo build --release   # in .planning/spikes/001-appcontainer-conpty-tui (already built)
.\target\release\appcontainer-conpty-tui.exe
```

At the prompt it spawns (cmd.exe in the AppContainer):
1. `echo HI`            ← the console signal (does an AppContainer process produce console output?)
2. `powershell -NoLogo` ← optional: does a heavier console client run in the AppContainer?
3. `claude`             ← optional bonus: does any TUI frame render? (claude may fail on network — a bare
                           AppContainer has no `internetClient` capability — but a rendered frame answers the
                           console question)
4. `exit`               ← ends the spike; the child exit code is interpreted.

## What to Expect

- **`echo HI` prints `HI` and the prompt returns, no error** → AppContainer CAN be a ConPTY console client →
  VALIDATED (proceed to 002–004).
- **Immediate exit `0xC0000142` (no usable prompt)** → AppContainer re-trips the same console wall as Low-IL →
  INVALIDATED → idea dead; 002–004 moot.
- **Prompt appears but `echo` hangs / no output** → PARTIAL → capture details (which command, any error).

The binary prints a `[SPIKE-001]` verdict line + the child exit code at the end.

## Investigation Trail

- 2026-05-28: Built the spike (standalone `windows-sys` crate, mirrors poc-broker `--conpty` but swaps the raw
  Low-IL token for an AppContainer via `CreateAppContainerProfile` + `PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES`,
  0 capabilities). Compiles clean (release).
- 2026-05-28: Operator field run on Win11 build-26200. AppContainer profile created OK; `cmd.exe` spawned in
  the AppContainer on the ConPTY (PID 43608); child exited **`0xc0000142`** → `[SPIKE-001] FAIL`. Same
  STATUS_DLL_INIT_FAILED as B′/B′′. INVALIDATED.

## Results

**INVALIDATED.** An AppContainer-confined `cmd.exe` attached to a ConPTY dies in DllMain with `0xC0000142`,
identical to the raw Low-IL approaches (B′/B′′) and the original Phase-30 crash. **Root reason:** an
AppContainer token is itself **Low integrity** (Low IL + AppContainer SID + capability SIDs), so its
console-subsystem client registration (`ConClntInitialize` → ALPC connect to the conhost) is denied exactly
as for a mandatory-label Low-IL child. AppContainer changes the *capability/ACL* model but NOT the integrity
level, so it does not escape the console wall.

**Conclusion for the idea:** the interactive TUI is **OS-blocked across every confinement primitive tested** —
raw Low-IL (hang / 0xC0000142) AND AppContainer (0xC0000142). There is no console-subsystem path for a
Low-integrity client. A real TUI requires the agent process to run at Medium+ integrity, which abandons the
structural isolation. Spikes 002–004 (AppContainer fs/net/grant isolation parity) are moot — cancelled.

**Signal for the build (architecture pivot, not a console fix):** the achievable model on Windows is
**sandbox-the-tools, not sandbox-the-TUI** — run `claude` itself at Medium IL in a real terminal (full TUI),
and confine the *operations it spawns* (Bash/file/network tool calls) by wrapping them with `nono run`
(Claude Code hooks already support this). That preserves the TUI while keeping the dangerous operations
kernel-isolated. This sidesteps the console wall entirely and is the recommended direction over any further
console/ConPTY work.

