---
status: pass
phase: 52-field-validation-closure-heavy-runtime-human-uat-doc-update
verified: "2026-05-26T19:17:42Z"
requirements: [REQ-WSRH-04, REQ-WSRH-06]
score: 4/4 success criteria met
---

# Phase 52 Verification — Field validation closure (heavy-runtime HUMAN-UAT + doc update)

**Goal:** Confirm on a real Windows 11 host that the Phase 51 implementation eliminates
the `0xC0000142` failure for `claude.exe`, record the reproduction-matrix verdicts, and
update the Windows POC handoff documentation to reflect the new `nono run` behavior for
heavy-runtime children.

**Verdict:** **PASS** — all four ROADMAP success criteria met; both requirements closed.

## ROADMAP Success Criteria

| SC | Criterion | Verdict | Evidence |
|----|-----------|---------|----------|
| SC-1 | `nono run --profile claude-code -- claude --version` → no `0xC0000142`, version string printed, exit 0 (234 MB self-contained `claude.exe`) | **PASS** | `52-HUMAN-UAT.md` Repro B: stdout `2.1.150 (Claude Code)`, exit_code 0, no 0xC0000142, binary 234,248,864 B / 2026-05-24, host build 26200, ts 2026-05-26T19:15:48Z |
| SC-2 | `nono run --profile claude-code -- cmd /c "echo hi"` (repro A) continues to pass on the same host/run — no plain-console-app regression | **PASS** | `52-HUMAN-UAT.md` Repro A: stdout `hi`, exit_code 0, no 0xC0000142, ts 2026-05-26T19:16:24Z |
| SC-3 | Both verdicts recorded with timestamps in the Phase 52 HUMAN-UAT artifact; VERIFICATION closes `status: pass` (not `human_needed`) | **PASS** | `52-HUMAN-UAT.md` `status: complete`, both verdicts timestamped, summary `passed: 2 / failed: 0`; this file is `status: pass` |
| SC-4 | `windows-poc-handoff.mdx` updated: heavy-runtime children supported via the Low-IL broker path; doc no longer claims `nono run` is limited to plain console apps for the claude-code profile | **PASS** | New `## Windows nono run — heavy-runtime children (Phase 52, validated 2026-05-26)` section (L609); profile-conditional envelope at L594; anchors at L573/L544/L642-643; TUI-limitation header intact at L542 |

## Requirements Closure

| Requirement | Disposition | Evidence |
|-------------|-------------|----------|
| REQ-WSRH-04 (heavy-runtime launch passes — live field confirmation of the `0xC0000142` fix) | **CLOSED** | Repro B PASS on Windows 11 build 26200 with `nono 0.57.0` BrokerLaunchNoPty + 234 MB self-contained `claude.exe` |
| REQ-WSRH-06 (HUMAN-UAT matrix executed with evidence + doc updated) | **CLOSED** | `52-HUMAN-UAT.md` (both repros PASS, full D-52-07 evidence) + `windows-poc-handoff.mdx` D-52-05/06 profile-conditional sweep + heavy-runtime subsection |

## Phase 51 SC-4 Deferral Closure

The ROADMAP SC-4 "positive supervised-spawn exit-0 on a real Windows host" test deferred
in `51-HUMAN-UAT.md` is **closed** by this phase's Repro B PASS. The `0xC0000142` /
STATUS_DLL_INIT_FAILED regression is confirmed absent end-to-end (not just at the
unit/integration layer): the Phase 51 `BrokerLaunchNoPty` Low-IL primary token (no
synthetic restricting SID) lets the heavy `claude.exe` DllMain WRITE-type activity
succeed.

## Doc Accuracy (security property — D-52-05)

The per-session-WFP-waiver and `NO_WRITE_UP`-not-`WRITE_RESTRICTED` claims for the
`claude-code` profile were verified against the code ground truth
(`crates/nono-cli/src/exec_strategy_windows/launch.rs:1100-1113`, cascade rule 3 at
~1123, waiver comment at 1204-1206/1214-1216) before being written. The doc is now
profile-conditional and does not overstate `claude-code` network isolation.

## Notes / Deviations

- **D-52-01 working-directory correction (operator-approved):** bare `%USERPROFILE%`
  fails the pre-spawn cwd-coverage gate (`execution directory outside supported
  allowlist`) because the `claude-code` profile grants `%USERPROFILE%\.claude` etc. but
  not `%USERPROFILE%` itself; `--allow-cwd %USERPROFILE%` is refused for overlapping the
  protected `.nono` state root. Both repros were run from the profile-covered
  `C:\Users\OMack\.claude`, faithful to D-52-01's intent. Repro commands unchanged.
  This also corrects the over-pessimistic `51-04-SUMMARY.md` claim (it conflated the
  cwd-coverage gate with the distinct Phase 27 launch-path gate, which fires only for
  cmd shapes resolving `C:\`, e.g. `cmd /c cd` — not `cmd /c echo hi`).
- **Non-interactive execution:** repros were run via piped stdio by the orchestrator on
  the operator's staged host and attested PASS by the operator. Exit code is the
  dispositive 0xC0000142 signal (a DLL-init failure cannot exit 0 with a version string).

## Human Verification

None outstanding. Both repros were executed live on the target host and attested PASS by
the operator at the execute-phase checkpoint (2026-05-26). No items require further human
testing.
