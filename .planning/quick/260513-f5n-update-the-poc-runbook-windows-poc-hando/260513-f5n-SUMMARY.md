---
quick_id: 260513-f5n
slug: update-the-poc-runbook-windows-poc-hando
mode: quick
status: complete
date: 2026-05-13
commit: 940dafd0
files-modified:
  - docs/cli/development/windows-poc-handoff.mdx
---

# Quick Task 260513-f5n — Summary

## One-liner

Updated POC runbook so Option C builds `nono-shell-broker` for the MSVC target and passes the now-mandatory `-BrokerPath` to `build-windows-msi.ps1` (and ships the broker alongside `nono.exe` in the portable zip), unblocking non-interactive Phase-31 POC builds.

## What changed

Single file: `docs/cli/development/windows-poc-handoff.mdx`. Two coupled edits committed atomically:

1. **Option C build block (lines 48-62).** Replaced the single `cargo build -p nono-cli ...` line with a two-binary build (`-p nono-cli -p nono-shell-broker`) plus a comment anchoring the change to Phase 31 Plan 04's sibling-deployment requirement (D-07 runtime cascade in `spawn_windows_child`). Added `-BrokerPath .\target\x86_64-pc-windows-msvc\release\nono-shell-broker.exe` to the `build-windows-msi.ps1` invocation. The `Output:` filename line on line 64 is preserved verbatim — MSI naming is unaffected.
2. **Portable archive snippet (lines 72-77).** Expanded `Compress-Archive -Path` to include both `nono.exe` and `nono-shell-broker.exe`. `-DestinationPath` filename unchanged.

The Warning callout about `-ServiceBinaryPath` for the machine MSI (lines 66-68) and all surrounding prose were left untouched per the plan's anti-scope.

## Why coupled in one commit

The broker is **mandatory** in `build-windows-msi.ps1` (Phase 31 Plan 04, line 12 of the script: `[Parameter(Mandatory = $true)] [string]$BrokerPath`). Documenting the MSI invocation without the matching cargo build line, or vice versa, would leave the runbook half-fixed: either operators wouldn't have the binary the next line consumes, or they'd build a binary the next line doesn't reference. The portable-zip block is the same shape question — sibling deployment is a single decision, not two. One atomic commit keeps the runbook internally consistent for any reader who lands on this revision.

## Verification

Both grep checks from the plan's Task `verify` blocks pass against the committed file:

- `grep -F "BrokerPath" docs/cli/development/windows-poc-handoff.mdx` → match at line 59 inside the Option C block (the `-BrokerPath ...` MSI flag, not the later Phase-31 narrative section).
- `grep -F "-p nono-shell-broker" docs/cli/development/windows-poc-handoff.mdx` → match at line 53 inside the cargo build invocation.
- `grep -A 4 'Compress-Archive' docs/cli/development/windows-poc-handoff.mdx` → both `nono.exe` and `nono-shell-broker.exe` appear in the `-Path` array (lines 74-75).

## Deviations

**Execution note (not a plan deviation):** The first attempt to apply the edits used the project-root absolute path (`C:\Users\OMack\Nono\docs\...`), which targeted the main repo's working tree instead of this worktree's tracked file. Caught immediately on the pre-commit HEAD check (which reported HEAD=main when run from the wrong cwd), then reverted with `git checkout -- docs/cli/development/windows-poc-handoff.mdx` in the main repo and re-applied against the worktree path. No commits landed in the main repo; the only commit is `940dafd0` on `worktree-agent-aba26731230245e05`. Recording this so the orchestrator knows the main repo working tree is clean.

**Add behavior:** `docs/cli/development/` is in `.gitignore` (line 13), but the runbook file is tracked (predates the ignore rule). `git add` rejected the path until `-f` was passed. Used `git add -f` on the specific file path — the file remains a tracked file with a new commit; nothing else in the ignored directory was touched.

No Rule 1/2/3 auto-fixes were needed and no Rule 4 architectural questions arose. The edits exactly match the target shapes in the plan.

## Self-Check

- File exists: `docs/cli/development/windows-poc-handoff.mdx` — FOUND.
- Commit exists: `940dafd0` — FOUND in branch history.
- Both grep checks pass against the committed content.

## Self-Check: PASSED
