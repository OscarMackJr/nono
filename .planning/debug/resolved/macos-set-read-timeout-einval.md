---
slug: macos-set-read-timeout-einval
status: resolved
trigger: "macOS-only unit test exec_strategy::tests::reconnect_survival fails: set_read_timeout(5s) -> EINVAL (os error 22) on the supervisor IPC UnixStream"
created: 2026-06-24
updated: 2026-06-24
---

# Debug Session: macOS set_read_timeout EINVAL (reconnect_survival)

## Symptoms

**Expected behavior:** `cargo test -p nono-cli` (bin "nono") passes on macOS. `SupervisorSocket::set_read_timeout(Some(5s))` should succeed (it succeeds on Linux).

**Actual behavior:** On `Test (macos-latest)` CI, 1 unit test fails: `1431 passed; 1 failed`.
```
test exec_strategy::tests::reconnect_survival ... FAILED
thread 'exec_strategy::tests::reconnect_survival' panicked at crates/nono-cli/src/exec_strategy.rs:4983:22:
set_read_timeout must succeed: SandboxInit("Failed to set socket read timeout: Invalid argument (os error 22)")
```
Ubuntu Test is fully GREEN (this test passes on Linux). macOS-only.

**Timeline / classification:** Test + the `set_read_timeout` wiring were added in `e9032edd`
(feat 59-02: "SC1 macOS keep-alive + SC2 set_read_timeout wiring + SC1 reconnect_survival test"),
i.e. **Phase 59 / v2.8** — PRE-EXISTING, NOT a v3.1/v3.2 regression. `crates/nono/src/supervisor/socket.rs::set_read_timeout` is older still (`d46d6026`). This surfaced as the last
macOS Test red on PR #12 only because every other (v3.x-introduced) failure was fixed first
(see resolved session ci-linux-cfg-compile-errors). It would fail on `main`'s macOS CI too.

**Reproduction:** macOS only. `NONO`-side: `SupervisorSocket::from_stream(parent_stream).set_read_timeout(Some(crate::timeouts::supervisor_ipc_read_timeout()))` where the default timeout = `Duration::from_secs(5)` (`SUPERVISOR_IPC_READ_TIMEOUT`, timeouts.rs:75). The test forks; child drops its IPC socket end then sleeps 350ms; parent sets the read timeout on `parent_stream` (a `UnixStream` from a socketpair).

## Key code locations
- Failing assert: `crates/nono-cli/src/exec_strategy.rs:4983` (test `reconnect_survival`, `#[cfg(unix)]` at ~4873).
- `SupervisorSocket::set_read_timeout`: `crates/nono/src/supervisor/socket.rs:192` — thin wrapper: `self.stream.set_read_timeout(timeout).map_err(... SandboxInit ...)`. `self.stream` is a `std::os::unix::net::UnixStream` (from `from_stream`).
- Timeout value: `crates/nono-cli/src/timeouts.rs:75,83` — default `Duration::from_secs(5)`.

## Constraints / environment
- This host is WINDOWS — these `#[cfg(unix)]` tests do NOT run locally; macOS CI (`Test (macos-latest)`) is the only verifier. Verification loop: push to branch `milestone/v2.13-carryforward-closeout`, then `gh pr checks 12 --repo OscarMackJr/nono` + `gh api repos/OscarMackJr/nono/actions/jobs/<id>/logs` (works mid-run; ANSI-strip with `sed 's/\x1b\[[0-9;]*m//g'`). ~6 min/cycle.
- SECURITY-adjacent (supervisor IPC). Do not weaken timeout/keep-alive semantics. DCO sign-off: `Oscar Mack Jr <oscar.mack.jr@gmail.com>`.

## Initial hypotheses
1. **macOS `SO_RCVTIMEO` rejects the value Rust derives from `Duration::from_secs(5)`** — unlikely for a plain 5s, but verify how Rust std converts Duration→timeval on macOS and whether the socket FD/type (socketpair `UnixStream`) matters. Check whether the stream is non-blocking or already in a state where `setsockopt(SO_RCVTIMEO)` returns EINVAL on Darwin.
2. **The socket is not a normal connected `UnixStream`** at the point of the call — e.g. `from_stream` wraps an fd that on macOS is a `SOCK_DGRAM` or an fd where `SO_RCVTIMEO` is invalid, or it was created via a raw fd / `socketpair` with flags Darwin treats differently than Linux.
3. **Peer already closed** (child dropped its end before the parent sets the timeout) interacts with macOS `setsockopt` — Linux tolerates it, Darwin returns EINVAL. Check ordering: can the parent set the timeout BEFORE the peer-close race, or is EINVAL actually about the value not the peer state?
4. **macOS-specific: the production code path may have the same bug** — `set_read_timeout` is called in real supervised runs (SC2). If it EINVALs on macOS in production too, this is a real macOS supervisor defect, not just a test artifact. Determine whether production supervised runs on macOS hit this (the test mirrors production: `set_read_timeout(supervisor_ipc_read_timeout())`).

## Current Focus

reasoning_checkpoint:
  hypothesis: "macOS rejects setsockopt(SO_RCVTIMEO) with EINVAL on AF_UNIX SOCK_STREAM socketpairs (Darwin kernel limitation). The reconnect_survival test calls set_read_timeout UNCONDITIONALLY under #[cfg(unix)] (line 4982), but PRODUCTION already gates the identical SC2 call to #[cfg(target_os = \"linux\")] (line 1568). Therefore only the TEST diverges from production; production macOS is correct."
  confirming_evidence:
    - "Production SC2 call (exec_strategy.rs:1568-1571) is gated #[cfg(target_os = \"linux\")] with an explicit comment (lines 1563-1567) documenting the macOS EINVAL kernel limitation and the poll(200ms) alternative."
    - "git: commit c3cf3855 'fix(68-02): D1 platform-gate set_read_timeout to Linux (macOS AF_UNIX rejects SO_RCVTIMEO)' changed the PRODUCTION call from #[cfg(unix)] to #[cfg(target_os=\"linux\")] precisely because supervised runs were ABORTING on macOS. The test was not updated in that commit."
    - "Test reconnect_survival verifies the SC1 keep-alive predicate (elapsed>=200ms + exit code 0, lines 5027-5037). The set_read_timeout call at 4982 is incidental scaffolding ('as we would in production'), NOT the behavior under test."
    - "Value/state ruled out: 5s Duration is valid; production uses the SAME supervisor_ipc_read_timeout() on Linux without issue. EINVAL is family/type-driven (AF_UNIX on Darwin), not Duration-driven or peer-closed-driven."
  falsification_test: "If EINVAL were value-driven or peer-closed-driven, Linux would also fail (it does not) AND production macOS would also need the same skip for a different reason. Production's Linux-only gate + green Ubuntu CI confirms it is family/type-driven."
  fix_rationale: "Gate the test's set_read_timeout call to #[cfg(target_os = \"linux\")], mirroring production exactly. On Linux the SC2 slowloris-guard call is still exercised; on macOS it is skipped just as production skips it. SC2 liveness control is NOT weakened — production behavior is unchanged on every platform; only the test is brought into alignment with production."
  blind_spots: "Cannot run #[cfg(unix)] tests on this Windows host; macOS CI on PR #12 is the only authoritative verifier. The SC1 keep-alive assertion timing (>=200ms) is unaffected by removing the macOS timeout call, but only macOS CI can confirm."

- next_action: Edit exec_strategy.rs:4981-4983 to gate the set_read_timeout call to #[cfg(target_os = \"linux\")] (matching production line 1568); correct the misleading 'as we would in production' comment. Then push to milestone/v2.13-carryforward-closeout and verify via macOS CI on PR #12.

## RESOLUTION (2026-06-24) — both macOS Test failures fixed; macOS Test GREEN

Two distinct PRE-EXISTING macOS failures (neither a v3.x regression), surfaced one-at-a-time
once the v3.x-introduced failures were cleared:

1. **`reconnect_survival` — deterministic** (`7c9b6b6e`). macOS rejects `setsockopt(SO_RCVTIMEO)`
   with EINVAL on AF_UNIX SOCK_STREAM socketpairs. Production already gates the SC2 read-timeout
   call to `#[cfg(target_os = "linux")]` (commit `c3cf3855` / Phase 68-02) and uses poll(200ms)
   on macOS; the test (`e9032edd` / Phase 59) predated that and called it unconditionally under
   `#[cfg(unix)]`. Fix: gate the test's call to Linux, matching production. SC2 not weakened.

2. **`test_open_shim_drop_cleans_up_directory` — flaky** (`1a9039a4`). `create_open_shim` copies its
   `nono_exe` arg; the test passed `current_exe()` (the multi-hundred-MB debug TEST binary), so the
   copy intermittently failed under CI load/disk → None → panic "create shim". The failure swapped
   with reconnect_survival across runs (flakiness signature). Fix: pass a tiny stand-in file (the
   test only checks dir-create + drop-cleanup; the helper is never executed). Pre-existing (PR #340).

Result: **Test (ubuntu) + Test (macos) both GREEN** on PR #12; Clippy/Rustfmt/Verify-FFI green too.
The entire PR #12 test+lint surface (Linux + macOS, unit + integration) now passes. Remaining reds
are external baseline only: Cargo Audit (RUSTSEC-2026-0185 quinn-proto), Docs Checks (orphan
windows-win-1706-option-1-workstream.mdx), Windows suite (chronic baseline).

## Evidence

- timestamp: 2026-06-24 — CI `Test (macos-latest)` job 83199337402: `1431 passed; 1 failed`, only `reconnect_survival`. Ubuntu Test fully green (same test passes on Linux).
- timestamp: 2026-06-24 — `git log -S "fn reconnect_survival"` → added in `e9032edd` (Phase 59 / v2.8). Pre-existing, not v3.x.
- timestamp: 2026-06-24 — checked: production SC2 call site exec_strategy.rs:1556-1571. found: the production `set_read_timeout` block is gated `#[cfg(target_os = "linux")]` (line 1568), with an in-code comment (1563-1567) stating "macOS setsockopt(SO_RCVTIMEO) returns EINVAL on AF_UNIX SOCK_STREAM socketpairs — a macOS kernel limitation. The macOS supervisor loop uses poll(200ms)". implication: PRODUCTION DOES NOT call set_read_timeout on macOS. The defect is test-only divergence — the test mirrors PRE-fix production behavior.
- timestamp: 2026-06-24 — checked: `git show c3cf3855`. found: commit "fix(68-02): D1 platform-gate set_read_timeout to Linux (macOS AF_UNIX rejects SO_RCVTIMEO)" changed PRODUCTION's gate from `#[cfg(unix)]` to `#[cfg(target_os = "linux")]` because "supervised runs were aborting on macOS before reaching the watchdog or RLIMIT enforcement." Touched only exec_strategy.rs (1 file, +7/-1). The reconnect_survival test's identical call at line 4982 was NOT updated. implication: confirmed root cause AND confirmed production is already correct on macOS — fix belongs in the test only.
- timestamp: 2026-06-24 — checked: reconnect_survival assertions (exec_strategy.rs:5022-5037) + docstring (4852-4872). found: the test verifies the SC1 keep-alive predicate (loop survives IPC POLLHUP when PTY relay active → elapsed >= 200ms + child exit code 0). The set_read_timeout call at 4982 is incidental ("Set the read timeout as we would in production (SC2)") and is not asserted on. implication: gating it to Linux does not weaken what the test verifies, and Linux still exercises the SC2 call.

## Resolution

root_cause: "macOS (Darwin) rejects setsockopt(SO_RCVTIMEO) with EINVAL (os error 22) on AF_UNIX SOCK_STREAM socketpairs — a documented Darwin kernel limitation (SO_RCVTIMEO is not honored for this socket family/type on macOS the way it is on Linux). NOT a Duration-value, non-blocking, or peer-closed-state issue: a plain 5s Duration is valid and the peer-closed race is irrelevant. PRODUCTION already handles this correctly: the SC2 set_read_timeout call (exec_strategy.rs:1568) is gated #[cfg(target_os = \"linux\")] (commit c3cf3855, Phase 68-02 D1), so macOS supervised runs skip it and rely on the poll(200ms) supervisor loop for bounded IPC behavior. The reconnect_survival test diverges from production: it calls set_read_timeout unconditionally under #[cfg(unix)] (line 4982), mirroring PRE-fix production behavior, which is why it EINVALs on macOS CI while production does not. Production macOS supervised runs are NOT defective."
fix: "APPLIED. Gated the test's set_read_timeout call (exec_strategy.rs:4990-4992) to #[cfg(target_os = \"linux\")], mirroring the production gate at line 1568. Replaced the misleading 'as we would in production' comment with an accurate explanation referencing commit c3cf3855 / Phase 68-02 D1 and the Darwin AF_UNIX SO_RCVTIMEO limitation. `let mut sock` remains used on both platforms (passed as &mut sock to run_supervisor_loop), so no unused-mut warning on macOS."
verification: "PENDING — Windows host cannot compile/run #[cfg(unix)] tests. Authoritative verifier is `Test (macos-latest)` on PR #12 (OscarMackJr/nono, branch milestone/v2.13-carryforward-closeout). Expected: reconnect_survival passes on macOS; Ubuntu Test stays green (Linux still exercises the SC2 call)."
files_changed: ["crates/nono-cli/src/exec_strategy.rs"]

## Eliminated

- hypothesis: EINVAL is driven by the Duration→timeval conversion or a sub-microsecond/zero Duration in the test build.
  evidence: supervisor_ipc_read_timeout() default is Duration::from_secs(5) (timeouts.rs:75,83); no test cfg override reduces it; production uses the identical value on Linux without error. A plain 5s is a valid SO_RCVTIMEO value.
  timestamp: 2026-06-24
- hypothesis: EINVAL is caused by the peer-closed / half-closed socketpair state (child drops its end before parent sets the timeout).
  evidence: production's c3cf3855 fix gates the call platform-wide on macOS for ALL supervised runs (not just peer-closed ones); the commit message attributes it to "AF_UNIX SOCK_STREAM socketpairs" kernel behavior, not connection state. Linux tolerates the same peer-closed sequence in the test. The cause is socket family/type on Darwin, not peer state.
  timestamp: 2026-06-24
- hypothesis: production macOS supervised runs (SC2) share the defect → fix belongs in production socket setup.
  evidence: production call site exec_strategy.rs:1568 is already #[cfg(target_os = "linux")]; macOS never reaches set_read_timeout in production. Fix belongs in the test only.
  timestamp: 2026-06-24
