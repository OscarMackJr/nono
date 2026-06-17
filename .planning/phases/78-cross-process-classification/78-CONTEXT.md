# Phase 78: Cross-Process Classification — Context

**Gathered:** 2026-06-17
**Status:** Ready for planning
**Source:** Operator decisions + daemon code map (composition over shipped Phase 74 daemon)

<domain>
## Phase Boundary

Add an authoritative cross-process classification path: `nono classify <pid>` answered by the
`nono-agentd` daemon over its existing control pipe via a new `Classify` verb. The daemon's
live, shared agent registry makes the verdict authoritative (the standalone Phase 73 path builds
a fresh empty registry every invocation, so it can only ever return a non-authoritative
structural answer). In scope: the `Classify` request/response verb + dispatch, the client wiring
in `nono classify`, the daemon-absent fallback, and the scripted test gate. Out of scope: any new
pipe, any new wire protocol, any change to the launch/reap lifecycle, any change to how the
AI_AGENT marker (daemon-minted AppContainer SID) is established (Phase 73/74 — already shipped).
</domain>

<decisions>
## Implementation Decisions (LOCKED)

### D1 — Reuse the existing control pipe (no new pipe)
The `Classify` verb is served on the existing `\\.\pipe\nono-agentd-control`
(`control_loop.rs:81`) with its existing **Medium-IL-only** SDDL
(`CONTROL_PIPE_SDDL = "D:P(A;;GA;;;SY)(A;;GA;;;BA)(A;;GA;;;OW)S:(ML;;NW;;;ME)"`,
`control_loop.rs:96`). This satisfies SC3 for free: a Low-IL caller cannot open the control pipe
at all (kernel denies at open via the `ML;;NW;;;ME` SACL) — it receives an access-denied error,
never a spoofable answer. NO new pipe and NO weakening of the control-pipe SDDL.

### D2 — Authoritative source = the daemon's shared agent registry
`Classify` resolves the verdict by (a) reading the target PID's AppContainer SID via
`nono::agent::read_process_appcontainer_sid(pid)` and (b) checking membership in the daemon's
LIVE `DaemonState::agent_registry` (`nono::AgentRegistry`, `mod.rs:306`; `classify(pid)` at
`agent.rs:143`). Because the daemon holds the real set of SIDs it minted at launch
(`launch.rs` step 7 insert / reap remove), this membership check is authoritative — only a
daemon-launched confined agent returns `AiAgent`. Honor the existing lock order: acquire
`agent_registry` before `tenants` (`mod.rs:262`).

### D3 — Classify response shape = verdict only, NO package SID (SC4)
The `Classify` response returns the verdict enum (`AiAgent` / `NotAnAgent`) and MUST NOT echo the
matched agent's package SID or any other per-tenant identifier. The daemon uses the SID internally
for the membership check but does not disclose it in the response. This is the SC4 mitigation:
the response for any PID — including one the caller did not launch — contains no cross-tenant SID
disclosure. (Contrast: the standalone Phase 73 JSON output echoes `package_sid` for the LOCAL
caller's own structural inspection; the cross-process daemon response deliberately omits it.)

### D4 — `nono classify <pid>` routing: daemon-first, structural fallback
`nono classify <pid>` first attempts the daemon control pipe (`windows_control_pipe_request`,
`agent_cli.rs:819`) with `{"action":"classify","pid":N}`:
- **Daemon running →** use the daemon's authoritative verdict (mark `authoritative=true`; no
  "structural only" disclaimer).
- **Daemon absent** (pipe open fails `ERROR_FILE_NOT_FOUND` / GLE=2, detected by
  `is_pipe_not_found`, `agent_cli.rs:984`) **→** fall back to the EXISTING standalone structural
  classification (`classify_runtime::run_classify`) with its non-authoritative disclaimer
  (`NOTE_*`). No regression to today's `nono classify` behavior; authoritative only when the
  daemon is up. (Operator decision 2026-06-17.)

### D5 — Caller-gating / tenant-safety (CLAS-02)
SC3 (Low-IL denied) is enforced structurally by the reused control-pipe SDDL (D1) — no new
caller IL-check code is required (the kernel gates at open, as the control loop already relies on;
`control_loop.rs:27-30`). SC4 (no cross-tenant disclosure) is enforced by the verdict-only
response shape (D3). The control pipe remains operator-only (Medium+ IL) with no per-tenant
gating — the operator is trusted to query any daemon-launched agent's status, and the response
leaks no tenant SID.

### D6 — Scripted gate
Unattended gate = `cargo test --bin nono-agentd -- classify`. Put deterministic unit tests in
`control_loop.rs`'s test module (compiled into the `nono-agentd` bin) covering: the `Classify`
request deserializes; the dispatch routes to the classify handler; a registry-hit PID yields
`AiAgent` and a miss yields `NotAnAgent`; the serialized response contains NO `package_sid` /
SID field (SC4, assert by string inspection). Mirror the existing control_loop unit-test idiom
(`control_loop.rs:957-1227`, e.g. `control_pipe_sddl_is_medium_il_only`,
`list_returns_tenants_when_populated`). Real-host cross-process behavior (SC1/SC2 with live PIDs)
is exercised by an integration test mirroring `daemon_handle_baseline.rs` (gated by
`NONO_DAEMON_INTEGRATION_TESTS=1`) — that env-gated path is the host gate, not the unattended gate.

### Claude's Discretion
- Exact Rust names for the new `ControlRequest::Classify { pid: u32 }` variant and the response
  type/serialization (a small response enum or a tagged struct — keep the existing
  `[4-byte LE length][JSON]` framing and serde conventions).
- Whether the response carries a minimal `authoritative: true` marker for the client to render.
</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Daemon control pipe + dispatch (the verb is added here)
- `crates/nono-cli/src/agent_daemon/control_loop.rs` — `ControlRequest` enum (309-324), dispatch match in `handle_control_connection` (417-444), `CONTROL_PIPE_SDDL` (96), framing/`write_framed_response` (927-975), unit-test module (957-1227).
- `crates/nono-cli/src/agent_daemon/mod.rs` — `DaemonState` (283-307), `agent_registry` field (306), lock-order note (262).

### Classification primitives (reused)
- `crates/nono/src/agent.rs` — `AgentRegistry` (78-172), `classify(pid)` (143-155), `read_process_appcontainer_sid` (198), `AgentClassification { AiAgent { package_sid }, NotAnAgent }` (42-59).
- `crates/nono-cli/src/classify_runtime.rs` — `run_classify` (55-95), structural outcome logic (74-87), `process_in_job` (154-174), human/JSON output (98-146), disclaimers (`NOTE_*`).
- `crates/nono-cli/src/cli.rs` — `Commands::Classify(ClassifyArgs)` (~755, 3160-3173).
- `crates/nono-cli/src/app_runtime.rs` — Classify dispatch (61-69).

### Client pipe transport (reused to send the verb)
- `crates/nono-cli/src/agent_cli.rs` — `windows_control_pipe_request` (819-977), `CreateFileW`/`WaitNamedPipeW` (845-879), framed write/read (889-970), `is_pipe_not_found` (984).

### Tests (mirror these)
- `crates/nono-cli/src/agent_daemon/control_loop.rs` unit tests (957-1227) — the `--bin nono-agentd -- classify` gate target.
- `crates/nono-cli/tests/daemon_handle_baseline.rs` — integration tests (gate `NONO_DAEMON_INTEGRATION_TESTS=1`, 100-102); cross-tenant denial test (980-1158) as the SC3/SC4 host-proof analog.
</canonical_refs>

<specifics>
## Specific Ideas

- The cross-tenant denial integration test (`daemon_handle_baseline.rs:980-1158`) is the closest
  analog for a real-host SC1/SC2/SC4 proof — it already stands up two real AppContainer tenants.
- Phase 73 shipped in-process structural classify as deliberately non-authoritative; Phase 78's
  daemon verb is the authoritative path. Never conflate the two (carried decision, STATE.md).
</specifics>

<deferred>
## Deferred Ideas

None new — Phase 78 is scoped to the Classify verb + routing + gate.
</deferred>

---

*Phase: 78-cross-process-classification*
*Context gathered: 2026-06-17 (operator decisions + daemon code map)*
