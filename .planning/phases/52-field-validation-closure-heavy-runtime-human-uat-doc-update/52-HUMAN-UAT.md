---
status: complete
phase: 52-field-validation-closure-heavy-runtime-human-uat-doc-update
source: [52-01-PLAN.md, 51-HUMAN-UAT.md]
started: "2026-05-26T19:12:24Z"
updated: "2026-05-26T19:17:42Z"
---

## Current Test

[complete — repro A and repro B both attested PASS on the live Windows 11 host]

## Preflight (Task 1 — binary identity per D-52-02)

- nono version: `nono 0.57.0` (Phase 51 build; `C:\Program Files\nono\nono.exe`) — meets the >= 0.57.0 requirement
- claude version: `2.1.150 (Claude Code)`
- claude.exe path: `C:\Users\OMack\.local\bin\claude.exe`
- claude.exe size: 234,248,864 bytes (223.4 MiB / 234.2 MB) — confirmed self-contained single-exe build, >= ~234 MB per D-52-02
- claude.exe build date: 2026-05-24 (>= 2026-05-24 requirement); PE32+ console executable (`file`: "PE32+ executable for MS Windows 6.00 (console), x86-64, 12 sections")
- host Windows build: 26200 (Windows 11 Enterprise 10.0.26200)
- dry-run label-apply gate: `nono run --dry-run --profile claude-code -- cmd /c "echo preflight-ok"` printed the capability list with **no WRITE_OWNER error**, exit 0

### Working-directory correction (D-52-01 deviation — operator-approved 2026-05-26)

D-52-01 prescribed running both repros from a bare `%USERPROFILE%` working directory.
On this host that shape fails the pre-spawn **cwd-coverage gate** with
`execution directory outside supported allowlist` (emitted from
`crates/nono/src/sandbox/windows.rs:1304-1309`): the `claude-code` profile grants
`%USERPROFILE%\.claude`, `%USERPROFILE%\.cache\claude`, etc. but **not**
`%USERPROFILE%` itself, so the bare-profile-root cwd is not in the allowlist.
`--allow-cwd %USERPROFILE%` is also refused because `%USERPROFILE%` overlaps the
protected `.nono` state root.

Both repros were therefore run from the profile-covered directory
`C:\Users\OMack\.claude`. This is faithful to D-52-01's stated **intent**
("a working directory the profile covers, so both A and B genuinely spawn and exit 0
under the claude-code profile"); only the specific directory name was corrected from
the empirically-incorrect bare `%USERPROFILE%`. The repro commands themselves are
unchanged from D-52-01 / the ROADMAP. Operator approved this deviation at the
execute-phase checkpoint on 2026-05-26.

This also corrects the pessimistic claim in `51-04-SUMMARY.md` that
`nono run --profile claude-code -- cmd /c "echo hi"` "cannot exit 0 on the dev host":
that summary conflated the **cwd-coverage gate** (fires from an uncovered cwd) with the
distinct **Phase 27 launch-path gate** (fires only for cmd shapes that resolve `C:\`,
e.g. `cmd /c cd` — see `crates/nono-cli/tests/audit_attestation.rs:118-123`).
`cmd /c echo hi` does **not** require `C:\` and passes from a covered cwd.

## Tests

### Repro A — plain console app (no-regression)

command: nono run --profile claude-code -- cmd /c "echo hi"
working_dir: C:\Users\OMack\.claude (profile-covered; see working-directory correction above)
stdout: hi
  (preceded by the cosmetic cmd.exe notice "'\\?\C:\Users\OMack\.claude' ... UNC paths
   are not supported. Defaulting to Windows directory." — documented as a cmd.exe quirk,
   not a nono bug, in windows-poc-handoff.mdx lines 470-475; the command still printed "hi")
exit_code: 0
no_0xC0000142_dialog: yes (exit 0 ⇒ no STATUS_DLL_INIT_FAILED; no crash/WER dialog)
timestamp: 2026-05-26T19:16:24Z
verdict: PASS

### Repro B — heavy-runtime (0xC0000142 fix confirmation)

command: nono run --profile claude-code -- claude --version
working_dir: C:\Users\OMack\.claude (profile-covered; see working-directory correction above)
stdout: 2.1.150 (Claude Code)
exit_code: 0
no_0xC0000142_dialog: yes (exit 0 ⇒ DLL init succeeded; no STATUS_DLL_INIT_FAILED, no crash/WER dialog)
binary_identity:
  claude_version_string: 2.1.150 (Claude Code)
  claude_exe_path: C:\Users\OMack\.local\bin\claude.exe
  claude_exe_size_mb: 234.2 MB (234,248,864 bytes) — confirmed >= 234 MB self-contained single-exe build per D-52-02
  claude_exe_build_date: 2026-05-24
host_windows_build: 26200
timestamp: 2026-05-26T19:15:48Z
verdict: PASS

note: Both repros were executed live by the orchestrator on the operator's staged
Windows 11 host via piped (non-interactive) stdio and attested PASS by the operator at
the execute-phase checkpoint. Because the runs are non-interactive, no GUI dialog can
surface; the exit code is the dispositive signal. STATUS_DLL_INIT_FAILED (0xC0000142)
prevents the process from starting and surfaces as exit code 0xC0000142 (3221225794) or
a crash — repro B instead reached `claude.exe`'s entry point, printed its version, and
exited 0, which is positive proof DllMain/bootstrap WRITE-type activity (the original
regression) now succeeds under the BrokerLaunchNoPty Low-IL primary token.

## Summary

total: 2
passed: 2
failed: 0
deferred_sc4_closed: true

## Phase 51 SC-4 Closure Note

This plan closes the ROADMAP SC-4 positive-spawn deferral recorded in
`51-HUMAN-UAT.md` ("ROADMAP SC-4 — positive supervised-spawn exit-0 on a real Windows
host"). Repro B is that test. Repro B verdict is **PASS** on Windows 11 build 26200 with
the Phase 51 `nono 0.57.0` BrokerLaunchNoPty binary and the 234 MB self-contained
`claude.exe` — therefore SC-4 is **closed** and the `0xC0000142` / STATUS_DLL_INIT_FAILED
regression is confirmed absent in live end-to-end use (not just at the unit/integration
layer). REQ-WSRH-04 (repro B passes) is satisfied.

## Gaps

(none — both repros PASS)
