---
quick: 260528-sch
status: complete
---

# Quick Task 260528-sch — Summary

Authored `260528-sch-SPEC.md`: a reviewer-ready spec for the **sandbox-the-tools** Windows pivot
(Medium-IL claude + full TUI; per-tool-call confinement via `nono run` through Claude Code hooks).

The spec is self-contained for external hand-off: it bundles the **testing & failures** matrix (D′
pipe-stdio shipped/no-TTY; B′ and B′′ ConPTY both `0xC0000142`; AppContainer spike INVALIDATED) with
commits + reproducer pointers, the root cause (Low-IL can't register with the Windows console
subsystem), the proposed architecture + diagram, the existing hook foundation (`hooks.rs` /
`nono-hook.sh`) and the gap (per-tool wrapping, Windows hook script, capability mapping), and a
**threat model** flagging the crux: whether Claude Code hooks are an *enforceable* interception point.

The **§7 Feedback Requested** section poses 6 numbered questions (Q1 enforceability = blocking) with
explicit `> R:`-inline return instructions, so a reviewer's answers come back in a form that can be
consumed directly into a future planned phase (queued for v2.9 per the research seed).

Authored inline by the orchestrator (full investigation context in-session); no executor spawn. No
source code changed — docs only under `.planning/`.
