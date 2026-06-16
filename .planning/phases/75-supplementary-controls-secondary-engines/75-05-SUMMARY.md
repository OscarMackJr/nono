---
phase: 75-supplementary-controls-secondary-engines
plan: "05"
subsystem: uat
tags: [windows, uat, live-host, daemon, copilot, nono-ts, supp-01, supp-02, supp-03]

# Dependency graph
requires:
  - phase: 75-supplementary-controls-secondary-engines
    plan: "01"
    provides: SUPP-02 per-agent WFP helpers
  - phase: 75-supplementary-controls-secondary-engines
    plan: "02"
    provides: SUPP-01 demote verb
  - phase: 75-supplementary-controls-secondary-engines
    plan: "03"
    provides: copilot-cli engine profile
  - phase: 75-supplementary-controls-secondary-engines
    plan: "04"
    provides: nono-ts confinedRun/confine binding
  - phase: 75-supplementary-controls-secondary-engines
    plan: "06"
    provides: GAP-75-A daemon-start type-50 fix (unblocked SC1/SC3/SC5)
  - phase: 75-supplementary-controls-secondary-engines
    plan: "07"
    provides: GAP-75-B capability-less launch fix (unblocked SC1/SC3/SC5)

provides:
  - 75-HUMAN-UAT.md FINAL verdicts (SC1-SC5 + A1/A2/A4) from live Win11 re-run 2026-06-16
  - nono-ts Windows binding-load fix (commit 2bac4e2 on 44-broker-ffi-lockstep)
  - GAP-75-C identified + gap-closure plan 75-08 authored (SC3 Node-ESM/AppContainer)

affects: [75-08, Phase 75 close]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Live UAT is the load-bearing gate (D-08): build-green-only would have shipped a copilot engine that confines but never completes, and a nono-ts binding that doesn't load on Windows"
    - "Confined long-running probe: AppContainer denies network (ping) and breaks waitfor init; a pure-compute `cmd /c \"for /l %i in () do @rem\"` is the reliable keep-alive for demote tests"

key-files:
  created:
    - .planning/phases/75-supplementary-controls-secondary-engines/75-08-PLAN.md
  modified:
    - .planning/phases/75-supplementary-controls-secondary-engines/75-HUMAN-UAT.md
    - ../nono-ts/index.js (commit 2bac4e2)
    - ../nono-ts/package.json (commit 2bac4e2)

requirements: [SUPP-01, SUPP-02, SUPP-03]
---

# Phase 75 Plan 05: Live Win11 UAT (SC1–SC5) Summary

## What was verified

Live UAT on Win11 Enterprise build 26200, dev-layout `target\release\nono.exe` + `nono-agentd.exe`
(v0.62.2). The 2026-06-15 run was blocked on two daemon defects; both were fixed + live-verified
this session (75-06 GAP-75-A daemon-start; 75-07 GAP-75-B capability-less launch), then SC1/SC3/SC5
were re-run on 2026-06-16.

| SC | Verdict | Summary |
|----|---------|---------|
| SC4 | ✅ PASS | daemon runs as USER (`TYPE 50 USER_OWN_PROCESS TEMPLATE`, empty start-name); `daemon start` → RUNNING via the 75-06 type-50 guard |
| SC1 | ✅ PASS | `agent demote` drops IL to Low + removes WFP filter, does NOT reap; agent still listed (long-running busy-loop agent) |
| SC2 | 🟡 PASS (D-05) / A1 DEFERRED | non-scoped launch OK with WFP service running; A1 two-agent isolation deferred (no network-scoped profile) |
| SC3 | 🟡 PARTIAL → 75-08 | copilot confinement enforced (write-outside denied; fail-secure gate); A4 = YES (Node-backed); engine doesn't complete due to Node ESM `lstat('C:\')` EPERM under AppContainer |
| SC5 | ✅ PASS | nono-ts `confinedRun` confines Node on Win11 (outside denied, inside allowed) after the binding-load fix + Low-IL broker profile |

## Findings & fixes (driven out by the live UAT)

1. **nono-ts Windows binding wouldn't load (FIXED this session, commit `2bac4e2`).** `index.js` had no
   `win32` loader branch (napi target omitted) and didn't re-export the `#[cfg(windows)]`
   `confinedRun`/`confine` functions — so it threw `Unsupported OS: win32` / `confinedRun is not a
   function` despite a valid `nono.win32-x64-msvc.node`. Added `x86_64-pc-windows-msvc` to
   `package.json` `napi.targets` + the `win32-x64` loader branch + the two exports. SC5 then passed.
2. **GAP-75-C (SC3) — Node-ESM engines fail under AppContainer (→ plan 75-08).** Copilot is a Node ESM
   app; Node's `realpathSync` does `lstat('C:\')` which AppContainer denies. Confinement is proven;
   completion is not. Authored 75-08 (spike-first: narrowest drive-root attribute grant or Node
   resolver mitigation + copilot-cli node-interpreter coverage + symlinked-exe coverage).
3. **nono-ts ergonomics (non-blocking follow-up, noted in 75-08 + UAT doc):** `confinedRun(profile=undefined)`
   uses the WriteRestricted token arm, under which Node/CLR die `0xC0000142`; it should default to the
   Low-IL broker arm for real engines and auto-cover the target exe's dir. SC5 passed with an explicit
   `windows_low_il_broker` profile + node-dir allow + `%USERPROFILE%` workspace (drive-root `C:\poc\*`
   workspaces fail the R-B3 ownership gate — re-confirming the Phase-60/72 mandatory-label lesson).

## Verification

- Pre-flight CI sweep (recorded in 75-HUMAN-UAT.md): workspace build + clippy + fmt + tests green
  modulo the documented pre-existing baseline failures (not Phase-75 regressions).
- Gap-closure 75-06/75-07 automated tests all green (`--bin nono` for agent_cli; `--bin nono-agentd`
  for agent_daemon).
- SC1/SC4/SC5 PASS, SC2 PASS (D-05), SC3 PARTIAL — all evidenced inline in 75-HUMAN-UAT.md.

## Disposition

Phase 75 is **functionally complete except SC3 engine-completion**, which is deferred to gap-closure
**75-08** (operator-approved disposition: "fix SC5 now, gap-plan SC3"). SUPP-01 and SUPP-02 fully
satisfied; SUPP-03 satisfied for nono-ts (Binding-2) and for Copilot confinement (Engine-2 confines;
end-to-end completion in 75-08). claude-code (native PE) confined cleanly in the 75-07 UAT as the
strong Engine-2 datapoint.

## Carry-forwards

- **75-08** (PRIORITY): Node-ESM/AppContainer drive-root fix + copilot-cli node coverage → SC3 end-to-end.
- nono-ts: default to Low-IL broker arm + auto-cover target exe dir in `confinedRun` (ergonomics).
- A1 empirical per-agent WFP isolation still deferred (needs a network-scoped test profile).
