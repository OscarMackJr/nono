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
| 001 | appcontainer-conpty-tui | standard | AppContainer process attached to a ConPTY renders console output without 0xC0000142 (KILLER) | PENDING | windows, appcontainer, conpty, console, tui |
| 002 | appcontainer-fs-deny | standard | AppContainer (no granted write dirs) structurally denies writes outside granted paths (≥ NO_WRITE_UP) | NOT STARTED (gated on 001) | windows, appcontainer, filesystem, security |
| 003 | appcontainer-net-deny | standard | AppContainer without `internetClient` capability blocks outbound network | NOT STARTED (gated on 001) | windows, appcontainer, network, security |
| 004 | appcontainer-selective-grants | standard | Per-path r+w grant via AppContainer SID ACL: granted=allowed, ungranted=denied | NOT STARTED (gated on 001) | windows, appcontainer, capabilities, grants |

**Run order:** 001 is a hard killer — 002–004 only proceed if 001 VALIDATES.
