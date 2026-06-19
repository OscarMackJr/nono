# Phase 83: Machine Policy Spine + Egress Control - Pattern Map

**Mapped:** 2026-06-18
**Files analyzed:** 11 new/modified files
**Analogs found:** 11 / 11 (every file has a strong in-repo analog — this is a wiring+hardening phase)

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `crates/nono/src/machine_policy.rs` (NEW) | model + reader (core lib) | file-I/O (registry read), transform | `crates/nono-cli/src/health.rs` §`probe_machine_policy_windows` (semantics) + `crates/nono-cli/src/windows_wfp_contract.rs` (serde type shape) | exact-semantics / role-match |
| `crates/nono/src/error.rs` (MODIFIED) | model (error enum) | — | existing `NonoError` struct variants (`LabelApplyFailed`, `UnsupportedKernelFeature`) in same file | exact (same file, same pattern) |
| `crates/nono/src/net_filter.rs` (MODIFIED) | utility (matcher) + test | transform / request-response | existing `HostFilter::check_host` + the wildcard tests in the same file | exact (same file) |
| `crates/nono-proxy/src/filter.rs` (MODIFIED) | service (async filter) | request-response | existing `ProxyFilter::new_strict` in same file | exact (same file) |
| `crates/nono-cli/src/bin/nono-wfp-service.rs` (MODIFIED) | service (kernel WFP) | event-driven / request-response | existing `build_policy_filter_specs` in same file | exact (same file; no change likely needed — request flip drives it) |
| `crates/nono-cli/src/windows_wfp_contract.rs` (MODIFIED, maybe) | model (IPC contract) | request-response | existing `WfpRuntimeActivationRequest` in same file | exact (same file) |
| `crates/nono-cli/src/agent_daemon/launch.rs` (MODIFIED) | controller (daemon hand-off) | request-response (IPC) | existing `wfp_filter_add` in same file | exact (same file) |
| `crates/nono-cli/src/agent_daemon/control_loop.rs` (MODIFIED, maybe) | controller (IPC dispatch) | request-response (IPC) | existing `ControlRequest` enum in same file | exact (same file) |
| `crates/nono-cli/data/policy.json` + `crates/nono-cli/data/network-policy.json` (MODIFIED) | config (data) | CRUD (static data) | **`network-policy.json` `hosts[]` group schema** (NOT `policy.json` fs groups) | exact-shape |
| `crates/nono-cli/build.rs` (MODIFIED, maybe) | config (embed) | batch (build-time) | existing `rerun-if-changed` + `include_str!` mechanism | exact (same file) |
| `dist/windows/nono.admx`/`.adml` via `scripts/build-windows-msi.ps1` (MODIFIED) | config (GPO template) | transform (here-string gen) | shipped Phase-82 `AllowedSuffixes`/`AllowedHosts` `<list>` policies (in the same here-string source) | role-match |
| `scripts/gates/egress-policy-deny.ps1` (NEW) | test (Dark Factory gate) | event-driven | `scripts/gates/wfp-egress-isolation.ps1` (two-function contract) | exact (structural twin) |

---

## Pattern Assignments

### `crates/nono/src/machine_policy.rs` (NEW — model + reader, core lib)

**Analogs:**
- Type shape: `crates/nono-cli/src/windows_wfp_contract.rs` (serde-derive, platform-neutral fields).
- Read semantics (present/unreadable/absent split on the SAME HKLM key): `crates/nono-cli/src/health.rs:386-418` `probe_machine_policy_windows`.
- Reader skeleton + list-subkey enumeration: RESEARCH.md §"Pattern 1" and §"Code Examples" (winreg `RegKey` + `raw_os_error()` + `enum_values()`).

**Type-shape pattern to copy** (`windows_wfp_contract.rs:1-20`) — platform-neutral serde struct so the type compiles on Linux/macOS (D-05, Pitfall 5):
```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WfpRuntimeActivationRequest {
    pub protocol_version: u32,
    pub request_kind: String,
    // ... all plain Vec<String>/Option<String>/u32 fields — no Windows types
}
```
`MachineEgressPolicy` follows this exact pattern: a `#[derive(Debug, Clone, ..., Serialize, Deserialize)]` struct of `Vec<String>` allowlist fields + preset tokens. Keep it platform-neutral; ONLY the reader fn is `#[cfg(target_os = "windows")]`-gated (non-Windows stub returns `Ok(None)`). Structure it so Phase 84 can add a `telemetry` section to the SAME read (Deferred Ideas).

**Present/unreadable/absent taxonomy to copy** (`health.rs:391-417`) — this is the Phase-82 reference split the new typed reader must align with:
```rust
// health.rs probe (string-matched on `reg query` stderr; the new reader does it TYPED):
if result.status.success() {
    SubsystemState::Ok                                    // present + readable
} else if combined.contains("access denied") {
    // present but unreadable  -> Phase 83: Err(PolicyLoadFailed)
} else {
    // absent / not configured -> Phase 83: Ok(None) fall-through
}
```
The new reader maps `raw_os_error() == Some(2)` (ERROR_FILE_NOT_FOUND) → `Ok(None)` (absent, D-07); every other `Err` (ACCESS_DENIED=5, malformed REG type) → `Err(NonoError::PolicyLoadFailed)` (D-07 abort). See RESEARCH.md Pattern 1 for the full winreg arm and Pitfall 1 for the `<list>` → N×REG_SZ subkey enumeration (Option A, locked in D-13).

**Re-export pattern** (`crates/nono/src/lib.rs:55, 81`): add `pub mod machine_policy;` and `pub use machine_policy::{MachineEgressPolicy, read_machine_egress_policy};` mirroring the existing `pub mod net_filter;` / `pub use net_filter::{FilterResult, HostFilter};` lines.

**Cargo gating** (RESEARCH.md Standard Stack): add `winreg = "0.56"` ONLY under `[target.'cfg(windows)'.dependencies]` in `crates/nono/Cargo.toml` (D-10, Pitfall 5). Gate the dep install behind a `checkpoint:human-verify` (slopcheck not run — A1).

---

### `crates/nono/src/error.rs` (MODIFIED — add `PolicyLoadFailed`)

**Analog:** existing struct-variant errors in the SAME file — `LabelApplyFailed` (`error.rs:206-214`) and `UnsupportedKernelFeature` (`error.rs:84-85`).

**Variant pattern to copy** (`error.rs:206-214`) — struct variant with `#[error(...)]` thiserror Display + a doc comment explaining fail-closed intent:
```rust
/// Failed to apply integrity label to ... (fail-closed).
#[error("Failed to apply integrity label to {path}: {hint} (HRESULT: 0x{hresult:08X})")]
LabelApplyFailed {
    path: PathBuf,
    hresult: u32,
    hint: String,
},
```
`PolicyLoadFailed` follows this exact shape (Claude's discretion on fields; RESEARCH.md uses `{ reason: String }`):
```rust
/// Machine egress policy key present but unreadable or malformed (fail-secure, D-07).
/// Distinct from absent (which falls through to per-user). Once the HKLM key
/// exists, ANY read/parse error aborts — never fall through to a permissive state.
#[error("Machine policy load failed: {reason}")]
PolicyLoadFailed { reason: String },
```

**Test pattern to copy** (`error.rs:398-430`): add Display + pattern-match tests mirroring `label_apply_failed_display_includes_path_hresult_and_hint` and `label_apply_failed_is_propagatable_via_result_alias` — assert the Display contains the reason and `matches!(err, NonoError::PolicyLoadFailed { .. })`.

---

### `crates/nono/src/net_filter.rs` (MODIFIED — SC-4 matrix + optional hardening)

**Analog:** existing `HostFilter::check_host` (`net_filter.rs:188-231`) and its wildcard tests (`net_filter.rs:274-294`) in the SAME file.

**Core matcher (already passes SC-4 — DO NOT rebuild, D-14)** (`net_filter.rs:215-231`):
```rust
// 4. exact host match
if self.allowed_hosts.contains(&lower_host) { return FilterResult::Allow; }
// 5. wildcard subdomain match (leading-dot suffix + length guard = component-safe)
for suffix in &self.allowed_suffixes {
    if lower_host.ends_with(suffix.as_str()) && lower_host.len() > suffix.len() {
        return FilterResult::Allow;
    }
}
// 6. not in allowlist -> deny
FilterResult::DenyNotAllowed { host: host.to_string() }
```
The `ends_with(".anthropic.com") && len >` form already rejects `anthropic.com` / `evilanthropic.com` / `anthropic.com.evil.com`. Optionally harden to `.split('.')` label comparison (Claude's discretion D-14), but keep the fail-secure default.

**Test pattern to copy** (`net_filter.rs:274-294`, `test_wildcard_does_not_match_bare_domain`): add the named EGRESS-03 four-case matrix exactly as RESEARCH.md §"Code Examples" `test_sc4_dns_component_matrix` shows — `api.anthropic.com` allowed; `anthropic.com`, `evilanthropic.com`, `anthropic.com.evil.com` rejected. Use the `public_ip()` helper already defined at `net_filter.rs:248`.

---

### `crates/nono-proxy/src/filter.rs` (MODIFIED — feed machine allowlist)

**Analog:** existing `ProxyFilter::new_strict` in the SAME file (`filter.rs:39-45`).

**Constructor pattern (already deny-by-default, EGRESS-01)** (`filter.rs:39-45`):
```rust
/// Create a strict proxy filter: an empty allowlist denies every host.
#[must_use]
pub fn new_strict(allowed_hosts: &[String]) -> Self {
    Self { inner: HostFilter::new_strict(allowed_hosts) }
}
```
Phase 83 wires this from the daemon: `ProxyFilter::new_strict(&machine_policy.expanded_allowlist())`. No new constructor needed — the wholesale-override allowlist (D-08) is just the `&[String]` passed in. `check_host` (`filter.rs:65-92`) already does `tokio::net::lookup_host` (DNS resolution lives proxy-side per D-01/D-02; Pitfall 6).

---

### `crates/nono-cli/src/agent_daemon/launch.rs` (MODIFIED — flip to proxy-only)

**Analog:** existing `wfp_filter_add` in the SAME file (`launch.rs:423-466`).

**Current request shape to FLIP** (`launch.rs:428-445`) — currently `blocked` mode, empty `localhost_ports` (blocks everything, WFP-01 model):
```rust
let req = WfpRuntimeActivationRequest {
    protocol_version: WFP_RUNTIME_PROTOCOL_VERSION,
    request_kind: "activate_blocked_mode".to_string(),
    network_mode: "blocked".to_string(),       // <- Phase 83: "proxy-only"
    // ...
    tcp_connect_ports: vec![],
    tcp_bind_ports: vec![],
    localhost_ports: vec![],                    // <- Phase 83: vec![proxy_port]
    session_sid: Some(package_sid.to_string()),
    outbound_rule_name: Some(format!("nono-agent-{tenant_id}")),
    inbound_rule_name: Some(format!("nono-agent-{tenant_id}-in")),
};
```
**Phase 83 change** (RESEARCH.md Pattern 3): `request_kind: "activate_proxy_mode"`, `network_mode: "proxy-only"`, `localhost_ports: vec![proxy_port]`. This single request change drives `build_policy_filter_specs` to emit PERMIT-loopback:proxy_port (weight 100) + BLOCK-all (weight 0) for the SID — no WFP-service code change needed. Thread the proxy port from daemon startup into the `wfp_filter_add` signature (Open Q3, A3). Keep the fail-secure error handling (`launch.rs:447-463`) verbatim: any non-success status → `Err(NonoError::SandboxInit(..))`.

---

### `crates/nono-cli/src/bin/nono-wfp-service.rs` (likely UNCHANGED — drives off the flipped request)

**Analog / reference (do NOT rebuild, RESEARCH.md anti-pattern):** `build_policy_filter_specs` (`nono-wfp-service.rs:1186-1282`).

**Existing permit+block emission** (`nono-wfp-service.rs:1215-1239`) — the `localhost_ports` loop already produces the loopback PERMIT, and `needs_outbound_block` already produces the block-all when `network_mode != "allow-all"`:
```rust
for port in &request.localhost_ports {
    specs.push(PolicyFilterSpec {
        action: FilterAction::Permit,
        port: Some(PortCondition::Remote(*port)),
        loopback_only: true,                     // <- loopback PERMIT for proxy port
        // ...
    });
}
let needs_outbound_block = request.network_mode != "allow-all"
    || !request.localhost_ports.is_empty();
if needs_outbound_block {
    specs.push(PolicyFilterSpec { action: FilterAction::Block, port: None, loopback_only: false, .. });
}
```
The `proxy-only` + `localhost_ports:[proxy_port]` request already yields exactly the force-through-proxy shape. Weight ordering (permit 100 > block 0) is at `nono-wfp-service.rs:~1497` — DO NOT change (Pitfall 4). Only add a `"proxy-only"` `network_mode` acceptance if the service whitelists modes; otherwise no change.

---

### `crates/nono-cli/src/windows_wfp_contract.rs` (MODIFIED only if a new request_kind needs a constant)

**Analog:** the SAME file's `WfpRuntimeActivationRequest` (`windows_wfp_contract.rs:5-20`). No structural change needed — `network_mode`/`request_kind` are already `String`, and `localhost_ports: Vec<u16>` already exists. Only touch this file if you add a named constant for the `"activate_proxy_mode"` kind.

---

### `crates/nono-cli/src/agent_daemon/control_loop.rs` (MODIFIED only if a new control verb is added)

**Analog:** the SAME file's `ControlRequest` enum (`control_loop.rs:307-334`).

**Tagged-enum pattern to copy** (`control_loop.rs:307-333`) if Phase 83 needs a new operator verb (likely NOT — the machine read happens at daemon startup, D-04/D-06, not via a control frame):
```rust
#[derive(serde::Deserialize, Debug)]
#[serde(tag = "action", rename_all = "lowercase")]
pub(crate) enum ControlRequest {
    Launch { profile: String, cmd: Vec<String> },
    List,
    Classify { pid: u32 },
    // ...
}
```
The `MachineEgressPolicy`-derived WFP permit instructions ride the EXISTING `wfp_filter_add` → `nono-wfp-control` path (launch.rs), NOT a new control verb. The Medium-IL SACL (`control_loop.rs:96-97`, `S:(ML;;NW;;;ME)`) already bars Low-IL callers — reuse, don't re-derive.

---

### `crates/nono-cli/data/network-policy.json` + `policy.json` (MODIFIED — egress preset groups)

**Analog (CRITICAL CORRECTION to D-12):** `data/network-policy.json` is the closest analog, NOT the filesystem `data/policy.json`. `policy.json` groups carry ONLY filesystem `allow`/`deny.access` semantics (`policy.json:6-75`); `network-policy.json` already has the exact domain-group schema the presets need.

**Group schema to copy / extend** (`network-policy.json:6-27`):
```json
"groups": {
  "llm_apis": {
    "description": "LLM provider API endpoints",
    "hosts": [
      "api.openai.com",
      "api.anthropic.com",
      ...
    ]
  }
}
```
Add new preset-token groups here (`anthropic`/`openai`/`github-api` → `*.anthropic.com`, `*.openai.com`, `api.github.com`, EGRESS-04). The ADMX named toggles write the GROUP TOKEN (D-11); `MachineEgressPolicy` expands token→`hosts[]` via this map. `network-policy.json` is already embedded via `include_str!` (`config/embedded.rs:18`) and serde-parsed (`embedded.rs:62-65`) — reuse that loader for token expansion. (If the planner prefers a single source, note the divergence: filesystem groups live in `policy.json`, domain groups in `network-policy.json`; the egress presets belong in the latter.)

---

### `crates/nono-cli/build.rs` (MODIFIED only if a new data file is added)

**Analog:** the SAME file's embedding mechanism (`build.rs:18-23`). `network-policy.json` already has its `rerun-if-changed` directive (`build.rs:20`) and is copied to `OUT_DIR`. If the egress presets extend the existing `network-policy.json`, NO build.rs change is needed. Only add a `rerun-if-changed` line if a NEW data file is introduced — copy the exact form:
```rust
println!("cargo:rerun-if-changed=data/network-policy.json");
```

---

### `dist/windows/nono.admx`/`.adml` via `scripts/build-windows-msi.ps1` (MODIFIED — named toggles)

**Analog:** the shipped Phase-82 `AllowedSuffixes`/`AllowedHosts` `<list>` policies, authored as here-strings in `scripts/build-windows-msi.ps1` (the `.admx`/`.adml` are GENERATED — edit the script, not the artifact; MEMORY: windows_msi_wxs_is_generated, RESEARCH.md Runtime State Inventory).

**Pattern:** add named-toggle `<policy>` elements (enable/disable presets that write a group TOKEN value, D-11) alongside the existing `<list>` policies in the here-string source. Pair with a `validate-windows-msi-contract.ps1`-style assertion (RESEARCH.md SC-5) that the generated ADMX contains the new toggles. Pitfall 1: the `<list>` materializes as N×REG_SZ subkey values (the reader enumerates them per D-13 Option A) — do NOT switch to REG_MULTI_SZ.

---

### `scripts/gates/egress-policy-deny.ps1` (NEW — Dark Factory gate)

**Analog:** `scripts/gates/wfp-egress-isolation.ps1` (structural twin — same WFP-01 daemon-path proof shape).

**Two-function contract to copy** (`wfp-egress-isolation.ps1:106-145, 147-278`) — exports EXACTLY `Test-Precondition` + `Invoke-Gate`, RETURNS a verdict object, NEVER calls `exit` or `Persist-Verdict` (the runner owns exit mapping PASS=0/FAIL=2/SKIP=3/internal=4):
```powershell
function Test-Precondition {
    # return $null when met; return a "reason string" -> SKIP_HOST_UNAVAILABLE
    # 1. admin check (netsh wfp show filters needs elevation)
    # 2. nono-wfp-control pipe reachable
    # 3. nono-agentd-control pipe reachable
}
function Invoke-Gate {
    # returns [ordered]@{ gate; verdict; reason; detail; timestamp }
    # verdict in { 'PASS' | 'FAIL' | 'SKIP_HOST_UNAVAILABLE' }
    # `throw` here = harness-internal error (exit 4), never a silent PASS
}
```
**SID-inspection helper to clone** (`wfp-egress-isolation.ps1:78-100`): `Get-NonoBlockSids` (parses `netsh wfp show filters` XML for `FWPM_CONDITION_ALE_USER_ID` block filters) and `Get-LaunchSid` (parses `sid=S-1-15-2-...` from the launch response).

**Phase-83 gate assertions** (SC-2 + SC-3, RESEARCH.md Validation Architecture):
- SC-2 (corrupted-key non-zero exit): seed a present-but-malformed/ACCESS_DENIED key, run the startup path, assert NON-ZERO exit (the fail-secure proof — Pitfall 3).
- SC-3 (dual-layer deny): under a machine policy of only `*.anthropic.com`, launch a confined agent through the daemon; assert (a) proxy denies an out-of-list host AND (b) `netsh wfp show filters` shows the per-SID block (proxy-bypass blocked).

**Invocation rule (MEMORY durable):** the gate runs via `pwsh -File scripts/verify-dark.ps1 --gate egress-policy-deny`, NEVER `pwsh -Command "<bare path>"` (swallows exit N→1).

---

## Shared Patterns

### Fail-secure read (Result<Option<...>>, never `unwrap_or`)
**Source:** RESEARCH.md Pattern 1/Pitfall 3; semantics from `health.rs:391-417`.
**Apply to:** `machine_policy.rs` reader + its `launch.rs`/daemon-startup caller.
`read_machine_egress_policy() -> Result<Option<MachineEgressPolicy>>`: `Ok(None)` ONLY for absent (ERROR_FILE_NOT_FOUND=2); every other error is `Err(PolicyLoadFailed)` propagated with `?`. The caller MUST `?` it (abort on startup) — NEVER `.ok()` / `.unwrap_or_default()` / `.unwrap_or_else(|_| per_user)` (CLAUDE.md footgun #2, fail-open vulnerability).

### Windows-cfg-gating with non-Windows stub
**Source:** `health.rs:375-383` (`probe_machine_policy` cfg-split), RESEARCH.md Pitfall 5.
**Apply to:** `machine_policy.rs` reader, any WFP/winreg-touching code.
The platform-neutral TYPE compiles everywhere; the `#[cfg(target_os = "windows")]` reader has a `#[cfg(not(target_os = "windows"))]` stub returning `Ok(None)`. Run the mandatory cross-target clippy (`--target x86_64-unknown-linux-gnu` AND `x86_64-apple-darwin`) per CLAUDE.md — Windows-host `cargo check` is NOT a substitute (MEMORY: feedback_clippy_cross_target).

### Single-read → two-consumers (no drift)
**Source:** RESEARCH.md Pattern 2 (D-04).
**Apply to:** daemon startup (`agent_daemon`), `filter.rs`, `launch.rs`.
ONE `read_machine_egress_policy()` at daemon startup → (a) `ProxyFilter::new_strict(&allowlist)` directly + (b) the SAME value's presence drives the per-SID WFP `proxy-only` request over the EXISTING IPC. The WFP service NEVER reads HKLM for egress policy (anti-pattern).

### thiserror struct-variant + Display/match tests
**Source:** `error.rs:206-214` + tests `error.rs:398-430`.
**Apply to:** `NonoError::PolicyLoadFailed`.
Struct variant with `#[error(...)]`, fail-closed doc comment, plus a Display-content test and a `matches!(...)` pattern-match test.

---

## No Analog Found

None. Every Phase-83 file maps to an in-repo analog (most are same-file edits). This is a wiring+hardening phase, not greenfield — the enforcement substrate already exists and is live-proven (WFP-01 gate).

## Metadata

**Analog search scope:** `crates/nono/src/`, `crates/nono-cli/src/` (agent_daemon, bin, health, build), `crates/nono-proxy/src/`, `crates/nono-cli/data/`, `scripts/gates/`.
**Files scanned (read):** error.rs, net_filter.rs, filter.rs (proxy), nono-wfp-service.rs (specs), windows_wfp_contract.rs, launch.rs, control_loop.rs, health.rs, build.rs, policy.json, network-policy.json, lib.rs, wfp-egress-isolation.ps1.
**Key correction surfaced:** D-12 should target `network-policy.json` (domain `hosts[]` groups), NOT `policy.json` (filesystem allow/deny groups) — flagged for the planner.
**Pattern extraction date:** 2026-06-18
