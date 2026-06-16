# Phase 75: Supplementary Controls + Secondary Engines — Research

**Researched:** 2026-06-15
**Domain:** Windows post-hoc IL-drop (demote), per-agent WFP egress (daemon→WFP-service coupling), GitHub Copilot CLI engine profile (node.exe), nono-ts binding parity (napi 2, confinedRun/confine)
**Confidence:** HIGH — all findings grounded in current in-tree code, shipped Phase 72/73/74 artifacts, and direct GitHub API/web verification for the Copilot CLI distribution question (D-06).

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01:** `demote` is a further IL-drop + kill-switch lever on an already-born-confined Phase 74 agent — it is demote-only, NEVER a standalone confinement boundary. Leak limits (handles leaked before drop, already-started children, network not covered) documented inline. NOT the original spike-002 "confine an unconfined process" framing.
- **D-02:** `nono agent demote <tenant_id>` targets a daemon `tenant_id` only. Demoting an arbitrary same-user PID is OUT of scope.
- **D-03:** `demote` does NOT reap/kill the agent. Whether it also deletes the agent's per-agent WFP filter is a PLANNING decision (not locked in discussion).
- **D-04:** Coupling is daemon → WFP control pipe, automatic at launch. The least-priv USER daemon sends the per-agent package SID to the elevated `nono-wfp-service` over the existing control pipe. The USER daemon never becomes elevated — it requests, the service enforces.
- **D-05:** Fail-secure when the WFP service is absent: if an agent's profile declares network scoping but `nono-wfp-service` is NOT reachable at launch, the daemon REFUSES to launch and names the missing service in an actionable error.
- **D-06:** Copilot CLI variant is a PLANNING decision (research/planning picks it). Lock only: it is a `node.exe` engine profiled like aider/langchain-python. Research resolves the concrete distribution.
- **D-07:** nono-ts parity = Windows shapes only. Mirror nono-py's `confined_run`/`confine` (Shape A + Shape B). Pin bump `0.33.0` → `0.62.x`, napi 2 KEPT. Do NOT extend to Unix Landlock/Seatbelt this phase.
- **D-08:** Live Win11 UAT required for SC3 (Copilot confined end-to-end) and SC5 (nono-ts confinedRun/confine on Win11). Build-green-only is not sufficient.

### Claude's Discretion

- **WFP keying field** — package SID (`FWPM_CONDITION_ALE_PACKAGE_ID`) vs user SID (`FWPM_CONDITION_ALE_USER_ID` + SID-scoped SD). Research discretion, provided result is per-agent (one agent's allowed domain never leaks to another).
- **Demote↔WFP-cut composition** (D-03) — planning's call whether `demote` deletes the WFP filter.
- **Control-pipe message shape** — reuse/extend the existing `WfpRuntimeActivationRequest` `session_sid` field; no net-new wire protocol unless the existing shape proves insufficient.
- **nono-ts examples/tests mirror** — whether to port a TS analog of nono-py's `examples/15_langchain_confined.py` + `tests/test_confined_run.py`; default to mirroring for SC5 proof.
- **Event Log IDs, verb output formats, error wording** — discretion within fail-secure.

### Deferred Ideas (OUT OF SCOPE)

- Generic post-hoc confine of arbitrary same-user PIDs — v2 deferred (blocked by post-hoc-IL-drop leak; needs different mechanism).
- Cross-platform nono-ts `confinedRun`/`confine` (Unix Landlock/Seatbelt) — out of scope (D-07).
- napi 3 migration — deliberate non-bump.
- Operator `net-scope`/explicit per-agent WFP verb — rejected in favor of auto-at-launch (D-04).
- Cursor native-Windows confinement — Linux/macOS/WSL-only (v2 deferred anti-feature).
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| SUPP-01 | Operator can demote a running/misbehaving agent on the fly (post-hoc token IL-drop) as a supplementary control, with leak/soundness limits documented (explicitly not a standalone boundary). | Spike-002 mechanism confirmed (§ Demote Mechanism); `OpenProcessToken` + `SetTokenInformation(TokenIntegrityLevel)` path verified in `windows_confinement_model.md`; hook points in `control_loop.rs` / `agent_cli.rs` / `cli.rs` identified. |
| SUPP-02 | Outbound network egress scoped per confined agent — WFP keyed to the agent's identity so each agent's network policy is enforced independently. | `WfpRuntimeActivationRequest.session_sid` in `windows_wfp_contract.rs` verified; `install_wfp_policy_filters` / `remove_wfp_policy_filters` in `nono-wfp-service.rs` verified (§ WFP Per-Agent Keying); hook points in `launch.rs` / `reap.rs` (Drop) identified. |
| SUPP-03 | Second non-Claude engine (GitHub Copilot CLI) ships as a profile; nono-ts reaches parity with nono-py (`confinedRun` + `confine`), proving abstraction across ≥2 engines and ≥2 bindings. | Copilot CLI distribution resolved (§ D-06 Resolution); `nono-ts/Cargo.toml` + `nono-ts/src/lib.rs` read; `nono-py/src/windows_confined_run.rs` reference implementation read in full. |
</phase_requirements>

---

## Summary

Phase 75 is a **composition-over-invention phase** adding three supplementary capabilities on top of the proven Phase 74 daemon foundation. Every primitive already exists in-tree; the delta is wiring and thin new verbs.

**SUPP-01 (demote):** The spike-002 post-hoc IL-drop mechanism (`OpenProcessToken` → `SetTokenInformation(TokenIntegrityLevel, Low)` targeting another same-user process) is proven and already documented in `windows_confinement_model.md`. The implementation is a new `Demote { tenant_id: String }` variant in `ControlRequest` (control_loop.rs), a new `AgentCommands::Demote` in cli.rs/agent_cli.rs, and a call to the Win32 IL-drop path on the tenant's process handle. The demote verb never reaps the agent (D-03); D-03 planning call is whether it also deletes the WFP filter. Leak limits must be documented at the verb.

**SUPP-02 (per-agent WFP egress):** The `nono-wfp-service` already accepts `WfpRuntimeActivationRequest { session_sid: Option<String> }` and installs a `FWPM_CONDITION_ALE_USER_ID`-keyed filter scoped to that SID's security descriptor. The daemon already mints a per-agent `package_sid` (E4 identity, a fresh `S-1-15-2-...` AppContainer SID) at `launch_agent`. The delta is: (1) the daemon sends an activation request to the WFP service's control pipe at launch time supplying `session_sid = package_sid`; (2) the `AgentTenant::Drop` sends a deactivation request to remove the filter on reap. This reuses the existing wire protocol verbatim — no new pipe, no new message type needed; `WfpRuntimeActivationRequest` already carries all needed fields (`request_kind = "activate"` / `"deactivate"`, `session_sid`). The fail-secure check (D-05) must gate launch: if the WFP service pipe is not reachable and the profile uses network scoping, refuse launch.

**SUPP-03a (Copilot CLI engine profile):** GitHub Copilot CLI is distributed as a native standalone executable (`copilot.exe`) — NOT a node.exe-wrapped npm script. The npm package `@github/copilot` installs `copilot.exe` as the primary binary. The WinGet install (`winget install GitHub.Copilot`) and the MSI (`copilot-x64.msi`) both produce a native `copilot.exe` (PE x64, confirmed via GitHub issue #1566 describing it as "a valid signed PE x64 executable"). An `npm-loader.js` is a shim that calls `copilot.exe` via `spawnSync` and exits with its code. Therefore the `windows_interpreters` field in `policy.json` for the Copilot profile should be empty or contain `node.exe` only if the fallback JS path matters — the PRIMARY binary is `copilot.exe` (a native PE), not `node.exe`. **This changes the profile shape from the aider/langchain pattern (python.exe interpreter) to a direct-exe profile.** The Copilot CLI profile in `policy.json` follows the `node-dev` profile pattern (no `windows_interpreters` needed since `copilot.exe` is a native PE, not a node script).

**SUPP-03b (nono-ts parity):** `nono-ts/src/lib.rs` currently exports `JsCapabilitySet`, `JsSandboxState`, `JsQueryContext`, `apply()`, `isSupported()`, `supportInfo()`. It needs two new Windows-cfg-gated exports: `confinedRun(exe, args, allow?, profile?, cwd?, timeout_secs?)` returning a `JsExecResult` (mirrors `ExecResult` in nono-py) and `confine(profile?, allow?, caps?)` (Shape B, re-exec guard). The `nono-ts/Cargo.toml` pin is `nono = { version = "0.33.0" }` — must bump to `0.62`. No other dependency changes. Non-Windows stubs (functions that throw "Windows only") must be exported for the cross-target clippy gate.

**Primary recommendation:** Wire in the stated order — SUPP-02 first (it has the highest security value and is entirely plumbing); SUPP-03a second (profile addition, low risk); SUPP-03b third (new file in sibling repo); SUPP-01 last (demote-only lever, lowest priority of the three). Each has a clear real-Win11 gate.

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Demote verb (SUPP-01) | nono-cli (daemon control_loop + agent_cli) | nono lib (Win32 IL-drop, Windows-cfg-gated) | CLI owns the operator verb and UX; library owns the platform mechanism |
| WFP per-agent filter add/remove (SUPP-02) | nono-wfp-service (elevated) | nono-cli/agent_daemon (daemon client side) | Elevated service owns the WFP kernel surface; daemon is a least-priv client |
| WFP launch gating (D-05) | nono-cli/agent_daemon/launch.rs | — | Daemon launch path is the gating point; already owns all fail-secure launch decisions |
| Copilot CLI engine profile (SUPP-03a) | nono-cli/data/policy.json | nono-cli exe-coverage gate | Profile is data; coverage gate is enforcement |
| nono-ts confinedRun/confine (SUPP-03b) | ../nono-ts/src/lib.rs (Windows-cfg-gated) | nono-ts/Cargo.toml (pin bump) | Binding surface owns the exports; Cargo.toml provides the versioned nono API |

---

## Standard Stack

### Core (unchanged — carry from Phases 71-74)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `windows-sys` | 0.59 (workspace pin — do NOT bump) | Win32 FFI: `OpenProcessToken`, `SetTokenInformation`, WFP control | Every needed API present at 0.59; bumping buys nothing, adds drift risk |
| `nono` (internal) | 0.62.2 | Confinement primitive; `AgentRegistry`; `package_sid_to_string` | Daemon already depends on it; nono-ts pin bump targets this version |
| `tokio` | 1.x (workspace) | Async daemon task; WFP service pipe client call within `launch_agent` | Already used throughout the daemon |
| `serde_json` | workspace | `WfpRuntimeActivationRequest` serialization to WFP control pipe | Already used in `windows_wfp_contract.rs` |
| `napi` / `napi-derive` / `napi-build` | 2 (napi9 feature — keep; DO NOT bump to 3) | nono-ts binding surface | nono-ts Cargo.toml already pins napi 2; napi 3 is a breaking migration, out of scope |
| `windows-service` | 0.7 (in-tree) | Already used by `nono-wfp-service`; if daemon binary adds service mode, same version | Align to what nono-wfp-service uses |

### Supporting (net-new for Phase 75)

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `std::os::windows::io::AsRawHandle` | stdlib | Get raw HANDLE from `OwnedHandle` in demote path | SUPP-01: open a process token from the AgentTenant's `process_handle` |
| `FWP_CONDITION_ALE_USER_ID` / `FWP_CONDITION_ALE_PACKAGE_ID` | windows-sys 0.59 | WFP condition field for per-agent SID filter | SUPP-02: keying field decision (see § WFP Per-Agent Keying) |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Reuse `WfpRuntimeActivationRequest.session_sid` for per-agent add/remove | New message type | Existing type has all needed fields (`request_kind`, `session_sid`, `network_mode`, `tcp_connect_ports`); a new type is unjustified complexity in the security-critical path |
| `FWPM_CONDITION_ALE_PACKAGE_ID` for per-agent filter | `FWPM_CONDITION_ALE_USER_ID` + SID-scoped SD | See § WFP Per-Agent Keying — recommendation is to stay with `ALE_USER_ID` + SID-scoped SD (already wired in the service) for minimal delta |
| `copilot.exe` direct-exe profile | `node.exe` + copilot script interpreter | Copilot CLI is a native PE (verified); no `windows_interpreters` needed; `node-dev` profile pattern is the template |

**Installation (nono-ts pin bump only):**
```toml
# ../nono-ts/Cargo.toml — only change in nono-ts
nono = { version = "0.62" }   # was "0.33.0"
```

No new crates. No new features to add to `windows-sys`. No external npm installs.

---

## Package Legitimacy Audit

> Phase 75 installs NO new external packages. The only Cargo.toml change is bumping the internal `nono` pin from `0.33.0` to `0.62` in `../nono-ts/Cargo.toml`. No registry legitimacy audit is required for an internal path-dep version bump.

**Packages removed due to slopcheck [SLOP] verdict:** none — no new packages.
**Packages flagged as suspicious [SUS]:** none.

---

## D-06 Resolution: GitHub Copilot CLI Distribution

**Finding:** GitHub Copilot CLI (`copilot.exe`) is a **native standalone PE x64 executable**, NOT a node.exe-wrapped npm script. [VERIFIED: GitHub release artifacts + GitHub issue #1566]

**Evidence:**
- Release `v1.0.62` (2026-06-13, latest) ships `copilot-win32-x64.zip` and `copilot-x64.msi` containing a native `copilot.exe` [VERIFIED: `gh release view --repo github/copilot-cli`]
- GitHub issue #1566 title: "Native copilot.exe binary silently exits with code 1 on Windows x64, npm-loader.js never falls back to JS implementation" — confirms `copilot.exe` is "a valid signed PE x64 executable" and `npm-loader.js` is a shim that calls `copilot.exe` via `spawnSync`, then exits unconditionally with the binary's exit code [CITED: github.com/github/copilot-cli/issues/1566]
- npm package `@github/copilot` installs this same native binary; the `.cmd` shim in `%AppData%\npm\` calls the binary [CITED: npmjs.com/package/@github/copilot]
- WinGet: `winget install GitHub.Copilot` installs the MSI (same `copilot.exe`) [CITED: docs.github.com/en/copilot/how-tos/copilot-cli/set-up-copilot-cli/install-copilot-cli]
- Node.js 22+ is listed as a prerequisite for the npm install path — but only because `npm-loader.js` is a Node script wrapper; the actual `copilot.exe` is native and does not invoke `node.exe` at runtime [CITED: GitHub Copilot CLI install docs]

**Profile shape implications:**
- `windows_interpreters` should be ABSENT or empty in the `copilot-cli` profile (no interpreter process, `copilot.exe` IS the engine binary)
- The profile follows the `node-dev` / bare-exe pattern: `"windows_low_il_broker": true`, NO `windows_interpreters` field
- E1 coverage: grant the directory containing `copilot.exe` (e.g. `%LOCALAPPDATA%\Programs\GitHub Copilot\` for MSI install, or `%AppData%\npm\` for npm global)
- The executable path must be confirmed on the actual Win11 test host before finalizing; install-path varies by install method [ASSUMED: exact install path — confirm via `where copilot` on test host]

**Recommended profile skeleton (policy.json):**
```json
"copilot-cli": {
  "extends": "default",
  "meta": {
    "name": "copilot-cli",
    "version": "1.0.0",
    "description": "GitHub Copilot CLI (copilot.exe native PE engine)",
    "author": "nono-project"
  },
  "security": {
    "groups": [],
    "signal_mode": "isolated"
  },
  "filesystem": {},
  "network": { "block": false },
  "workdir": { "access": "readwrite" },
  "windows_low_il_broker": true
}
```
Note: no `"windows_interpreters"` field because `copilot.exe` is a native PE.

---

## Architecture Patterns

### System Architecture Diagram

```
SUPP-01: Demote flow
  operator → nono agent demote <tenant_id>
      ↓ agent_cli.rs (windows_control_pipe_request → ControlRequest::Demote)
      ↓ control_loop.rs → handle_demote(state, tenant_id)
      ↓ lookup AgentTenant.process_handle → OpenProcessToken(TOKEN_ADJUST_DEFAULT)
      ↓ SetTokenInformation(TokenIntegrityLevel, Low)    ← spike-002 Win32 path
      ↓ OPTIONAL: send WFP deactivation request (D-03 planning call)
      ↓ return status to operator (demote only; agent NOT reaped)

SUPP-02: Per-agent WFP egress (auto at launch / remove at reap)
  daemon launch_agent() [launch.rs]
      ↓ after job/AppContainer creation, before ResumeThread:
      ↓ if profile has network_scoping AND wfp-service reachable:
          send WfpRuntimeActivationRequest {
            request_kind: "activate",
            session_sid: Some(package_sid),
            network_mode: "blocked",   // or per-profile allowed ports
            tcp_connect_ports: [...],
            ... }
          over \\.\pipe\nono-wfp-control (Windows named pipe, existing)
      ↓ ELSE if profile has network_scoping AND wfp-service NOT reachable:
          return Err (D-05 fail-secure; never silently launch)
      ↓ AgentTenant::Drop (reap.rs)
          send WfpRuntimeActivationRequest { request_kind: "deactivate", session_sid: ... }
          → nono-wfp-service removes SID-keyed filter

SUPP-03a: Copilot CLI engine profile
  policy.json "copilot-cli" profile (native PE, no windows_interpreters)
      ↓ exe-coverage gate (E1: copilot.exe directory must be in allowed paths)
      ↓ nono agent launch --profile copilot-cli -- copilot [args]
      ↓ Phase 71 engine-neutral launch path (BrokerLaunchNoPty arm)
      ↓ confined copilot.exe child (AppContainer + Job)

SUPP-03b: nono-ts confinedRun / confine
  ../nono-ts/src/lib.rs
      ↓ #[cfg(target_os = "windows")] mod windows_confined_run;
          confinedRun(exe, args, allow?, profile?, cwd?, timeout_secs?) → JsExecResult
          confine(profile?, allow?, caps?) → void (re-exec guard, spawn+exit)
      ↓ #[cfg(not(target_os = "windows"))]
          confinedRun(...) → throw Error("confinedRun is Windows-only")
          confine(...) → throw Error("confine is Windows-only")
      ↓ nono pin: 0.33.0 → 0.62 in Cargo.toml
```

### Recommended Project Structure Changes

```
crates/nono-cli/src/agent_daemon/
├── control_loop.rs        # ADD: ControlRequest::Demote { tenant_id } + handle_demote()
├── launch.rs              # ADD: wfp_filter_add() call after AppContainer creation
└── reap.rs                # ADD: wfp_filter_remove() call in AgentTenant::Drop

crates/nono-cli/src/
├── agent_cli.rs           # ADD: AgentCommands::Demote handler
└── cli.rs                 # ADD: AgentCommands::Demote { tenant_id: String }

crates/nono-cli/data/
└── policy.json            # ADD: "copilot-cli" profile (native PE, no windows_interpreters)

../nono-ts/src/
├── lib.rs                 # ADD: confinedRun / confine exports (Windows-cfg-gated)
└── windows_confined_run.rs   # NEW: Shape A + Shape B, mirrors nono-py
../nono-ts/Cargo.toml     # BUMP: nono 0.33.0 → 0.62
```

---

## Demote Mechanism

**Win32 path (spike-002, PARTIAL — proven on Win11 26200.8390):**

```rust
// windows-cfg-gated; uses windows-sys 0.59 (no new imports needed)
//
// SAFETY: caller must hold a valid OwnedHandle to the tenant's process.
// TOKEN_ADJUST_DEFAULT covers integrity label changes on same-user processes.
fn demote_tenant_il(process_handle: HANDLE) -> nono::Result<()> {
    let mut token: HANDLE = std::ptr::null_mut();
    // SAFETY: process_handle is valid; TOKEN_ADJUST_DEFAULT | TOKEN_QUERY is
    // sufficient for SetTokenInformation(TokenIntegrityLevel).
    let ok = unsafe {
        OpenProcessToken(process_handle, TOKEN_ADJUST_DEFAULT | TOKEN_QUERY, &mut token)
    };
    if ok == 0 { return Err(/* GLE */); }
    let _token_guard = TokenGuard(token);

    let mut low_label: TOKEN_MANDATORY_LABEL = unsafe { std::mem::zeroed() };
    let mut low_sid: [u8; SECURITY_MAX_SID_SIZE as usize] = [0; SECURITY_MAX_SID_SIZE as usize];
    let mut sid_size = SECURITY_MAX_SID_SIZE;
    // SAFETY: WinLowLabelSid (9) is the documented constant for Low Integrity.
    unsafe {
        CreateWellKnownSid(WinLowLabelSid, std::ptr::null_mut(), low_sid.as_mut_ptr().cast(), &mut sid_size)
    };
    low_label.Label.Sid = low_sid.as_mut_ptr().cast();
    low_label.Label.Attributes = SE_GROUP_INTEGRITY;

    // SAFETY: low_label is a valid TOKEN_MANDATORY_LABEL pointing to a valid SID.
    let ok = unsafe {
        SetTokenInformation(
            token,
            TokenIntegrityLevel,
            &low_label as *const TOKEN_MANDATORY_LABEL as *mut _,
            std::mem::size_of::<TOKEN_MANDATORY_LABEL>() as u32
                + GetLengthSid(low_label.Label.Sid),
        )
    };
    if ok == 0 { return Err(/* GLE */); }
    Ok(())
}
```

**Hook point in daemon:** `control_loop.rs` → new `handle_demote(state, tenant_id)`:
1. Lock `state.tenants`, look up `AgentTenant` by `tenant_id`.
2. Clone or borrow `process_handle.as_raw_handle()` as HANDLE.
3. Call `demote_tenant_il(handle)`.
4. Return status string to operator.
5. Do NOT remove from tenant map, do NOT close job handle (D-03 — demote is NOT reap).

**D-03 planning call — demote↔WFP composition:** Recommend YES, demote SHOULD also delete the per-agent WFP filter (call `wfp_filter_remove(package_sid)` inside `handle_demote`). Rationale: if the operator is invoking demote as an incident-response lever, severing network egress at the same time is the most useful action; not doing so leaves the (now-IL-demoted) agent with full egress still enabled, which is a security regression. Planner should lock this at planning time.

**Leak limits — must be documented at the `demote` verb:**
1. **Handles opened before drop:** Any file handles, sockets, registry keys, or job handles the agent opened BEFORE the IL-drop continue to function at Medium IL (open-time access check; the check is not re-evaluated on IL-drop). [VERIFIED: spike-002 finding, `windows_confinement_model.md`]
2. **Already-started children:** Child processes spawned before the IL-drop are NOT retroactively affected; they continue at their original IL. [VERIFIED: spike-002]
3. **Legitimate handles also severed:** IL-drop at Low means the agent can no longer open Medium-IL resources it was legitimately using (e.g. its own profile directory if not relabeled Low). This may cause the agent to crash or malfunction. The operator must accept this. [VERIFIED: spike-002]
4. **Network not auto-covered (without D-03 WFP-cut):** The IL-drop alone does not block outbound network; existing sockets remain open; new TCP connections to Medium-IL ports may fail but WFP-enforced blocking requires the separate SUPP-02 filter. [VERIFIED: spike-002]
5. **Demote is one-way:** There is no API to raise a running process's IL back to Medium from outside. [ASSUMED: consistent with all Win32 IL documentation; no recovery API documented]

---

## WFP Per-Agent Keying

### Existing service wiring (confirmed by reading `nono-wfp-service.rs`)

The elevated `nono-wfp-service` at `\\.\pipe\nono-wfp-control` already:
- Accepts `WfpRuntimeActivationRequest` with optional `session_sid: Option<String>` field [VERIFIED: `windows_wfp_contract.rs` line 14]
- `install_wfp_policy_filters` branches: if `session_sid` is Some → calls `sid_to_security_descriptor(sid_str)` → installs a `FWPM_CONDITION_ALE_USER_ID` + `FWP_SECURITY_DESCRIPTOR_TYPE` filter keyed to that SID [VERIFIED: `nono-wfp-service.rs` lines 1557–1589]
- `remove_wfp_policy_filters` removes by deterministic GUID key derived from the request fields [VERIFIED: lines 1591–1620]
- CONTROL_PIPE_SDDL: `"D:(A;;GA;;;SY)(A;;GA;;;BA)(A;;GRGW;;;IU)(A;;GRGW;;;OW)"` — interactive users (IU) have read/write access, so the daemon (running as the current user) can write requests to it [VERIFIED: `nono-wfp-service.rs` line 56]

### WFP condition field recommendation

**Use `FWPM_CONDITION_ALE_USER_ID` + SID-scoped security descriptor** (the existing path) rather than introducing `FWPM_CONDITION_ALE_PACKAGE_ID`.

Rationale:
- `FWPM_CONDITION_ALE_PACKAGE_ID` matches the AppContainer package SID and IS available in windows-sys 0.59 and WFP on Win8+. It would be cleaner for per-agent scoping because the package SID is unique per agent AND distinct from the user SID.
- However, the existing `install_wfp_policy_filters` already has the `session_sid` → `ALE_USER_ID` + SD path working and tested in production (Phase 62). Adding a `FWPM_CONDITION_ALE_PACKAGE_ID` branch is net-new WFP filter logic that has NOT been tested in-tree.
- Since the daemon's per-agent `package_sid` is a unique `S-1-15-2-...` SID (NOT the user SID), passing it as `session_sid` in `WfpRuntimeActivationRequest` will produce a filter that matches only that specific AppContainer SID. The `ALE_USER_ID` condition with a SID-scoped SD containing an AppContainer SID IS the correct per-agent filter — it matches only traffic from the process with that package SID in its token.
- **Verdict: pass `package_sid` as `session_sid` in `WfpRuntimeActivationRequest`. No code changes needed in the WFP service. The daemon sends the per-agent package SID; the service installs an `ALE_USER_ID`-keyed filter scoped to that SID. One agent's allowed domain cannot leak to another because each SID is unique.**
- [ASSUMED: the ALE_USER_ID condition correctly matches AppContainer SIDs (i.e., it is not user-SID-only). Verify on the Win11 test host; if ALE_USER_ID does not match AppContainer tokens, switch to FWPM_CONDITION_ALE_PACKAGE_ID.]

### Daemon→WFP-service control-pipe message shape

**No new message type needed.** `WfpRuntimeActivationRequest` has all required fields:

```rust
// At launch (in launch_agent, after job assignment, before ResumeThread):
let wfp_req = WfpRuntimeActivationRequest {
    protocol_version: WFP_RUNTIME_PROTOCOL_VERSION,  // = 1
    request_kind: "activate".to_string(),
    network_mode: "blocked".to_string(),  // or per-profile; "blocked" = deny-all-then-allow
    preferred_backend: "wfp".to_string(),
    active_backend: "wfp".to_string(),
    runtime_target: "".to_string(),         // not needed with session_sid
    tcp_connect_ports: vec![443, 80],       // example; from profile network config
    tcp_bind_ports: vec![],
    localhost_ports: vec![],
    target_program_path: None,              // not needed with session_sid
    session_sid: Some(package_sid.clone()), // the E4 per-agent AppContainer SID
    outbound_rule_name: Some(format!("nono-agent-{tenant_id}")),
    inbound_rule_name: Some(format!("nono-agent-{tenant_id}-in")),
};
// Serialize + send over \\.\pipe\nono-wfp-control (existing framed JSON protocol)

// At reap (in AgentTenant::Drop or wfp_filter_remove called from reap task):
let deactivate_req = WfpRuntimeActivationRequest {
    request_kind: "deactivate".to_string(),
    session_sid: Some(package_sid.clone()),
    outbound_rule_name: Some(format!("nono-agent-{tenant_id}")),
    inbound_rule_name: Some(format!("nono-agent-{tenant_id}-in")),
    ..zeroed_fields
};
```

**Hook points:**
- **Add at launch:** `launch.rs::launch_agent()` — after `assign_process_to_agent_job` and before `ResumeThread` (step 6→7 in the existing sequence). D-05 fail-secure gate: if `wfp_filter_add(package_sid)` returns Err, terminate the suspended process and return Err — never resume an agent whose WFP scope cannot be enforced.
- **Remove at reap:** Either in `AgentTenant::Drop` (Step 3, after `DeleteAppContainerProfile`) or in the reap task after `tenants.remove()`. The Drop approach ensures cleanup even on daemon crash (though the daemon crash also kills the agent via `KILL_ON_JOB_CLOSE`, so the WFP filter becomes stale until the WFP service's startup sweep removes it). Both approaches are acceptable; the startup sweep in `nono-wfp-service` provides the safety net.
- **WFP service availability check:** Before the activate call, probe `\\.\pipe\nono-wfp-control` with a short timeout. If the service is absent AND the profile's `network.block` is true or `network_mode` is not `allow-all`, return Err (D-05).

---

## nono-ts Parity Shape

### Current exports (confirmed from `nono-ts/src/lib.rs`)

`JsCapabilitySet` (allow_path, allow_file, block_network, allow_command, block_command, platform_rule, deduplicate, path_covered, fs_capabilities, is_network_blocked, summary), `JsSandboxState` (from_caps, to_json, from_json, to_caps, net_blocked), `JsQueryContext` (new, query_path, query_network), `apply()`, `isSupported()`, `supportInfo()`. No `confinedRun` or `confine`.

### Net-new exports to add

```typescript
// Type mirrors nono-py ExecResult
interface JsExecResult {
  stdout: Buffer;   // Vec<u8> → napi Buffer
  stderr: Buffer;
  exitCode: number;
}

// Shape A: spawn a confined child via nono.exe run
// Windows-only; non-Windows: throws "confinedRun is Windows-only"
export function confinedRun(
  exe: string,
  args: string[],
  allow?: string[],
  profile?: string,
  cwd?: string,
  timeoutSecs?: number
): JsExecResult;

// Shape B: born-confined re-exec at process startup
// Windows-only; non-Windows: throws "confine is Windows-only"
export function confine(
  profile?: string,
  allow?: string[],
  caps?: CapabilitySet
): void;
```

### Implementation pattern (mirrors nono-py `windows_confined_run.rs` exactly)

- New file `../nono-ts/src/windows_confined_run.rs` with `#![cfg(windows)]`
- `find_nono_exe()`: check `NONO_EXE` env var first, then PATH search for `nono.exe` (same as nono-py)
- `confined_run`: build `Command::new(nono_path).arg("run").arg("--profile").arg(...).arg("--allow").arg(...).arg("--").arg(exe).args(args)`, capture stdout/stderr, return `JsExecResult`
- `confine`: check `NONO_ALREADY_CONFINED=1` guard first; build re-exec command with `NONO_ALREADY_CONFINED=1` in child env; `spawn().wait()`; `std::process::exit(code)`
- Non-Windows stubs in `lib.rs`: `#[cfg(not(target_os = "windows"))]` implementations that return `Err(Error::new(Status::GenericFailure, "confinedRun is Windows-only"))` / same for confine

### napi export shape (Rust side)

```rust
// In lib.rs, Windows-cfg-gated section:
#[cfg(target_os = "windows")]
mod windows_confined_run;

#[napi(object)]
pub struct JsExecResult {
    pub stdout: Vec<u8>,     // napi Buffer
    pub stderr: Vec<u8>,
    pub exit_code: i32,
}

#[napi]
#[cfg(target_os = "windows")]
pub fn confined_run(
    exe: String,
    args: Vec<String>,
    allow: Option<Vec<String>>,
    profile: Option<String>,
    cwd: Option<String>,
    timeout_secs: Option<f64>,
) -> napi::Result<JsExecResult> {
    windows_confined_run::confined_run(exe, args, allow, profile, cwd, timeout_secs)
}

#[napi]
#[cfg(not(target_os = "windows"))]
pub fn confined_run(
    _exe: String, _args: Vec<String>, _allow: Option<Vec<String>>,
    _profile: Option<String>, _cwd: Option<String>, _timeout_secs: Option<f64>,
) -> napi::Result<JsExecResult> {
    Err(napi::Error::new(napi::Status::GenericFailure, "confinedRun is Windows-only"))
}
// Same pattern for confine()
```

### Pin bump discipline

Only one change in `../nono-ts/Cargo.toml`:
```toml
nono = { version = "0.62" }   # was "0.33.0"
```

Check for any `path` dep override if present (the sibling-repo layout may use a path dep in dev); if so, ensure the version field is updated as well. No other pin changes — napi 2 stays, napi-derive 2 stays, napi-build 2 stays.

**Cross-target clippy gate:** Any cfg-gated code in nono-ts (the new `windows_confined_run.rs` and the `#[cfg(not(target_os = "windows"))]` stubs) must pass `cargo clippy --workspace --target x86_64-unknown-linux-gnu` and `--target x86_64-apple-darwin` (CLAUDE.md MUST). Since this is a sibling repo, the implementer must verify this in the nono-ts workspace, not the nono workspace.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| WFP filter add/remove from daemon | Custom WFP FwpmFilterAdd0 calls in the daemon binary | `WfpRuntimeActivationRequest` over `\\.\pipe\nono-wfp-control` | Existing service handles all WFP API complexity, error recovery, and startup sweep; daemon lacks elevation for FwpmFilterAdd0 anyway |
| IL-drop token manipulation | Custom token struct cloning | `OpenProcessToken` + `SetTokenInformation(TokenIntegrityLevel)` + `CreateWellKnownSid(WinLowLabelSid)` | The exact spike-002 proven path; already documented and exercised |
| nono-ts Shape A/B logic | New broker protocol or custom exec | `Command::new(nono_path).arg("run")...` (mirrors nono-py exactly) | nono.exe already handles all OS-level confinement; binding is a thin wrapper |
| Copilot CLI profile coverage determination | Runtime path probing | Static `policy.json` entry with operator-supplied `--allow` path | Same pattern as aider/langchain-python profiles |

**Key insight:** The entire Phase 75 delta is integration and wiring, not mechanism invention. Every confinement primitive, every Win32 API call, and every wire protocol is already in production. The planner should size tasks accordingly — they are all small.

---

## Common Pitfalls

### Pitfall 1: Demote hook point — grabbing the process HANDLE from OwnedHandle across lock boundaries

**What goes wrong:** `AgentTenant.process_handle` is an `OwnedHandle`. The `demote` handler in `control_loop.rs` needs to call Win32 APIs on the raw HANDLE, but must not hold the `tenants` mutex while making blocking Win32 calls (would deadlock if the reap task fires concurrently).

**Why it happens:** Naively holding `state.tenants.lock()` across the `OpenProcessToken` + `SetTokenInformation` calls is tempting. If the reap task concurrently removes the tenant (which also locks `tenants`), there is no deadlock — the two lock acquisitions are sequential, not nested. But holding the lock during a syscall is unnecessarily coarse.

**How to avoid:** Lock `state.tenants`, clone the raw HANDLE via `DuplicateHandle` into a local `OwnedHandle`, then release the lock. Perform the IL-drop on the duplicated handle. This mirrors the existing reap task pattern in `launch.rs::duplicate_process_handle_for_reap()` which is the canonical pattern for this problem.

**Warning signs:** `OpenProcessToken` called inside a `state.tenants.lock()` block.

### Pitfall 2: WFP filter remove in Drop — blocking async context

**What goes wrong:** `AgentTenant::Drop` is called synchronously from within the tokio reap task's `state.tenants.lock().remove()`. Writing to a named pipe from a `Drop` inside a tokio task using the synchronous `std::fs::File` pipe API is fine (it does not block the async executor for long), but using `tokio::net::windows::named_pipe` in a Drop would require `Handle::current()` which may not be available from a `spawn_blocking` context.

**How to avoid:** Use a blocking synchronous `std::fs::OpenOptions::new().read(true).write(true).open(r"\\.\pipe\nono-wfp-control")` call in the Drop for the WFP deactivation request (same as the existing WFP service client in `probe_runtime_activation_mode` uses std::io::Read). OR: fire the WFP deactivation request from the reap TASK (before the `tenants.remove()` that drops the tenant), keeping Drop clean.

**Recommendation:** Prefer firing the WFP deactivation request from the reap task (before Drop), keeping `AgentTenant::Drop` focused on handle cleanup only.

### Pitfall 3: D-05 fail-secure gate ordering in launch_agent

**What goes wrong:** The WFP activation request is sent AFTER `ResumeThread`, meaning the agent runs unconfined for a brief window if the WFP service is unreachable.

**How to avoid:** The WFP activation MUST occur BEFORE `ResumeThread` (between steps 6 and 8 in `launch.rs`). If the activation fails and D-05 requires refusal: terminate the suspended process (same as `terminate_suspended_process` on job-assign failure), clean up state, return Err.

**Warning signs:** Any code path that calls `ResumeThread` before `wfp_filter_add` returns Ok.

### Pitfall 4: nono-ts cross-target clippy on sibling repo

**What goes wrong:** The nono-ts repo is NOT a Cargo workspace member of the nono repo. Cross-target clippy in the nono repo does NOT catch clippy errors in `../nono-ts/`. The implementer must run cross-target clippy in the nono-ts directory separately.

**How to avoid:** After adding `windows_confined_run.rs` and the `#[cfg]`-gated stubs, run `cargo clippy --target x86_64-unknown-linux-gnu` and `cargo clippy --target x86_64-apple-darwin` FROM the `../nono-ts/` directory. If the cross-toolchain is not installed, mark the verification PARTIAL per `.planning/templates/cross-target-verify-checklist.md`.

### Pitfall 5: Copilot CLI AppContainer gotcha

**What goes wrong:** `copilot.exe` spawns `node.exe` internally as part of its JS fallback path (`npm-loader.js` → `index.js`). If the confined `copilot.exe` tries to spawn `node.exe` as a subprocess and `node.exe`'s directory is not covered by the policy, the spawn will fail (exe-coverage gate fires).

**How to avoid:** Monitor what child processes the confined `copilot.exe` spawns during the SC3 Win11 UAT. If `node.exe` appears as a grandchild, add `node.exe`'s directory to the `copilot-cli` profile's `windows_interpreters` field. The issue #1566 scenario (native binary exits code 1 → fallback to JS) means the nominal path is `copilot.exe` only, but the fallback path uses node.exe. Test both paths.

### Pitfall 6: WFP filter stale on daemon crash

**What goes wrong:** If the daemon crashes without reaping agents cleanly, the per-agent WFP filters installed by SUPP-02 may remain in the WFP filter table indefinitely, blocking legitimate traffic for a SID that no longer corresponds to any running agent.

**How to avoid:** This is already handled by the existing `nono-wfp-service` startup sweep (`EVENT_ID_SWEEP_COMPLETE`) — the service removes stale filters on startup. Ensure the per-agent filters use deterministic GUIDs (derived from the rule name / SID) that the sweep can identify and reclaim. The existing `build_policy_filter_specs` already uses `spec.key` (a GUID from the rule names); the agent's rule name (`nono-agent-{tenant_id}`) must be stable and derivable from the agent's SID to enable sweep-based cleanup.

---

## Runtime State Inventory

This is not a rename/refactor phase. No stored data, live service config, OS-registered state, secrets, or build artifacts carry Phase 75 identifiers that need migrating.

However, re-assert the carry-forward from Phase 74 (durable operational items):
- **WFP service registration (persistent):** `nono-wfp-service` must be installed and running for SUPP-02 to function. The daemon's D-05 gate checks reachability at launch time; if not installed, a clear error is emitted. No migration needed.
- **AppContainer profiles in HKCU:** Each agent launched creates a `nono.session.<id>` AppContainer profile in `HKCU\SOFTWARE\Classes\Local Settings\Software\Microsoft\Windows\CurrentVersion\AppContainer\Storage`. These are cleaned up by `DeleteAppContainerProfile` in `AgentTenant::Drop`. No Phase 75 specific state.

---

## Validation Architecture

> `workflow.nyquist_validation` status: Not explicitly false in config — treat as enabled.

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in test runner (`cargo test`) |
| Config file | `Makefile` targets (`make test`, `make test-cli`) |
| Quick run command | `cargo test -p nono-cli --target x86_64-pc-windows-msvc` (Windows host) |
| Full suite command | `make ci` (clippy + fmt + tests; requires Windows host for nono-cli) |
| nono-ts | `npm test` or `node_modules/.bin/jest` in `../nono-ts/` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| SUPP-01 | `nono agent demote <id>` drops IL of tenant's process to Low; does NOT reap the agent | unit (stub state, skip real IL-drop) + live Win11 UAT | `cargo test -p nono-cli demote` | ❌ Wave 0 |
| SUPP-01 | Demoting a non-existent tenant_id returns a clear error | unit | `cargo test -p nono-cli demote_unknown_tenant` | ❌ Wave 0 |
| SUPP-01 | Leak limits paragraph is present in `nono agent demote --help` or verb output | static / unit | inspect help text | ❌ Wave 0 |
| SUPP-02 | WFP filter added at agent launch (profile with network_scoping) | unit (mock wfp-service pipe) + live Win11 UAT | `cargo test -p nono-cli wfp_filter_add_at_launch` | ❌ Wave 0 |
| SUPP-02 | WFP filter removed at agent reap | unit (mock wfp-service pipe) | `cargo test -p nono-cli wfp_filter_remove_at_reap` | ❌ Wave 0 |
| SUPP-02 | D-05: launch refuses with actionable error when wfp-service absent and profile has network_scoping | unit (service pipe unreachable) | `cargo test -p nono-cli wfp_absent_fail_secure` | ❌ Wave 0 |
| SUPP-02 | D-05: launch proceeds when wfp-service absent and profile has `network: { block: false }` (no network scoping) | unit | `cargo test -p nono-cli wfp_absent_no_scoping_ok` | ❌ Wave 0 |
| SUPP-03a | `copilot-cli` profile present in policy.json with `windows_low_il_broker: true` and no `windows_interpreters` for the native PE path | unit (policy parsing test) | `cargo test -p nono-cli copilot_cli_profile_present` | ❌ Wave 0 |
| SUPP-03a | Copilot CLI confined end-to-end on real Win11 host (SC3 UAT gate) | live Win11 UAT | manual: `nono agent launch --profile copilot-cli -- copilot ask "hello"` | N/A — UAT |
| SUPP-03b | nono-ts: `confinedRun` is exported and callable on Windows | unit (in nono-ts repo) | `cargo test --target x86_64-pc-windows-msvc` in `../nono-ts/` | ❌ Wave 0 |
| SUPP-03b | nono-ts: `confine` is exported and callable on Windows | unit | same | ❌ Wave 0 |
| SUPP-03b | nono-ts: non-Windows stubs throw `"confinedRun is Windows-only"` | cross-target unit | `cargo test --target x86_64-unknown-linux-gnu` in `../nono-ts/` | ❌ Wave 0 |
| SUPP-03b | nono-ts `confinedRun` confines a node/JS process on real Win11 (SC5 UAT) | live Win11 UAT | manual: TS test script calling `confinedRun` | N/A — UAT |
| SUPP-03b | Cross-target clippy (Linux + macOS) green on nono-ts | static | `cargo clippy --target x86_64-unknown-linux-gnu` in `../nono-ts/` | N/A — CI |

### Sampling Rate

- **Per task commit:** `cargo test -p nono-cli` (Windows host) for daemon changes; `cargo test` in `../nono-ts/` for binding changes
- **Per wave merge:** `make ci` on nono repo; `npm test` in nono-ts repo
- **Phase gate:** Full suite green + all live Win11 UAT gates (SC3, SC5) before `/gsd:verify-work`

### Wave 0 Gaps

- [ ] `crates/nono-cli/tests/` or inline: `wfp_filter_add_at_launch`, `wfp_filter_remove_at_reap`, `wfp_absent_fail_secure`, `wfp_absent_no_scoping_ok` — mock wfp-service pipe tests
- [ ] Inline in `control_loop.rs` or separate: `demote_returns_ok_for_known_tenant`, `demote_returns_err_for_unknown_tenant`
- [ ] `crates/nono-cli/tests/` or policy test: `copilot_cli_profile_present`
- [ ] `../nono-ts/src/`: `windows_confined_run.rs` with unit tests mirroring nono-py's `test_find_nono_exe_*`
- [ ] `../nono-ts/src/lib.rs`: non-Windows stub test (`confined_run_windows_only_stub`)

---

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | n/a (daemon already authenticated per Phase 74; SUPP-01 adds no new auth surface) |
| V3 Session Management | yes (demote terminates the agent session's effective privilege) | IL-drop is irreversible (no escape hatch); demote is operator-only via Medium-IL control pipe (T-74-07-01 gate) |
| V4 Access Control | yes | SUPP-02: per-agent filter must not bleed to other agents; D-04 privilege split enforced |
| V5 Input Validation | yes | `tenant_id` in Demote request validated against `DaemonState::tenants` (unknown → error, never crash); WFP rule-name derived from `tenant_id` hex string (safe, no injection surface in SDDL) |
| V6 Cryptography | no | demote/WFP use standard Win32; no cryptographic material involved |

### Known Threat Patterns for Phase 75 Stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Agent calls `nono agent demote <own-tenant>` to self-demote and signal compromise | Spoofing | Control pipe SDDL requires Medium IL minimum (`S:(ML;;NW;;;ME)`) — AppContainer/Low-IL agents cannot open the pipe (T-74-07-01). Demote can only come from the interactive operator. |
| Stale WFP filter after daemon crash allows escaped agent to use filtered SID | Elevation of Privilege | WFP service startup sweep removes orphaned filters; deterministic rule-name → GUID derivation enables sweep identification |
| Tenant A sends `demote` request claiming Tenant B's `tenant_id` | Tampering | `handle_demote` looks up `tenant_id` in `DaemonState::tenants` (operator-visible list only); tenant IDs are 32-char random hex; guessing is computationally infeasible |
| WFP activation request carries malicious SID string for SDDL injection | Tampering | `sid_to_security_descriptor` calls `ConvertStringSidToSidW` for validation first; invalid SID strings return early before SDDL construction |
| nono-ts `confine()` infinite re-exec | DoS | `NONO_ALREADY_CONFINED=1` guard (exact string match) checked FIRST in `confine()`; cannot be bypassed by "1extra" (T-72-02-04 carry-forward) |

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Post-hoc IL-drop as primary confinement | IL-drop as demote-only IR lever layered on spawn-time confinement | v2.12 Phase 74 / SUPP-01 | Leak limits documented explicitly; "detect-and-confine" anti-feature owned and rejected |
| Per-session WFP filter (one user-level filter) | Per-agent WFP filter keyed to AppContainer package SID | SUPP-02 (Phase 75) | Agent A's allowed domain cannot leak to Agent B |
| nono-ts binding: CapabilitySet / apply only | nono-ts adds confinedRun + confine (Windows-cfg-gated) | SUPP-03b (Phase 75) | TypeScript/Node agents can be confined without spawning the full CLI separately |

**Deprecated/outdated:**
- `gh copilot` extension: deprecated 2025-10-25; replaced by standalone GitHub Copilot CLI `copilot.exe`. Do NOT profile the extension. [CITED: inventivehq.com/knowledge-base/copilot/how-to-migrate-from-gh-copilot]

---

## Project Constraints (from CLAUDE.md)

- **Unwrap policy:** `.unwrap()` and `.expect()` strictly forbidden in non-test production code; `clippy::unwrap_used` enforced. Use `map_err`, `?`, and explicit error returns.
- **Error handling:** All errors via `NonoError` and `?` propagation. No `panic!` in library or daemon code.
- **Path security:** Validate and canonicalize paths; use `Path::starts_with()` (not string `.starts_with()`).
- **Unsafe code:** Restrict to Win32 FFI; all unsafe blocks must have `// SAFETY:` doc comments.
- **Arithmetic:** Use `checked_*`/`saturating_*` for any security-critical math.
- **Memory:** `zeroize` for sensitive data (not applicable for SUPP-01/02/03 — no keys).
- **DCO sign-off:** All commits: `Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>`.
- **Cross-target clippy MUST:** Any cfg-gated Unix code touched MUST be verified via `cargo clippy --workspace --target x86_64-unknown-linux-gnu` AND `--target x86_64-apple-darwin`. For nono-ts, run from the `../nono-ts/` directory. Mark PARTIAL and defer to CI if cross-toolchain not installed.
- **No kernel driver:** User-mode only. ADR-65 No-go.
- **Fail-secure default:** Any coverage/auth/service-reachability error → deny. Never silently degrade.
- **Library-vs-CLI boundary:** Win32 demote mechanism belongs in a `crate::sandbox::windows` function or inline in `agent_daemon`; the verb and UX belong in `nono-cli`. The actual IL-drop helper can live in the daemon module since the daemon binary already uses raw Windows APIs directly (see `launch.rs`).
- **GSD workflow enforcement:** Use `/gsd:quick`, `/gsd:debug`, or `/gsd:execute-phase` entry points before making file edits.

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `FWPM_CONDITION_ALE_USER_ID` with a SID-scoped SD matching an AppContainer package SID (`S-1-15-2-...`) correctly filters traffic from that AppContainer to that SID only. | WFP Per-Agent Keying | If ALE_USER_ID does not match AppContainer tokens (it may only match user tokens), per-agent isolation fails; switch to `FWPM_CONDITION_ALE_PACKAGE_ID`. Verify on Win11 test host. |
| A2 | `copilot.exe` install path is `%LOCALAPPDATA%\Programs\GitHub Copilot\copilot.exe` for MSI install and `%AppData%\npm\copilot.cmd` + underlying PE for npm global install. | D-06 Resolution | If the actual path differs, the E1 coverage grant in the copilot-cli profile points to the wrong directory. Verify via `where copilot` on test host. |
| A3 | The `nono-wfp-service` startup sweep can identify and remove per-agent WFP filters created by SUPP-02 based on the deterministic rule-name → GUID derivation. | WFP Per-Agent Keying / Pitfall 6 | If the sweep cannot identify per-agent filters (e.g. GUIDs are random not deterministic), stale filters accumulate after daemon crash. The sweep must be verified to cover the SUPP-02 filter naming scheme. |
| A4 | `copilot.exe` in its nominal code path (no JS fallback) does not spawn `node.exe` as a subprocess. | D-06 Resolution / Pitfall 5 | If copilot.exe spawns node.exe routinely, the copilot-cli profile needs `windows_interpreters: ["node.exe"]`. Discovered during SC3 Win11 UAT subprocess monitoring. |
| A5 | `demote` is one-way — no Win32 API exists to raise a running process's IL back from Low to Medium without spawning a new process. | Demote Mechanism | If a reverse-demote API exists, the security model documented in leak-limits is incomplete. (Training knowledge: no such reverse API documented; this is standard Win32 behavior.) |

---

## Open Questions (RESOLVED)

1. **A1: ALE_USER_ID vs ALE_PACKAGE_ID empirical verification** — **RESOLVED: accepted as a monitored assumption with a UAT gate.**
   - What we know: The WFP service currently uses `FWPM_CONDITION_ALE_USER_ID` + SID-scoped SD with `session_sid`. The Phase 62 spike validated this blocks traffic for the SID. The Phase 62 spike used the user's own SID (not an AppContainer package SID), so the ALE_USER_ID + AppContainer SID combo has NOT been directly tested in-tree.
   - What's unclear: Does `FWPM_CONDITION_ALE_USER_ID` match on AppContainer SIDs in the token, or only on the user SID?
   - **RESOLUTION:** Plan 75-05 (SC2 / A1 gate) is the empirical check on Win11 — launch two agents with different allowed domains and confirm Agent A's domain does not work from Agent B's process. If `FWPM_CONDITION_ALE_USER_ID` does not match AppContainer SIDs, a gap-closure plan (75-06) switches the keying field to `FWPM_CONDITION_ALE_PACKAGE_ID`. The assumption is accepted for planning; the live gate de-risks it before phase sign-off.

2. **D-03 planning decision: does demote also delete the WFP filter?** — **RESOLVED: YES.**
   - What we know: D-03 leaves this to planning. Research recommendation is YES (see § Demote Mechanism rationale).
   - **RESOLUTION:** Locked in plan 75-02 — `handle_demote` DOES call `wfp_filter_remove(package_sid)` after a successful IL-drop, since leaving egress open after an IL-drop is a security regression. WFP-cut failure on demote is non-fatal (logged warning; the demote itself still succeeds).

3. **WFP filter GUID scheme for daemon-originated filters** — **RESOLVED: deferred to executor read-first of `build_policy_filter_specs`.**
   - What we know: The service's `build_policy_filter_specs` uses `spec.key` (a GUID field); the GUID is derived from `outbound_rule_name`/`inbound_rule_name` in the request. The rule names for SUPP-02 are `nono-agent-{tenant_id}` (unique per agent).
   - **RESOLUTION:** Plan 75-01's `<read_first>` mandates reading `build_policy_filter_specs` in `nono-wfp-service.rs` to confirm the GUID derivation scheme so the deactivation request reconstructs the same GUIDs. Acceptance criterion: the deactivate request removes exactly the filter the activate request installed (verified by the `wfp_filter_remove_at_reap` test). No daemon-side GUID storage is needed if derivation is deterministic from the rule name.

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Windows 10/11 (real host) | SUPP-01 demote (OpenProcessToken), SUPP-02 WFP, SC3 Copilot UAT, SC5 nono-ts UAT | ✓ (per project memory) | Win11 26200 | — (no fallback; UAT gates require real host) |
| `nono-wfp-service` installed + running | SUPP-02 at runtime | Must verify at test time | — | D-05 fail-secure: launch is refused, not silently degraded |
| `copilot.exe` installed | SC3 Win11 UAT | Must install on test host | v1.0.62 (latest) | — |
| Node.js 22+ | copilot.exe npm install only | Must verify on test host | — | WinGet install of copilot.exe bypasses node.js requirement |
| `@napi-rs/cli` | nono-ts build | Must be in nono-ts dev dependencies | in-tree | — |
| Cross-compilation toolchain (Linux/macOS targets) | Cross-target clippy | Likely absent on Win11 host | — | Mark PARTIAL, defer to CI (per CLAUDE.md cross-target rule) |

**Missing dependencies with no fallback:**
- Real Win11 host required for SC3 and SC5 UAT gates (already the case per project setup)

**Missing dependencies with fallback:**
- Cross-compilation toolchain: defer to CI cross-target clippy run (PARTIAL per checklist)

---

## Sources

### Primary (HIGH confidence)
- In-tree (authoritative, read in this session): `crates/nono-cli/src/agent_daemon/control_loop.rs` (ControlRequest enum, framing, CONTROL_PIPE_SDDL), `crates/nono-cli/src/agent_daemon/launch.rs` (launch_agent sequence, hook points), `crates/nono-cli/src/agent_daemon/reap.rs` (AgentTenant, Drop impl), `crates/nono-cli/src/agent_daemon/mod.rs` (DaemonState), `crates/nono-cli/src/agent_cli.rs` (AgentCommands), `crates/nono-cli/src/cli.rs` (AgentCommands enum — no Demote variant yet), `crates/nono-cli/src/bin/nono-wfp-service.rs` (CONTROL_PIPE_NAME, PIPE_SDDL, sid_to_security_descriptor, install_wfp_policy_filters, remove_wfp_policy_filters), `crates/nono-cli/src/windows_wfp_contract.rs` (WfpRuntimeActivationRequest fields), `crates/nono-cli/data/policy.json` (aider/langchain-python profiles with windows_interpreters, node-dev profile pattern), `../nono-py/src/windows_confined_run.rs` (complete Shape A + B reference), `../nono-ts/src/lib.rs` (current exports surface), `../nono-ts/Cargo.toml` (nono = "0.33.0", napi 2)
- `.claude/skills/spike-findings-nono/references/windows-confinement-model.md` — spike-002 PARTIAL: demote mechanism proven; leak limits documented
- `.planning/phases/75-supplementary-controls-secondary-engines/75-CONTEXT.md` — all decisions (D-01 through D-08)
- `.planning/REQUIREMENTS.md` — SUPP-01/02/03 requirement text
- `proj/DESIGN-engine-abstraction.md` — E1–E5 contract, E4 = AppContainer package SID
- `.planning/research/SUMMARY.md`, `STACK.md`, `ARCHITECTURE.md`, `PITFALLS.md` — milestone research

### Secondary (MEDIUM confidence)
- GitHub release API: `gh release view --repo github/copilot-cli` — confirmed v1.0.62 (2026-06-13), Windows artifacts `copilot-win32-x64.zip`, `copilot-x64.msi`
- GitHub issue #1566 `github/copilot-cli`: "Native copilot.exe binary silently exits with code 1... it is a valid signed PE x64 executable" — confirmed native PE, npm-loader.js shim pattern [CITED]
- GitHub changelog 2026-01-14: `@github/copilot` npm package, `winget install GitHub.Copilot` [CITED]
- GitHub docs: `docs.github.com/en/copilot/how-tos/copilot-cli/set-up-copilot-cli/install-copilot-cli` — install methods [CITED]
- `inventivehq.com/knowledge-base/copilot/how-to-migrate-from-gh-copilot` — `gh copilot` extension deprecated 2025-10-25 [CITED]

### Tertiary (LOW confidence — flagged)
- A3 (WFP startup sweep covers SUPP-02 filters): inferred from reading `build_policy_filter_specs` structure; the exact GUID-derivation scheme was not fully traced in this session. Verify during planning.

---

## Metadata

**Confidence breakdown:**
- SUPP-01 demote mechanism: HIGH — spike-002 proven Win32 path; hook points confirmed in code
- SUPP-02 WFP keying: HIGH for existing service path; MEDIUM for A1 (ALE_USER_ID + AppContainer SID matching — not empirically tested with AppContainer SIDs in nono in-tree)
- SUPP-03a Copilot CLI distribution: HIGH — confirmed via GitHub release API + issue #1566
- SUPP-03b nono-ts parity shape: HIGH — nono-py reference read in full; nono-ts current surface read in full; napi 2 pattern confirmed

**Research date:** 2026-06-15
**Valid until:** 2026-07-15 (stable, composition-over-invention phase; only risk is Copilot CLI changing distribution method, which can be re-verified quickly)
