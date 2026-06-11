# TODO: macOS resource-limit enforcement (--timeout / RLIMIT_NPROC) does not fire

**Captured:** 2026-06-11 (Phase 65 gate-65-A live macOS UAT, Assertion 5)
**Severity:** high — a documented REQ-RESL-NIX-03 feature is non-functional on macOS
**Source:** `.planning/phases/65-minifilter-adr-macos-live-re-validation/65-HUMAN-UAT.md` (A5)

## Problem
On a **real macOS host** (`Oscars-MacBook-Pro`, Apple Silicon), with `NONO_RESL_HOST_VALIDATED=1`:
- `macos_timeout_kills_at_deadline` — FAILED: nono did not exit within 12s. `--timeout 5s`
  did NOT SIGKILL `sleep 60` at ~5s.
- `macos_max_processes_blocks_on_rlimit_nproc` — FAILED: nono did not exit within 20s.
  `--max-processes 5` did NOT make `fork()` fail (EAGAIN).

**Key correction:** this is NOT a GitHub-runner fidelity issue (the earlier hypothesis). The
macOS `--timeout` supervisor watchdog and `setrlimit(RLIMIT_NPROC)` enforcement genuinely do
**not fire on a real macOS host**. REQ-RESL-NIX-03's macOS enforcement was never host-validated
(Phase 37 host-blocked); gate-65-A confirmed it is broken.

## Scope note
This is SEPARATE from the macOS Seatbelt backend (MACOS-03), which re-validated cleanly at
gate-65-A (A1–A4: file/path deny enforcement + 722 native lib tests pass). The two `#[gated]`
tests stay behind `NONO_RESL_HOST_VALIDATED` (off CI) until this is fixed.

## Fix
- Investigate the macOS supervised path: does the `--timeout` Instant+SIGKILL watchdog run on
  macOS, and does `setrlimit(RLIMIT_NPROC)` get applied to the child before exec? (The watchdog
  is nono's own supervisor code and should be cross-platform — its non-firing is likely a nono
  bug, not an OS limitation. RLIMIT_NPROC on macOS counts all UID processes and may need a
  different approach.)
- Once fixed, re-run A5 with `NONO_RESL_HOST_VALIDATED=1` on a real macOS host.

## Acceptance
Both `macos_timeout_kills_at_deadline` and `macos_max_processes_blocks_on_rlimit_nproc` pass on
a real macOS host with `NONO_RESL_HOST_VALIDATED=1`.
