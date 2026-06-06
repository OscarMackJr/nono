# Phase 59: Supervisor IPC Robustness - Context

**Gathered:** 2026-06-06
**Status:** Ready for planning

<domain>
## Phase Boundary

Make the supervisor's IPC loop survive transient child disconnects and enforce bounded read timeouts, on both platforms (REQ-IPC-01). The Unix side absorbs upstream's named-socket hardening intent (Phase 54 Cluster C2 — cross-platform-core portions); the Windows side **translates** that intent onto the fork's Named-Pipe AIPC path (Phase 18) — explicitly a translate-not-cherry-pick because AF_UNIX is unix-only.

In scope: supervisor IPC keep-alive on transient child close, bounded read timeouts, robust accept (re-accept). Out of scope: the AF_UNIX named-socket *mechanism* switch itself as a verbatim port (`be7681c` is unix-only and is the divergent surface, not a parity target), any new IPC features, and the `bw://`/`allow_domain` work (other phases).
</domain>

<decisions>
## Implementation Decisions

### Read timeout (SC2)
- **D-01:** Add a supervisor IPC read-timeout as a named constant in `crates/nono-cli/src/timeouts.rs` (default **5s**, matching upstream `d1851c9`'s "increase supervisor listener read timeout to 5s"), **with a `NONO_*` env-var override and `MAX_TIMEOUT` clamp**, consistent with the Phase 55 / plan 55-05 timeouts.rs pattern. Do NOT use a bare literal and do NOT add a per-run CLI flag (internal knob; env override is sufficient).
- **D-02:** On Unix, wire the **already-present but unused** `SupervisorSocket::set_read_timeout()` (`crates/nono/src/supervisor/socket.rs:192`) to this constant on accepted connections (mirrors upstream `284ae1d` "add read timeout on accepted listener connections"). Windows reads the same constant value.

### Windows AIPC parity approach (SC4)
- **D-03:** Windows named-pipe reads block under `PIPE_WAIT` and do not honor socket-style read timeouts. Achieve the bounded-read + robust-accept parity via a **`PeekNamedPipe` poll-until-data-or-deadline loop** (read only once data is available), with a watchdog-cancel on deadline overrun, plus a **robust re-accept loop**. This is the lower-risk translation; the full overlapped/async-I/O rewrite (`ReadFile` overlapped + `WaitForSingleObject` + `CancelIoEx`) was explicitly rejected as too large/risky for this phase. The plan SUMMARY MUST document this as the translate-not-cherry-pick rationale (SC4 requirement).

### Keep-alive / re-accept semantics (SC1)
- **D-04:** When the sandboxed child closes its IPC connection, the supervisor **keeps its loop alive and re-accepts** new connections — matching upstream `51f56b8` ("keep supervisor loop alive when child closes direct IPC socket") and `9820a2e` ("include URL listener in supervisor loop keep-alive conditions"). Scope the re-accept/keep-alive to the **URL-open / direct-IPC listener** (the listener upstream C2 targets, #959), NOT a blanket rewrite of every supervisor IPC read path. Also absorb `f956fb6` (set accepted connections to blocking mode) and ensure the child sandbox grants the supervisor-socket capability (`4a22e94`) where the fork's capability model requires it.

### Verification strategy (SC1/SC2)
- **D-05:** Primary proof is **cross-platform CI-runnable integration tests**: (a) child closes its IPC connection then reconnects → supervisor survives and re-accepts; (b) a slow/silent child holding an open connection → the bounded read timeout fires and the supervisor is not blocked indefinitely. **Plus** a documented **Windows live-repro** for the named-pipe path (given OS-specific named-pipe timing behavior and the project's Windows-UAT pattern). Existing IPC tests (`tests/aipc_handle_brokering_integration.rs`, in-crate `socket.rs`/`socket_windows.rs` tests) currently cover round-trip only — the new disconnect/timeout scenarios are net-new.

### Claude's Discretion
- Exact constant name (e.g. `SUPERVISOR_IPC_READ_TIMEOUT`) and env-var name (follow the `NONO_*_TIMEOUT` convention already in timeouts.rs).
- `PeekNamedPipe` poll interval / watchdog mechanics on Windows (planner/researcher choose; keep it within the bounded-deadline contract).
- Whether re-accept is a bounded retry count or unbounded loop (choose per upstream behavior + fail-secure).
- Cross-target clippy for the unix-gated `socket.rs` changes is PARTIAL/CI-deferred on this Windows host per CLAUDE.md.
</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase 54 audit — the locked input for this phase
- `.planning/phases/54-upst7-audit/54-DIVERGENCE-LEDGER.md` §"Cluster C2: Supervisor named-socket IPC (URL-open helpers, #959)" — the 8 upstream v0.58.0 commits to absorb and their per-commit dispositions; the `split` decision (cross-platform-core absorbed, AF_UNIX unix-only, Windows = translate). This is the authoritative scope source.

### Upstream commits to absorb (cross-platform-core intent, from C2)
- `51f56b8` — keep supervisor loop alive when child closes direct IPC socket (SC1 core)
- `9820a2e` — include URL listener in supervisor loop keep-alive conditions
- `284ae1d` — add read timeout on accepted listener connections (SC2 core)
- `d1851c9` — increase supervisor listener read timeout to 5s (the 5s default in D-01)
- `f956fb6` — set accepted listener connections to blocking mode
- `c15c76a` — review-comment fixes on supervisor socket IPC
- `4a22e94` — grant UnixSocketCapability for supervisor socket in child sandbox
- `be7681c` — (unix-only mechanism; replace fd-based IPC with named socket) — reference only, NOT a Windows parity target

### Roadmap & requirements
- `.planning/ROADMAP.md` §"Phase 59: Supervisor IPC Robustness" — goal + 4 success criteria
- `.planning/REQUIREMENTS.md` — REQ-IPC-01 (line ~43)

### Pattern to reuse
- `crates/nono-cli/src/timeouts.rs` — Phase 55 (55-05) centralized timeout module (named constants + `NONO_*` env overrides + `MAX_TIMEOUT` clamp); add the supervisor IPC read timeout here (D-01).
- `.planning/phases/55-upst7-cherry-pick-wave/55-05-SUMMARY.md` — the timeouts.rs convention + env-var/clamp pattern.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crates/nono/src/supervisor/socket.rs:192` — `SupervisorSocket::set_read_timeout()` exists but has **no callers**; wire it for the Unix bounded-read (D-02).
- `crates/nono-cli/src/timeouts.rs` — drop the new constant + env override here (D-01).
- `crates/nono/src/supervisor/socket.rs:75-114` — Unix `UnixListener::bind()` (umask guard, `0o700`); the accept path that needs the re-accept/keep-alive + read-timeout wiring.
- `crates/nono/src/supervisor/socket_windows.rs` — `bind_aipc_pipe()` (~748-782, `PIPE_UNLIMITED_INSTANCES`), `finalize_server_connection()` (~1307, `ConnectNamedPipe`), `read_frame()` (~321-339, unbounded `read_exact` — the gap). `PIPE_CONNECT_TIMEOUT_MS = 5000` already exists for *connect* (not read).

### Established Patterns
- Both platforms share an identical length-prefixed-JSON `read_frame()` framing protocol → the robustness logic is a structural **translate** (same protocol, different transport), not a code cherry-pick.
- `MAX_MESSAGE_SIZE` (64 KiB) cap already enforced; bounded read timeout is the missing slowloris/hang defense.
- Fork fail-closed posture: a hung/timed-out IPC read should surface as a bounded error, not an indefinite block.

### Integration Points
- Unix accept loop in `socket.rs` (bind/accept) + the supervisor run loop that owns the URL-open/direct-IPC listener.
- Windows AIPC accept in `socket_windows.rs` + `exec_strategy_windows/supervisor.rs` (Phase 18 AIPC wiring).
- Child-sandbox capability grant path (for `4a22e94` equivalent) where the fork models the supervisor-socket capability.

</code_context>

<specifics>
## Specific Ideas

- Mirror upstream's 5s read-timeout value exactly (D-01) so behavior matches the upstream lineage the ledger tracks.
- Keep the Windows translation deliberately conservative (`PeekNamedPipe` poll over an overlapped-I/O rewrite) to bound risk on the divergent named-pipe surface.

</specifics>

<deferred>
## Deferred Ideas

- Full overlapped/async-I/O rewrite of the Windows named-pipe read path (true kernel-level read timeout / cancellation) — rejected for this phase as too large/risky; revisit only if the `PeekNamedPipe` poll proves insufficient.
- Blanket robustness hardening of **all** supervisor IPC read paths (control socket + AIPC + URL-open) beyond the URL-open/direct-IPC listener upstream targets — out of scope; this phase matches upstream C2 scope.
- C11 timeout-constants work itself (already absorbed in Phase 55 / plan 55-05; Phase 59 only adds the one supervisor-IPC constant).

*None of these block Phase 59.*

</deferred>

---

*Phase: 59-supervisor-ipc-robustness*
*Context gathered: 2026-06-06*
