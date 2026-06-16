---
phase: 72-nono-py-binding-in-process-exec-proof
plan: 01
subsystem: infra
tags: [windows, appcontainer, low-il, sandbox, spike, confinement, nono-py]

# Dependency graph
requires:
  - phase: 71-windows-engine-abstraction
    provides: langchain-python profile + windows_interpreters engine-coverage gate (prerequisite for spike)
provides:
  - "Born-confined soundness PASS verdict on real Win11 Build 26200"
  - "72-01-SPIKE-REPORT.md with full instrumentation output and 3 harness-bug fix documentation"
  - "ROADMAP.md Phase 72 SC2/SC3 reworded to Windows-equivalent born-confined broker re-exec language"
  - "Proven nono.exe invocation pattern for Wave 2 (cwd=workspace + --allow python-dir + python.exe engine)"
affects:
  - 72-02-nono-py-binding
  - 72-03-contract-doc
  - 72-04-langchain-proof

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Born-confined self-re-exec: Shape B soundness requires nono.exe as the FIRST call in main() before any privileged handle is opened"
    - "Interpreter coverage gate: confined command must be the profile-covered interpreter (python.exe), not a utility (whoami.exe)"
    - "D-52-01 cwd-coverage: child cwd must be within an --allow grant to prevent implicit relative-path escapes"

key-files:
  created:
    - .planning/phases/72-nono-py-binding-in-process-exec-proof/72-01-SPIKE-REPORT.md
    - .planning/phases/72-nono-py-binding-in-process-exec-proof/72-01-SUMMARY.md
  modified:
    - .planning/ROADMAP.md (SC2/SC3 reword applied during planning phase; idempotent verification in this plan)

key-decisions:
  - "Shape B born-confined self-re-exec is sound on Win11: child runs Low-IL + AppContainer, deny/allow invariants confirmed"
  - "ROADMAP.md SC2/SC3 reword was pre-applied during planning (2026-06-14 D-05); Task 2 confirmed idempotent — no edit needed"
  - "Wave 2 plans (72-02, 72-03) are unblocked by this PASS verdict"
  - "Proven invocation: nono run --profile langchain-python --allow <ws> --allow <python-dir> -- python.exe; cwd=workspace (D-52-01)"

patterns-established:
  - "Spike driver harness must use the profile-covered interpreter as the directly confined command, not a utility binary"
  - "Always pass --allow <interpreter-dir> in addition to --allow <workspace> to cover interpreter path resolution"
  - "Child cwd must be set to the allowed workspace (D-52-01 cwd-coverage rule)"

requirements-completed: [ABI-01]

# Metrics
duration: checkpoint-gated (human execution on Win11 host)
completed: 2026-06-14
---

# Phase 72 Plan 01: Born-Confined Soundness Spike Summary

**Shape B born-confined self-re-exec proven sound on Win11 Build 26200: child token shows Low-IL (S-1-16-4096) + AppContainer, deny outside workspace confirmed (PermissionError EACCES), allow inside workspace confirmed (ok.txt written), ordering invariant satisfied by code review.**

## Performance

- **Duration:** Checkpoint-gated (human operator ran spike on real Win11 host)
- **Started:** 2026-06-14
- **Completed:** 2026-06-14
- **Tasks:** 2 (Task 1: write SPIKE-REPORT; Task 2: verify ROADMAP idempotent)
- **Files modified:** 2 (72-01-SPIKE-REPORT.md created; ROADMAP.md — no edit needed)

## Accomplishments

- Spike PASS: all 4 soundness invariants green on real Win11 Build 26200 with nono v0.62.2
- Documented 3 harness-bug fixes (driver evolution 5ab36ada → 12e6c9f6) so verdict is reproducible
- Task 2 idempotent: ROADMAP.md SC2/SC3 were already reworded to born-confined language during planning; `grep -c "born.confined"` returns 3 (SC2, SC3, Pitfall P3 row); no edit required
- Wave 2 plans (72-02 parallel with 72-03) are now unblocked

## Task Commits

1. **Task 1: Execute born-confined spike** — spike run by human operator; SPIKE-REPORT written by Claude (this commit)
2. **Task 2: Verify ROADMAP.md** — idempotent; no ROADMAP edit needed; confirmed in this commit

**Plan metadata:** (this docs commit)

## Files Created/Modified

- `.planning/phases/72-nono-py-binding-in-process-exec-proof/72-01-SPIKE-REPORT.md` — Full spike verdict, child token output, 4-invariant proof, 3 harness-bug fixes
- `.planning/phases/72-nono-py-binding-in-process-exec-proof/72-01-SUMMARY.md` — This file
- `.planning/ROADMAP.md` — Verified (no change needed; SC2/SC3 already use born-confined language)

## Decisions Made

**Task 2 idempotent:** ROADMAP.md Phase 72 SC2 and SC3 were reworded to Windows-equivalent
born-confined language during the planning phase (2026-06-14, D-05). `grep -c "born.confined"`
returns 3 — once in SC2, once in SC3, once in the Pitfall P3 table row. "Sandbox::apply" appears
only inside a historical bracketed note `[Reworded ... per D-05: Windows Sandbox::apply is
preview-only; ...]` — acceptable context explaining the reword, not functional framing. No edit
was needed.

**Proven invocation pattern (for 72-04):** `nono run --profile langchain-python --allow <ws> --allow <python-dir> -- python.exe -c "..."` with child `cwd=<ws>` (D-52-01). This is the reusable template for the Wave 3 LangChain proof.

## Deviations from Plan

### Auto-fixed Issues

None — plan executed as specified. Task 1 (SPIKE-REPORT) followed the orchestrator-supplied
instrumentation verbatim. Task 2 confirmed idempotent — ROADMAP.md already satisfied all
acceptance criteria.

## Issues Encountered

The original spike driver (5ab36ada) had 3 harness bugs that caused the first operator run to
crash/timeout. These were NOT soundness failures — each gate fired fail-secure exactly as
designed:

1. **Wrong confined command (`whoami.exe`):** The `langchain-python` `windows_interpreters`
   engine-coverage gate refused `whoami.exe`. Fix: use `python.exe` as the directly-confined
   command; run `whoami /groups` as a subprocess inside the confined python child.

2. **Missing `--allow <python-dir>`:** nono's interpreter-path coverage check denied the spawn
   because `python.exe` resolved to an uncovered path. Fix: add `--allow <python-dir>` to
   the nono invocation.

3. **Missing cwd coverage (D-52-01):** Child cwd was uncovered. Fix: set `cwd=workspace` in
   the subprocess call.

Additionally, the 60s timeout was too short for cold-start Windows Defender scanning of the
unsigned binary; raised to 180s.

All three fix-triggering gates are corroborating evidence that nono enforces fail-secure
confinement policy at the interpreter-coverage, path-coverage, and cwd-coverage boundaries.

## User Setup Required

None — spike was human-executed on the operator's own Win11 host using the existing
dev-layout `target\release\nono.exe` and the `langchain-python` profile built in Phase 71.

## Next Phase Readiness

Wave 2 is fully unblocked:
- **72-02** (Rust `nono-py` binding crate: `confined_run` / `confine` wrapping nono.exe) — ready
- **72-03** (E1–E5 engine-agnostic Python API contract docs) — ready (parallel with 72-02)
- **72-04** (Wave 3: LangChain proof + UAT) — depends on 72-02 and 72-03; reusable invocation pattern documented in SPIKE-REPORT

No blockers. The born-confined soundness invariant is proven.

---
*Phase: 72-nono-py-binding-in-process-exec-proof*
*Completed: 2026-06-14*
