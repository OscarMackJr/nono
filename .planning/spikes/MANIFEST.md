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

---

# Series 2 — SEED-004: Engine-Agnostic Confinement (daemon + token-label)

> Spike numbering continues from the shared directory sequence (next dir = `002`). Series 1's
> cancelled `002–004` rows above were never built (no directories); this series owns the `002+` dirs.

## Idea

Generalize the (now-hardened, sandbox-the-tools) confinement model **beyond Claude Code** so nono can
mediate *any* agent engine — Aider, GitHub Copilot CLI, Cursor, a custom Python/LangChain loop. The seed's
vision: a long-running local **security daemon** that detects/labels arbitrary `AI_AGENT` process tokens and
confines them, replacing the Claude-Code-specific `PreToolUse → nono run` hook. Reference:
`.planning/seeds/SEED-004-multi-engine-agent-pluggability.md`.

The pivotal Windows constraint: confinement (mandatory label / restricting SID) is set at process
**creation**; you generally cannot lower a *running* process's primary-token integrity from outside. So the
killer question is whether the seed's literal "detect-and-confine-after-the-fact" model is feasible, or
whether it must pivot to a **daemon-as-launcher** model.

## Requirements

- Carried forward from prior work: confinement of *spawned tool processes* via Low-IL primary token /
  AppContainer + WFP is proven (broker arm; `windows_appcontainer_wfp_validated`); the TUI itself is OS-blocked
  (spike 001). Do NOT re-test those.
- Spikes are standalone `windows-sys` Rust binaries (mirroring spike 001 / poc-broker), run by the operator on
  real Win11 and reported back. User-mode only — no kernel driver / `PsSetCreateProcessNotifyRoutine`
  (out of scope per the WFP-driver placeholder pattern).
- Any proposed engine-agnostic model MUST preserve isolation ≥ the current per-invocation `nono run` model
  (NO_WRITE_UP for unauthorized FS, deny outbound network unless granted).

## Spikes

| # | Name | Type | Validates | Verdict | Tags |
|---|------|------|-----------|---------|------|
| 002 | post-hoc-token-confine | standard | Given an arbitrary process the daemon did NOT spawn, when it tries to apply nono confinement from outside (lower IL on the running primary token), then NEW unauthorized writes are denied — **KILLER**: settles whether "confine any AI_AGENT token" is feasible or must pivot to daemon-as-launcher | IN PROGRESS | windows, daemon, token, integrity, security |
| 003 | daemon-as-launcher | standard | Given a persistent daemon, when an arbitrary engine is launched *through* it (engine-neutral generalization of the broker), then it runs confined regardless of engine (cmd.exe AND python.exe) | DEFINED (pending 002) | windows, daemon, broker, launcher |
| 004 | agent-marker-multitenant | standard | Given multiple agents launched via the daemon, when it marks each (AI_AGENT job/SID/PID registry) and serves per-agent capability requests over one persistent multi-client pipe, then policies resolve independently and the marker is tamper-evident | DEFINED (pending 002) | windows, daemon, ipc, multitenant |
| 005 | engine-agnostic-abstraction | standard | Given the nono-py/C binding, when a raw Python/LangChain agent invokes the nono primitive directly (no Claude hook), then it is confined equivalently — proving ≥2 engines through one abstraction boundary | DEFINED (pending 002) | bindings, python, abstraction, engine-agnostic |

**Run order = risk order:** 002 first (killer). If 002 invalidates post-hoc confinement (expected — Windows
won't relabel a running token soundly), 003–005 build the daemon-as-launcher model instead.
