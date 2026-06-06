# Phase 59: Supervisor IPC Robustness - Research

**Researched:** 2026-06-06
**Domain:** Supervisor↔child IPC robustness (Unix AF_UNIX socketpair + Windows AIPC named-pipe); bounded read timeouts; keep-alive/re-accept
**Confidence:** HIGH (all claims grounded in fork source at file:line; no external-package research needed)

## Summary

Phase 59 hardens the supervisor's IPC event loop so a transient child disconnect does not tear down supervision, and so a slow/silent child cannot hang the supervisor's read indefinitely (REQ-IPC-01, SC1/SC2). The work splits cleanly along the platform boundary the CLAUDE.md library/CLI architecture already enforces: the Unix side wires the **already-present-but-unused** `SupervisorSocket::set_read_timeout()` and tightens the keep-alive logic in the supervisor poll loop; the Windows side **translates** the same intent onto the fork's `PIPE_WAIT` AIPC named-pipe path via a `PeekNamedPipe` poll-until-data-or-deadline loop plus a robust re-accept loop. The Windows transport is a genuinely divergent surface (named pipe, not AF_UNIX), so this is a translate-not-cherry-pick, exactly as the Phase 54 ledger dispositioned Cluster C2 as `split`.

A material finding corrects one CONTEXT.md framing assumption: **the fork's production Unix supervisor IPC does NOT use a filesystem-bound named socket** — it uses `SupervisorSocket::pair()` (an AF_UNIX `socketpair`) with the child end inherited via the `NONO_SUPERVISOR_FD` env var (`exec_strategy.rs:622`). The `bind()`/`connect()` filesystem-socket path on `socket.rs` exists but has **zero production callers** (only tests). This has two direct planning consequences: (1) upstream `be7681c` (replace fd-based IPC with named socket) is not just "unix-only reference" but is the *opposite* of the fork's chosen mechanism — the fork already preserves fd-based IPC, so there is nothing to port there; (2) upstream `4a22e94` (grant `UnixSocketCapability` for the supervisor socket) is **moot for the fork** — a `socketpair` fd is inherited across `fork()` and never traverses the filesystem, so no Landlock/Seatbelt socket-path capability grant is required. See the per-commit map below.

**Primary recommendation:** Add one constant + env override to `timeouts.rs` (`SUPERVISOR_IPC_READ_TIMEOUT` / `NONO_SUPERVISOR_IPC_READ_TIMEOUT`, 5s default, `MAX_TIMEOUT` clamp). On Unix, call `sock.set_read_timeout(Some(...))` once after the socket is wrapped, and unify the two supervisor poll loops on the already-correct `sock_fd_active` keep-alive pattern (the simpler macOS loop at `exec_strategy.rs:2329` hard-`break`s on `POLLHUP` — that is the SC1 bug to fix). On Windows, replace the unbounded `read_exact` in `socket_windows.rs::read_frame` (321-339) with a `PeekNamedPipe`-gated bounded read, and convert the capability-pipe server loop (`supervisor.rs:561-600`) from hard-break-on-error to a re-accept loop via `DisconnectNamedPipe` + `ConnectNamedPipe`.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Timeout constant + `NONO_*` env override + clamp | CLI (`nono-cli`) | — | Policy/UX knob; `timeouts.rs` already owns all timeout config (CLAUDE.md library-is-policy-free boundary) |
| `set_read_timeout()` primitive on the socket | Library (`nono`) | — | Pure transport primitive on `SupervisorSocket`; already exists (`socket.rs:192`) |
| Unix supervisor poll loop (keep-alive / re-poll) | CLI (`nono-cli`) | — | `run_supervisor_loop` in `exec_strategy.rs` is CLI-owned supervision policy |
| Windows AIPC bounded-read (`PeekNamedPipe` loop) | Library (`nono`) | — | Transport-level read framing lives in `socket_windows.rs::read_frame` |
| Windows capability-pipe re-accept loop | CLI (`nono-cli`) | Library (helper) | Loop lives in `exec_strategy_windows/supervisor.rs`; may call a new library helper for disconnect+reconnect |
| Capability grant for supervisor socket (`4a22e94`) | — (N/A) | — | Fork uses inherited socketpair fd, no filesystem socket → no grant needed |

## Standard Stack

No new external dependencies. This phase uses only crates already in the workspace.

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `libc` | (workspace) | `poll(2)`, `pollfd`, `POLLHUP`/`POLLERR`/`POLLIN` in Unix supervisor loop | Already the Unix syscall surface [VERIFIED: codebase `exec_strategy.rs:2302`] |
| `windows-sys` | 0.59 | `PeekNamedPipe`, `ConnectNamedPipe`, `DisconnectNamedPipe`, pipe error codes | Already the fork's sole Windows API binding [VERIFIED: codebase `socket_windows.rs:20-39`] |
| `std::time::{Duration,Instant}` | std | deadline math for the bounded poll loop | std; `Instant + Duration` overflow already guarded by `MAX_TIMEOUT` clamp in `timeouts.rs:103` |
| `tracing` | (workspace) | `debug!`/`warn!` on disconnect/timeout/re-accept | Established logging surface [VERIFIED: codebase] |

**Installation:** None. No `Cargo.toml` changes. (Confirm zero new deps in the plan's threat-flags — matches the C11 precedent in `55-05-SUMMARY.md` "No new Cargo deps".)

## Package Legitimacy Audit

> Not applicable — this phase installs **no** external packages. All APIs used are already-vendored workspace crates (`libc`, `windows-sys 0.59`) and `std`. No registry verification, slopcheck, or postinstall audit required.

## Architecture Patterns

### System Architecture Diagram

```
                    ┌─────────────────────────────────────────────┐
                    │            nono-cli SUPERVISOR (parent)       │
                    │                                               │
   child closes ──▶ │  run_supervisor_loop  (poll loop, 200ms tick)│
   IPC / slow read  │   ├─ Unix:   libc::poll(sock_fd, POLLIN)      │
                    │   │     ├─ POLLHUP/ERR ─▶ keep-alive?         │ ◀── SC1 fix:
                    │   │     │      • macOS loop @2329: BREAKS (bug)│     unify on
                    │   │     │      • Linux loop @2455: sock_fd_active
                    │   │     └─ POLLIN ─▶ sock.recv_message()      │     keep-alive
                    │   │            └─ read_frame (read_exact)     │ ◀── SC2 fix:
                    │   │               + set_read_timeout(5s) ◀────┼──── wire this
                    │   │                                           │
                    │   └─ Windows: cap-pipe thread @supervisor.rs  │
                    │         loop { recv_message() }               │ ◀── SC1+SC4 fix:
                    │           ├─ Ok ─▶ handle_windows_supervisor… │     PeekNamedPipe
                    │           └─ Err ─▶ BREAK (bug) ▶ re-accept    │     bounded read +
                    │                    DisconnectNamedPipe +       │     DisconnectNamed-
                    │                    ConnectNamedPipe (re-loop)  │     Pipe re-accept
                    └───────────────────────┬───────────────────────┘
                                            │ length-prefixed-JSON frame
                                            │ [4B u32 BE len][N B payload], 64KiB cap
                    ┌───────────────────────▼───────────────────────┐
                    │   SANDBOXED CHILD                              │
                    │   Unix:   inherited socketpair fd (NONO_SUPERVISOR_FD)
                    │   Windows: CreateFileW(\\.\pipe\nono-aipc-…)   │
                    └────────────────────────────────────────────────┘
```

Data flow: child issues a `SupervisorMessage` (CapabilityRequest / OpenUrl / AIPC handle request) → supervisor reads one length-prefixed frame → dispatches → replies. The robustness gap is entirely in the **read side** (unbounded blocking read + tear-down-on-disconnect). The framing protocol is identical on both platforms (`MAX_MESSAGE_SIZE = 64 KiB`, 4-byte big-endian length prefix) — confirming this is a structural translate, not a code cherry-pick.

### Pattern 1: Wire the existing `set_read_timeout` (Unix, SC2)
**What:** `SupervisorSocket::set_read_timeout(Some(Duration))` already exists at `socket.rs:192` and delegates to `UnixStream::set_read_timeout`. It currently has **no callers** (CONTEXT.md D-02 assumption VERIFIED — grep across `crates/` finds only the definition and the test-helper note in `aipc_sdk.rs:37`).
**When to use:** Call it once, immediately after the supervisor wraps its socket end (`SupervisorSocket::pair()` at `exec_strategy.rs:622`, or `from_stream` at 4067/4182), before entering `run_supervisor_loop`.
**Example:**
```rust
// In nono-cli/src/exec_strategy.rs, after constructing the supervisor `sock`:
// Source: pattern mirrors pty_proxy.rs:389 set_read_timeout usage
sock.set_read_timeout(Some(crate::timeouts::supervisor_ipc_read_timeout()))?;
```
**Note on interaction with `poll`:** the Unix loops already gate `recv_message()` behind `libc::poll(..., 200)` returning `POLLIN`, so the socket only reads when data is *ready*. The read timeout is a **slowloris/partial-frame defense**: once `read_exact` starts (4-byte length, then N-byte payload) a child that sends 1 byte then stalls would block the read indefinitely without the timeout. With `set_read_timeout`, `read_exact` returns `WouldBlock`/`TimedOut` after 5s; the planner must ensure that error is treated as a **non-fatal, keep-alive** condition on the URL/direct-IPC listener (not a hard break) — fail-closed for *that* frame, loop continues.

### Pattern 2: Unify the Unix keep-alive on `sock_fd_active` (SC1)
**What:** There are **two** `run_supervisor_loop` definitions in `exec_strategy.rs`, cfg-split:
- `#[cfg(not(target_os = "linux"))]` (macOS), `exec_strategy.rs:2284` — the **simpler, buggy** loop. At 2332-2335 it does `if pfds[0].revents & (POLLHUP|POLLERR) != 0 { break; }` and at 2351-2354 it `break`s on any `recv_message` error. This is the SC1 bug: child closing the socket kills supervision.
- `#[cfg(target_os = "linux")]` (Linux), `exec_strategy.rs:2455` — the **already-robust** loop. It carries `sock_fd_active` (declared 2478) and at 2533-2541 / 2559-2566 it *demotes* `sock_fd_active = false` (continuing for seccomp/proxy/PTY) instead of breaking when those facilities are present.
**When to use:** The macOS loop must adopt the same keep-alive semantics so that, for the **URL-open/direct-IPC listener** (D-04 scope), a child disconnect leaves the supervisor running and able to re-accept. Note the Linux loop *also* still breaks when none of seccomp/proxy/PTY is present (2537-2540, 2559-2563) — for the URL-open listener D-04 explicitly wants keep-alive even in that case. Plan must decide the keep-alive predicate precisely (see Open Question 1).
**Anti-pattern to avoid:** A blanket rewrite of *every* supervisor IPC read path. CONTEXT.md D-04 + Deferred scope this to the URL-open / direct-IPC listener (the listener upstream C2 / issue #959 targets), NOT the seccomp/proxy notify fds or the AIPC handle-brokering path.

### Pattern 3: Windows `PeekNamedPipe` bounded-read translation (SC4)
**What:** The Windows AIPC pipes are created `PIPE_WAIT` (blocking) — VERIFIED at `socket_windows.rs:767` (`bind_aipc_pipe`), `:1256`, `:1288` (control pipes). Under `PIPE_WAIT`, `ReadFile`/`read_exact` blocks until bytes arrive, so socket-style read timeouts are not honored — exactly D-03's premise. The fix is a poll-before-read loop using `PeekNamedPipe` (which is non-destructive and returns `lpBytesAvailable` without consuming) gated by a `Instant`-based deadline.
**When to use:** Inside (or wrapping) `socket_windows.rs::read_frame` (321-339), and/or a new `recv_message_bounded(deadline)` that the capability-pipe loop calls.
**Example (design sketch — planner refines poll interval):**
```rust
// Source: design per CONTEXT.md D-03; PeekNamedPipe signature from
// windows-sys Win32::System::Pipes. Non-destructive availability probe.
fn read_exact_bounded(reader: &File, buf: &mut [u8], deadline: Instant) -> Result<()> {
    let handle = reader.as_raw_handle() as HANDLE;
    let mut filled = 0usize;
    while filled < buf.len() {
        let mut avail: u32 = 0;
        // SAFETY: handle is a live pipe handle; out-params are stack locals.
        let ok = unsafe {
            PeekNamedPipe(handle, std::ptr::null_mut(), 0,
                          std::ptr::null_mut(), &mut avail, std::ptr::null_mut())
        };
        if ok == 0 {
            // ERROR_BROKEN_PIPE (109) / ERROR_PIPE_NOT_CONNECTED (233) => peer gone
            return Err(/* classify: disconnect, surface to re-accept */);
        }
        if avail == 0 {
            if Instant::now() >= deadline {
                return Err(/* bounded timeout, non-fatal for keep-alive */);
            }
            std::thread::sleep(POLL_INTERVAL); // e.g. 10ms (Claude's discretion, D-03)
            continue;
        }
        // bytes are ready; a blocking ReadFile of min(avail, remaining) won't hang
        let want = (avail as usize).min(buf.len() - filled);
        let n = read_some(handle, &mut buf[filled..filled + want])?;
        filled += n;
    }
    Ok(())
}
```
**Watchdog-cancel (D-03):** `PeekNamedPipe` itself does not block, so the deadline check inside the loop *is* the watchdog — no `CancelIoEx` needed for the poll path. `CancelIoEx`/overlapped I/O is the explicitly-deferred heavyweight alternative (Deferred Ideas). The one residual blocking call is the post-peek `ReadFile` of `min(avail, remaining)` bytes; because `avail > 0` was just observed, that read completes promptly (the kernel buffer already holds the bytes). This is the conservative-by-design tradeoff CONTEXT.md mandates.

### Pattern 4: Windows re-accept loop (SC1)
**What:** The capability-pipe server loop (`exec_strategy_windows/supervisor.rs:561-600`) currently does `match sock.recv_message() { Ok(..) => …, Err(..) => break }` — VERIFIED at 591-598 it logs "Capability pipe closed" and **breaks the thread**, permanently disabling capability expansion for the rest of the session after a single transient close. The translation of `51f56b8` keep-alive is: on a disconnect-class error, `DisconnectNamedPipe(server_handle)` then `ConnectNamedPipe(server_handle, …)` (re-arm the same `PIPE_UNLIMITED_INSTANCES` server) and continue the loop, bounded by `terminate_requested`.
**Win32 error codes to classify (planner MUST handle):**
- `ERROR_BROKEN_PIPE` (109) — peer closed; re-accept.
- `ERROR_PIPE_NOT_CONNECTED` (233) — already disconnected; re-arm.
- `ERROR_PIPE_CONNECTED` (535) — `ConnectNamedPipe` returns 0 but client connected between `CreateNamedPipe` and `ConnectNamedPipe`; **treat as success** (the existing `finalize_server_connection` at 1313 already handles this exact case — reuse that idiom).
- `ERROR_NO_DATA` (232) — pipe being closed on the write side; treat as disconnect.
**Note:** `bind_aipc_pipe` already uses `PIPE_UNLIMITED_INSTANCES` (`socket_windows.rs:768`), so re-accepting a fresh instance is structurally supported. The control pipes (`create_low_integrity_named_pipe:1257`, `create_named_pipe:1289`) use `1` instance — re-accept on *those* must `DisconnectNamedPipe`+`ConnectNamedPipe` the same handle rather than create a new instance.

### Anti-Patterns to Avoid
- **Blanket robustness rewrite** of all IPC read paths (control + AIPC + URL-open). Scope to the URL-open/direct-IPC listener (D-04, Deferred).
- **Overlapped/async I/O rewrite** on Windows — explicitly rejected (Deferred Ideas) as too large/risky for this phase.
- **Bare timeout literal.** Must go through `timeouts.rs` with `NONO_*` override + `MAX_TIMEOUT` clamp (D-01).
- **Per-run CLI flag** for the timeout — D-01 says internal knob, env override only.
- **Treating a 5s read timeout as a hard error** that kills supervision — it must be a keep-alive condition for the URL listener.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Env-var timeout parsing + clamp | A new ad-hoc `std::env::var` parse | `timeouts.rs::env_duration_secs` + a new `supervisor_ipc_read_timeout()` fn | Already handles `MAX_TIMEOUT=3600s` overflow clamp + warn-on-unparseable (`timeouts.rs:105-127`) [VERIFIED] |
| Unix socket read timeout | `select`/`poll`-with-timeout wrapper around `read` | `SupervisorSocket::set_read_timeout` (`socket.rs:192`) → `UnixStream::set_read_timeout` | Kernel-level `SO_RCVTIMEO`; already implemented and `#[must_use]`-style error-propagating |
| Windows "client connected during accept" race | Custom GLE checks | Reuse `finalize_server_connection`'s `ERROR_PIPE_CONNECTED`-is-success idiom (`socket_windows.rs:1313`) | Established, tested pattern in the same file |
| Pipe non-destructive availability probe | A 1-byte peek-read + pushback buffer | `PeekNamedPipe` (`lpBytesAvailable` out-param) | Purpose-built Win32 API, non-consuming, already in `windows-sys` |

**Key insight:** Nearly every primitive this phase needs already exists in the fork — the work is *wiring and loop-control*, not new mechanism. The single genuinely-new code is the Windows `PeekNamedPipe` bounded-read helper.

## Runtime State Inventory

> This is a code-only robustness change (loop control + a timeout constant). No stored data, no service config, no OS-registered state, no secrets, no build artifacts are renamed or migrated.

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | None — no datastore keys/collections touched. Verified: change is confined to in-memory poll loops + one const. | none |
| Live service config | None — `nono-wfp-service` SDDL/registration untouched; this is supervisor↔child IPC, not the WFP control pipe. | none |
| OS-registered state | None — no Task Scheduler / service / pm2 names. | none |
| Secrets/env vars | New env var `NONO_SUPERVISOR_IPC_READ_TIMEOUT` is **read-only config**, not a secret; the existing `NONO_SUPERVISOR_FD` (child fd inheritance) is untouched. | document the new var in `docs/cli/usage/flags.mdx` (mirror the 55-05 precedent) |
| Build artifacts | None — no package renames, no `Cargo.toml` version changes. | none |

## Per-Commit Map (Cluster C2 → concrete fork change sites)

> Authoritative scope: `54-DIVERGENCE-LEDGER.md` §"Cluster C2" (disposition `split`, 9 commits). Mapping each absorbed commit's *intent* to a fork code site.

| Upstream SHA | Intent | Fork disposition | Concrete change site |
|--------------|--------|------------------|----------------------|
| `51f56b8` | keep supervisor loop alive when child closes direct IPC socket | **ABSORB (core)** | Unix: fix macOS loop `break` at `exec_strategy.rs:2334`; Windows: re-accept in `supervisor.rs:597` |
| `9820a2e` | include URL listener in supervisor loop keep-alive conditions | **ABSORB** | Unix keep-alive predicate must cover the URL-open/direct-IPC listener (the `from_stream` path at `exec_strategy.rs:4067/4182` + open-url helper) |
| `284ae1d` | add read timeout on accepted listener connections | **ABSORB (core, SC2)** | Unix: call `set_read_timeout(socket.rs:192)`; Windows: `PeekNamedPipe` deadline loop |
| `d1851c9` | increase supervisor listener read timeout to 5s | **ABSORB (value)** | `timeouts.rs`: `SUPERVISOR_IPC_READ_TIMEOUT = Duration::from_secs(5)` (D-01) |
| `f956fb6` | set accepted listener connections to blocking mode | **ABSORB / verify** | Unix `socketpair` ends are already blocking by default; the *interaction* is: after `set_read_timeout`, a re-accepted/keep-alive connection should be returned to blocking for normal frames. Windows pipes are already `PIPE_WAIT` (blocking). Mostly a no-op-verify on the fork; document. |
| `c15c76a` | review-comment fixes on supervisor socket IPC | **ABSORB (polish)** | Apply equivalent hygiene to the fork's loop edits (error classification, logging). Inspect `git show c15c76a` for the specific lints when implementing. |
| `4a22e94` | grant `UnixSocketCapability` for supervisor socket in child sandbox | **N/A for fork** | Fork uses inherited `socketpair` fd (`NONO_SUPERVISOR_FD`), not a filesystem-bound named socket → no path capability grant needed. Document the divergence in the SUMMARY (translate-not-cherry-pick rationale). |
| `be7681c` | replace fd-based IPC with named socket for URL-open helpers (#959) | **NOT a target (reference only)** | This is the unix-only *mechanism switch* the fork deliberately does NOT adopt — the fork keeps fd-based socketpair IPC. Nothing to port. (CONTEXT.md line 53.) |
| `ed47520` | style: format debug message for line length | **trivial / optional** | Cosmetic; absorb only if a touched debug line happens to align. windows-touch:no. |

## Common Pitfalls

### Pitfall 1: Read timeout converting a healthy idle child into a kill
**What goes wrong:** Wiring `set_read_timeout(5s)` and then treating the resulting `WouldBlock`/`TimedOut` from `read_exact` as a fatal break would kill supervision of a child that simply isn't sending IPC for >5s (the common case — most children send IPC rarely).
**Why it happens:** The Unix loops already gate `recv_message` behind `poll(POLLIN)`, so a full-frame read only starts when data is ready — but a *partial* frame (length prefix arrives, payload stalls) hits the timeout. The timeout must mean "abandon this frame, keep supervising," not "child is dead."
**How to avoid:** On the URL/direct-IPC listener, classify `TimedOut`/`WouldBlock` as keep-alive; only `POLLHUP`/`POLLERR` (genuine disconnect) triggers re-accept. Add an explicit test (Validation §) for slow-partial-frame → timeout-fires → supervisor survives.
**Warning signs:** Interactive sessions dying after idle periods; CI flake in the timeout test.

### Pitfall 2: Windows `PeekNamedPipe` busy-spin
**What goes wrong:** A tight `PeekNamedPipe` loop with no sleep pegs a CPU core while waiting for a quiet child.
**How to avoid:** `std::thread::sleep(POLL_INTERVAL)` (10ms suggested; Claude's discretion per D-03) between peeks. Bound total wait by the deadline.
**Warning signs:** Supervisor thread at 100% CPU during idle.

### Pitfall 3: Re-accepting the wrong instance count
**What goes wrong:** Calling `CreateNamedPipeW` again to re-accept on a `1`-instance control pipe (`create_named_pipe:1289`) fails or leaks; the AIPC pipe (`PIPE_UNLIMITED_INSTANCES`) tolerates it.
**How to avoid:** For re-accept, reuse the *existing* server handle: `DisconnectNamedPipe(h)` then `ConnectNamedPipe(h, NULL)`. Do not create a fresh instance for control pipes.
**Warning signs:** `ERROR_PIPE_BUSY` / handle leaks on reconnect.

### Pitfall 4: Cross-target clippy blind spot on the Unix edits
**What goes wrong:** `socket.rs` and the Linux/macOS `run_supervisor_loop` arms are cfg-gated Unix code. Windows-host `cargo check`/clippy will NOT exercise them (CLAUDE.md MUST/NEVER + `feedback_clippy_cross_target`).
**How to avoid:** Mark the cross-target clippy gate **PARTIAL/CI-deferred** per `.planning/templates/cross-target-verify-checklist.md` (the C cross-linker `ring`-build blocker is known on this host, per 55-05-SUMMARY). Files in scope: `crates/nono/src/supervisor/socket.rs`, `crates/nono-cli/src/exec_strategy.rs` (both loop arms). No merge to main until live CI (ubuntu/macos runners) passes `-D warnings -D clippy::unwrap_used`.
**Warning signs:** A `.unwrap()`/`.expect()` slipping into a Unix arm undetected on the Windows host.

### Pitfall 5: Pre-existing baseline test failures masking regressions
**What goes wrong:** Treating `try_set_mandatory_label_…` (nono lib) and the 4 `nono-cli` profile/protected-paths failures as Phase-59 regressions.
**How to avoid:** These are documented environmental baseline failures (`nono_cli_windows_baseline_test_failures` memory; 55-05-SUMMARY D-55-E4). Use baseline-aware comparison (red→red carry-forward is acceptable). Also note `helper_stamps_session_token_from_env` is a known parallel env-var race (passes in isolation).

## Code Examples

### Add the timeout constant + accessor (timeouts.rs)
```rust
// crates/nono-cli/src/timeouts.rs — new entries (mirror DETACH_STARTUP_TIMEOUT pattern at :39/:74)
/// Bounded read timeout for the supervisor IPC listener (URL-open / direct
/// IPC). Matches upstream d1851c9 (5s). Defends against a slow/silent child
/// holding a partial frame.
pub const SUPERVISOR_IPC_READ_TIMEOUT: Duration = Duration::from_secs(5);

/// Read `NONO_SUPERVISOR_IPC_READ_TIMEOUT` (seconds), clamped to MAX_TIMEOUT.
pub fn supervisor_ipc_read_timeout() -> Duration {
    env_duration_secs("NONO_SUPERVISOR_IPC_READ_TIMEOUT", SUPERVISOR_IPC_READ_TIMEOUT)
}
```
> Note: `env_duration_secs` (`timeouts.rs:105`) is NOT `#[cfg(unix)]`, so the secs-accessor compiles on Windows too — Windows reads the same constant value (D-02). Keep the new const **un-cfg'd** so both platforms share it (unlike the `#[cfg(unix)]` poll-interval consts).

### Unix wiring point
```rust
// crates/nono-cli/src/exec_strategy.rs — after wrapping the supervisor socket end,
// before run_supervisor_loop. Propagate the error with ? (NonoError; no unwrap).
sock.set_read_timeout(Some(crate::timeouts::supervisor_ipc_read_timeout()))?;
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Hard-`break` on child IPC close (macOS loop) | Keep-alive + re-accept on the URL/direct listener | This phase | Supervisor survives transient disconnects (SC1) |
| Unbounded `read_exact` on the IPC frame | Bounded read (Unix `SO_RCVTIMEO`; Windows `PeekNamedPipe` deadline) | This phase | Slowloris/hang defense (SC2) |
| Upstream: fd-based → named socket (`be7681c`) | Fork keeps fd-based socketpair IPC | n/a (fork divergence) | `be7681c`/`4a22e94` are not fork targets |

**Deprecated/outdated:**
- The framing of "absorb upstream's named-socket *mechanism*" — corrected: the fork preserves fd-based IPC; only the keep-alive/timeout/blocking-mode *hardening intent* is absorbed.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | 10ms is a reasonable `PeekNamedPipe` poll interval | Pattern 3 | Too short = CPU spin; too long = latency. Mitigated: Claude's-discretion per D-03; tune in plan. |
| A2 | The post-peek `ReadFile(min(avail,remaining))` never blocks meaningfully because `avail>0` was just observed | Pattern 3 | If the kernel races a partial drain, a short blocking read could occur; acceptable (bytes were present). Verify under live-repro. |
| A3 | `f956fb6` (blocking-mode) is effectively a no-op-verify on the fork (socketpair ends already blocking; pipes already `PIPE_WAIT`) | Per-Commit Map | If `set_read_timeout` leaves the socket in a non-blocking-for-normal-frames state, normal reads could spuriously `WouldBlock`. Verify the timeout semantics in the disconnect/reconnect test. |
| A4 | `c15c76a` review fixes are cosmetic/error-classification only | Per-Commit Map | Could contain a substantive fix. Plan should `git show c15c76a` before finalizing. |
| A5 | Re-accept should be an unbounded loop bounded only by `terminate_requested` (vs. a retry cap) | Open Q1 | A malicious/looping child could thrash reconnect. Mitigation: fail-secure — bound by child liveness (`waitpid`/process-exit) which the loop already checks. |

## Open Questions

1. **Exact keep-alive predicate for the Unix URL-open/direct-IPC listener.**
   - What we know: The Linux loop demotes `sock_fd_active=false` (continue) only when seccomp/proxy/PTY is present (`exec_strategy.rs:2534`), else breaks. D-04 wants keep-alive for the URL listener even without those facilities.
   - What's unclear: Whether the keep-alive should be unconditional for the URL/direct listener, or gated on "is this the open-url helper path vs. the capability-elevation path."
   - Recommendation: Scope keep-alive to the listener that serves URL-open/direct IPC (the `from_stream` paths and the open-url helper rendezvous), leaving the capability-elevation + seccomp/proxy paths' existing break-conditions intact. Confirm the exact call sites with the planner by tracing `run_supervisor_loop` callers (`execute_supervised:552`, the test at 4067/4182).

2. **Whether re-accept is bounded-retry or unbounded (D-04 Claude's discretion).**
   - Recommendation: unbounded but bounded by child liveness + `terminate_requested` (fail-secure). The loop already calls `waitpid(WNOHANG)` (Unix, 2391) and checks `terminate_requested` (Windows, 562); reconnect cannot outlive the child.

3. **Does the open-url helper actually re-connect, or is it one-shot?**
   - What we know: `open_url_runtime.rs:28` wraps the inherited fd via `from_stream` and sends one request. The shim (`create_open_shim:3216`) `exec`s a fresh helper per URL.
   - What's unclear: Whether multiple browser-open events in one session each spawn a fresh helper (each its own connect) — if so, the supervisor *must* keep its listener alive across helper exits (this is precisely `9820a2e`'s intent).
   - Recommendation: Confirm by reading the macOS open-shim flow; this is the concrete scenario the disconnect/reconnect test should model.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain (host) | build/test on Windows dev host | ✓ | 1.77+ (Edition 2021) | — |
| `x86_64-unknown-linux-gnu` target | cross-target clippy (Unix arms) | ✓ (target) / ✗ (C cross-linker) | — | PARTIAL: defer to live CI per checklist |
| `x86_64-apple-darwin` target | cross-target clippy (macOS arm) | ✓ (target) / ✗ (C cross-linker) | — | PARTIAL: defer to live CI |
| Real Win11 console | Windows live-repro (D-05) | ✓ (operator UAT) | build 26200 | — (supervised runs need a real console, not git-bash; `feedback_windows_supervised_needs_real_console`) |

**Missing dependencies with no fallback:** None blocking.
**Missing dependencies with fallback:** Cross-target C linker absent on this Windows host → cross-target clippy is PARTIAL/CI-deferred (documented, expected, matches every recent phase).

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test runner (`#[test]`), `tempfile`, `proptest` (cli) |
| Config file | none — Cargo-native; integration tests under `crates/nono-cli/tests/` |
| Quick run command | `cargo test -p nono --lib supervisor::socket` (Unix arms) |
| Full suite command | `make test` (or `cargo test --workspace`) |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| REQ-IPC-01 (SC1) | child closes IPC then reconnects → supervisor survives & re-accepts | integration | `cargo test -p nono-cli --test supervisor_ipc_robustness reconnect_survival` | ❌ Wave 0 |
| REQ-IPC-01 (SC2) | slow/silent child holding open connection → bounded read timeout fires, supervisor not blocked | integration | `cargo test -p nono-cli --test supervisor_ipc_robustness bounded_read_timeout` | ❌ Wave 0 |
| REQ-IPC-01 (timeouts.rs) | env override + MAX_TIMEOUT clamp for the new const | unit | `cargo test -p nono-cli timeouts::` | ⚠️ extend existing |
| REQ-IPC-01 (SC4) | Windows AIPC bounded-read + re-accept (named-pipe timing) | live-repro | documented Windows UAT (operator, real console) | ❌ Wave 0 (UAT doc) |
| REQ-IPC-01 | existing round-trip still green (regression) | integration | `cargo test -p nono-cli --test aipc_handle_brokering_integration` | ✅ exists |

### Sampling Rate
- **Per task commit:** `cargo test -p nono --lib supervisor::socket` + `cargo build --workspace`
- **Per wave merge:** `make test` (baseline-aware: 5 documented env failures are red→red carry-forwards)
- **Phase gate:** full suite green (modulo documented baseline) + the Windows live-repro signed off, before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `crates/nono-cli/tests/supervisor_ipc_robustness.rs` — net-new integration tests for SC1 (reconnect survival) and SC2 (bounded timeout). The existing fork test at `exec_strategy.rs:4057-4067` ("fork a child that closes its socket and exits, supervisor sees POLLHUP and returns") is the **opposite** of the new desired behavior — it asserts the *old* break-on-close; that test will need updating to assert keep-alive/re-accept for the URL listener path.
- [ ] Extend `timeouts.rs` unit tests to cover `NONO_SUPERVISOR_IPC_READ_TIMEOUT` parse/clamp (save/restore env per CLAUDE.md env-var test rule).
- [ ] Windows live-repro doc: a slow-child + a disconnect-then-reconnect script run from a real Win11 console (cross-platform CI cannot deterministically exercise named-pipe timing — D-05).
- [ ] CI cross-platform note: the SC1/SC2 integration tests must be cfg-structured so they *compile* on all 3 platforms (Unix uses socketpair fork; Windows uses the AIPC pipe). Mirror the `aipc_handle_brokering_integration.rs` `#![cfg(target_os="windows")]` / empty-binary-elsewhere pattern, or write a Unix-gated test + a Windows-gated test in one file.

*Note on Nyquist sampling: the timeout (5s) and the 200ms poll tick set the temporal resolution. A reconnect test must wait > one poll tick (200ms) to deterministically observe re-accept; a timeout test must hold a partial frame > 5s (or override `NONO_SUPERVISOR_IPC_READ_TIMEOUT` to a small value to keep CI fast — recommend the test set it to e.g. 1s with save/restore).*

## Security Domain

> `security_enforcement` is absent from config.json (= enabled). This is a security-critical sandbox (CLAUDE.md). Threat surface is the supervisor↔child trust boundary.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | yes | Peer cred check already present: `peer_credentials`/`SO_PEERCRED` (Unix `socket.rs:380`); Windows session-token + restricting-SID DACL on the pipe (`socket_windows.rs`). Phase 59 must NOT weaken these. |
| V3 Session Management | yes | Re-accept must re-verify peer identity on reconnect (not blindly trust a re-connecting client). Windows: the per-session SID/token gate already runs at connect. |
| V4 Access Control | yes | The capability-grant / handle-brokering authorization is unchanged; keep-alive must not bypass `seen_request_ids` replay protection (`exec_strategy.rs:2345`, `supervisor.rs:560`). |
| V5 Input Validation | yes | `MAX_MESSAGE_SIZE = 64 KiB` frame cap already enforced (both platforms); the bounded read is itself a V5 control (partial-frame DoS). |
| V6 Cryptography | no | No crypto in this path. |

### Known Threat Patterns for supervisor IPC

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Slowloris / partial-frame hang (child sends length prefix, stalls) | Denial of Service | Bounded read timeout (this phase, SC2) + existing 64 KiB cap |
| Reconnect thrash (child loops connect/disconnect) | Denial of Service | Re-accept bounded by child liveness (`waitpid`/`terminate_requested`); fail-secure |
| Reconnect impersonation (different principal connects after re-accept) | Spoofing | Re-verify peer creds / session SID on every accept; do NOT cache trust across reconnect (V3) |
| Timeout-induced fail-open (treating timeout as "allow" / dropping enforcement) | Elevation / Tampering | Fail-closed: a timed-out frame is abandoned, supervision continues; no capability is granted on a partial/timed-out request |
| Keep-alive leaking the seccomp/proxy break-conditions | Tampering | Scope keep-alive to the URL/direct listener ONLY; preserve existing seccomp/proxy/PTY demotion logic (D-04, Anti-pattern) |

**Fail-secure invariant (CLAUDE.md):** On any IPC error (timeout, disconnect, malformed frame), deny the in-flight request and keep the sandbox intact. Never widen capabilities or drop enforcement because the IPC channel hiccuped.

## Sources

### Primary (HIGH confidence)
- Fork source (read this session):
  - `crates/nono/src/supervisor/socket.rs` (Unix socketpair, `set_read_timeout:192`, `bind:79`, framing 218-236)
  - `crates/nono/src/supervisor/socket_windows.rs` (`read_frame:321`, `bind_aipc_pipe:748` `PIPE_WAIT:767`, `finalize_server_connection:1307`, error consts 20-39, `PIPE_CONNECT_TIMEOUT_MS:106`)
  - `crates/nono-cli/src/exec_strategy.rs` (macOS loop `2284`/buggy break `2332-2354`; Linux loop `2455`/`sock_fd_active` `2478,2533-2566`; `pair():622`; `from_stream` `4067/4182`; close-on-exit test `4057`)
  - `crates/nono-cli/src/exec_strategy_windows/supervisor.rs` (cap-pipe loop `561-600`, break-on-close `591-598`, `bind_low_integrity_with_session_sid:511`)
  - `crates/nono-cli/src/timeouts.rs` (full module; `env_duration_secs:105`, `MAX_TIMEOUT:103`)
  - `crates/nono-cli/tests/aipc_handle_brokering_integration.rs` (existing round-trip test shape)
  - `crates/nono/src/capability.rs` (`UnixSocketCapability` model)
- `.planning/phases/54-upst7-audit/54-DIVERGENCE-LEDGER.md` §"Cluster C2" + §"ADR review" + §"Cross-cluster" (authoritative scope; `split` disposition)
- `.planning/phases/55-upst7-cherry-pick-wave/55-05-SUMMARY.md` (timeouts.rs convention, MAX_TIMEOUT clamp, cross-target PARTIAL precedent)
- `.planning/phases/59-supervisor-ipc-robustness/59-CONTEXT.md` (locked decisions D-01..D-05)
- `.planning/REQUIREMENTS.md` (REQ-IPC-01, line 43)
- `./CLAUDE.md` (library/CLI boundary, fail-secure, unwrap policy, cross-target clippy MUST)

### Secondary (MEDIUM confidence)
- Project memory: `nono_cli_windows_baseline_test_failures`, `feedback_clippy_cross_target`, `feedback_windows_supervised_needs_real_console` (baseline test + cross-target + console-UAT context)

### Tertiary (LOW confidence)
- None. No WebSearch was required; the phase is fully grounded in fork source + locked context.

## Project Constraints (from CLAUDE.md)

- **Library is policy-free:** the timeout *value/override* belongs in CLI `timeouts.rs`; only the `set_read_timeout` primitive + Windows `read_frame` bounded-read live in the `nono` library.
- **No `.unwrap()`/`.expect()`** (clippy::unwrap_used `-D`): propagate via `?`/`NonoError`. Applies to all new code; test modules may `#[allow(clippy::unwrap_used)]`.
- **Fail-secure:** IPC error → deny in-flight request, keep sandbox intact; never widen caps or drop enforcement on a hiccup.
- **Cross-target clippy MUST:** `socket.rs` + the Unix `run_supervisor_loop` arms are cfg-gated Unix code → verify on `x86_64-unknown-linux-gnu` + `x86_64-apple-darwin`, or mark PARTIAL/CI-deferred per `.planning/templates/cross-target-verify-checklist.md`. Windows-host `cargo check` is NOT a substitute.
- **Env-var tests** must save/restore `NONO_SUPERVISOR_IPC_READ_TIMEOUT` (parallel-test env-var rule).
- **Arithmetic:** deadline math reuses the `MAX_TIMEOUT` clamp (already guards `Instant + Duration` overflow).
- **DCO sign-off** on every commit.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — no new deps; all APIs verified in-tree.
- Architecture/scope: HIGH — both supervisor loops, both `read_frame`s, pipe flags, and the keep-alive divergence read directly from source at file:line.
- Per-commit map: MEDIUM-HIGH — dispositions from the locked ledger; `c15c76a` exact diff not inspected (flagged A4).
- Windows `PeekNamedPipe` design: MEDIUM — design is sound and conservative per D-03, but poll interval and the post-peek read race need live-repro confirmation (A1, A2).

**Research date:** 2026-06-06
**Valid until:** ~2026-07-06 (stable; the only volatility is the unread `c15c76a` diff and live Windows named-pipe timing).
