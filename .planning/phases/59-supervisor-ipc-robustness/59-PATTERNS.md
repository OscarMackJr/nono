# Phase 59: Supervisor IPC Robustness - Pattern Map

**Mapped:** 2026-06-06
**Files analyzed:** 6 (5 modified, 1 net-new)
**Analogs found:** 6 / 6

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `crates/nono-cli/src/timeouts.rs` (modify: add const + accessor) | config | transform (env→Duration) | same file — `DETACH_STARTUP_TIMEOUT` + `detach_startup_timeout()` (lines 39, 74) | exact (in-file precedent) |
| `crates/nono/src/supervisor/socket.rs` (modify: wire `set_read_timeout`, keep-alive hook) | service (library transport primitive) | request-response / streaming | self — `set_read_timeout` (line 192) already exists, `read_frame` (218) | exact |
| `crates/nono-cli/src/exec_strategy.rs` (modify: macOS loop keep-alive + read-timeout wiring) | service (supervision policy loop) | event-driven (poll loop) | Linux `run_supervisor_loop` `sock_fd_active` (line 2455) vs buggy macOS loop (line 2284) | exact (sibling cfg arm) |
| `crates/nono/src/supervisor/socket_windows.rs` (modify: `PeekNamedPipe` bounded `read_frame`) | service (library transport primitive) | streaming (named-pipe read) | self — `read_frame` (321), `finalize_server_connection` ERROR_PIPE_CONNECTED idiom (1307) | exact / role-match |
| `crates/nono-cli/src/exec_strategy_windows/supervisor.rs` (modify: re-accept loop) | service (supervision policy loop) | event-driven (recv loop) | Linux `sock_fd_active` keep-alive (`exec_strategy.rs:2455`) — translated to pipe re-accept | role-match (cross-platform translate) |
| `crates/nono-cli/tests/supervisor_ipc_robustness.rs` (net-new) | test | event-driven (integration) | `crates/nono-cli/tests/aipc_handle_brokering_integration.rs` (cfg structure) + `exec_strategy.rs:4057` close-on-exit test (Unix fork shape) | exact (structure) |

## Pattern Assignments

### `crates/nono-cli/src/timeouts.rs` (config, env→Duration transform)

**Analog:** Same file — the `DETACH_STARTUP_TIMEOUT` const + `detach_startup_timeout()` accessor pair. This is the Phase 55 (55-05) convention D-01 mandates. Reuse `env_duration_secs` (line 105) verbatim — it already handles the `MAX_TIMEOUT` (3600s) clamp + warn-on-unparseable.

**Const declaration pattern** (timeouts.rs:37-39) — note: **do NOT cfg-gate** the new const (Windows reads the same value per D-02; `env_duration_secs` itself is not `#[cfg(unix)]`):
```rust
/// Maximum time to wait for a detached session to create its session file
/// and attach socket.
pub const DETACH_STARTUP_TIMEOUT: Duration = Duration::from_secs(30);
```

**Accessor pattern** (timeouts.rs:72-76) — the new accessor mirrors this exactly:
```rust
/// Read `NONO_DETACH_STARTUP_TIMEOUT` (seconds). Returns the default when
/// the variable is absent or unparseable.
pub fn detach_startup_timeout() -> Duration {
    env_duration_secs("NONO_DETACH_STARTUP_TIMEOUT", DETACH_STARTUP_TIMEOUT)
}
```

**Clamp helper to reuse as-is** (timeouts.rs:105-127) — no changes needed; the new accessor calls it:
```rust
fn env_duration_secs(var: &str, default: Duration) -> Duration {
    match std::env::var(var) {
        Ok(val) => match val.parse::<u64>() {
            Ok(secs) => {
                let d = Duration::from_secs(secs);
                if d > MAX_TIMEOUT {
                    warn!("{var}={val} exceeds maximum ({} s), clamping", MAX_TIMEOUT.as_secs());
                    MAX_TIMEOUT
                } else { d }
            }
            Err(_) => {
                warn!("{var}={val:?} is not a valid number of seconds, using default");
                default
            }
        },
        Err(_) => default,
    }
}
```

**New code to add** (from RESEARCH.md, D-01; 5s default = upstream `d1851c9`):
```rust
/// Bounded read timeout for the supervisor IPC listener (URL-open / direct
/// IPC). Matches upstream d1851c9 (5s). Defends against a slow/silent child
/// holding a partial frame.
pub const SUPERVISOR_IPC_READ_TIMEOUT: Duration = Duration::from_secs(5);

/// Read `NONO_SUPERVISOR_IPC_READ_TIMEOUT` (seconds), clamped to MAX_TIMEOUT.
pub fn supervisor_ipc_read_timeout() -> Duration {
    env_duration_secs("NONO_SUPERVISOR_IPC_READ_TIMEOUT", SUPERVISOR_IPC_READ_TIMEOUT)
}
```

---

### `crates/nono/src/supervisor/socket.rs` (library transport primitive, Unix)

**Analog:** Self. The primitive D-02 needs already exists and has zero callers — the work is wiring, not new mechanism.

**Existing `set_read_timeout` to wire** (socket.rs:191-196) — already error-propagating via `?`/`NonoError`, no `unwrap`:
```rust
/// Set a read timeout on the socket.
pub fn set_read_timeout(&self, timeout: Option<std::time::Duration>) -> Result<()> {
    self.stream
        .set_read_timeout(timeout)
        .map_err(|e| NonoError::SandboxInit(format!("Failed to set socket read timeout: {e}")))
}
```

**Frame protocol the timeout protects** (socket.rs:218-236) — the two `read_exact` calls (length prefix, then payload) are exactly the slowloris surface; `set_read_timeout` makes a stalled partial frame return `WouldBlock`/`TimedOut` instead of blocking forever. `MAX_MESSAGE_SIZE` (64 KiB) cap already enforced:
```rust
fn read_frame(&mut self) -> Result<Vec<u8>> {
    let mut len_bytes = [0u8; LENGTH_PREFIX_SIZE];
    self.stream.read_exact(&mut len_bytes)
        .map_err(|e| NonoError::SandboxInit(format!("Failed to read message length: {e}")))?;
    let len = u32::from_be_bytes(len_bytes);
    if len > MAX_MESSAGE_SIZE { /* reject oversized */ }
    let mut payload = vec![0u8; len as usize];
    self.stream.read_exact(&mut payload)
        .map_err(|e| NonoError::SandboxInit(format!("Failed to read message payload: {e}")))?;
    Ok(payload)
}
```

**Note for planner:** `set_read_timeout` is called from the CLI (`exec_strategy.rs`) after wrapping the socket, NOT inside the library — the *value* is CLI policy (`timeouts.rs`), the primitive is library. Keeps the CLAUDE.md library-is-policy-free boundary intact.

---

### `crates/nono-cli/src/exec_strategy.rs` (supervision policy loop, macOS arm)

**Analog:** The **Linux sibling arm** in the same file — `run_supervisor_loop` at line 2455. The macOS arm (line 2284) is the SC1 bug; it must adopt the Linux `sock_fd_active` keep-alive pattern.

**BUGGY macOS pattern to fix** (exec_strategy.rs:2332-2354) — hard-`break` on POLLHUP and on any recv error:
```rust
if pfds[0].revents & (libc::POLLHUP | libc::POLLERR) != 0 {
    debug!("Supervisor socket closed by child");
    break;                                    // ◀── SC1 bug: kills supervision
}
if pfds[0].revents & libc::POLLIN != 0 {
    match sock.recv_message() {
        Ok(msg) => { /* handle */ }
        Err(e) => {
            debug!("Error receiving supervisor message: {}", e);
            break;                            // ◀── SC1 bug + SC2: timeout becomes a kill
        }
    }
}
```

**CORRECT pattern to mirror — Linux `sock_fd_active` keep-alive** (exec_strategy.rs:2478, 2533-2566) — demote the fd to inactive (`-1` in pollfd) instead of breaking when keep-alive facilities are present; the macOS arm needs the equivalent for the URL-open/direct-IPC listener (D-04):
```rust
let mut sock_fd_active = true;                // line 2478
// ...
if sock_fd_active && pfds[0].revents & (libc::POLLHUP | libc::POLLERR) != 0 {
    if notify_raw_fd.is_some() || proxy_notify_raw_fd.is_some() || pty.is_some() {
        debug!("Supervisor socket closed, continuing for seccomp/proxy/PTY");
        sock_fd_active = false;               // ◀── keep-alive: demote, don't break
    } else {
        debug!("Supervisor socket closed by child");
        break;
    }
}
if sock_fd_active && pfds[0].revents & libc::POLLIN != 0 {
    match sock.recv_message() {
        Ok(msg) => { /* handle_supervisor_message(...) */ }
        Err(e) => {
            debug!("Error receiving supervisor message: {}", e);
            if notify_raw_fd.is_none() && proxy_notify_raw_fd.is_none() && pty.is_none() {
                break;
            }
            sock_fd_active = false;           // ◀── keep-alive on recv error too
        }
    }
}
```
The demoted fd is fed back as `-1` into pollfd so `poll` ignores it (line 2483: `fd: if sock_fd_active { sock_fd } else { -1 }`).

**Read-timeout wiring point** (RESEARCH.md, after `SupervisorSocket::pair()` at exec_strategy.rs:622, before entering the loop):
```rust
sock.set_read_timeout(Some(crate::timeouts::supervisor_ipc_read_timeout()))?;
```

**Open question for planner (RESEARCH Open Q1):** the exact keep-alive predicate for the URL-open/direct-IPC listener — D-04 wants keep-alive even when seccomp/proxy/PTY are absent. The Linux arm currently still breaks in that case (line 2537-2540). Scope keep-alive to the URL/direct listener; do NOT broaden to seccomp/proxy notify fds (Anti-pattern).

**`#[allow(clippy::unwrap_used)]` reminder:** both loop arms are cfg-gated Unix code → cross-target clippy is PARTIAL/CI-deferred on this Windows host (CLAUDE.md MUST; `.planning/templates/cross-target-verify-checklist.md`).

---

### `crates/nono/src/supervisor/socket_windows.rs` (library transport primitive, Windows)

**Analog:** Self. Two existing idioms to reuse.

**Unbounded `read_frame` to replace** (socket_windows.rs:321-339) — structurally identical to the Unix `read_frame` (same 4-byte BE length prefix + `MAX_MESSAGE_SIZE` cap), confirming the **translate-not-cherry-pick** framing. The `read_exact` calls block under `PIPE_WAIT`:
```rust
fn read_frame(&mut self) -> Result<Vec<u8>> {
    let mut len_bytes = [0u8; LENGTH_PREFIX_SIZE];
    self.reader.read_exact(&mut len_bytes)        // ◀── blocks unbounded under PIPE_WAIT
        .map_err(|e| NonoError::SandboxInit(format!("Failed to read message length: {e}")))?;
    let len = u32::from_be_bytes(len_bytes);
    if len > MAX_MESSAGE_SIZE { /* reject */ }
    let mut payload = vec![0u8; len as usize];
    self.reader.read_exact(&mut payload)          // ◀── blocks unbounded under PIPE_WAIT
        .map_err(|e| NonoError::SandboxInit(format!("Failed to read message payload: {e}")))?;
    Ok(payload)
}
```

**`ERROR_PIPE_CONNECTED`-is-success idiom to reuse** (socket_windows.rs:1307-1324) — the re-accept loop must reuse this exact pattern when re-arming via `ConnectNamedPipe` (a client racing in between `Disconnect` and `Connect` returns 0 with GLE=535, which is success):
```rust
fn finalize_server_connection(server_handle: HANDLE, pipe_name: &str) -> Result<File> {
    let connected = unsafe { ConnectNamedPipe(server_handle, std::ptr::null_mut()) };
    if connected == 0 {
        let err = std::io::Error::last_os_error();
        if err.raw_os_error() != Some(ERROR_PIPE_CONNECTED as i32) {
            drop(unsafe { OwnedHandle::from_raw_handle(server_handle) });
            return Err(NonoError::SandboxInit(format!(
                "Failed to accept Windows supervisor pipe connection on {pipe_name}: {err}. ..."
            )));
        }
    }
    Ok(file_from_handle(server_handle))
}
```

**New `PeekNamedPipe` bounded-read to add** (RESEARCH Pattern 3; `windows-sys` `Win32::System::Pipes::PeekNamedPipe` — non-destructive availability probe, deadline check IS the watchdog, no `CancelIoEx` needed):
```rust
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
        if ok == 0 { return Err(/* ERROR_BROKEN_PIPE 109 / ERROR_PIPE_NOT_CONNECTED 233 → disconnect */); }
        if avail == 0 {
            if Instant::now() >= deadline { return Err(/* bounded timeout, non-fatal */); }
            std::thread::sleep(POLL_INTERVAL);  // ~10ms; Claude's discretion (D-03). MUST sleep — busy-spin pitfall.
            continue;
        }
        let want = (avail as usize).min(buf.len() - filled);
        let n = read_some(handle, &mut buf[filled..filled + want])?;  // post-peek read of avail bytes won't hang
        filled += n;
    }
    Ok(())
}
```

**Win32 error codes to classify** (RESEARCH Pattern 4): `ERROR_BROKEN_PIPE` (109) → re-accept; `ERROR_PIPE_NOT_CONNECTED` (233) → re-arm; `ERROR_PIPE_CONNECTED` (535) → success (reuse idiom above); `ERROR_NO_DATA` (232) → disconnect. Error consts already vendored at socket_windows.rs:20-39.

---

### `crates/nono-cli/src/exec_strategy_windows/supervisor.rs` (supervision policy loop, Windows)

**Analog:** Linux `sock_fd_active` keep-alive (`exec_strategy.rs:2455`), **translated** to pipe re-accept (cross-platform translate per the C2 `split` disposition). Same intent (keep loop alive on child close), different transport (named pipe vs socketpair fd).

**BUGGY break-on-close pattern to fix** (supervisor.rs:561-600) — single transient close permanently disables capability expansion for the session:
```rust
let mut seen_request_ids = HashSet::new();
loop {
    if terminate_requested.load(Ordering::SeqCst) { break; }
    match sock.recv_message() {
        Ok(msg) => { /* handle_windows_supervisor_message(...) + audit */ }
        Err(e) => {
            tracing::debug!(session_id = %session_id, error = %e, "Capability pipe closed");
            break;                              // ◀── SC1 bug: tears down on first transient close
        }
    }
}
```

**Translation target** (RESEARCH Pattern 4 / D-04): on a disconnect-class error, `DisconnectNamedPipe(server_handle)` then `ConnectNamedPipe(server_handle, ...)` (re-arm the SAME handle — reuse the ERROR_PIPE_CONNECTED idiom) and `continue`, bounded by `terminate_requested` (the loop already checks it — fail-secure liveness bound, no separate retry cap needed per RESEARCH Open Q2). `bind_aipc_pipe` uses `PIPE_UNLIMITED_INSTANCES` (socket_windows.rs:768) so re-accept is structurally supported; **control pipes use 1 instance** → must re-arm the existing handle, NOT create a fresh instance (RESEARCH Pitfall 3).

**Security invariant (do NOT regress):** re-accept must NOT bypass `seen_request_ids` replay protection (carried at supervisor.rs:560) and must re-verify the session SID/token on reconnect (V3 — do not cache trust across reconnect).

---

### `crates/nono-cli/tests/supervisor_ipc_robustness.rs` (test, net-new — Wave 0)

**Analog (cfg structure):** `crates/nono-cli/tests/aipc_handle_brokering_integration.rs`. **Analog (Unix fork test shape):** the existing close-on-exit test at `exec_strategy.rs:4057`.

**Cross-platform cfg-gate header** (aipc_handle_brokering_integration.rs:24-29) — but note this phase needs BOTH a Unix-gated and a Windows-gated test in one file (not a single `windows`-only gate), so use per-test `#[cfg(...)]` rather than a file-level `#![cfg(target_os = "windows")]`:
```rust
//! Cross-platform compile: this file is `#[cfg(target_os = "windows")]`-
//! gated, so it produces an empty test binary on Linux/macOS; CI
//! `cargo build -p nono-cli --tests` still passes everywhere.
#![cfg(target_os = "windows")]
#![allow(clippy::unwrap_used)]
```

**Win32 round-trip assertion shape to mirror** (aipc_handle_brokering_integration.rs:36-68) — explicit `// SAFETY:` on every unsafe FFI call, `last_os_error()` in assert messages, close handles to avoid leaks:
```rust
#[test]
fn integration_event_broker_round_trip() {
    // SAFETY: anonymous event creation with NULL attributes/name.
    let source: HANDLE = unsafe { CreateEventW(std::ptr::null_mut(), 0, 0, std::ptr::null()) };
    assert!(!source.is_null(), "CreateEventW failed: {}", std::io::Error::last_os_error());
    // ... broker call, assert grant shape ...
    // SAFETY: both HANDLEs are live.
    unsafe { CloseHandle(dup as usize as HANDLE); CloseHandle(source); }
}
```

**Unix fork test shape to mirror/UPDATE** (exec_strategy.rs:4057-4091) — the SC1 reconnect-survival test models a child that closes then reconnects. **IMPORTANT:** the existing test at 4057 asserts the OLD break-on-close behavior ("It should poll the socket, see POLLHUP when child exits, and return") — RESEARCH Wave-0 Gaps flag that this test must be UPDATED to assert keep-alive/re-accept for the URL-listener path:
```rust
match unsafe { fork() } {
    Ok(ForkResult::Child) => {
        drop(child_stream);
        drop(parent_stream);
        unsafe { libc::_exit(42) };
    }
    Ok(ForkResult::Parent { child }) => {
        drop(child_stream);
        let mut sock = SupervisorSocket::from_stream(parent_stream);
        // cfg-split call into run_supervisor_loop (Linux 9-arg, non-Linux 6-arg)
    }
}
```

**Env-var save/restore for the timeouts unit test** (RAII guard + serialization lock — pattern from `trust_cmd.rs:2218-2244`, required by CLAUDE.md parallel-test env rule). The SC2 timeout test should override `NONO_SUPERVISOR_IPC_READ_TIMEOUT` to ~1s to keep CI fast (RESEARCH Nyquist note):
```rust
let prev = std::env::var("NONO_SUPERVISOR_IPC_READ_TIMEOUT").ok();
std::env::set_var("NONO_SUPERVISOR_IPC_READ_TIMEOUT", "1");
// ... exercise ...
match prev {
    Some(v) => std::env::set_var("NONO_SUPERVISOR_IPC_READ_TIMEOUT", v),
    None => std::env::remove_var("NONO_SUPERVISOR_IPC_READ_TIMEOUT"),
}
```

**Test→Req map (RESEARCH Validation):** `reconnect_survival` (SC1), `bounded_read_timeout` (SC2), `timeouts::` unit extension (env+clamp), plus a documented Windows live-repro (SC4 — named-pipe timing not deterministic in CI). Reconnect test must wait > one poll tick (200ms) to observe re-accept; timeout test holds a partial frame > the overridden timeout.

## Shared Patterns

### Error handling / fail-secure
**Source:** `crates/nono/src/error.rs` (`NonoError`) + the `.map_err(|e| NonoError::SandboxInit(format!(...)))?` idiom used throughout `socket.rs`/`socket_windows.rs`.
**Apply to:** All library-side new code (`socket.rs` wiring, `socket_windows.rs` `read_exact_bounded`).
- No `.unwrap()`/`.expect()` in non-test code (clippy `-D clippy::unwrap_used`).
- A timeout/disconnect/malformed-frame error MUST deny the in-flight request and keep the sandbox intact — never widen caps on an IPC hiccup (CLAUDE.md fail-secure; RESEARCH Security Domain).
```rust
.map_err(|e| NonoError::SandboxInit(format!("Failed to read message length: {e}")))?
```

### Timeout config (env override + clamp)
**Source:** `crates/nono-cli/src/timeouts.rs::env_duration_secs` (line 105) + `MAX_TIMEOUT` (line 103).
**Apply to:** The new `SUPERVISOR_IPC_READ_TIMEOUT` const/accessor only. Library reads no env; CLI owns the value (policy-free boundary).
**Anti-patterns (RESEARCH):** no bare timeout literal; no per-run CLI flag (env override only).

### `// SAFETY:` docs on unsafe FFI
**Source:** every `unsafe` block in `socket.rs` (e.g. line 247), `socket_windows.rs`, and `aipc_handle_brokering_integration.rs`.
**Apply to:** `PeekNamedPipe`/`ConnectNamedPipe`/`DisconnectNamedPipe` calls in the Windows changes; all FFI in the new test. CLAUDE.md mandates `// SAFETY:` comments on all unsafe.

### Cross-target clippy gate (PARTIAL/CI-deferred)
**Source:** `.planning/templates/cross-target-verify-checklist.md`; `55-05-SUMMARY.md` precedent.
**Apply to:** `crates/nono/src/supervisor/socket.rs` + both `run_supervisor_loop` arms in `exec_strategy.rs` (cfg-gated Unix). Windows-host `cargo check` does NOT exercise these → mark the verify REQ PARTIAL, defer to live ubuntu/macos CI (`-D warnings -D clippy::unwrap_used`).

## No Analog Found

None. Every file has a strong in-tree analog — this phase is wiring and loop-control over existing primitives, not new mechanism. The single genuinely-new code unit (Windows `PeekNamedPipe` bounded-read helper) still copies the `read_frame` framing + the `ERROR_PIPE_CONNECTED` idiom from its own file.

## Metadata

**Analog search scope:** `crates/nono/src/supervisor/`, `crates/nono-cli/src/`, `crates/nono-cli/src/exec_strategy_windows/`, `crates/nono-cli/tests/`
**Files scanned:** timeouts.rs, socket.rs, socket_windows.rs, exec_strategy.rs (both loop arms + close-on-exit test), exec_strategy_windows/supervisor.rs, aipc_handle_brokering_integration.rs, trust_cmd.rs (env-guard), aipc_sdk.rs (env-guard)
**Pattern extraction date:** 2026-06-06
