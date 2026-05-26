# Plan 52-01 Summary — HUMAN-UAT reproduction matrix

**Status:** Complete
**Plan:** 52-01 (wave 1)
**Requirements:** REQ-WSRH-04, REQ-WSRH-06
**Executed:** 2026-05-26 (inline on the operator's live Windows 11 host — human-action checkpoints)

## What was done

Executed the Phase 52 two-command reproduction matrix on the operator's real Windows 11
host (build 26200) with the Phase 51 `nono 0.57.0` binary and the 234 MB self-contained
`claude.exe`, then recorded operator-attested verdicts in `52-HUMAN-UAT.md`.

- **Repro A** (`nono run --profile claude-code -- cmd /c "echo hi"`): **PASS** — printed `hi`, exit 0.
- **Repro B** (`nono run --profile claude-code -- claude --version`): **PASS** — printed `2.1.150 (Claude Code)`, exit 0, **no 0xC0000142 / STATUS_DLL_INIT_FAILED**.

Both verdicts attested by the operator at the execute-phase checkpoint. Binary identity
(claude 2.1.150; `claude.exe` 234,248,864 B, 2026-05-24, PE32+ self-contained), host
Windows build (26200), exact commands, full stdout, exit codes, and timestamps are
recorded in `52-HUMAN-UAT.md` per D-52-07.

## Key outcome

- **ROADMAP SC-4 positive-spawn deferral (from 51-HUMAN-UAT.md) is CLOSED.** Repro B is
  that deferred test; it PASSES live. The `0xC0000142` regression is confirmed absent
  end-to-end (not just at the unit/integration layer) — the Phase 51 BrokerLaunchNoPty
  Low-IL primary token lets the heavy `claude.exe` DllMain WRITE-type activity succeed.
- **REQ-WSRH-04 satisfied** (repro B passes on real hardware with the heavy binary).

## Deviation (operator-approved)

D-52-01 prescribed bare `%USERPROFILE%` as the working directory, but on this host that
fails the **cwd-coverage gate** (`execution directory outside supported allowlist`,
`windows.rs:1304-1309`): the `claude-code` profile grants `%USERPROFILE%\.claude`,
`.cache\claude`, etc. — not `%USERPROFILE%` itself; and `--allow-cwd %USERPROFILE%` is
refused for overlapping the protected `.nono` state root. Both repros were run from the
profile-covered `C:\Users\OMack\.claude`, faithful to D-52-01's stated intent (covered
cwd so both A and B genuinely spawn and exit 0). The repro commands are unchanged.

This also corrects `51-04-SUMMARY.md`'s pessimistic claim that repro A "cannot exit 0":
that conflated the cwd-coverage gate with the distinct Phase 27 launch-path gate, which
fires only for cmd shapes resolving `C:\` (e.g. `cmd /c cd`), not for `cmd /c echo hi`
(see `audit_attestation.rs:118-123`).

## Gate for Plan 52-02

Repro B = PASS ⇒ the D-52-03 hard-stop does **not** fire. Plan 52-02 (doc update +
VERIFICATION close) is unblocked and proceeds.

## key-files

created:
- .planning/phases/52-field-validation-closure-heavy-runtime-human-uat-doc-update/52-HUMAN-UAT.md
- .planning/phases/52-field-validation-closure-heavy-runtime-human-uat-doc-update/52-01-SUMMARY.md

## Self-Check: PASSED

- 52-HUMAN-UAT.md exists with repro A PASS and repro B PASS, full evidence, timestamps.
- Binary identity confirms the heavy 234 MB self-contained build (no thin-binary false positive).
- Phase 51 SC-4 closure note present; deferred_sc4_closed: true.
- Repro B PASS ⇒ Plan 52-02 proceeds.
