---
slug: nopty-broker-stdout-swallowed
status: resolved
trigger: |
  PS C:\Users\OMack\nono-poc> nono run --profile claude-code -- claude --version
  ... broker: spawned Low-IL child child_pid=23840
  ... broker: child exited child_exit_code=0
  (NO "2.1.152 (Claude Code)" line printed — just the next PS prompt)
  Running `claude --version` directly (outside nono) prints "2.1.152 (Claude Code)".
created: 2026-05-27
updated: 2026-05-27
---

# Debug Session: nopty-broker-stdout-swallowed

## Symptoms

- **Expected behavior:** `nono run --profile claude-code -- claude --version` echoes the child's stdout (the `2.1.152 (Claude Code)` version line) to the user's console, then exits 0.
- **Actual behavior:** The Low-IL child spawns (`child_pid=23840`) and exits cleanly (`child_exit_code=0`), but the version string NEVER reaches the terminal — the child's stdout appears to be drained by the supervisor relay without being echoed to the parent console. Only the next PowerShell prompt follows.
- **Error message / code:** None — there is no error. Exit code 0. The failure is *silent output loss*, not a crash.
- **Baseline:** `claude --version` run directly (outside nono) prints `2.1.152 (Claude Code)`, so the binary and PATH are fine. The defect is in nono's no-PTY broker stdout relay path.
- **Timeline:** Observed 2026-05-27 on the v0.57.3 dev-layout build that already carries the HANDLE_LIST-dedup broker fix (`d8b7ce00`, resolved session `broker-nopty-createproc-gle87`). That fix made the process *spawn* (closed GLE=87); it did NOT touch the echo-to-console path. So this output-loss bug is either newly-exposed (now that spawns succeed) or long-standing on the no-PTY path.
- **Reproduction:** Native PowerShell console, dev-layout binary: `C:\Users\OMack\Nono\target\release\nono.exe run --profile claude-code --allow-cwd -- claude --version` from a profile-covered cwd. (Also reproduced via the installed signed `nono` on PATH.)
- **Platform:** Windows 11 Enterprise build 26200 (win32). Windows backend (Job Objects / Integrity Level / WFP). nono v0.57.3. Phase 51 `WindowsTokenArm::BrokerLaunchNoPty` no-PTY broker path.

## Current Focus

- hypothesis: **[ROOT CAUSE — CONFIRMED against production source; FIX BEING APPLIED]** On the `BrokerLaunchNoPty` foreground path, the supervisor's pipe-source relay (`supervisor.rs::start_logging`, the `stdout_read != 0` branch, lines 660-741) drains the merged child stdout/stderr and writes those bytes ONLY to (1) the session log file (`session_log_path`) and (2) the `active_attachment` named pipe IF a `nono attach` client is connected. It NEVER writes to `std::io::stdout()` / the foreground console. By contrast the PTY relay `start_interactive_terminal_io` (lines 857-887) DOES `let mut stdout = std::io::stdout(); stdout.write_all(&buf[..n]); stdout.flush();`. So on `nono run` (non-interactive → `start_streaming` calls `start_logging`, not the PTY relay), the child's `2.1.152` version line is consumed off the pipe (child never blocks → exits 0) but never displayed. This is a pure relay-echo gap, NOT a flush/race or a wrong-handle bug.
- test: [DONE] read `start_logging` end-to-end + the PTY relay for contrast + the `start_streaming` dispatch + the foreground-vs-detached discriminator (`is_windows_detached_launch`).
- expecting: [CONFIRMED] `start_logging` has no console-write; the PTY path does. The fix adds a foreground console echo to the `start_logging` pipe-source thread, GATED so it does NOT fire on the genuinely-detached re-exec (`NONO_DETACHED_LAUNCH=1`), where the outer `nono run --detached` has already returned to the user's prompt and there is no foreground console to write to.
- next_action: RESOLVED 2026-05-27. Fix applied + operator PowerShell verify PASS (probe + `claude --version` both print, exit 0). Committed; session moved to resolved/.
- reasoning_checkpoint:
    hypothesis: "The no-PTY supervised relay `supervisor.rs::start_logging` (pipe-source branch) drains child stdout to the session log file (+ optional attach pipe) but never echoes it to the foreground console (`std::io::stdout()`), so `nono run -- claude --version` consumes the version line off the pipe yet displays nothing — while exiting 0."
    confirming_evidence:
      - "supervisor.rs:660-741 (start_logging pipe-source thread): the ONLY write sinks are `log_file.write_all` (line 703) and the `active_attachment` named-pipe `WriteFile` guarded by `if let Some(sendable) = *lock` (lines 719-738). There is NO `std::io::stdout()` and NO `GetStdHandle(STD_OUTPUT_HANDLE)` write in this function."
      - "supervisor.rs:872,880-881 (start_interactive_terminal_io / PTY path): explicitly `let mut stdout = std::io::stdout(); stdout.write_all(&buf[..n]); stdout.flush();` — proving the console-echo exists on the PTY path and is ABSENT on the pipe path. The asymmetry is the bug."
      - "supervisor.rs:433-440 (start_streaming): non-interactive `nono run` takes the `else` branch → `start_logging()` (NOT the PTY relay). `claude --version` is non-interactive (launch_runtime.rs:359 hardcodes interactive_pty=false for run), so it deterministically lands on the no-echo relay."
      - "launch.rs:1688-1689 child_stdio = [stdin_read, stdout_write, stdout_write] (CR-01 merge); the supervisor reads the merged single stdout_read — consistent with bytes being drained (child exits 0) but not forwarded to the console."
      - "Reconciles Phase 52: the sibling resolved session broker-nopty-createproc-gle87 proved (git provenance) that Phase 52's '2.1.150 PASS' ran a Program Files 0.57.0 build predating both the CR-01 merge AND a spawn-succeeding BrokerLaunchNoPty (GLE=87 only resolved TODAY via d8b7ce00). So the no-PTY echo path was never actually exercised end-to-end at Phase 52 — version-string sameness was the trap, not proof the echo works."
    falsification_test: "If, after adding the gated `std::io::stdout()` echo to start_logging's pipe-source thread (read pattern unchanged), `nono run --profile claude-code --allow-cwd -- claude --version` from a real PowerShell console prints `2.1.152 (Claude Code)` and still exits 0 (and a `cmd /c \"echo HELLO_STDOUT_PROBE\"` probe also now prints), the no-echo root cause is CONFIRMED. If output is STILL missing after the echo is added, the loss is elsewhere (wrong inherited handle / child not actually writing to the pipe) and this hypothesis is wrong."
    fix_rationale: "The PTY relay already echoes to the console; the pipe relay does not. Adding the identical `stdout.write_all + flush` to the pipe-source thread addresses the exact missing step (the root cause), not a symptom. Read pattern is untouched (single merged stdout_read drain) so the CR-01 deadlock cannot reappear, and the broker HANDLE_LIST/d8b7ce00 dedup is not touched. The echo is gated behind `!is_windows_detached_launch()` so the genuinely-detached supervisor (no foreground console; output goes to log + attach client by design) keeps its current behavior — preventing a regression on the `nono run --detached` path."
    blind_spots: "(1) Live verification is operator-only (Oscar, native PowerShell, dev-layout binary) — this agent's MSYS shell would confound it; the falsification test's final arm is the operator run. (2) Need to confirm `is_windows_detached_launch()` is the correct foreground/detached discriminator at supervisor runtime (it reads NONO_DETACHED_LAUNCH, set only on the inner detached re-exec) — verified in launch.rs:2066-2076. (3) `std::io::stdout()` on Windows is line/block-buffered by the Rust std lock; the explicit `.flush()` after each chunk (mirroring the PTY path) covers prompt display. (4) The merged stream now carries child stderr onto the parent's stdout (not stderr) — acceptable + already the documented CR-01/Phase-17 merge behavior for the no-PTY path; not a regression."

## Evidence

- timestamp: 2026-05-27
  checked: Orchestrator pre-localization — `crates/nono-cli/src/exec_strategy_windows/launch.rs` BrokerLaunchNoPty arm (1631-1690, 1878-1879) + grep of the relay wiring; cross-ref resolved `broker-nopty-createproc-gle87.md` and `claude-exe-dll-init-failed.md`.
  found: The no-PTY arm sets `startup_info.hStdOutput = startup_info.hStdError = pipes.stdout_write` and `child_stdio = [stdin_read, stdout_write, stdout_write]`; the inline comment (1678-1688) states the supervisor relay `supervisor.rs start_logging` "drains ONLY stdout_read" and that stderr was merged into stdout to avoid the CR-01 relay deadlock. The relay's WRITE destination was not confirmed (function lives in supervisor.rs, not yet read).
  implication: Strong lead that the drain-without-echo is the cause, but the WRITE target of `start_logging` is UNVERIFIED. The debugger MUST read `start_logging` and confirm where the bytes go before proposing a fix. Distinct from the GLE=87 spawn bug (already resolved).

- timestamp: 2026-05-27
  checked: Production source read of `supervisor.rs::start_logging` (lines 612-744), the PTY relay `start_interactive_terminal_io` (lines 829-959), the `start_streaming` dispatch (lines 433-441), `execute_supervised` runtime wiring (mod.rs:710-827), and the foreground/detached discriminator `is_windows_detached_launch` (launch.rs:2066-2076).
  found: CONFIRMED the hypothesis with direct source evidence. `start_logging`'s pipe-source thread (the `stdout_read != 0` branch) writes the drained bytes ONLY to (1) the session log file (`log_file.write_all`, line 703) and (2) the `active_attachment` named pipe IF a client is attached (lines 719-738). There is NO write to `std::io::stdout()` / `GetStdHandle(STD_OUTPUT_HANDLE)` anywhere in the function. The PTY relay `start_interactive_terminal_io` DOES echo to console (`let mut stdout = std::io::stdout()` line 872; `stdout.write_all(&buf[..n]); stdout.flush()` lines 880-881). `start_streaming` (line 434) routes non-interactive `nono run` to `start_logging` (NOT the PTY relay). `detached_stdio` is populated for BOTH the foreground BrokerLaunchNoPty path and the genuinely-detached `NONO_DETACHED_LAUNCH=1` path, so the echo must be gated by `is_windows_detached_launch()` (true only on the inner detached re-exec) to avoid regressing the detached path.
  implication: ROOT CAUSE CONFIRMED — pure relay-echo gap, not flush/race/wrong-handle. The falsification test (read stdout in start_logging) would have found a console write if the hypothesis were wrong; there is none. Fix: add a `!is_windows_detached_launch()`-gated `std::io::stdout().write_all + flush` to the pipe-source thread, mirroring the PTY relay, with the read pattern UNCHANGED (single merged stdout_read drain → no CR-01 deadlock; HANDLE_LIST / d8b7ce00 dedup untouched).

## Eliminated

- **Claude binary / PATH fault:** ELIMINATED — `claude --version` outside nono prints `2.1.152 (Claude Code)`.
- **Process spawn / token / trust-gate failure:** ELIMINATED — broker logs `spawned Low-IL child` + `child exited child_exit_code=0`; this is purely an output-relay defect, not a spawn defect (that was `broker-nopty-createproc-gle87`, resolved).

## Resolution

- root_cause: |
    On the FOREGROUND no-PTY supervised path (WindowsTokenArm::BrokerLaunchNoPty, selected for a
    non-interactive `nono run` such as `claude --version`), the supervisor's pipe-source relay
    `crates/nono-cli/src/exec_strategy_windows/supervisor.rs::start_logging` (the `stdout_read != 0` branch)
    drains the merged child stdout/stderr and writes those bytes ONLY to (1) the session log file
    (`session_log_path`, `log_file.write_all`) and (2) the `active_attachment` named pipe IF a `nono attach`
    client is connected. It NEVER echoes the bytes to the foreground console (`std::io::stdout()`). The PTY
    relay `start_interactive_terminal_io` DOES echo to `std::io::stdout()` (lines 872, 880-881), but `nono run`
    is non-interactive so `start_streaming` (supervisor.rs:434) routes it to `start_logging`, not the PTY relay.
    Result: the child's `2.1.152 (Claude Code)` line is consumed off the pipe (so the child does not block on a
    full pipe and exits 0) but is never displayed — silent output loss, exit 0. This is a pure relay-echo gap,
    not a flush/race or wrong-handle bug; verified by direct source read (no console write exists in start_logging).

    Reconciles the Phase 52 "2.1.150 PASS" contradiction: per the sibling resolved session
    broker-nopty-createproc-gle87 (git provenance evidence), Phase 52's UAT ran a Program Files 0.57.0 build that
    predated both the CR-01 stderr-merge AND a spawn-succeeding BrokerLaunchNoPty (the GLE=87 spawn failure was
    only resolved TODAY via the broker HANDLE_LIST dedup, commit d8b7ce00). So the no-PTY foreground echo path was
    never actually exercised end-to-end at Phase 52 — version-string sameness was the trap, not proof of echo.

- fix: |
    APPLIED (nono-cli supervisor-side, minimal). In crates/nono-cli/src/exec_strategy_windows/supervisor.rs,
    `start_logging` pipe-source bridge thread:
      - Compute `let echo_to_console = !super::launch::is_windows_detached_launch();` BEFORE spawning the thread
        (foreground vs genuinely-detached discriminator; NONO_DETACHED_LAUNCH=1 is set only on the inner detached
        re-exec, where there is no foreground console and output reaches the user via the log + `nono attach`).
      - Inside the thread, create `let mut console_stdout = if echo_to_console { Some(std::io::stdout()) } else
        { None };` once before the read loop.
      - In the read loop, after the existing best-effort `log_file.write_all + flush`, add a best-effort
        `if let Some(stdout) = console_stdout.as_mut() { let _ = stdout.write_all(&buf[..n]); let _ =
        stdout.flush(); }` — mirroring the PTY relay's echo. Best-effort (no `?`) so a closed/redirected stdout
        cannot kill the bridge thread (Pitfall 1).
      - The READ pattern is UNCHANGED — still a single merged `source_handle` drain — so the CR-01 deadlock cannot
        reappear, and the broker HANDLE_LIST dedup (commit d8b7ce00) is untouched. The genuinely-detached path
        (NONO_DETACHED_LAUNCH=1) keeps its prior log-only behavior (echo gated off).
    Build/clippy/test on the Windows host: `cargo build -p nono-cli -p nono-shell-broker` clean;
    `cargo clippy -p nono-cli -p nono-shell-broker --bins -- -D warnings -D clippy::unwrap_used` clean (the
    production bins; the `--all-targets` failures are PRE-EXISTING test-file debt in offline_verify_extended_trust_bundle.rs
    + a dead-code fn in profile/mod.rs, neither touched by this fix); `cargo test -p nono-shell-broker` 20/0;
    `detached_token_gate_tests` (the discriminator this fix relies on) 3/0. `cargo build --release -p nono-cli
    -p nono-shell-broker` clean — target\release\nono.exe (2026-05-27 18:00, carries the fix) and
    nono-shell-broker.exe (18:58 rebuilt; unchanged by this fix) both postdate the change.
    Cross-target Unix clippy gate (CLAUDE.md) does NOT apply: the change is entirely within the Windows-only
    exec_strategy_windows module and references super::launch::is_windows_detached_launch() — no shared or
    Unix-cfg code touched.

- verification: |
    PASS — operator-run, native PowerShell console, dev-layout binary (target\release\nono.exe rebuilt 18:00),
    profile-covered cwd (%USERPROFILE%\.claude), 2026-05-27 22:07 + 22:24:
      1. `... run --profile claude-code --allow-cwd -- cmd /c "echo HELLO_STDOUT_PROBE"`
         → printed `HELLO_STDOUT_PROBE`, child_exit_code=0. (cmd also emitted its own benign
            "UNC paths are not supported" notice about the \\?\ cwd — unrelated to the relay; the
            echo still reached the console, which is the point.)
      2. `... run --profile claude-code --allow-cwd -- claude --version`
         → printed `2.1.152 (Claude Code)`, child_exit_code=0, no GetLastError=87, no 0xC0000142.
    Both lines now reach the foreground console — the final falsification arm passed, so the
    relay-echo-gap root cause is CONFIRMED. Self-verify on the Windows host (build + bin-clippy clean,
    broker tests 20/0, detached-gate tests 3/0) held. Fix committed; session moved to resolved/.

- files_changed:
    - crates/nono-cli/src/exec_strategy_windows/supervisor.rs   # start_logging: gated std::io::stdout() echo on the FOREGROUND no-PTY relay (read pattern unchanged; CR-01 + d8b7ce00 untouched)
