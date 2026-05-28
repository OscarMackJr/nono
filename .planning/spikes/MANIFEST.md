# Spike Manifest

## Idea

Explore whether a Windows **AppContainer** can host an interactive TUI (e.g. claude Code) while preserving
structural filesystem/network isolation **at least equivalent to** nono's current Low-IL mandatory-label
sandbox. Motivation: the Low-IL `nono shell` cannot host an interactive TUI — a Low-IL child cannot register
with the Windows console subsystem (ConPTY approaches B′/B′′ both die `0xC0000142`; pipe-stdio D′ has no TTY,
shipped). AppContainer is a different confinement primitive; this spike series tests whether it escapes the
console wall AND still denies unauthorized filesystem writes + network.

See `.planning/research/windows-low-il-tui-blocked.md` (research seed) and
`.planning/debug/resolved/nono-shell-claude-hang.md` (resolved debug session) for the full background and the
proven-dead approaches.

## Requirements

- Any AppContainer-based shell MUST provide isolation **≥** the Low-IL model: structurally deny unauthorized
  filesystem writes (the NO_WRITE_UP guarantee) and deny outbound network unless granted.
- Do NOT re-test B′/B′′ (raw Low-IL ConPTY) — proven dead (`0xC0000142`).
- Spikes are standalone `windows-sys` Rust binaries (mirroring `poc-broker`), run by the operator on Win11
  build-26200 and reported back (OS-behavior facts → CLI/observable verdicts).

## Spikes

| # | Name | Type | Validates | Verdict | Tags |
|---|------|------|-----------|---------|------|
| 001 | appcontainer-conpty-tui | standard | AppContainer process attached to a ConPTY renders console output without 0xC0000142 (KILLER) | ✗ INVALIDATED | windows, appcontainer, conpty, console, tui |
| 002 | appcontainer-fs-deny | standard | AppContainer (no granted write dirs) structurally denies writes outside granted paths (≥ NO_WRITE_UP) | CANCELLED (001 dead) | windows, appcontainer, filesystem, security |
| 003 | appcontainer-net-deny | standard | AppContainer without `internetClient` capability blocks outbound network | CANCELLED (001 dead) | windows, appcontainer, network, security |
| 004 | appcontainer-selective-grants | standard | Per-path r+w grant via AppContainer SID ACL: granted=allowed, ungranted=denied | CANCELLED (001 dead) | windows, appcontainer, capabilities, grants |

**Run order:** 001 was a hard killer — it INVALIDATED (AppContainer `cmd.exe` on a ConPTY → `0xc0000142`,
same as raw Low-IL). AppContainer is Low integrity, so it does not escape the console-subsystem wall. 002–004
cancelled.

## Outcome

The interactive TUI is **OS-blocked across every confinement primitive tested**: raw Low-IL (hang /
0xC0000142, B′/B′′) and AppContainer (0xC0000142, spike 001). No console-subsystem path exists for a
Low-integrity client. **Recommended direction (architecture pivot, no further console work):**
sandbox-the-tools, not sandbox-the-TUI — run `claude` at Medium IL with its real TUI, and wrap the operations
it spawns (Bash/file/network) with `nono run` via Claude Code hooks. Updated the research seed
(`.planning/research/windows-low-il-tui-blocked.md`) accordingly.
