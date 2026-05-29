---
quick: 260528-sch
type: docs
autonomous: true
---

# Quick Task 260528-sch: Spec the sandbox-the-tools (Windows tool-wrapping) pivot for external review

## Objective

Produce a self-contained, reviewer-ready SPEC for the "sandbox-the-tools, not sandbox-the-TUI"
architecture pivot: run `claude` at Medium IL with its full interactive TUI, and confine the operations
it spawns (Bash/file/network tool calls) by wrapping each with `nono run` via Claude Code hooks. The
spec must bundle the current testing + failures (the proven-dead Low-IL/AppContainer TUI attempts) and
be framed so an external reviewer can return structured feedback that is consumable back into a planned
phase.

## Task

- Author `260528-sch-SPEC.md` covering: problem, root cause, the testing/failure matrix (with commits +
  reproducers), the proposed architecture, the existing hook foundation + gap, the threat model (hook
  enforceability is the crux), and a numbered **Feedback Requested** section with explicit return
  instructions.

Authored inline by the orchestrator (the full investigation context lives in this session); no executor
spawn. Sources: the research seed, resolved debug session, AppContainer spike, and `hooks.rs` /
`nono-hook.sh`.
