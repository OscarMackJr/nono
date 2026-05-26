# Plan 52-02 Summary — Profile-conditional doc sweep + VERIFICATION close

**Status:** Complete
**Plan:** 52-02 (wave 2)
**Requirements:** REQ-WSRH-06
**Gate:** Repro B PASS (52-HUMAN-UAT.md) — D-52-03 hard-stop did not fire; plan proceeded.

## What was done

1. **Gate check (Task 1, checkpoint:decision):** Confirmed `52-HUMAN-UAT.md` records
   repro B verdict PASS (all three D-52-04 criteria: no 0xC0000142, version printed,
   exit 0). Resolved to **proceed**.

2. **D-52-05/06 consistency sweep (Task 2)** of `docs/cli/development/windows-poc-handoff.mdx`:
   - **Anchor 1 (L594):** primary `nono run` envelope claim made **profile-conditional** —
     no longer universally `WRITE_RESTRICTED` + per-session WFP. `claude-code` → Low-IL
     broker (`NO_WRITE_UP`, not `WRITE_RESTRICTED`; per-session WFP waived → AppID-based
     filtering); other profiles → `WRITE_RESTRICTED` + per-session SID + full WFP.
   - **Anchor 2 (L573):** shell-path cross-ref made profile-aware — `nono run --profile
     claude-code` now shares the broker per-session-WFP waiver; other profiles retain
     `WRITE_RESTRICTED` + per-session SID.
   - **Anchor 3 (L544):** `claude --version` framing updated to record the Phase 52
     positive result (heavy 234 MB self-contained `claude.exe` works under `nono run
     --profile claude-code`). TUI-limitation header/body untouched (out of scope).
   - **Anchor 4 (L642-643):** smoke-checklist "Non-TUI commands" and "Quick verification"
     rows made profile-aware + heavy-runtime aware.
   - **New section (L609):** `## Windows nono run — heavy-runtime children (Phase 52,
     validated 2026-05-26)` — problem (WRITE_RESTRICTED restricting-SID double-gate vs
     heavy DllMain WRITE activity → 0xC0000142), Phase 51 fix (`windows_low_il_broker:
     true` → `BrokerLaunchNoPty` Low-IL primary token, no restricting SID), security
     tradeoff (NO_WRITE_UP preserved; per-session WFP waived → AppID filtering; other
     profiles unchanged), dated validation line (D-52-08), canonical invocation.

   Factual basis for the WFP-waiver / NO_WRITE_UP claims verified against
   `crates/nono-cli/src/exec_strategy_windows/launch.rs:1100-1113, ~1123, 1204-1206,
   1214-1216` before writing (T-52B-01 mitigation).

3. **52-VERIFICATION.md (Task 3)** created with `status: pass` — all 4 ROADMAP SCs met,
   REQ-WSRH-04 and REQ-WSRH-06 closed, Phase 51 SC-4 deferral closed.

## Verification greps (all pass)

- `WRITE_RESTRICTED`: L594 now explicitly "no longer universally WRITE_RESTRICTED"; all
  remaining occurrences are profile-aware.
- `heavy-runtime` / `Low-IL broker` / `BrokerLaunchNoPty` / `NO_WRITE_UP` / `AppID-based`:
  multiple matches in the new section (L609-631).
- dated validation line `validated 2026-05-26` present (L611).
- TUI-limitation header intact: exactly 1 match at L542.
- line count 670 (was 643; within the expected ~660-700).
- All cross-reference anchors resolve to existing headers (heavy-runtime L609,
  defense-in-depth L570, TUI-limitation L542, shell-envelope L550).

## Scope note (operator decision)

Operator selected "Record corrected cwd" (not "Also note doc fix") at the execute-phase
checkpoint, so Plan 52-02 stayed at planned scope: the working-directory correction is
recorded in `52-HUMAN-UAT.md` / `52-VERIFICATION.md` but the doc's new canonical
invocation references the existing "Working directory choice" section + "not a bare drive
root" rather than adding a dedicated cwd-correction call-out.

## key-files

created:
- .planning/phases/52-field-validation-closure-heavy-runtime-human-uat-doc-update/52-VERIFICATION.md
- .planning/phases/52-field-validation-closure-heavy-runtime-human-uat-doc-update/52-02-SUMMARY.md

modified:
- docs/cli/development/windows-poc-handoff.mdx (committed with `git add -f` — gitignored-but-tracked)

## Self-Check: PASSED

- Doc is profile-conditional; no universal `nono run` WRITE_RESTRICTED claim remains.
- Heavy-runtime subsection present with the security tradeoff + dated validation line.
- TUI-limitation section intact.
- 52-VERIFICATION.md status: pass; both REQ IDs closed.
- Doc committed via `git add -f` with DCO sign-off (see Task 3 commit).
