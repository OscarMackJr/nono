---
quick_id: 260907-civ
slug: ci-validation-followups
date: 2026-09-07
status: complete
commits: [c16794a2, d751eb8e, 22c1cd6b, e75f6f5d]
milestone: v3.7
ci_runs: [34136554776, 34139960572, 34142202429]
---

# Quick Task 260907-civ — Summary

Four follow-up fixes found by *watching CI*, all validated on real runners.
CI went **10 green / 12 red → ~16 green / 5 red**, every remaining red attributed.

| Commit | Fix | Verdict |
|---|---|---|
| `c16794a2` | R-B3 checkout ownership in `windows-test-harness.ps1` | ✅ Windows Smoke green ×2 |
| `d751eb8e` | harness `$LASTEXITCODE` fail-closed | ✅ Windows Build green ×2 |
| `22c1cd6b` | re-sign user trust policy after `--force` | ✅ full chain passes |
| `e75f6f5d` | R-B3 fixture ownership, 32 sites | ✅ R-B3 count **0** |

## The R-B3 class, gotten wrong three times

1. Patched the one test the log named → a sibling took its place.
2. Fixed the harness (checkout ownership) → Smoke green, but Integration failed on
   a **second sub-class**: test-*created* workspace fixtures.
3. Patched all 32 fixture sites → R-B3 finally 0.

Root of the repeated miss: **I let the error message define the class.** Each log
named one workspace, and I fixed that kind of workspace. The actual invariant is
"any workspace reaching a live `nono run` must be user-owned", which has two
independent sources — the checkout and per-test fixtures.

Key fact that rules out the cheap fix: **Windows does not inherit owner from the
parent directory.** A new object's owner comes from the creating token's default
owner (`BUILTIN\Administrators` under elevation), so owning `%TEMP%` in the
harness could never have fixed fixtures — it had to be per-fixture, exactly as
`260815-gfd` did for its 15 tests.

## `$LASTEXITCODE` — the inverse of 260815-f2x

Windows Build printed `Validated Windows MSI contract...` and then died with
`Command failed ... with exit code ` — **empty**. `$LASTEXITCODE` is set only by
*native* executables; a `.ps1` invocation leaves it `$null`, and `$null -ne 0` is
`True` in PowerShell. Confirmed directly: `pwsh -c '$x=$null; $x -ne 0'` → True.

Same confusion as `260815-f2x`, **inverted**: that was fail-OPEN (a job reporting
success while its log said "22 failed"); this was fail-CLOSED. Both treat
`$LASTEXITCODE` as describing the last *statement* rather than the last *native
process*. Fixed with an explicit `$global:` reset, `$?` captured immediately, and
a predicate proven across all five shapes.

## Trust CLI — same mistake twice

Two corrections to one assertion, the second caused by the first: added an
`init --user` without noticing the suite had already created that policy, then
`--force`d the write without noticing the policy was **signed** (stale bundle →
digest mismatch). In a suite mutating shared state across ~500 lines, "this
command now succeeds" is not evidence the sequence is correct.

## Findings worth carrying

1. **The error message does not define the class.** Three R-B3 attempts, each
   fixing what the log named rather than what it implied. By contrast the two
   fixes that held first time (`$LASTEXITCODE`, ECHILD isolation) were both
   preceded by mechanical enumeration.
2. **Fail-fast was hiding a lot.** Nearly every "new" red was pre-existing code
   that had never executed in CI — `nono-agentd`, `socket_access_run`, the
   harness exit-code bug. Each was verified masked by grepping prior logs for
   zero occurrences, not asserted.
3. **Windows owner semantics**: token default owner, not parent inheritance.

## Open, none introduced here

- ⚠️ **macOS security fail-open** — `filesystem_deny_blocks_unix_socket_connect_on_macos`:
  a denied socket path connects successfully. Reproduced on both runs. Highest value.
- `nono-agentd` DACL ×3 — elevated-ownership class in a binary the crate-internal
  helper cannot reach (4th instance of that class).
- `socket_access_run` `EPROTOTYPE` — `SOCK_DGRAM` client vs `SOCK_STREAM` listener.
- Shell output-capture residual (post-PTY-fix; `setsid` crash is gone).
- Phase 118 parent-of-protected (Windows Regression).
- `windows_run_redirects_profile_state_vars_into_writable_allowlist` — now clears
  R-B3 and fails its own profile-state assertion. Newly reachable.
