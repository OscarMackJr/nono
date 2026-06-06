# SECURITY.md — Phase 59: Supervisor IPC Robustness

**Audited:** 2026-06-06
**Auditor:** gsd-security-auditor (code-evidence verification, not documentation acceptance)
**ASVS Level:** 2 (default; not configured)
**block_on:** high (default; not configured)
**Disposition:** SECURED — 14/14 threats closed (12 mitigate verified in code, 2 accept verified)

This phase hardened the supervisor IPC channel (trust boundary: untrusted sandboxed
child → supervisor) against DoS / spoofing / fail-open on both Unix (inherited
`socketpair` fd) and Windows (AIPC named pipes). Every declared mitigation was
verified by reading the cited implementation lines — not by accepting the plan,
SUMMARY, or verification report.

---

## Threat Verification

| Threat ID | Category | Disposition | Verdict | Evidence (file:line) |
|-----------|----------|-------------|---------|----------------------|
| T-59-02 | DoS | mitigate | CLOSED | `crates/nono-cli/src/timeouts.rs:84` const `SUPERVISOR_IPC_READ_TIMEOUT=5s`; `:92-97` accessor delegates to `env_duration_secs("NONO_SUPERVISOR_IPC_READ_TIMEOUT", ...)`; `:134-156` clamps to `MAX_TIMEOUT` (3600s, `:132`) and falls back to default on parse error (`:149-150`). Attacker env cannot set unbounded/overflowing timeout. |
| T-59-02b | Tampering | accept | CLOSED | No bare literal at any IPC-read call site. Unix routes through `crate::timeouts::supervisor_ipc_read_timeout()` (`exec_strategy.rs:1293`); Windows routes through `DEFAULT_READ_TIMEOUT` const (`socket_windows.rs:124`) + CLI `recv_message_with_timeout(supervisor_ipc_read_timeout())` (`supervisor.rs:590,595`). Other `Duration::from_secs(5)` matches are unrelated subsystems/tests, not IPC reads. (Accepted-risk entry recorded below.) |
| T-59-01a | DoS | mitigate | CLOSED | `exec_strategy.rs:1291-1294` wires `sock.set_read_timeout(Some(crate::timeouts::supervisor_ipc_read_timeout()))?` before `run_supervisor_loop`; `?` propagation = fail-secure. `bounded_read_timeout` integration test drives the pub `recv_message()` path. |
| T-59-01b | DoS | mitigate | CLOSED | `exec_strategy.rs:2443` `waitpid(child, WNOHANG)` in-loop bounds reconnect by child liveness; loop terminates on child exit (`:2456-2468`). No infinite tight loop. |
| T-59-01c | Elevation/Tampering | mitigate | CLOSED | `exec_strategy.rs:2386-2397`: a timeout (`WouldBlock`/`timed out`) is keep-alive only — "loop continues, sandbox intact, no capability granted". No capability grant on partial/timed-out frame. |
| T-59-01d | Tampering | mitigate | CLOSED | `exec_strategy.rs:2323` keep-alive scoped to the single `sock_fd` (demote-to-`-1` at `:2330`); PTY relay fds (`pty_master/client/attach/resize`) retain existing logic — seccomp/proxy/PTY demotion untouched. |
| T-59-01e | Spoofing | mitigate | CLOSED (citation corrected) | Load-bearing clause verified: the Unix IPC channel is an anonymous inherited `socketpair` (`socket.rs:59 pair()`, `socket_path: None`) — not filesystem-reachable, no `bind`/`listen`/`accept`, so no third-party process can connect/impersonate. macOS "re-accept" is fd-demote-keep-alive on the SAME inherited fd, not a new connection. NOTE: the cited `peer_credentials`/SO_PEERCRED check (`socket.rs:380`) is wired only on the SEPARATE attach-listener path (`pty_proxy.rs:1171-1187`), NOT the socketpair reconnect path — there is no `accept()` on a socketpair to "re-run" on. The structural inheritance property (second clause) is the actual mitigation and it holds. See finding below. |
| T-59-03a | DoS | mitigate | CLOSED | `socket_windows.rs:564-655` `read_exact_bounded` uses `PeekNamedPipe` (`:578-587`) non-destructive probe with deadline check (`:609-615`); `read_frame_with_timeout` (`:674-692`) computes `Instant + timeout` via `checked_add` and applies it to both length-prefix and payload reads. Slow/silent child cannot block indefinitely. |
| T-59-03b | DoS | mitigate | CLOSED | `socket_windows.rs:616` `std::thread::sleep(POLL_INTERVAL)` (10ms, `:131`) on the `avail==0` path, bounded by deadline. No busy-spin. |
| T-59-03c | DoS | mitigate | CLOSED | `supervisor.rs:591-593,651-657` re-accept loop bounded by `terminate_requested` (checked at loop top AND before re-accept); `disconnect_and_reconnect` failure → `break` (`:666-673`). No infinite tight loop, no separate retry cap. |
| T-59-03d | Spoofing | mitigate | CLOSED | `supervisor.rs:589` `seen_request_ids` NOT reset on reconnect; `:595-606` every message re-checked by `handle_windows_supervisor_message` against `session_token` via `constant_time_eq` (`supervisor.rs:2095-2097`, `constant_time_eq` at `:1511`). Restricting-SID DACL on the pipe unchanged (re-arm reuses same handle). |
| T-59-03e | Elevation/Tampering | mitigate | CLOSED | `supervisor.rs:621-632` `[timeout]` → keep-alive continue (partial frame discarded, no grant); `socket_windows.rs:683-687` `MAX_MESSAGE_SIZE` (64 KiB) cap preserved; `seen_request_ids` replay set preserved across re-accept (T-59-03d). No capability granted on partial/timed-out/malformed frame. |
| T-59-03f | Tampering | mitigate | CLOSED | `socket_windows.rs:479-514` `disconnect_and_reconnect` re-arms the SAME `server_handle` via `DisconnectNamedPipe` + `ConnectNamedPipe` (ERROR_PIPE_CONNECTED-is-success idiom `:504`) — does NOT create a fresh instance. Avoids ERROR_PIPE_BUSY/handle leak on 1-instance control pipes. |
| T-59-SC | Tampering | accept | CLOSED | Zero Cargo.toml changes across the entire Phase 59 commit range (deba18ae..ae1892b9 — verified via `git diff --name-only`). No new deps; only already-vendored `windows-sys 0.59`, `libc`, `std`. No `[ASSUMED]`/`[SUS]` packages. (Accepted-risk entry recorded below.) |

**Score: 14/14 CLOSED** (12 mitigate verified present in code; 2 accept verified).

---

## Accepted Risks Log

- **T-59-02b (Tampering — bare timeout literal drift):** ACCEPTED. Mitigated
  structurally — all IPC-read timeouts route through a single per-platform
  const/accessor (`timeouts.rs:84` / `socket_windows.rs:124`); no per-call literal
  exists at any IPC-read site to drift. Verified by grep: the only
  `Duration::from_secs(5)` occurrences are in unrelated subsystems and test code.

- **T-59-SC (Tampering — package-install supply chain):** ACCEPTED. No npm/pip/cargo
  installs in this phase; zero dependency-manifest changes (git-verified). No
  legitimacy checkpoint required.

---

## Auditor Findings (non-blocking)

### F-59-AUDIT-01 — T-59-01e citation does not match the transport (CLOSED, citation corrected)

The threat register cites "Existing SO_PEERCRED/peer_credentials check
(socket.rs:~380) re-runs on each accept" as the primary mitigation for reconnect
impersonation on Unix. Independent code reading shows:

- The supervisor IPC channel is an anonymous inherited `socketpair`
  (`SupervisorSocket::pair()`, `socket.rs:59`, `socket_path: None`). A socketpair
  has **no `accept()` and no filesystem path** — there is nothing for a third
  party to connect to, so there is no per-accept credential re-check to perform.
- `peer_credentials` (`socket.rs:380`) is invoked only at `pty_proxy.rs:1173`
  (`authenticate_attach_peer`) — a **separate** AF_UNIX *attach listener*, not the
  capability/IPC socketpair.

**Disposition unchanged (CLOSED):** the threat's load-bearing clause — "socketpair
fd inherited (not filesystem-reachable)" — IS verified and IS the actual structural
mitigation. The impersonation surface is bounded by fd inheritance, exactly as
stated. The "SO_PEERCRED re-runs on each accept" wording is descriptively
incorrect for this transport but does not weaken the guarantee. Recommend the
register text be corrected to drop the per-accept SO_PEERCRED claim and rely on the
inheritance property for the socketpair path.

### Tracked WARNINGs from 59-REVIEW.md (all fail-secure, none capability-granting)

These were raised by the code reviewer, accepted by the user as tracked-not-fixed,
and re-confirmed during this audit. None grant a capability; all degrade
availability / keep-alive precision only. They do **not** open any threat in the
register:

- **WR-01** (`socket_windows.rs:564-654`): `read_exact_bounded` checks the deadline
  only when `avail==0`; a 1-byte-per-tick slow-trickle can extend a read past the
  deadline (≈11 min for a max frame). Fail-secure (no grant until a complete valid
  frame). Partially weakens T-59-03a's bound for the trickle variant. **Recommend
  hoisting the deadline check to the top of the loop** (reviewer supplied the fix).
- **WR-02** (`exec_strategy.rs:2391-2406`): macOS keep-alive classifier matches
  `"timed out"`/`"WouldBlock"`/`"would block"` substrings, but a `SO_RCVTIMEO`
  timeout's `io::Error` Display is the libc strerror ("Resource temporarily
  unavailable"), not those substrings — so a real timeout may be misclassified as a
  disconnect (demote, not keep-alive). Weakens T-59-01a/T-59-01c keep-alive INTENT
  on macOS but remains fail-secure (no grant). **Recommend classifying on
  `io::ErrorKind` / a `[timeout]` tag** (mirroring Windows). CI-deferred (Unix cfg
  not compilable on the Windows host).
- **WR-03** (`exec_strategy.rs:2609-2618`): Linux arm treats every `recv_message`
  error identically — a partial-frame timeout permanently demotes the IPC socket
  for the session. Fail-secure. Same kind-based fix as WR-02.
- **WR-04** (`supervisor.rs:2091` before `:2095`): `seen_request_ids.insert` runs
  BEFORE the `constant_time_eq` session-token check, so an unauthenticated
  pipe-reaching peer can pollute the replay namespace and pre-empt a request_id.
  Fail-secure (no grant to the bad peer); pipe is DACL-gated; request_ids are
  child-generated (guessing required). Inverts the intended T-59-03d/T-59-03e
  ordering invariant. **Recommend moving the token check above the replay
  insert.** Low exploitability, tracked.
- **WR-05** (`socket_windows.rs:329-340,1137-1153`): the per-run package-SID
  rendezvous READ-ACE relies on `Drop` for cleanup; an abnormal supervisor exit
  leaves the per-run rendezvous file + ACE on disk. Per-run unique SID = small
  residual; best-effort cleanup. Tracked, not blocking.

---

## Unregistered Flags

The `## Threat Flags` sections of all three SUMMARYs (59-01, 59-02, 59-03) report
**no new network endpoints, auth paths, file-access patterns, or schema changes**.

One implementation-time surface DID appear that is not in the original register:
the **AppContainer cap-pipe reachability fix** (debug session
`appcontainer-cap-pipe-unreachable`, documented in 59-03-SUMMARY.md) — a new
`grant_sid_read_on_path` lib primitive + a `(A;;0x0012019F;;;<package_sid>)` ACE in
`build_capability_pipe_sddl` + `validate_package_sid_for_sddl`. This is new DACL/SID
attack surface introduced during Phase 59 execution.

- **Classification:** `unregistered_flag` (WARNING, not a blocker).
- **Mapping:** It is adjacent to T-59-03d (Windows reconnect spoofing / per-session
  SID) but was NOT in the plan-time register (it surfaced during Task-3 UAT).
- **Audit status:** 59-REVIEW.md explicitly reviewed this surface and found it
  well-constructed: validate-before-embed enforced, no null/world/AU fallback on any
  error path, every fallible step fail-closed, `// SAFETY:` on all FFI, per-run
  package-SID-only, revert-on-Drop. No BLOCKER-class defect (see WR-05 for the only
  residual, fail-secure).
- **Recommendation:** Register this DACL/SID grant surface as a tracked threat in a
  Phase-62-adjacent (AppContainer/broker) milestone so future changes to
  `build_capability_pipe_sddl` / `grant_sid_read_on_path` are re-audited against an
  explicit threat entry.

---

## Verification Method Notes

- All `mitigate` threats verified by reading the cited file:line in the
  implementation (not the plan/SUMMARY). Where a citation was imprecise (T-59-01e),
  the actual transport was traced and the load-bearing mechanism re-confirmed.
- `accept` threats verified: T-59-02b by grep for bare literals at IPC sites;
  T-59-SC by `git diff --name-only` over the full phase commit range for
  Cargo.toml changes (zero found).
- Implementation files were NOT modified (read-only audit). Recommended fixes
  (WR-01..WR-05, F-59-AUDIT-01) are for the implementing agent, not patched here.
- Unix cfg arms (`exec_strategy.rs` macOS/Linux, `socket.rs`) could not be compiled
  on the Windows host; their verification is by source reading + the cross-target
  clippy CI-deferral recorded in the SUMMARYs.

---

*Audited 2026-06-06 — gsd-security-auditor. 14/14 threats CLOSED. 1 unregistered
flag (AppContainer DACL/SID surface) logged as WARNING. Phase may ship; recommend
register-text correction for T-59-01e and tracking the AppContainer SID surface.*
