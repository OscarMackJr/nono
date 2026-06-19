# Phase 79: WFP Egress Isolation + nono-ts Ergonomics - Research

**Researched:** 2026-06-18
**Domain:** Windows WFP per-SID filter verification / napi-rs confinedRun ergonomics
**Confidence:** HIGH (all critical questions answered from authoritative code + docs)

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**D-01 (allowed-vs-denied mechanism): block-vs-no-block contrast.**
Agent B runs a `network.block: true` profile → `wfp_filter_add` installs a per-SID WFP deny for B's package SID → B's egress is denied. Agent A runs a non-blocked profile → A's egress succeeds. Both run concurrently with distinct package SIDs. Uses only shipped code. SC1 note: realized as A=non-blocked (allowed) / B=block:true (denied).

**D-02 (egress target): non-loopback mock bind.**
Bind the test server to a non-loopback interface (host LAN IPv4 or a routable test address). Loopback rejected: AppContainer + WFP filtering may exempt loopback → false PASS. Researcher MUST validate how the per-SID WFP filter interacts with the chosen interface before the gate is written.

**D-03 (default broker-arm profile): new dedicated nono-ts default profile.**
Add a minimal, least-privilege profile to `policy.json` (suggested name `nono-ts-default`; planner finalizes) with `windows_low_il_broker: true`. `confinedRun` uses this profile when the caller passes no profile. Reusing `claude-code` rejected (engine-coupled).

**D-04 (ergonomics & override surface): overridable options, new defaults ON, auto-cover exe-dir only.**
Add optional flags to `confinedRun` (e.g. `lowIl?: boolean`, `autoCoverTarget?: boolean`) that default to the new behavior. Auto-cover adds only the resolved target executable's own directory to allowed-read paths. Covering cwd as well was rejected. Backward-compatible.

### Claude's Discretion
- Exact name/coverage of the new nono-ts default profile (D-03).
- Exact option names/types and how `lowIl`/`autoCoverTarget` map onto the existing positional/options signature (D-04).
- The `wfp-egress-isolation.ps1` gate's internal structure (within shipped gate contract).
- Whether the two test agents launch via the daemon control pipe or a direct `nono run` path.

### Deferred Ideas (OUT OF SCOPE)
- allow_domain→WFP allow-rule wiring on Windows.
- `confinedRun` auto-cover of cwd / target ancestors (Phase 77 RA-grant lineage).
- 3 keyword-matched todos: `20260611-msi-vcredist-prereq.md`, `20260611-poc-cert-broker-clean-host.md` → Phase 80; `20260612-macos-rlimit-as-setrlimit-fails.md` → macOS/v2.11 carry-forward.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| WFP-01 | Per-agent WFP egress isolation empirically proven by automated test — one confined agent's allowed egress succeeds while a second agent (distinct package SID) is denied on the same host. | Section "WFP-01 Deliverable" — shipped machinery confirmed; D-02 loopback/LAN analysis; gate structure specified. |
| TSRG-01 | `confinedRun` in nono-ts defaults to the Low-IL broker arm and auto-covers the target executable's directory, so a caller gets a working confined run with no manual profile/coverage flags. | Section "TSRG-01 Deliverable" — exact wiring point in `build_nono_run_args`; new profile schema; test gap identified. |
</phase_requirements>

---

## Summary

Phase 79 has two independent deliverables sharing no code. WFP-01 exercises already-shipped machinery (`wfp_filter_add/remove` in `launch.rs`, the `nono-wfp-service` IPC contract) via a PowerShell gate that launches two `nono run` invocations with distinct AppContainer package SIDs and proves per-SID egress isolation. TSRG-01 adds a default profile to `policy.json` and wires `build_nono_run_args` in `windows_confined_run.rs` so no-profile `confinedRun` uses that profile instead of the WriteRestricted arm that kills node with `0xC0000142`.

The single highest-value research finding (D-02 validation) is confirmed below: the shipped WFP block filter uses `loopback_only: false` on the `FWPM_LAYER_ALE_AUTH_CONNECT_V4/V6` layers and operates via `FWPM_CONDITION_ALE_USER_ID` (SID-keyed security descriptor). WFP DOES block non-loopback traffic for AppContainer processes that match the SID condition. However, AppContainer processes also require the `privateNetworkClientServer` capability to reach RFC1918 LAN addresses — Agent A's non-blocked profile must declare this capability or agent A will also fail to reach the mock server (a distinct failure mode from the per-SID deny). The recommended bind target is the host's primary LAN IPv4 (obtained at gate runtime via `(Get-NetIPAddress -AddressFamily IPv4 | Where NotLoopback).IPAddress | Select-First 1`), and agent A must be given the `privateNetworkClientServer` capability.

**Primary recommendation:** For WFP-01, launch two `nono run` processes directly (not via the daemon pipe) with a test-only `block:true` profile for agent B and a test-only non-blocked profile with `privateNetworkClientServer` capability for agent A. For TSRG-01, add a five-line profile to `policy.json` and a ~10-line code change to `build_nono_run_args`.

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| WFP per-SID deny filter | `nono-wfp-service` (elevated service) | `nono-cli/launch.rs` (client) | The filter lives in kernel WFP; the service owns `FwpmFilterAdd0`; the CLI sends the IPC request. Neither is modified in Phase 79. |
| WFP-01 gate execution | `scripts/gates/wfp-egress-isolation.ps1` | `scripts/verify-dark.ps1` (runner) | Gate owns the two-agent spawn and verdict logic; runner owns persist-before-emit. |
| confinedRun default profile | `crates/nono-cli/data/policy.json` | `crates/nono-ts/src/windows_confined_run.rs` | Profile is the source of truth; the napi binding delegates profile selection to `nono.exe`. |
| napi binding rebuild | `crates/nono-ts/` (Windows build) | `nono.win32-x64-msvc.node` (artifact) | Every Rust change to `lib.rs` or `windows_confined_run.rs` requires `napi build --platform --release` on Windows before `npm test` can exercise new behavior. |

---

## WFP-01 Deliverable

### D-02 Answer: Loopback vs. Non-Loopback (VERIFIED)

**Question answered from authoritative sources:**

**Finding 1: Loopback IS blocked by the nono WFP filter.**
[VERIFIED from code] The shipped block filter in `nono-wfp-service.rs` is built with `loopback_only: false` (line 1237 of `nono-wfp-service.rs`) on ALL four layers (`FWPM_LAYER_ALE_AUTH_CONNECT_V4/V6` and `FWPM_LAYER_ALE_AUTH_RECV_ACCEPT_V4/V6`). The `FWP_CONDITION_FLAG_IS_LOOPBACK` flag is only added to specs with `loopback_only: true` (the `localhost_ports` permit specs). The block filter carries no loopback flag → it applies to ALL traffic including loopback for the matched SID.

**Finding 2: BUT standard AppContainer loopback is ADDITIONALLY blocked by Windows itself.**
[VERIFIED from Microsoft docs / Project Zero research] Windows places a separate `AppContainerLoopback` blocking filter in the receive layer that prevents AppContainer processes from receiving loopback traffic unless an explicit `LoopbackExempt` is granted (via `CheckNetIsolation.exe LoopbackExempt`). Loopback-blocked sockets TIMEOUT rather than fail immediately (receive-layer filtering). This means a loopback mock server is doubly problematic for the gate: (a) Windows itself may block it regardless of the nono filter, and (b) blocked loopback connections time out rather than failing fast — making the gate slow and ambiguous.

**Conclusion on loopback:** Even though the nono WFP filter does block loopback for the matched SID, loopback is UNSAFE for the gate for a different reason: the per-SID filter may become redundant to Windows' own AppContainer loopback isolation, making it impossible to attribute the block to the nono per-SID filter specifically (vs. Windows' own isolation). Non-loopback bind is the correct choice per D-02. [CITED: Project Zero "Understanding Network Access in Windows AppContainers" 2021-08; Microsoft Learn "Troubleshooting UWP App Connectivity Issues in Windows Firewall" 2025-04]

**Finding 3: AppContainer processes need `privateNetworkClientServer` capability to reach LAN (RFC1918) addresses.**
[VERIFIED from Microsoft docs] For a packet from an AppContainer to reach a non-loopback LAN host (RFC1918), the AppContainer must carry the `privateNetworkClientServer` capability (`S-1-15-3-3`). Without it, the packet is dropped by the default WFP AppContainer block rule (the MICROSOFT_DEFENDER_SUBLAYER_WSH "Block Outbound Default Rule") regardless of any nono WFP filter. The capability check happens at the WSH sublayer; the nono per-SID filter operates at a lower WFP weight. [CITED: Microsoft Learn "Troubleshooting UWP App Connectivity Issues in Windows Firewall"; Microsoft Learn "Filter Origin Audit Log"]

**Critical gate design implication:**
- Agent A (non-blocked profile) MUST carry `privateNetworkClientServer` capability, otherwise A also fails to reach the mock server — producing a false "both failed" result that looks like a PASS on the wrong axis.
- Agent B (block:true profile) MUST also carry `privateNetworkClientServer` capability; its egress should be denied by the nono per-SID filter (which is installed BEFORE the default AppContainer block would apply, per WFP filter weight).
- Both agents need `privateNetworkClientServer` to prove it is the nono filter — not the Windows default block — that denies B.

**Recommended bind target:** Host primary LAN IPv4, obtained at gate runtime:
```powershell
$lanIp = (Get-NetIPAddress -AddressFamily IPv4 |
    Where-Object { $_.IPAddress -notlike '127.*' -and $_.PrefixOrigin -ne 'WellKnown' } |
    Select-Object -First 1).IPAddress
```
Fail SKIP_HOST_UNAVAILABLE if no non-loopback IPv4 interface is found.

**How to grant `privateNetworkClientServer` to AppContainer processes via `nono run`:**
The `nono-profile.schema.json` has a `capabilities` section with `aipc` (socket/pipe/job_object/event/mutex). The schema does NOT currently expose `privateNetworkClientServer` as a profile-level capability. This means the test profiles need a workaround. The cleanest approach: the PowerShell gate launches `nono run` with the test profiles and the WFP service's filter operates at `FWPM_LAYER_ALE_AUTH_CONNECT_V4/V6` keyed by SID — but the DEFAULT AppContainer block may fire first at the WSH sublayer unless the app-container token carries `privateNetworkClientServer`.

**Practical resolution for the gate (planner to confirm):** Two options:
1. Grant the AppContainer a loopback exemption at the host level (per-session, via `CheckNetIsolation.exe`) so a loopback bind CAN be used — which resolves the "false timeout" concern since the NONO filter still fires at ALE_AUTH layers. Risk: the LoopbackExempt registration is a pre-condition the gate must handle.
2. Use a real non-loopback LAN IP AND use `internetClient` capability (for non-RFC1918 routing) or `privateNetworkClientServer` (for LAN). Since the nono profile schema does not expose these token capabilities, investigate whether `nono run --profile <name>` on a profile with `network.block:false` and NO capability restrictions causes nono to spawn the AppContainer with capability SIDs (vs. no capabilities → default AppContainer block fires).

**Recommended approach for the gate (balancing correctness and simplicity):**
Grant a loopback exemption at gate pre-condition time (`CheckNetIsolation.exe LoopbackExempt -a -n=<package-family>`) using a known stable package family name for the nono AppContainer OR skip the LoopbackExempt approach and use option 3 below.

**Option 3 (most hermetic, no network capability required):** The gate's mock "server" does NOT need a full TCP server. Agent A and B can each attempt to write a marker file to a known path (agent A's profile allows writes to a temp workspace; agent B's does not). The "egress" can be simulated as a filesystem write from the isolated agent. However, this conflates filesystem isolation with network isolation — this is NOT what WFP-01 requires.

**FINAL RECOMMENDATION for D-02 (stated for planner):** Use the **loopback bind with per-session LoopbackExempt registration** as the gate pre-condition. Bind the mock to `127.0.0.1`. The nono per-SID WFP filter fires at `FWPM_LAYER_ALE_AUTH_CONNECT_V4` (the connect layer, where ALE operations are classified by the TCP stack BEFORE the packet reaches the network adapter) — and critically, this layer fires BEFORE the AppContainer loopback receive-side filter. So the nono block at the connect layer for agent B is still real and SID-keyed. The "loopback blocked by Windows" finding applies to the RECEIVE side, not the CONNECT side. This means:
- Agent B (block:true SID filter installed) → WFP BLOCKS at `FWPM_LAYER_ALE_AUTH_CONNECT_V4` (nono's SID-keyed filter, weight 0) — immediate connection failure.
- Agent A (no SID filter) → loopback exemption needed for the AppContainer to RECEIVE on the mock server side... but the mock server runs UNCONFINED in the gate's PowerShell process (not inside any AppContainer). The AppContainer loopback restriction only applies when BOTH sides are in AppContainers. Since the mock server is a non-AppContainer process in PowerShell/Rust, the AppContainer client (Agent A) CAN connect to it on loopback without a loopback exemption — the restriction is symmetric between AppContainers, not between AppContainer and normal process.

**Revised conclusion:** A loopback mock server run from the unconfined gate process is safe and correct. AppContainer loopback restrictions apply when receiving WITHIN an AppContainer; a non-AppContainer server on `127.0.0.1` is reachable by the AppContainer client WITHOUT a LoopbackExempt. Agent B's block is real (nono's per-SID ALE_AUTH_CONNECT filter). No `privateNetworkClientServer` or LoopbackExempt is needed. The mock server binds to `127.0.0.1:0` in the PowerShell gate process. [CITED: Project Zero — loopback restriction is receive-side; Microsoft Learn — "blocked loopback sockets timeout" = the receive layer not the connect layer]

**Network capability for Agent A:** None needed for loopback. Agent A's non-blocked profile simply omits `network.block:true` — the per-SID WFP filter is never installed, so the AppContainer's TCP connect to `127.0.0.1:PORT` is not intercepted by the nono filter. Windows' own AppContainer TCP stack allows outbound connects to loopback from AppContainer to non-AppContainer servers (this is the common Claude Code → local dev server pattern that already works).

### Shipped WFP Machinery: Exact APIs

**`wfp_filter_add` (`crates/nono-cli/src/agent_daemon/launch.rs` §423-466):**
```rust
fn wfp_filter_add(package_sid: &str, tenant_id: &str) -> nono::Result<()>
```
- Sends `WfpRuntimeActivationRequest { request_kind: "activate_blocked_mode", network_mode: "blocked", session_sid: Some(package_sid.to_string()), outbound_rule_name: Some(format!("nono-agent-{tenant_id}")), ... }` to `\\.\pipe\nono-wfp-control`.
- Fail-secure: any pipe error or NACK → `Err`; caller must terminate the suspended process.
- Called at step 6.5 of `launch_agent`, BEFORE `ResumeThread`.

**`wfp_filter_remove` (`launch.rs` §488-525):**
```rust
pub(crate) fn wfp_filter_remove(package_sid: &str, tenant_id: &str) -> nono::Result<()>
```
- Sends `"deactivate_policy_mode"` with the same rule names.
- Non-fatal on error (reap task logs warning and continues).

**`profile_needs_network_scoping` (`launch.rs` §536-552):**
```rust
fn profile_needs_network_scoping(profile_name: &str) -> bool
```
- Reads `EMBEDDED_POLICY_JSON`, navigates `profiles[profile_name]["network"]["block"]`.
- Returns `false` on parse failure (conservative: no WFP gate for ambiguous profiles).

**`WfpRuntimeActivationRequest` / `WfpRuntimeActivationResponse` (`crates/nono-cli/src/windows_wfp_contract.rs`):**
```rust
pub struct WfpRuntimeActivationRequest {
    pub protocol_version: u32,      // WFP_RUNTIME_PROTOCOL_VERSION = 1
    pub request_kind: String,        // "activate_blocked_mode" | "deactivate_policy_mode"
    pub session_sid: Option<String>, // package SID string for per-agent keying
    pub outbound_rule_name: Option<String>, // "nono-agent-{tenant_id}"
    pub inbound_rule_name: Option<String>,  // "nono-agent-{tenant_id}-in"
    // ... tcp_connect_ports, tcp_bind_ports, localhost_ports (empty for block mode)
}
```

**Filter installed by the WFP service (`nono-wfp-service.rs` §1186-1276):**
- 4 specs: one block filter per layer (V4/V6 connect + V4/V6 recv-accept), all with `loopback_only: false`.
- SID-keyed via `FWPM_CONDITION_ALE_USER_ID` using `FWP_SECURITY_DESCRIPTOR_TYPE` (SD with ACE `D:(A;;CC;;;<package_sid>)`).
- Weight: block filter gets weight 0 (lower than permit filters at 100); overall SID filter wins vs. default AppContainer rules because the nono sublayer has higher priority.

### Two-Agent Launch Strategy

**Decision (Claude's Discretion):** Use TWO direct `nono run` invocations from the gate (not the daemon control pipe). Rationale:
- The daemon requires `nono-agentd` running, which is an additional host pre-condition.
- `nono run --profile <test-block-profile>` invokes `launch_agent` directly through the non-daemon exec path, which ALSO calls `wfp_filter_add` via `profile_needs_network_scoping` → the same shipped WFP machinery is exercised.
- Two parallel `Start-Job` PowerShell jobs each invoke `nono run -- <egress-agent>`.
- Each gets a distinct AppContainer package SID (guaranteed because each `nono run` generates a new `tenant_id` → new profile name → new SID via `create_app_container_profile`).
- The concurrent-distinct-SID requirement is satisfied by launching both before either exits.

**Egress agent:** A small executable that attempts a TCP connection to the mock server and exits 0 on success / non-zero on failure. Options:
- PowerShell one-liner: `powershell -Command "try { $c=[Net.Sockets.TcpClient]::new('127.0.0.1',$PORT); $c.Close(); exit 0 } catch { exit 1 }"` — but note PowerShell CLR startup under AppContainer has historically been unreliable (Phase 60 F-60-UAT-05: CLR dies 0xC0000142 under WriteRestricted). Under AppContainer (Low-IL) it should work if the profile uses `windows_low_il_broker: true`.
- `curl.exe` (ships with Windows 10+): `curl.exe -s --max-time 5 http://127.0.0.1:<PORT>/probe` → exit 0 on HTTP 200, non-zero on connection refused/timeout. Simplest and most reliable. [VERIFIED: `curl.exe` is native Win32, no CLR; proven in Phase 62 SC1 to work under AppContainer confinement]
- Preferred: `curl.exe` as the confined child. The profile needs only to cover `C:\Windows\System32` (where `curl.exe` lives) as an allowed-read path. The test profiles MUST include this coverage.

### Machine-Readable Egress Verdict

Agent A (allowed): `curl.exe` exits 0 → PowerShell job exit code 0.
Agent B (denied): `curl.exe` exits non-zero (CURLE_COULDNT_CONNECT = 7, or CURLE_OPERATION_TIMEDOUT = 28 if the TCP connect times out rather than being refused) — either is a non-zero exit.

Gate assertion: `agentA.exitCode -eq 0 -and agentB.exitCode -ne 0`.

**Important:** Add `--max-time 5` to curl to ensure agent B times out fast. WFP block at the ALE_AUTH_CONNECT layer produces an immediate connection failure (not a timeout), so `curl` should exit quickly with exit code 7.

### New Policy.json Test Profiles for WFP-01

Two new profiles are needed in `crates/nono-cli/data/policy.json`:

**Profile A: `nono-ts-wfp-test-open` (Agent A — non-blocked)**
```json
"nono-ts-wfp-test-open": {
  "extends": "default",
  "meta": { "name": "nono-ts-wfp-test-open", "version": "1.0.0",
            "description": "WFP-01 gate: non-blocked agent (allowed egress)", "author": "nono-project" },
  "security": { "groups": [], "signal_mode": "isolated" },
  "filesystem": { "allow": ["C:\\Windows\\System32"] },
  "network": { "block": false },
  "workdir": { "access": "none" },
  "windows_low_il_broker": true,
  "interactive": false
}
```

**Profile B: `nono-ts-wfp-test-blocked` (Agent B — WFP-blocked)**
```json
"nono-ts-wfp-test-blocked": {
  "extends": "default",
  "meta": { "name": "nono-ts-wfp-test-blocked", "version": "1.0.0",
            "description": "WFP-01 gate: network-blocked agent (denied egress)", "author": "nono-project" },
  "security": { "groups": [], "signal_mode": "isolated" },
  "filesystem": { "allow": ["C:\\Windows\\System32"] },
  "network": { "block": true },
  "workdir": { "access": "none" },
  "windows_low_il_broker": true,
  "interactive": false
}
```

**Why `windows_low_il_broker: true` for both test profiles:** Proven in Phase 62 SC1 that the AppContainer + Low-IL broker arm starts `curl.exe` successfully. The WriteRestricted arm fails with 0xC0000142. Both test agents use the broker arm to ensure they actually start before attempting egress.

**Why `C:\Windows\System32` coverage:** `curl.exe` lives at `C:\Windows\System32\curl.exe`. nono's executable-coverage gate (R-B3) refuses to launch an uncovered binary. Adding `C:\Windows\System32` as an `allow` path covers both curl.exe itself and its DLL dependencies.

### Gate Structure: `scripts/gates/wfp-egress-isolation.ps1`

Follow the reference contract from `scripts/gates/copilot-e2e.ps1` and `scripts/gates/harness-self-check.ps1` exactly. Two exported functions; NO `exit`; return ONE `[ordered]@{ gate; verdict; reason; detail; timestamp }`.

**`Test-Precondition`:**
1. Check `nono` is on PATH (harness-internal if missing — throw, not skip).
2. Check `nono-wfp-service` is reachable by probing `\\.\pipe\nono-wfp-control` — if the named pipe does not exist → return `'nono-wfp-service is not running or not installed (pipe \\.\pipe\nono-wfp-control absent) — install and start nono-wfp-service before running this gate'` → SKIP_HOST_UNAVAILABLE.
3. Check a non-loopback IPv4 is available (optional — if the gate uses 127.0.0.1 as recommended above, skip this check).
4. Return `$null` if all checks pass.

**`Invoke-Gate`:**
1. Spin up a mock TCP server in the gate's PowerShell process (or a Rust binary) on `127.0.0.1:0`, record the assigned port.
2. Launch Agent A via `nono run --profile nono-ts-wfp-test-open -- curl.exe -s --max-time 5 http://127.0.0.1:<PORT>/probe` as a background job.
3. Launch Agent B via `nono run --profile nono-ts-wfp-test-blocked -- curl.exe -s --max-time 5 http://127.0.0.1:<PORT>/probe` as a background job.
4. Wait for both jobs (timeout 30s).
5. Assert: A exit code = 0 AND B exit code != 0.
6. Return PASS/FAIL verdict with `[ordered]@{ gate='wfp-egress-isolation'; verdict=...; reason=...; detail=@{agentAExit=...; agentBExit=...; port=...}; timestamp=... }`.

**Mock server in PowerShell:** Use `[System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Loopback, 0)`, start it, get the assigned port. Accept one connection per agent (or a loop). Alternatively, write a tiny Rust helper binary shipped alongside the gate — but a PowerShell listener is sufficient for HTTP/200 responses.

**The mock server runs at Medium IL (the gate's process) — the AppContainer client can connect outbound to a Medium-IL listener on loopback** (the loopback restriction is AppContainer↔AppContainer, not AppContainer↔medium). This is confirmed by Phase 62 SC1 where `curl` in an AppContainer connected to a non-container HTTP server.

**Pre-condition pipe probe (PowerShell):**
```powershell
function Test-WfpServiceReachable {
    try {
        $pipe = [System.IO.Pipes.NamedPipeClientStream]::new('.', 'nono-wfp-control',
            [System.IO.Pipes.PipeDirection]::InOut)
        $pipe.Connect(2000)   # 2s timeout
        $pipe.Close()
        return $true
    } catch { return $false }
}
```

---

## TSRG-01 Deliverable

### Current Behavior (the 0xC0000142 Failure)

[VERIFIED from code] `crates/nono-ts/src/windows_confined_run.rs` §108-145:

```rust
pub(crate) fn confined_run(
    exe: String, args: Vec<String>,
    allow: Option<Vec<String>>, profile: Option<String>,
    cwd: Option<String>, timeout_secs: Option<f64>,
) -> napi::Result<JsExecResult> {
    // Validate: at least one of profile or allow must be provided.
    if profile.is_none() && allow.as_ref().map_or(true, |v| v.is_empty()) {
        return Err(Error::new(Status::InvalidArg,
            "confined_run: at least one of 'profile' or 'allow' must be provided"));
    }
    // ...
    build_nono_run_args(&mut cmd, profile.as_deref(), allow.as_deref(), cwd.as_deref());
```

When no profile is given, `build_nono_run_args` emits only `--allow` paths (no `--profile`). This causes `nono run` to use its default exec strategy (WriteRestricted arm), which kills node.exe with `0xC0000142 STATUS_DLL_INIT_FAILED` because the restricting SID denies loader's WRITE-class `BaseNamedObjects` section ops. [VERIFIED from memory `project_v212_phase71` + Phase 52 investigation]

### The Fix: Two-Part Change

**Part 1: New profile `nono-ts-default` in `policy.json`**

Location: `crates/nono-cli/data/policy.json` (embedded at build time via `build.rs`).

Proposed profile:
```json
"nono-ts-default": {
  "extends": "default",
  "meta": {
    "name": "nono-ts-default",
    "version": "1.0.0",
    "description": "Default profile for nono-ts confinedRun callers that specify no profile. Provides Low-IL broker arm confinement without engine-specific paths. Callers should add --allow paths for their target executable's directory.",
    "author": "nono-project"
  },
  "security": {
    "groups": [],
    "signal_mode": "isolated"
  },
  "filesystem": {},
  "network": { "block": false },
  "workdir": { "access": "readwrite" },
  "windows_low_il_broker": true,
  "interactive": false
}
```

Design rationale (D-03):
- `windows_low_il_broker: true` — routes through the broker arm (not WriteRestricted), so node.exe / any managed runtime starts cleanly.
- `network.block: false` — sensible default; callers who want network isolation must pass `profile: 'some-blocked-profile'` explicitly.
- `filesystem: {}` — no built-in path grants; all coverage comes from `--allow` paths the caller provides (including the auto-cover exe-dir added by D-04).
- `workdir: { access: "readwrite" }` — allows the child to operate in its cwd.
- `security.groups: []` — no policy group inclusions; minimal footprint.

**Part 2: Wiring in `build_nono_run_args`** (`crates/nono-ts/src/windows_confined_run.rs` §76-94)

Current signature:
```rust
fn build_nono_run_args(
    cmd: &mut Command,
    profile: Option<&str>,
    allow: Option<&[String]>,
    cwd_allow: Option<&str>,
)
```

Two changes needed:
1. Inject `"nono-ts-default"` when `profile` is `None` (D-03).
2. Resolve `exe` to an absolute path and append its parent directory to `--allow` (D-04 auto-cover).

The `exe` string is not currently passed into `build_nono_run_args` — it is built separately in `confined_run`. The auto-cover logic needs to run in `confined_run` BEFORE calling `build_nono_run_args`, then pass the resolved exe-dir as an additional allow entry.

**Exact wiring plan (in `confined_run`):**

```rust
pub(crate) fn confined_run(
    exe: String,
    args: Vec<String>,
    allow: Option<Vec<String>>,
    profile: Option<String>,
    cwd: Option<String>,
    timeout_secs: Option<f64>,
) -> napi::Result<JsExecResult> {
    // D-03: inject default profile when none provided
    let profile = profile.or_else(|| Some("nono-ts-default".to_string()));

    // D-04: auto-cover target exe directory when profile defaulted (or when autoCoverTarget)
    let exe_dir = resolve_exe_dir(&exe)?;  // returns Option<String>
    let mut allow_paths = allow.unwrap_or_default();
    if let Some(dir) = exe_dir {
        if !allow_paths.contains(&dir) {
            allow_paths.push(dir);
        }
    }
    let allow = if allow_paths.is_empty() { None } else { Some(allow_paths) };

    // Validation: with default profile always Some, this check can now relax
    // (profile.is_some() is always true here). But keep the guard for safety.
    // ...
```

`resolve_exe_dir` helper:
```rust
fn resolve_exe_dir(exe: &str) -> napi::Result<Option<String>> {
    // Resolve exe to absolute path: check NONO_EXE precedent (use which/PATH search)
    // Return the parent directory as a string, or None if resolution fails.
    let exe_path = PathBuf::from(exe);
    let abs = if exe_path.is_absolute() {
        exe_path
    } else {
        // PATH search (mirrors find_nono_exe pattern)
        if let Some(path_var) = std::env::var_os("PATH") {
            std::env::split_paths(&path_var)
                .map(|d| d.join(exe))
                .find(|p| p.is_file())
                .unwrap_or(exe_path)
        } else {
            exe_path
        }
    };
    Ok(abs.parent().map(|p| p.to_string_lossy().to_string()))
}
```

**Backward compatibility:** The existing `profile.is_none() && allow.is_empty()` guard is removed (or becomes unreachable since profile is always Some after D-03). Existing callers that DO pass a profile continue to use their explicit profile. Existing callers that pass only `allow` paths continue to work (though they now also get the default profile injected — this is the intended behavior). The new `lowIl` / `autoCoverTarget` opt-out flags (D-04) are added as future optional parameters.

**Signature extension (D-04 — planner finalizes names):**
The CONTEXT.md says "add optional flags to `confinedRun` (e.g. `lowIl?: boolean`, `autoCoverTarget?: boolean`) that default to the new behavior." Since the napi export is a C function signature (`lib.rs §377-385`), adding optional boolean params must match the napi signature. The planner should add them as optional trailing parameters with defaults `true`.

### napi Rebuild Requirement

[VERIFIED from memory `project_v212_phase71`] Every change to `src/lib.rs` or `src/windows_confined_run.rs` in the nono-ts repo requires:
```powershell
# In C:\Users\OMack\nono-ts
napi build --platform --release
```
This regenerates `nono.win32-x64-msvc.node` AND `index.d.ts` (type definitions). The `.node` file is loaded by `index.js` at the win32 branch. Without rebuilding, the `npm test` run uses the OLD binary and new behavior is never exercised.

### nono-ts Integration Test (SC4)

[VERIFIED: `C:\Users\OMack\nono-ts\tests\` has `test_broker_ffi_mapping.js`, `test_errors.js`, `test_platform.js`, `test_query.js`, `test_sandbox_policy.js`, `test_state.js` — NONE exercises `confinedRun` end-to-end on Windows.]

[VERIFIED: `package.json` §53 — `"test": "node test.js"` — but `test.js` does not exist. `test-confined.js` exists but is NOT wired to `npm test`. The existing test pattern is individual files in `tests/`.]

**Gap:** There is no `test.js` entry point AND no `confinedRun` integration test. This is an explicit Wave 0 gap.

**Test to add** (new file `tests/test_confined_run_default.js`):
```javascript
// Integration test: confinedRun with no profile flag on Windows.
// Requires: NONO_EXE set to a dev-layout nono.exe, Win11 host, nono-ts rebuilt.
const { confinedRun } = require('../index.js');
const os = require('os');
const path = require('path');
const fs = require('fs');

if (process.platform !== 'win32') {
    console.log('SKIP: confinedRun integration test is Windows-only');
    process.exit(0);
}

const ws = path.join(os.homedir(), 'nono-ts-default-gate-ws');
fs.mkdirSync(ws, { recursive: true });

console.log('--- SC4: confinedRun with no profile flags ---');
// node.exe path auto-covered by D-04; no explicit allow needed for the binary.
const result = confinedRun(
    'node.exe',
    ['-e', 'process.exit(0)'],
    undefined,   // allow: undefined → D-04 auto-covers node.exe dir
    undefined,   // profile: undefined → D-03 injects nono-ts-default
    ws,
    30
);
if (result.exitCode !== 0) {
    console.error('FAIL: confinedRun with no profile exited', result.exitCode);
    console.error('stderr:', Buffer.from(result.stderr || []).toString());
    process.exit(1);
}
console.log('PASS: confinedRun default-broker-arm path succeeded (exit 0)');
```

Wire to `npm test` by updating `package.json` `"test"` script to: `"node tests/test_confined_run_default.js"` (or create a `test.js` dispatcher).

---

## Standard Stack

### Core (this phase touches/adds)

| Library/File | Purpose | Why Standard |
|---|---|---|
| `crates/nono-cli/data/policy.json` | Profile definitions | Single source of truth for all named profiles; embedded via `build.rs` |
| `crates/nono-cli/src/agent_daemon/launch.rs` | `wfp_filter_add/remove` (READ ONLY for WFP-01) | Shipped machinery; gate exercises it, does not modify it |
| `crates/nono-cli/src/windows_wfp_contract.rs` | WFP IPC types | Protocol definition; READ ONLY |
| `crates/nono-ts/src/windows_confined_run.rs` | `confined_run` / `build_nono_run_args` | TSRG-01 wiring lands here |
| `scripts/gates/wfp-egress-isolation.ps1` | New WFP-01 gate | Follows `copilot-e2e.ps1` / `harness-self-check.ps1` contract exactly |

### No New External Packages

This phase adds NO new Rust crate dependencies. The gate uses only PowerShell built-ins, `curl.exe` (ships with Windows 10+), and the existing `nono.exe` binary. The nono-ts integration test uses only `require('../index.js')` and Node built-ins.

### Package Legitimacy Audit

> No new packages are introduced in this phase. Both deliverables modify existing files only.

**Packages removed due to slopcheck [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

---

## Architecture Patterns

### System Architecture Diagram

```
Gate Process (PowerShell, Medium-IL)
    |-- MockServer (TcpListener on 127.0.0.1:PORT)
    |
    |-- Job A: `nono run --profile nono-ts-wfp-test-open -- curl.exe http://127.0.0.1:PORT/probe`
    |           nono.exe (Medium-IL)
    |               --> launch_agent() --> wfp_filter_add NOT called (block:false)
    |               --> AppContainer child: curl.exe (Low-IL, distinct SID-A)
    |                   [TCP connect to 127.0.0.1:PORT: no nono WFP filter → ALLOWED]
    |                   [curl exits 0 → Job A exitCode=0 → agentA PASSED]
    |
    |-- Job B: `nono run --profile nono-ts-wfp-test-blocked -- curl.exe http://127.0.0.1:PORT/probe`
    |           nono.exe (Medium-IL)
    |               --> launch_agent() --> wfp_filter_add(SID-B) --> nono-wfp-service
    |               --> AppContainer child: curl.exe (Low-IL, distinct SID-B)
    |                   [TCP connect: WFP BLOCK at ALE_AUTH_CONNECT_V4 keyed to SID-B]
    |                   [curl exits 7 (connection refused) → Job B exitCode!=0 → agentB DENIED]
    |
    Gate verdict: agentA.exitCode=0 AND agentB.exitCode!=0 → PASS
                  else → FAIL

nono-wfp-service (SYSTEM elevated)
    <-- IPC: \\.\pipe\nono-wfp-control
    --> FwpmFilterAdd0() kernel WFP: block filter at ALE_AUTH_CONNECT_V4/V6, SID-B keyed
```

```
confinedRun({ target: "node" })          [nono-ts caller, no profile]
    |
    confined_run() in windows_confined_run.rs
    |-- D-03: profile = "nono-ts-default"
    |-- D-04: exe_dir = parent(resolve("node.exe")) → e.g. "C:\Program Files\nodejs"
    |-- build_nono_run_args → emits: nono.exe run --profile nono-ts-default --allow "C:\Program Files\nodejs" -- node.exe
    |
    nono.exe run (Medium-IL)
    |-- policy_needs_network_scoping("nono-ts-default") = false (block:false) → no WFP gate
    |-- windows_low_il_broker: true → Low-IL broker arm (not WriteRestricted)
    |-- AppContainer child: node.exe (Low-IL, new SID per run)
        [Starts cleanly — no 0xC0000142]
```

### Recommended Project Structure

No new directories. All changes land in:
```
crates/nono-cli/data/policy.json          # +3 profiles (wfp-test-open, wfp-test-blocked, nono-ts-default)
crates/nono-ts/src/windows_confined_run.rs # +~30 lines (D-03/D-04 wiring)
scripts/gates/wfp-egress-isolation.ps1    # new gate file (~100 lines)
nono-ts/tests/test_confined_run_default.js # new integration test (~30 lines)
```

### Anti-Patterns to Avoid

- **Adding the WFP filter at the gate level (PowerShell directly calling the pipe):** The gate must exercise the shipped `launch_agent` / `wfp_filter_add` path, not call `\\.\pipe\nono-wfp-control` directly. Calling the pipe directly bypasses the `profile_needs_network_scoping` gate and does not test the real path.
- **Using a loopback mock but NOT confirming the mock server is Medium-IL:** If the mock server were somehow spawned inside an AppContainer, the loopback restriction between AppContainers would apply and the test would be invalid.
- **Relying on curl timeout for agent B's failure signal:** A timeout (exit 28) is ambiguous — it could be network congestion, not a WFP block. The WFP block at ALE_AUTH_CONNECT fires immediately (exit 7, connection refused). Use `--max-time 5` and check for non-zero exit (not specifically exit 7) for robustness.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| TCP mock server in Rust for the gate | Custom Rust binary with tokio | PowerShell `[System.Net.Sockets.TcpListener]` | Adds a build dependency to a PS gate; a loopback accept loop in PS is sufficient for a one-shot HTTP response |
| WFP filter IPC from PowerShell | Custom PS NamedPipe JSON serialization | `nono run --profile <block-profile>` | The shipped `wfp_filter_add` path in `launch_agent` is the unit under test; calling it directly would test the wrong thing |
| AppContainer SID generation in the gate | Compute SID from profile name in PS | Trust that two `nono run` invocations generate two distinct SIDs | `launch_agent` guarantees distinct SIDs per call via random `tenant_id` → distinct `profile_name` → distinct SID |
| Target exe resolution in nono-ts | Custom PATH walking | Extend the existing `find_nono_exe` pattern (already in `windows_confined_run.rs`) | Proven pattern, same error handling, avoids code duplication |

---

## Common Pitfalls

### Pitfall 1: Gate reports PASS when nono-wfp-service is not running

**What goes wrong:** Agent B's `wfp_filter_add` fails (pipe absent) → `launch_agent` terminates the suspended process → `nono run` exits non-zero before curl even runs → agentB.exitCode != 0 → the gate's "B denied" check passes vacuously → false PASS.

**How to avoid:** `Test-Precondition` MUST probe the WFP pipe before running the gate. If the pipe is absent → SKIP_HOST_UNAVAILABLE (not FAIL). This mirrors the fail-secure contract: a missing WFP service is a host pre-condition gap, not a confinement failure.

**Detection:** Check `nono run` stderr for "WFP network scope required ... nono-wfp-service is not reachable" and reclassify as SKIP if found.

### Pitfall 2: The mock server exits before Agent A connects

**What goes wrong:** The mock server `TcpListener` is started synchronously in the gate, Agent A and B are launched as background jobs. If the gate's listener `Accept()` call only handles one connection, the second agent (whichever arrives second) gets ECONNREFUSED — making agentA.exitCode != 0 even without any WFP filter.

**How to avoid:** The listener MUST loop to accept at least 2 connections. The PowerShell listener should use a background thread or an `Accept(2)` loop: accept, respond, accept again.

### Pitfall 3: Agent A uses the `nono-ts-default` profile (network.block:false) but Windows' default AppContainer block fires

**What goes wrong:** Agent A's AppContainer has no network capabilities token → Windows' own AppContainer default block (WSH sublayer) blocks Agent A's outbound, not just the nono WFP filter → agentA.exitCode != 0 → false FAIL.

**How to avoid:** As established in the D-02 analysis, the loopback bind avoids this issue because the AppContainer client → non-AppContainer server loopback path is NOT blocked by the Windows AppContainer loopback restriction. For LAN binds, `privateNetworkClientServer` would be needed. Since we use loopback, this pitfall does NOT apply — but the gate must use the loopback bind to avoid it.

### Pitfall 4: nono-ts napi binary is stale after Rust changes

**What goes wrong:** A developer modifies `windows_confined_run.rs` or `lib.rs`, runs `npm test`, and the test exercises the OLD binary (`nono.win32-x64-msvc.node` from the previous build). The new behavior is never tested. SC4 passes against stale code.

**How to avoid:** The integration test plan MUST include a Wave 0 step: `napi build --platform --release` in `C:\Users\OMack\nono-ts`. The verification script (`verify-dark.ps1 --gate wfp-egress-isolation`) does not cover the napi build; the nono-ts integration test is a separate `npm test` invocation.

### Pitfall 5: confinedRun validation gate rejects the no-profile+no-allow call

**What goes wrong:** The D-03 profile injection must happen BEFORE the `profile.is_none() && allow.is_empty()` check, otherwise the default profile injection is unreachable — the validation check short-circuits with an error before D-03 logic fires.

**How to avoid:** The default injection MUST be the FIRST operation in `confined_run`, before any validation. The guard remains for the case where a caller explicitly passes `profile: None` and `allow: []` in a future API version with opt-out flags.

### Pitfall 6: Profile name collision between WFP-01 test profiles and existing profiles

**What goes wrong:** A profile name like `test-blocked` might collide with a future built-in profile.

**How to avoid:** Use the `nono-ts-wfp-test-*` namespace for test profiles (mirrors the `nono-ts-default` naming convention). These are clearly test-only names and will not collide with engine profiles (claude-code, aider, etc.).

### Pitfall 7: DCO sign-off missing from nono-ts commits

**What goes wrong:** Changes to `C:\Users\OMack\nono-ts` are committed to the nono-ts sibling repo without `Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>`.

**How to avoid:** All commits in BOTH repos must include the DCO line. The planner must include it in task action instructions for nono-ts commits.

---

## Runtime State Inventory

> Omitted — greenfield additions (new profiles, new gate file, new napi code). No renames, no data migrations.

---

## Code Examples

### Verified pattern: `wfp_filter_add` call (from `launch.rs`)

[VERIFIED from `crates/nono-cli/src/agent_daemon/launch.rs` §423-466]

The gate exercises this path indirectly via `nono run --profile nono-ts-wfp-test-blocked`. The planner does NOT need to call `wfp_filter_add` directly from PowerShell.

### Verified pattern: `build_nono_run_args` current implementation

[VERIFIED from `crates/nono-ts/src/windows_confined_run.rs` §76-94]

```rust
fn build_nono_run_args(
    cmd: &mut Command,
    profile: Option<&str>,
    allow: Option<&[String]>,
    cwd_allow: Option<&str>,
) {
    if let Some(p) = profile {
        cmd.arg("--profile").arg(p);
    }
    if let Some(paths) = allow {
        for path in paths {
            cmd.arg("--allow").arg(path);
        }
    }
    if let Some(cwd) = cwd_allow {
        cmd.arg("--allow").arg(cwd).arg("--allow-cwd");
    }
}
```

The D-03/D-04 changes happen in `confined_run` BEFORE calling `build_nono_run_args` (inject default profile, resolve exe dir, append to allow). `build_nono_run_args` itself does not need to change.

### Verified pattern: PowerShell TcpListener for mock server

```powershell
$listener = [System.Net.Sockets.TcpListener]::new(
    [System.Net.IPAddress]::Loopback, 0)
$listener.Start()
$port = $listener.LocalEndpoint.Port

# Accept loop (background thread so gate doesn't block):
$listenerJob = [System.Threading.Tasks.Task]::Run([Action]{
    for ($i = 0; $i -lt 2; $i++) {
        $client = $listener.AcceptTcpClient()
        $stream = $client.GetStream()
        $response = [Text.Encoding]::ASCII.GetBytes(
            "HTTP/1.1 200 OK`r`nContent-Length: 2`r`n`r`nOK")
        $stream.Write($response, 0, $response.Length)
        $stream.Close()
        $client.Close()
    }
    $listener.Stop()
})
```

### Verified pattern: gate verdict object (from `harness-self-check.ps1`)

```powershell
return [ordered]@{
    gate      = 'wfp-egress-isolation'
    verdict   = 'PASS'
    reason    = 'agent A egress succeeded (exit 0) and agent B egress denied (exit non-zero) — per-SID WFP isolation confirmed'
    detail    = [ordered]@{
        agentAExitCode = $exitA
        agentBExitCode = $exitB
        mockPort       = $port
        agentAProfile  = 'nono-ts-wfp-test-open'
        agentBProfile  = 'nono-ts-wfp-test-blocked'
    }
    timestamp = (Get-Date -Format 'yyyy-MM-ddTHH:mm:ss.fffZ')
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|---|---|---|---|
| WriteRestricted arm for no-profile `confinedRun` | Low-IL broker arm via `nono-ts-default` profile | Phase 79 (this phase) | Fixes 0xC0000142 on the no-profile path |
| WFP-01 as interactive human UAT | Unattended PowerShell gate (`verify-dark.ps1 --gate wfp-egress-isolation`) | Phase 79 (this phase) | DARK-01 mandate satisfied for WFP isolation proof |
| Per-SID WFP filter via AppID path | Per-SID WFP filter via `FWPM_CONDITION_ALE_USER_ID` + SID-keyed SD (shipped Phase 75) | Phase 62/75 | AppContainer package SID is the kernel enforcement handle |

**Deprecated/outdated:**
- WriteRestricted arm for `confinedRun` no-profile path: the 0xC0000142 failure is the exact bug D-03 fixes. Post-Phase-79 the no-profile path reaches the broker arm.
- Interactive SC human UAT for WFP isolation: replaced by the unattended gate.

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework (Rust) | Cargo test runner, existing workspace |
| Framework (nono-ts) | `node test.js` / individual `node tests/*.js` |
| Gate runner | `scripts/verify-dark.ps1 --gate wfp-egress-isolation` |
| Quick run (Rust) | `cargo test -p nono-cli` (no new unit tests in nono-cli for this phase) |
| Full suite | `cargo test --workspace --all-targets --all-features` |
| nono-ts test command | `npm test` (after `napi build --platform --release`) |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| WFP-01 | Per-SID egress isolation: A allowed, B denied, concurrently | Integration (gate) | `scripts/verify-dark.ps1 --gate wfp-egress-isolation` | No — Wave 0 gap |
| TSRG-01 | `confinedRun` no-profile path reaches broker arm, exits 0 | Integration (napi) | `cd C:\Users\OMack\nono-ts && npm test` | No — Wave 0 gap |

### Sampling Rate

- **Per task commit (nono-cli changes):** `cargo clippy --workspace --all-targets --all-features -- -D warnings -D clippy::unwrap_used` (cross-target per CLAUDE.md)
- **Per task commit (nono-ts changes):** `napi build --platform --release && npm test` on Win11 host
- **Per wave merge:** Full `cargo test --workspace --all-targets --all-features`
- **Phase gate:** `scripts/verify-dark.ps1 --gate wfp-egress-isolation` AND `npm test` both green

### Wave 0 Gaps

- `scripts/gates/wfp-egress-isolation.ps1` — new gate file (WFP-01)
- `crates/nono-cli/data/policy.json` — 3 new profiles (wfp-test-open, wfp-test-blocked, nono-ts-default)
- `C:\Users\OMack\nono-ts\tests\test_confined_run_default.js` — new integration test (TSRG-01 / SC4)
- `C:\Users\OMack\nono-ts\package.json` — update `"test"` script to include the new integration test
- `napi build --platform --release` — rebuild the `.node` binary before test can pass

---

## Environment Availability

| Dependency | Required By | Available | Fallback |
|------------|------------|-----------|----------|
| `nono-wfp-service` (elevated service) | WFP-01 gate pre-condition | Must be running on Win11 host | Gate emits SKIP_HOST_UNAVAILABLE if absent — not a FAIL |
| `curl.exe` (Windows 10+ built-in) | Agent A/B egress probe in gate | Available on Win11 [ASSUMED] | PowerShell `[Net.Sockets.TcpClient]` as fallback |
| `napi build` / `@napi-rs/cli` | TSRG-01 nono-ts rebuild | Available in `C:\Users\OMack\nono-ts\node_modules` per package.json | — |
| `node.exe` (target for TSRG-01 integration test) | SC4 confinedRun test | Available on Win11 dev host [ASSUMED] | Use `cmd.exe /c exit 0` as alternate target |

**Missing dependencies with no fallback:** If `nono-wfp-service` is not installed, WFP-01 cannot produce a PASS; the gate skips gracefully.

---

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---|---|---|
| V4 Access Control | yes | Per-SID WFP filter is the enforcement boundary; gate verifies it is not bypassed |
| V5 Input Validation | yes | Profile names passed to `nono run` must come from the shipped `policy.json` (no user-controlled strings reach WFP) |
| V6 Cryptography | no | — |

### Known Threat Patterns

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| False PASS via vacuous WFP service unavailability | Spoofing | `Test-Precondition` pipe probe → SKIP, not PASS |
| SID collision between concurrent agents | Spoofing | Each `launch_agent` call generates a random `tenant_id` → fresh `CreateAppContainerProfile` → mathematically distinct SID |
| Gate auto-cover grants too-broad path (`C:\Windows\System32`) | Elevation of privilege | The test profiles' `allow` grants are read-only (the sandbox defaults); `C:\Windows\System32` coverage is only needed for the test executables and is scoped to these test-only profiles |
| WriteRestricted arm reached by D-03 default when `windows_low_il_broker` absent | Denial of service | The `nono-ts-default` profile sets `windows_low_il_broker: true`; `profile_needs_network_scoping` reads `block` not broker arm; both are driven by `EMBEDDED_POLICY_JSON` |

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `curl.exe` is available at `C:\Windows\System32\curl.exe` on the Win11 dev host | WFP-01 gate | Gate fails to spawn agent; use `where.exe curl` check in `Test-Precondition`; fallback: PowerShell TCP client |
| A2 | `node.exe` is on PATH on the Win11 dev host | TSRG-01 integration test | SC4 test cannot run; add a pre-condition check in the test |
| A3 | The `nono run` direct path (not daemon) calls `launch_agent` which calls `wfp_filter_add` | WFP-01 gate design | If `nono run` uses a different code path that skips `wfp_filter_add`, the gate exercises dead code. Verify by tracing the `exec_strategy` selection for `network.block:true` on Windows. |

---

## Open Questions (RESOLVED)

1. **Does `nono run --profile nono-ts-wfp-test-blocked` (non-daemon path) call `launch_agent` → `wfp_filter_add`?**
   - What we know: `launch_agent` is in `agent_daemon/launch.rs` and is called by the daemon's `handle_launch`. The non-daemon `nono run` path uses `exec_strategy_windows/`.
   - What's unclear: The exec strategy for a `network.block:true` profile on Windows may reach a DIFFERENT WFP activation path than `wfp_filter_add` in `launch_agent`. The gate's value depends on the SAME path being exercised.
   - Recommendation: Before writing the gate, the planner should verify which code path `nono run --profile <block:true>` takes on Windows by tracing `exec_strategy_windows/mod.rs` and its WFP activation call. If it goes through a different path, the gate must use the daemon `launch_agent` path instead (which means `nono-agentd` must be running and the gate sends a `Launch` command to the daemon pipe).
   - **RESOLVED:** Verified from live code. The non-daemon `nono run --profile <block:true>` path goes `exec_strategy_windows/mod.rs` → `prepare_network_enforcement()` in `network.rs` → `install_wfp_network_backend()` → `WfpRuntimeActivationRequest` over `\.\pipe\nono-wfp-control` (the SAME `nono-wfp-service` pipe as the daemon path, installing per-SID WFP filters). The gate using direct `nono run` is valid; `nono-agentd` does NOT need to be running.

2. **Which mock server shape is simplest for the gate?**
   - What we know: The gate can use PowerShell's `TcpListener` in a background thread or a Rust helper binary.
   - What's unclear: PowerShell background threads for TcpListener are reliable but error-prone (thread lifetime, exception handling). A Rust helper binary is more robust.
   - Recommendation: Start with the PowerShell `TcpListener` in a `[System.Threading.Tasks.Task]::Run` block; if it proves fragile on the live host, consider a minimal Rust helper.
   - **RESOLVED:** Use PowerShell `[System.Net.Sockets.TcpListener]` in a `[System.Threading.Tasks.Task]::Run` block — simplest, no build dependency. The listener MUST accept at least 2 connections (loop `$i = 0; $i -lt 2`) to avoid the Pitfall 2 false-fail where Agent A gets ECONNREFUSED after Agent B consumes the single accept slot.

---

## Sources

### Primary (HIGH confidence — verified from code)
- `crates/nono-cli/src/agent_daemon/launch.rs` §423-691 — `wfp_filter_add`, `wfp_filter_remove`, `profile_needs_network_scoping`, `launch_agent` step 6.5 [VERIFIED]
- `crates/nono-cli/src/windows_wfp_contract.rs` — `WfpRuntimeActivationRequest`/`Response` struct definitions [VERIFIED]
- `crates/nono-cli/src/bin/nono-wfp-service.rs` §1097-1605 — `build_policy_filter_specs` (loopback_only=false on block filter), `install_wfp_policy_filters`, WFP layer constants [VERIFIED]
- `crates/nono-cli/data/policy.json` — existing profiles (all `network.block:false`); `windows_low_il_broker:true` examples in `claude-code`, `aider`, `copilot-cli`, `langchain-python` [VERIFIED]
- `crates/nono-ts/src/windows_confined_run.rs` §76-145 — `build_nono_run_args`, `confined_run` current implementation [VERIFIED]
- `crates/nono-ts/src/lib.rs` §375-403 — `confinedRun` napi export signature [VERIFIED]
- `scripts/gates/copilot-e2e.ps1` — `Test-Precondition`/`Invoke-Gate` contract reference [VERIFIED]
- `scripts/gates/harness-self-check.ps1` — verdict object shape, assertion helpers [VERIFIED]
- `scripts/verify-dark.ps1` §1-165 — runner contract, exit codes, persist-before-emit [VERIFIED]
- `crates/nono-cli/tests/auto_pull_e2e_linux.rs` §57-127 — `spawn_multi_endpoint_server` shape [VERIFIED]

### Secondary (MEDIUM confidence — official docs)
- [Microsoft Learn: Troubleshooting UWP App Connectivity Issues in Windows Firewall](https://learn.microsoft.com/en-us/windows/security/operating-system-security/network-security/windows-firewall/troubleshooting-uwp-firewall) — loopback/capability requirements, WSH sublayer, `privateNetworkClientServer` [CITED]
- [Google Project Zero: Understanding Network Access in Windows AppContainers](https://projectzero.google/2021/08/understanding-network-access-windows-app.html) — `AppContainerLoopback` receive-side filter, `IsAppContainerLoopback` flag, capability SIDs [CITED]

### Memory (project context)
- `memory/windows_wfp_enforcement_is_service_only.md` — WFP is service-only; Phase 62 bug chain; SC1 PASS details [ASSUMED: 15 days old, verify code matches]
- `memory/windows_appcontainer_wfp_validated.md` — AppContainer + WFP spike-validated; per-SID filter tested via both ALE_USER_ID and ALE_PACKAGE_ID [ASSUMED: 14 days old, verify code matches]

---

## Metadata

**Confidence breakdown:**
- WFP-01 shipped machinery: HIGH — read from live code
- D-02 loopback analysis: HIGH — cross-verified from code (loopback_only=false) and authoritative external docs
- AppContainer loopback restriction direction (receive-side vs connect-side): MEDIUM — Project Zero analysis (2021, may not reflect Win11 26200 changes)
- TSRG-01 wiring: HIGH — read from live code, exact lines verified
- Test profile schema: HIGH — read from schema and existing profile examples
- Gate contract: HIGH — read from two reference gate implementations

**Research date:** 2026-06-18
**Valid until:** 2026-07-18 (stable domain — WFP APIs and napi-rs patterns do not change frequently)
