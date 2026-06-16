# Spike Wrap-Up Summary

**Date:** 2026-06-13
**Spikes processed:** 3
**Feature areas:** Windows confinement model; Engine-agnostic confinement (SEED-004)
**Skill output:** `./.claude/skills/spike-findings-nono/`

## Processed Spikes

| # | Name | Type | Verdict | Feature Area |
|---|------|------|---------|--------------|
| 001 | appcontainer-conpty-tui | standard | ✗ INVALIDATED | Windows confinement model |
| 002 | post-hoc-token-confine | standard | ⚠ PARTIAL | Windows confinement model |
| 003 | daemon-as-launcher | standard | ✓ VALIDATED | Engine-agnostic confinement |

## Key Findings

- **Engine-agnostic confinement is feasible (003, VALIDATED).** One persistent launcher confined cmd.exe,
  powershell.exe, and python.exe identically via `nono run` — granted write lands, outside write denied. The
  Claude-specificity was only ever in the PreToolUse hook, not the confinement primitive. **Daemon-as-launcher
  (confine at spawn) is the sound primary model.**
- **Post-hoc IL-drop works but is unsound (002, PARTIAL).** A daemon CAN lower a running same-user process's
  primary-token IL from outside (High/Med→Low confirmed) and block its new writes — but leaked pre-existing
  handles, no restricting-SID retrofit, blunt vs authorized writes, and no network make it a *supplementary
  "demote a misbehaving agent" control*, not a standalone boundary.
- **TUI is OS-blocked (001, INVALIDATED).** A Low-IL/AppContainer console client dies `0xC0000142`; hence
  sandbox-the-tools, not sandbox-the-TUI. (Foundation that motivated SEED-004.)
- **Two build contracts emerged:** (1) the launch policy must COVER each engine's executable/interpreter path
  (python under `%LOCALAPPDATA%` was fail-secure refused until `--allow`'d); (2) grants must be ABSOLUTE
  (engines don't uniformly inherit the launcher CWD). Plus the operational R-B3 gotcha: the agent workspace
  must be user-owned.

## Not Yet Spiked (future)

- **Persistent multi-tenant daemon** (was 004): token/job *reuse* across many agents + an `AI_AGENT` marker +
  one persistent multi-client capability pipe (generalize `crates/nono/src/supervisor/socket_windows.rs`).
- **Formal abstraction via `nono-py`** (was 005): confine a real Python/LangChain agent through the binding
  (no Claude hook), proving ≥2 engines through one boundary + the exe-coverage contract.

These shade into implementation; take SEED-004 to `/gsd:new-milestone` or `/gsd:plan-phase` citing this skill.
