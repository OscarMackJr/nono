---
id: SEED-004
status: dormant
planted: 2026-06-08
planted_during: v2.10 (Phase 63)
trigger_when: milestone scope includes multi-agent/multi-engine support, agent-engine abstraction, a security daemon, or enforcing rules on non-Claude agents
scope: x-large
priority: P3
---

# SEED-004: Multi-Agent & Multi-Engine Pluggability (Beyond Claude Code)

## Why This Matters

A modern enterprise doesn't standardize on one model or agent framework. Developers are split across Microsoft Copilot, GitHub Copilot CLI, Cursor, Aider, and custom in-house Python/LangChain agents. nono today is coupled to Claude Code via hook scripts.

**The CISO/CTO pitch:** treat the *agent engine as a variable*, not a hardcoded hook. A single local **security daemon** applies the same mandatory rules to **any process token labeled `AI_AGENT`** — whether it's Claude Code, an Aider loop, GitHub Copilot CLI, or a custom Python script.

## When to Surface

**Trigger:** when a milestone targets multi-engine/multi-agent support, an agent-engine abstraction layer, a persistent local security daemon, or applying nono policy to non-Claude agents.

This seed will surface during `/gsd:new-milestone` when the milestone scope matches.

## Scope Estimate

**X-Large / architectural — likely warrants a `/gsd:spike` first.** This is the biggest of the five and reshapes the integration model:
- Decouple policy enforcement from the Claude-Code-specific PreToolUse hook (`sandbox-the-tools` model) into an engine-agnostic mechanism.
- A long-running local daemon that detects/labels `AI_AGENT` process tokens and confines them (vs. the current per-invocation `nono run` wrapping).
- Token-labeling strategy on Windows (mandatory labels / job objects) for arbitrary child engines — builds on the existing IL/AppContainer work but generalizes the trigger.
- Define the abstraction boundary: what every engine must expose for nono to mediate it.

## Breadcrumbs

- `crates/nono-cli/src/hooks.rs` + `project_sandbox_the_tools` — current Claude-Code-specific PreToolUse → `nono run` model (the thing being generalized).
- `crates/nono-cli/src/exec_strategy*` — Direct/Monitor/Supervised execution strategies (the per-engine confinement primitive).
- Windows IL/AppContainer/broker work: `windows_appcontainer_wfp_validated`, `project_win_lowil_tui_blocked` — token-labeling building blocks.
- `bindings/c/`, `../nono-py/`, `../nono-ts/` — existing language bindings (an entry point for custom Python/LangChain agents to call the primitive directly).

## Notes

Captured 2026-06-08 (CISO/CTO horizon). Consider spiking the daemon + token-label trigger before committing a milestone. Sibling seeds: [[SEED-001]], [[SEED-002]], [[SEED-003]], [[SEED-005]].
