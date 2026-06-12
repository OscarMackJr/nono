# TODO: macOS resource-limit enforcement (--timeout / RLIMIT_NPROC) does not fire

**Captured:** 2026-06-11 (Phase 65 gate-65-A live macOS UAT, Assertion 5)
**Severity:** high — a documented REQ-RESL-NIX-03 feature is non-functional on macOS
**Source:** `.planning/phases/65-minifilter-adr-macos-live-re-validation/65-HUMAN-UAT.md` (A5)
**Resolves phase:** 68 — macOS Resource-Limit Enforcement Fix (v2.11; RESL-MAC-01/RESL-MAC-02)

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

## Attempt 1 — Phase 68-01 (2026-06-12): FAILED
The first fix (setpgid(0,0) in the supervised child arm + real `libc::setrlimit(RLIMIT_NPROC, baseline+N)`
in both the Direct `install_pre_exec` and the Supervised `ForkResult::Child` arm; `uid_process_count()`
parent-side baseline; commits `1b2e2ad0`, `f94c1c1b`, `3583bacc`) **did NOT make enforcement fire** on the
real host. `cargo build -p nono-cli` succeeded on macOS, so the changes are live in the binary — but both
gated tests still fail identically (timeout watchdog and RLIMIT_NPROC both no-op at runtime). The fix is
insufficient or targets a path the supervised run does not actually take. **Open the next round via `/gsd:debug`**
with host-side diagnostics (see suggested first probe below). Confound to keep separate: pre-existing
`audit_attestation` sandbox-init failure `set_read_timeout EINVAL` — see `20260612-macos-supervisor-ipc-rcvtimeo-einval.md`.

Suggested first host probe (Mac): run the timeout case with verbose logging to learn which exec path runs and
whether the watchdog thread is even spawned:
`NONO_LOG=debug ./target/debug/nono run --timeout 5s --read=/bin --read=/usr --read=/private -- sleep 60`
