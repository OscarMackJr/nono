---
phase: 30-windows-nono-shell-architecture
plan: 05
subsystem: windows-shell-architecture
tags: [windows, procmon, wave-2-investigation, kernel-driver-deferral, csrss-alpc-integrity-mismatch, failure-mode-finding, v3.0-deferral]

requires:
  - phase: 30-windows-nono-shell-architecture
    provides: Wave 1 cascade arm (30-02), field-smoke evidence (30-04 wave2-trigger-launch)
provides:
  - "Wave 2 ProcMon investigation localized failure to CSRSS console-subsystem ALPC denial during KernelBase.dll DllMain"
  - "Six sixth-option candidates analyzed; option 6e (defer to v3.0) selected per D-04 timebox"
  - "Phase 30 terminal state shipped: SHELL-01 ✘ deferred; cookbook reverted per Option Rev-B; debug session resolved with failure-mode finding"
  - "Wave 1 cascade arm code preserved in tree as guard for future v3.0 / Phase 31 broker-process work"
affects: [Phase 31, SHELL-01, v2.3 milestone, nono shell on Windows]

tech-stack:
  added: []
  patterns:
    - "Wave-2 ProcMon trace methodology — PID-scoped filtering for child-process startup analysis"
    - "Manual IL/PID diagnostic pattern (carried forward from Plan 30-04) for verifying sandbox actually applied"

key-files:
  created:
    - .planning/phases/30-windows-nono-shell-architecture/30-WAVE-2-PROCMON.md (Tasks 2+3 ProcMon analysis + sixth-option synthesis + Final outcome section)
    - .planning/debug/resolved/nono-shell-status-dll-init-failed.md (moved from .planning/debug/ via git mv; appended ## Resolution section)
  modified:
    - .planning/PROJECT.md (SHELL-01 row → ✘ deferred to v3.0)
    - .planning/STATE.md (Key Decisions v2.3 Phase 30 entry → final-state failure-mode entry; stopped_at flipped to "Phase 30 deferred to v3.0")
    - docs/cli/development/windows-poc-handoff.mdx (cookbook reverted per RESEARCH Option Rev-B; new "deferred to v3.0" section)
    - .gitignore (*.pml rule added for ProcMon trace binaries)

key-decisions:
  - "Task 4 outcome: exhaust-without-fix (option 6e). Rationale: D-04 timebox is 3-5 working days; all viable user-mode options (6a AppContainer 1-2 weeks, 6b broker-process 1+ week) exceed the budget. 6c-d-f all break phase contracts (D-05 TUI / D-06 mandatory-label)."
  - "Failure surface localized: STATUS_DLL_INIT_FAILED (0xC0000142) at CSRSS console-subsystem ALPC handshake during KernelBase!ConClntInitialize at Low-IL. RESEARCH Pitfall 2 (Microsoft-documented integrity-mismatch) confirmed in field."
  - "PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE inherited handles do NOT bypass CSRSS attach — they're a separate communication path. This is the structural Win32 user/kernel boundary that defeats user-mode workarounds."
  - "Wave 1 cascade arm code (WindowsTokenArm::LowIlPrimary + helpers + tests) preserved in tree, NOT reverted — serves as guard for whenever Phase 31 broker-process pattern or v3.0 kernel-driver work activates this path."
  - "Cookbook revert per Option Rev-B (text replacement, NOT git revert): preserves Phase 30 institutional knowledge while honestly resetting POC user expectations to nono run for non-TUI."

patterns-established:
  - "ProcMon trace methodology for child-process startup investigation: PID-scoped filter (Process Name + specific child PID) + Process Tree (CTRL+T) for parent/child lifecycle + 'Result is REPARSE → Exclude' to drop registry-symlink noise. The early broad recipe (Path-contains rules, OR'd Process Name list) over-captured WMI service activity; PID-scoping was decisive."
  - "Failure-mode-finding ship pattern: when a phase exhausts its timebox without surfacing a workable option, ship the institutional knowledge (✘ in PROJECT.md + Resolution section in resolved debug session + WAVE-2-PROCMON.md technical evidence) rather than churning into the next phase. Future plans inherit the analysis."

requirements-completed: [D-04, D-07, D-10]  # All three requirements from plan frontmatter satisfied (D-04 timebox honored; D-07 cookbook revert per Option Rev-B; D-10 SHELL-01 → ✘ failure path; PROJECT.md/STATE.md/debug session reach terminal state)

duration: ~90 min interactive (Tasks 1-2 ProcMon trace + analysis ~30 min; Tasks 3+4 doc write + sixth-option synthesis ~30 min; Tasks 5+6 cookbook revert + bookkeeping ~30 min)
completed: 2026-05-08
---

# Phase 30 Plan 30-05: Wave 2 exhaust-without-fix → Phase 30 deferred to v3.0

**Wave 2 ProcMon investigation localized the silent-launch failure to CSRSS console-subsystem ALPC denial during KernelBase.dll DllMain at Low-IL — RESEARCH Pitfall 2 confirmed in field. All viable user-mode options exceed D-04 timebox; Phase 30 ships failure-mode finding with cookbook reverted per Option Rev-B and Phase 31 broker-process pattern queued as the strongest follow-up candidate.**

## Performance

- **Duration:** ~90 min interactive (3 task groups: trace capture+analysis ~30 min, doc+synthesis ~30 min, terminal bookkeeping ~30 min)
- **Started:** 2026-05-08 ~09:25 EDT (after `/gsd-execute-phase 30 --wave 4` invocation)
- **Completed:** 2026-05-08 ~14:15 UTC
- **Tasks:** 6 total (Task 5 SKIPPED per `exhaust-without-fix`; remaining 5 tasks all completed)
- **Files committed:** 5 modified + 1 renamed (debug session move) + 1 new (.gitignore *.pml rule)

## Accomplishments

- Captured a ProcMon trace localizing the silent-launch failure within ~30 minutes (faster than D-04's 3-5 working day timebox anticipated)
- Surfaced and refined the trace methodology — initial broad-recipe filter caught WMI service noise; iterative PID-scoping made the diagnostic decisive
- Localized failure to specific Win32 mechanism: CSRSS console-subsystem ALPC handshake at KernelBase!ConClntInitialize (RESEARCH Pitfall 2 realized)
- Synthesized six sixth-option candidates with quantified effort estimates and contract-compliance assessments
- Shipped Phase 30 terminal state (PROJECT/STATE/cookbook/debug session/30-WAVE-2-PROCMON) atomically per plan single-commit discipline
- Preserved Wave 1 cascade arm code as institutional guard — Phase 31 inherits a ready foundation rather than starting from scratch

## Task Commits

Plan 30-05 work landed across 2 atomic commits:

1. **Tasks 2+3 (ProcMon analysis + 30-WAVE-2-PROCMON.md authored)** — `d9030cc5` (docs)
2. **Tasks 5+6 (failure-path terminal close — cookbook revert + bookkeeping flip + debug session resolution)** — `5a79969a` (docs)

Task 1 was a `checkpoint:human-action` (manual ProcMon GUI setup) — no commit. Task 4 was a `checkpoint:decision` (option 6e selection) — no commit.

## Files Created/Modified

### Created
- `.planning/phases/30-windows-nono-shell-architecture/30-WAVE-2-PROCMON.md` (213 lines) — full investigation document including failure surface analysis, RESEARCH category match with sub-classification, six-option hypothesis space, sixth-option proposal recommendation, trace evidence file references, RESEARCH cross-references, timebox tracking, and Final outcome section
- `.gitignore` rule `*.pml` (ProcMon trace binaries; large + binary, evidence preserved in CSV/MD)

### Modified
- `.planning/PROJECT.md` — SHELL-01 row flipped from ⚠ "Wave 2 ProcMon investigation in flight" to ✘ "structurally incompatible...deferred to v3.0 kernel mini-filter driver work"
- `.planning/STATE.md` — Key Decisions (v2.3) Phase 30 entry replaced with final-state failure-mode narrative; stopped_at: "Phase 30 deferred to v3.0 (Wave 2 exhausted; SHELL-01 → ✘; cookbook reverted per RESEARCH Option Rev-B)"; last_updated bumped
- `docs/cli/development/windows-poc-handoff.mdx` — Option Rev-B text replacement: top-of-doc Note rewritten; Step 4 nono-shell instruction replaced with nono-run guidance; Step 5 "Interactive verification (manual)" block removed; Step 6 user-handoff table rows mentioning nono shell stripped; "Known limitation" section RETAINED; new "`nono shell` on Windows is deferred to v3.0" section added enumerating the four failure modes
- `.planning/phases/30-windows-nono-shell-architecture/30-WAVE-2-PROCMON.md` (after initial creation) — appended Final outcome section

### Renamed (git mv)
- `.planning/debug/nono-shell-status-dll-init-failed.md` → `.planning/debug/resolved/nono-shell-status-dll-init-failed.md` — frontmatter status flipped to `resolved`; resolved_by: phase-30-plan-05; appended `## Resolution` section with four-failure-mode analysis, six-option summary, Phase 31 follow-up scope, and out-of-scope sibling concern pointers

## Investigation evidence

### ProcMon trace methodology (refined during execution)

The plan's RESEARCH § ProcMon Trace Plan (lines 232-243) specified an initial filter recipe with broad Path-contains rules + 4-process Process Name OR list. That recipe over-captured WMI service activity (`svchost.exe` + `WmiPrvSE.exe` background work in `wbemcore.dll`/`repdrvfs.dll`). After two refinement iterations:

| Filter rule | Action | Notes |
|---|---|---|
| Process Name is `nono.exe` | Include | Initial scope |
| PID is 35976 | Include | The powershell.exe child PID, identified via Process Tree (CTRL+T) — decisive scoping |
| Operation is `Process Create` | Include | Catches child spawn |
| Operation is `Load Image` | Include | DLL load chain |
| Operation is `Process Exit` | Include | The fatal event |
| Operation is `Thread Exit` | Include | Pre-Process-Exit |
| Result is `REPARSE` | Exclude | Drops registry-symlink redirect noise |

PID-scoping was decisive — Process Tree showed the parent/child lifecycle and exit codes directly, sidestepping the filter-iteration problem.

### Diagnostic chain

| Step | Evidence | Conclusion |
|---|---|---|
| 1 | Process Tree showed `conhost.exe --headless` (PID 24988, 124 ms lifespan) + `powershell.exe -NoLogo` (PID 35976, **35 ms lifespan**) as children of `nono.exe` (PID 5776) | Hypothesis B: child created, child died fast |
| 2 | Process Exit Status: `-1073741502` = `0xC0000142` = `STATUS_DLL_INIT_FAILED` | Loader killed process during static init |
| 3 | Load Image chain: powershell.exe + ntdll + kernel32 + KernelBase, then 6.8 ms gap with zero further Load Image events, then Thread Exit + Process Exit | Failure in DllMain of one of the four loaded DLLs |
| 4 | Sub-classification: PowerShell is /SUBSYSTEM:CONSOLE; KernelBase!BaseDllInitialize → ConClntInitialize → CSRSS ALPC connect; CSRSS port DACL excludes Low-IL | RESEARCH Pitfall 2 confirmed |
| 5 | PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE inherited handles are a separate communication path (terminal I/O between nono.exe ↔ conhost.exe), do NOT bypass CSRSS attach | No structural workaround at this layer in user mode |

### Sixth-option analysis (full detail in 30-WAVE-2-PROCMON.md)

| Option | Effort | Phase 30 contract | Choice |
|---|---|---|---|
| 6a — AppContainer model | 1-2 weeks | Exceeds D-04 timebox; new D-decisions | Phase 31 candidate |
| 6b — Broker-process pattern | ~1 week | Exceeds D-04 timebox slightly; Microsoft-documented | **Strongest Phase 31 candidate** |
| 6c — Pre-AllocConsole sequencing | 1-2 days experimental | Theoretically fails (PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE replaces inherited console) | Not chosen |
| 6d — Job Object UI restrictions | Re-discuss-phase + new phase | Violates D-06 | Not viable for Phase 30 contract |
| **6e — Defer to v3.0** | **~1 day** | **Honors D-04 timebox** | **CHOSEN** |
| 6f — Pipe-stdio instead of ConPTY | Breaks contract | Violates D-05 (TUI rendering) | Not viable for Phase 30 contract |

## Self-Check: PASSED

All Plan 30-05 failure-path acceptance gates verified pre-commit:

- [x] PROJECT.md ✘ SHELL-01 with deferred-to-v3.0 (1 match)
- [x] PROJECT.md NO ✔ SHELL-01 (0 matches)
- [x] STATE.md mentions deferred to v3.0 (4 matches; ≥1 required)
- [x] STATE.md stopped_at: Phase 30 deferred (1 match)
- [x] debug/resolved/ file exists with status: resolved + ## Resolution
- [x] debug/ non-resolved file does NOT exist (file moved via git mv)
- [x] resolved file resolved_by: phase-30-plan-05 (1 match)
- [x] 30-WAVE-2-PROCMON.md ## Final outcome section present
- [x] cookbook deferred-to-v3.0 section present
- [x] cookbook Known limitation section RETAINED
- [x] cookbook old "Use nono shell, not nono run" line absent
- [x] cookbook Step 5 Interactive verification (manual) heading absent
- [x] *.pml gitignored
- [x] Build clean: cargo build --workspace passes
- [x] Test clean: 831/831 nono-cli unit tests pass
- [x] Single atomic commit `5a79969a` with DCO sign-off (Tasks 5+6 terminal close)

## Phase 31 follow-up scope (preserved as institutional knowledge)

The strongest candidate for `nono shell` on Windows is **option 6b — broker-process pattern**: a small Medium-IL intermediary binary that:

1. Created by `nono.exe` as Medium-IL child
2. Inherits ConPTY handles + opens CSRSS console (succeeds at Medium-IL)
3. Calls `SetTokenInformation(TokenIntegrityLevel, Low)` on its own process token to self-degrade
4. Spawns `powershell.exe -NoLogo` as a Low-IL child via `CreateProcessW`; the child INHERITS the already-attached console (KernelBase's DllMain skips CSRSS attach when a console is inherited)

Microsoft-documented pattern; some browser sandbox implementations use it. Trade-off: the broker becomes a new attack surface boundary between Medium and Low IL — Phase 31 will need rigorous threat-modeling.

Alternative (longer-deferred): v3.0 kernel mini-filter driver — full filesystem-level write enforcement at the OS layer, decoupled from token IL entirely. CONTEXT.md `<deferred>` block enumerates this; would require kernel-mode signing infrastructure we don't currently have.

Both paths inherit the Phase 30 evidence (`30-WAVE-2-PROCMON.md` + the resolved debug session) as the technical foundation; they don't restart investigation.
