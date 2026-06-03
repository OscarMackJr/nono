---
quick_id: 260603-e0x
slug: windows-native-development-status-summar
status: complete
created: 2026-06-03
completed: 2026-06-03
---

# Quick Task 260603-e0x — SUMMARY

## What was produced

`260603-e0x-WINDOWS-STATUS.md` — a self-contained, copy-paste-shareable status summary of
the fork's Windows-native development, written for upstream (macOS/Linux) nono maintainers.
Answers the three asks directly:

1. **Available on Windows** — §2: core sandbox + CLI, supervisor lifecycle (attach/detach/ps/stop,
   ATCH/SESS/CLEAN), `nono shell` broker, no-PTY Low-IL broker (v2.7), WFP network + ports +
   proxy creds + ETW learn + AIPC handle brokering, Job-Object resource limits, signed-MSI
   release automation, and the just-completed out-of-box WFP kernel enforcement (Phase 62, 5/5 UAT).
2. **In POC / under test** — §3: the "sandbox-the-tools" confined coding loop (v2.9, PR #4) —
   Medium-IL agent + per-tool-call Low-IL `nono` jail; confined file edits + Bash; honest
   defense-in-depth (not full isolation) framing; TUI-at-Low-IL is OS-blocked; HUMAN-UAT on
   live Win11 26200.
3. **Next milestones** — §5: Phase 61 ship v2.9 → v2.8 UPST7 upstream sync (v0.58/v0.59) →
   v3.0 deferrals (kernel minifilter, EDR UAT).

Plus a parity-mapping table (§1, Windows primitives ↔ Landlock/Seatbelt) and an explicit
limitations/deferrals section (§4) so nothing is overclaimed.

## Approach

Synthesis-only, executed inline (no code, no subagent) — the orchestrator already held the
full STATE.md / PROJECT.md / ROADMAP.md context, so an executor subagent would have re-derived
it with less fidelity. All claims trace to PROJECT.md (Validated requirements), ROADMAP.md
(phases/milestones), STATE.md (current position), and the Phase 62 UAT/SECURITY artifacts.

## Notes / accuracy guards

- Framed as fork→upstream communication (the "macOS maintainers" = upstream).
- POC limits stated honestly: defense-in-depth not isolation; network/WebFetch/MCP/Task denied;
  Low-IL TUI OS-blocked; claude.exe AppContainer read-grant model deferred.
- v3.0 deferrals (kernel minifilter placeholder, EDR UAT) and the LOW Phase-62 accepted risk
  (AR-62-10, ServiceConfig crash-loop bound) called out so the picture isn't rosier than reality.
- The internal v2.8/v2.9 milestone-bookkeeping tangle (v2.9 phases 60/61/62 leapfrogged the
  v2.8 UPST7 phases 53–59) was intentionally NOT exposed to maintainers — §5 presents a clean
  forward ordering instead.

## Where it lives

`.planning/quick/260603-e0x-windows-native-development-status-summar/260603-e0x-WINDOWS-STATUS.md`
— ready to share as-is (PR description, issue comment, or relocate into `docs/` if a tracked
maintainer-facing doc is wanted).
