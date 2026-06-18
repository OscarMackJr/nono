# Phase 83: Machine Policy Spine + Egress Control - Research

**Researched:** 2026-06-18
**Domain:** Windows machine-policy registry reading (fail-secure), WFP force-through-proxy SID confinement, L7 FQDN allowlist plumbing, GPO ADMX presets, Dark Factory gate
**Confidence:** HIGH (all six investigation areas grounded in the actual codebase; the enforcement substrate already exists and is largely wired)

## Summary

Phase 83 is overwhelmingly a **wiring + hardening** phase, not a greenfield one. Every enforcement substrate it needs already exists in the repo and is live-proven:

- The **WFP per-SID force-through-proxy filter set** is already buildable by `nono-wfp-service`. `build_policy_filter_specs` (nono-wfp-service.rs:1186) already emits a `loopback_only` PERMIT filter for each `localhost_port` (weight 100 for SID-keyed) plus a block-all filter (weight 0) keyed on `FWPM_CONDITION_ALE_USER_ID = AppContainer SID`. The "block everything except loopback-to-proxy" shape D-01/D-02 describes is exactly what `network_mode: "proxy-only"` + `localhost_ports: [proxy_port]` produces today. [VERIFIED: crates/nono-cli/src/bin/nono-wfp-service.rs]
- The **L7 FQDN matcher** (`HostFilter` / `ProxyFilter`) already passes the entire SC-4 reject matrix via leading-dot `ends_with` + `len >` (net_filter.rs:222). `api.anthropic.com` matches `.anthropic.com`; `anthropic.com`, `evilanthropic.com`, and `anthropic.com.evil.com` are all rejected today — there are existing tests proving it. [VERIFIED: crates/nono/src/net_filter.rs]
- The **control IPC** (`\\.\pipe\nono-agentd-control`) already exists with a tagged-enum `ControlRequest`, and the daemon already drives `nono-wfp-service` per-agent via `wfp_filter_add` (launch.rs:423). [VERIFIED: crates/nono-cli/src/agent_daemon/control_loop.rs, launch.rs]
- The **ADMX template** already ships `AllowedSuffixes` + `AllowedHosts` list policies under `HKLM\SOFTWARE\Policies\nono` (Phase 82). [VERIFIED: dist/windows/nono.admx]
- The **Phase-82 health probe** establishes the present/unreadable/not-configured taxonomy on the exact same HKLM key. [VERIFIED: crates/nono-cli/src/health.rs:386]
- The **Dark Factory gate contract** (two functions `Test-Precondition` + `Invoke-Gate`, return-a-verdict-never-exit) and a near-perfect structural analogue (`wfp-egress-isolation.ps1`, the WFP-01 gate) already exist. [VERIFIED: scripts/verify-dark.ps1, scripts/gates/wfp-egress-isolation.ps1]

**What Phase 83 must actually build:** (1) a new `MachineEgressPolicy` type in the core lib + a Windows-cfg-gated `winreg` reader with the D-07 fail-secure taxonomy and `NonoError::PolicyLoadFailed`; (2) a single daemon-startup read that configures `ProxyFilter` AND derives the per-SID WFP permit instructions; (3) flipping the existing per-agent WFP request from `blocked`/empty-loopback to `proxy-only`/loopback-proxy-port; (4) policy.json egress/domain preset groups + token→FQDN expansion; (5) an ADMX named-toggle preset surface; (6) the `egress-policy-deny` gate.

**Primary recommendation:** Build the `MachineEgressPolicy` type + reader first (it is the spine everything else reads from), then wire the two consumers (ProxyFilter direct, WFP via existing IPC). Do NOT rebuild the WFP filter machinery or the FQDN matcher — reuse them. The single biggest correctness risk is the **ADMX `<list>` serialization shape** (it writes N×REG_SZ named values into a subkey, NOT a single REG_MULTI_SZ value) — D-13 and the EGRESS-01 requirement text both say `REG_MULTI_SZ`, but the shipped Phase-82 ADMX uses `<list>` which does not produce REG_MULTI_SZ. This must be reconciled (see Pitfall 1 + Open Question 1).

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**WFP Enforcement Model (EGRESS-02, SC-3)**
- **D-01:** Force-through-proxy. Kernel WFP does NOT resolve FQDNs. `nono-wfp-service` blocks the agent's AppContainer SID from ALL direct outbound EXCEPT the local proxy's loopback listener. `nono-proxy` is the single L7 chokepoint for the FQDN allowlist.
- **D-02:** WFP permit set = loopback proxy endpoint only. Permit `127.0.0.1`/`::1` on the proxy's listener port only; block all other outbound. No direct DNS egress permit (DNS is proxied). If research finds an agent path that resolves outside the proxy, surface it — default is loopback-proxy-only.
- **D-03:** SC-3 verification: out-of-list domain rejected at BOTH layers — proxy rejects the proxied request AND WFP blocks a direct SID→out-of-list-IP bypass. Both derive from the same `MachineEgressPolicy`.

**Single-Struct Hand-Off / No Drift (POLICY-03, EGRESS-02)**
- **D-04:** The daemon/CLI startup path is the SOLE HKLM policy reader. Deserializes `MachineEgressPolicy` exactly once, configures the in-process proxy allowlist directly, and passes the same struct (or derived per-SID WFP permit instructions) to `nono-wfp-service` over the EXISTING control IPC. The WFP service NEVER reads the registry for egress policy.
- **D-05:** The `MachineEgressPolicy` TYPE lives in the nono core library (canonical type), even though the daemon is the sole reader. Both layers consume the one deserialized instance.
- **D-06:** Read timing = startup snapshot. Read HKLM once at daemon/process startup; hold for process lifetime. GPO change takes effect on next daemon restart — document restart-to-apply explicitly.

**Failure Taxonomy + Precedence (POLICY-01, POLICY-02)**
- **D-07:** Fail-secure boundary:
  - Key ABSENT → fall through to per-user config (NOT a failure).
  - Key PRESENT but unreadable (e.g. ERROR_ACCESS_DENIED) → abort with typed `NonoError::PolicyLoadFailed`. Never fall through.
  - Key PRESENT but malformed (wrong REG_* type, bad UTF-16, unparseable) → also abort with `NonoError::PolicyLoadFailed`. Malformed treated identically to unreadable.
- **D-08:** Precedence = wholesale override. A valid machine policy FULLY REPLACES the per-user egress allowlist; per-user `allow_domain` is ignored entirely. Per-user can never widen the fleet allowlist. (Rejected: union and intersection.)
- **D-09:** Registry read uses the 64-bit view (`KEY_WOW64_64KEY`) regardless of host process bitness.
- **D-10:** `nono` gains the `winreg` crate as the registry reader. Keep Windows-cfg-gated so non-Windows targets compile.

**ADMX Presets / Egress Allowlist Shape (EGRESS-01, EGRESS-04)**
- **D-11:** ADMX named toggles write group TOKENS, not literal FQDNs. Enabling a named toggle ("Allow Anthropic", etc.) writes a stable group token; nono owns token→FQDN mapping and expands at deserialize. Provider lists updatable in nono without re-issuing the ADMX fleet-wide.
- **D-12:** Preset token→FQDN map reuses the existing embedded `policy.json` groups (`crates/nono-cli/data/policy.json`, embedded via `build.rs`). One source of truth for group→FQDN. (If policy.json groups carry only filesystem/command semantics, surface it — default is to extend policy.json with egress/domain groups.)
- **D-13:** The allowlist is authored as wildcard FQDNs in `REG_MULTI_SZ` (e.g. `*.anthropic.com`). The policy's presence switches deny-by-default enforcement on.

**DNS-Component Matching (EGRESS-03)**
- **D-14:** Reuse + harden the existing matcher, don't rebuild. Core `HostFilter` (net_filter.rs:~222) already does leading-dot suffix matching, component-safe for the SC-4 reject set. Phase 83 hardens to explicit DNS-label comparison where useful, adds the SC-4 test matrix, ensures the SAME matcher is fed by `MachineEgressPolicy` (proxy side). Any matching ambiguity fails secure (deny).

### Claude's Discretion
- Exact `NonoError::PolicyLoadFailed` variant shape and how `winreg` error kinds map onto unreadable-vs-malformed (D-07 principle holds regardless).
- Precise serialization of the per-SID WFP permit instructions over control IPC (struct vs derived command), as long as it originates from the one deserialized `MachineEgressPolicy` (D-04).
- Whether to harden `HostFilter` to explicit `.split('.')` label comparison or keep the leading-dot `ends_with` form, provided the SC-4 matrix passes (D-14).

### Deferred Ideas (OUT OF SCOPE)
- Per-agent-launch policy re-read (rejected for Phase 83 in favor of the startup snapshot, D-06).
- WFP FQDN→IP resolution enforcement (rejected in favor of force-through-proxy, D-01).
- Telemetry/compliance config deserialized from the same policy source (TELEM-* / Phase 84). BUT: structure `MachineEgressPolicy` so Phase 84 can add a telemetry section to the SAME single read without re-architecting.
- Reviewed todos not folded: MSI VC++ prereq, POC-cert broker clean-host, macOS RLIMIT defect — all out of scope.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| POLICY-01 | Read `HKLM\SOFTWARE\Policies\nono` at startup using `KEY_WOW64_64KEY`; present overrides per-user, absent falls through | Area 1 — `winreg` `RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey_with_flags(path, KEY_READ \| KEY_WOW64_64KEY)`; D-09 64-bit view. Phase-82 health probe semantics align. |
| POLICY-02 | Present-but-unreadable key fails secure with typed `NonoError` | Area 1 — `io::Error::kind()` / `raw_os_error()` maps `ERROR_FILE_NOT_FOUND`(2)→fall-through; `ERROR_ACCESS_DENIED`(5)→abort; malformed→abort. New `NonoError::PolicyLoadFailed` variant. |
| POLICY-03 | Egress allowlist (+ telemetry, P84) deserialized from a SINGLE policy source; no two layers read divergent config | Area 3 — single daemon-startup read → ProxyFilter (direct) + WFP (via control IPC). WFP service never reads registry. Structure for P84 telemetry section. |
| EGRESS-01 | Admin defines deny-by-default allowlist as wildcard FQDNs; presence switches enforcement on | Area 5 — `ProxyFilter::new_strict` is the deny-by-default constructor. ⚠ ADMX `<list>` produces N×REG_SZ, not REG_MULTI_SZ (Pitfall 1). |
| EGRESS-02 | Allowlist enforced by BOTH proxy + WFP from the same deserialized source | Area 2+3 — proxy filter from struct; WFP per-SID `proxy-only` + loopback-proxy-port permit from same struct, over existing IPC. |
| EGRESS-03 | Wildcard FQDN matching uses DNS-component comparison; ambiguity fails secure | Area 4 — existing `HostFilter` leading-dot `ends_with`+`len>` already passes SC-4. Optionally harden to `.split('.')` label compare (D-14 discretion). |
| EGRESS-04 | Ship AI-provider presets (`*.anthropic.com`, `*.openai.com`, `api.github.com`) | Area 5 — new egress/domain groups in policy.json + ADMX named toggles writing group tokens (D-11/D-12). |
</phase_requirements>

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| HKLM machine-policy read (fail-secure) | CLI/daemon (policy owner) | core lib (TYPE only, D-05) | Library is policy-free per CLAUDE.md; READING + decisions live in CLI/daemon. The `MachineEgressPolicy` TYPE is canonical in core lib so both layers share one shape. |
| `MachineEgressPolicy` type + token→FQDN expansion | core lib (type) + CLI (expansion via embedded policy.json) | — | Type in lib (D-05); the policy.json group map is a CLI/data artifact (D-12). |
| L7 FQDN allowlist enforcement | nono-proxy (`ProxyFilter`) | core lib (`HostFilter` matcher) | Proxy is the single L7 chokepoint (D-01). Matcher logic in lib, async-DNS wrapper in proxy. |
| L3/4 per-SID kernel egress confinement | nono-wfp-service (kernel WFP) | daemon (drives it via IPC) | WFP makes proxy-bypass structurally impossible (D-01). Service receives derived permit instructions; never reads registry (D-04). |
| Single-source hand-off (no drift) | daemon startup path | control IPC `\\.\pipe\nono-agentd-control` | One read, two consumers (D-04). |
| ADMX template + Intune OMA-URI | dist/windows + build-windows-msi.ps1 | — | Generated artifacts; admin-facing fleet-push surface (D-11). |

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `winreg` | `0.56.0` | Safe Rust HKLM read with `KEY_WOW64_64KEY` | The de-facto Rust registry crate; already earmarked in Phase-82 code comments (provision_windows.rs:360, main.rs:103). D-10. [ASSUMED — see Package Legitimacy Audit] |
| `windows-sys` | `0.59` (already in workspace) | WFP FFI (`FwpmFilterAdd0` etc.), already used by nono-wfp-service | No new dep; reuse. CLAUDE.md confirms 0.59 workspace pin. [VERIFIED: workspace Cargo.toml] |
| `serde` / `serde_json` | (already in workspace) | `MachineEgressPolicy` (de)serialization + control IPC frames | Already the serialization stack for `WfpRuntimeActivationRequest`. [VERIFIED] |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `winreg` 0.56 | `windows-registry` 0.6.1 (Microsoft's official crate) | `windows-registry` is the newer MS-blessed crate, but `winreg` is what the Phase-82 comments already earmarked (D-10), is mature, and its `io::Error`-based error surface maps cleanly onto the D-07 taxonomy via `raw_os_error()`. Stick with `winreg` per the locked decision. |
| `winreg` reader | continuing the Phase-82 `reg query` subprocess (health.rs) | The subprocess approach cannot reliably distinguish malformed (wrong REG_* type) from absent, and spawning a child is heavier + harder to fail-secure. D-10 mandates the crate. The subprocess probe stays in `health.rs` (read-only diagnostic) but the authoritative egress read uses `winreg`. |

**Installation:**
```bash
# In crates/nono/Cargo.toml, Windows-only target dependency:
# [target.'cfg(target_os = "windows")'.dependencies]
# winreg = "0.56"
```
Add to the `[target.'cfg(windows)'.dependencies]` table in `crates/nono/Cargo.toml` so non-Windows targets never pull it (D-10, CLAUDE.md cross-target rule).

**Version verification:** `cargo search winreg` returned `winreg = "0.56.0"` on 2026-06-18. [VERIFIED: crates.io via cargo search] — but per the package-name provenance rule, treat as `[ASSUMED]` until slopcheck/official-doc confirmation (see audit below).

## Package Legitimacy Audit

> slopcheck was not available in this research environment (pip install not attempted in sandbox). Per the graceful-degradation rule, the one NEW package is tagged `[ASSUMED]` and the planner MUST gate its install behind a `checkpoint:human-verify` task.

| Package | Registry | Age | Downloads | Source Repo | slopcheck | Disposition |
|---------|----------|-----|-----------|-------------|-----------|-------------|
| `winreg` | crates.io | mature (years) | very high (millions; transitive dep of many crates) | github.com/gentoo90/winreg-rs | not run | `[ASSUMED]` — planner adds checkpoint:human-verify before adding to Cargo.toml |

**Packages removed due to slopcheck [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none
**Verification step for the planner:** confirm `winreg` at https://crates.io/crates/winreg resolves to `gentoo90/winreg-rs`, current version `0.56.x`, before adding the dependency. (Cargo.lock already pins everything else; this is the only new external crate.)

## Architecture Patterns

### System Architecture Diagram

```
                    ┌─────────────────────────────────────────────┐
  GPO ADMX  ──────► │  HKLM\SOFTWARE\Policies\nono                 │
  Intune OMA-URI    │   AllowedSuffixes\  (N × REG_SZ list)        │
  (admin push)      │   AllowedHosts\     (N × REG_SZ list)        │
                    │   <preset toggles>  (group TOKEN values)     │
                    └───────────────────┬─────────────────────────┘
                                        │  read ONCE at startup (D-04, D-06)
                                        │  winreg + KEY_WOW64_64KEY (D-09)
                                        ▼
              ┌──────────────────────────────────────────────────┐
              │  nono daemon / CLI startup path  (SOLE reader)    │
              │  read_machine_egress_policy() -> Result<Option<…>>│
              │    None  → key absent → fall through to per-user  │
              │    Some  → wholesale override (D-08)              │
              │    Err(PolicyLoadFailed) → ABORT fail-secure(D-07)│
              │                                                    │
              │  MachineEgressPolicy { suffixes, hosts, presets } │
              │   └─ expand preset TOKENS → FQDNs via policy.json │
              └───────────┬───────────────────────┬──────────────┘
                          │ (a) direct            │ (b) derive per-SID
                          │ in-process            │ WFP permit instructions
                          ▼                       ▼  (same struct, D-04)
        ┌────────────────────────┐   ┌────────────────────────────────────┐
        │ nono-proxy ProxyFilter │   │ \\.\pipe\nono-agentd-control (IPC)  │
        │  new_strict(allowlist) │   │   → wfp_filter_add(sid, proxy_port) │
        │  L7 FQDN deny-default  │   └──────────────┬─────────────────────┘
        │  (HostFilter matcher)  │                  │ \\.\pipe\nono-wfp-control
        └───────────┬────────────┘                  ▼
                    │              ┌──────────────────────────────────────┐
   confined agent ──┤              │ nono-wfp-service (LocalSystem, kernel)│
   (AppContainer    │              │  per-SID filters under NONO_SUBLAYER: │
    package SID)    │              │   PERMIT loopback:proxy_port (wt 100) │
                    │              │   BLOCK  all other outbound (wt 0)    │
                    └──────────────┤   keyed on ALE_USER_ID = package SID  │
   all egress forced to loopback   └──────────────────────────────────────┘
   proxy:port; WFP blocks any direct bypass (D-01/D-02)
```

The agent has only one egress path: loopback→proxy. The proxy resolves DNS + enforces the L7 FQDN allowlist. A direct SID→out-of-list-IP attempt (proxy bypass) is BLOCKed by the per-SID WFP block filter (SC-3 dual-layer deny).

### Recommended Code Structure
```
crates/nono/src/
├── lib.rs                      # re-export MachineEgressPolicy (pub)
└── machine_policy.rs           # NEW: MachineEgressPolicy type (D-05);
                                #   Windows-cfg-gated winreg reader (D-07/D-09/D-10);
                                #   non-Windows stub returning Ok(None)

crates/nono/src/error.rs        # NEW variant NonoError::PolicyLoadFailed { reason }

crates/nono-cli/data/policy.json     # NEW egress/domain preset groups (D-12)
crates/nono-cli/src/agent_daemon/
├── mod.rs / launch.rs          # daemon startup: read policy once;
                                #   configure ProxyFilter; flip wfp_filter_add → proxy-only
crates/nono-proxy/src/filter.rs # feed allowlist into ProxyFilter::new_strict
dist/windows/nono.admx + .adml  # NEW named-toggle preset policies (D-11)
scripts/build-windows-msi.ps1   # here-string source for the ADMX (EDIT THIS, not the .wxs/.admx)
scripts/gates/egress-policy-deny.ps1  # NEW Dark Factory gate
```

### Pattern 1: Fail-secure winreg read with the D-07 taxonomy
**What:** Open the 64-bit hive; distinguish absent (fall-through) from unreadable/malformed (abort).
**When to use:** The single startup read (D-04).
**Example:**
```rust
// crates/nono/src/machine_policy.rs  (Windows arm)
// Source pattern: winreg 0.56 RegKey API + std::io::Error::raw_os_error()
#[cfg(target_os = "windows")]
pub fn read_machine_egress_policy() -> Result<Option<MachineEgressPolicy>> {
    use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_64KEY};
    use winreg::RegKey;

    const POLICY_PATH: &str = r"SOFTWARE\Policies\nono";
    const ERROR_FILE_NOT_FOUND: i32 = 2;   // key ABSENT  -> fall through
    // (ERROR_ACCESS_DENIED = 5, ERROR_PATH_NOT_FOUND = 3 -> abort)

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    // D-09: KEY_WOW64_64KEY forces the 64-bit view regardless of process bitness.
    let key = match hklm.open_subkey_with_flags(POLICY_PATH, KEY_READ | KEY_WOW64_64KEY) {
        Ok(k) => k,
        Err(e) if e.raw_os_error() == Some(ERROR_FILE_NOT_FOUND) => return Ok(None), // D-07 absent
        Err(e) => {
            // PRESENT but unreadable (ACCESS_DENIED, etc.) -> fail secure (D-07).
            return Err(NonoError::PolicyLoadFailed {
                reason: format!("machine policy key present but unreadable: {e}"),
            });
        }
    };

    // Read the allowlist values. ANY malformed shape (wrong REG_* type, bad UTF-16,
    // unparseable) is treated identically to unreadable -> abort (D-07).
    let policy = parse_machine_egress_policy(&key).map_err(|reason| {
        NonoError::PolicyLoadFailed { reason }
    })?;
    Ok(Some(policy))
}

#[cfg(not(target_os = "windows"))]
pub fn read_machine_egress_policy() -> Result<Option<MachineEgressPolicy>> {
    Ok(None) // macOS/Linux have no HKLM; out of scope (CONTEXT.md domain).
}
```
Notes:
- `winreg`'s errors are `std::io::Error`, so `raw_os_error()` gives the Win32 code directly. `ERROR_FILE_NOT_FOUND` (2) on the `open_subkey` is "key absent." `ERROR_ACCESS_DENIED` (5) is "present but unreadable." This is exactly the Phase-82 health-probe split, but typed instead of string-matched on `reg query` stderr. [VERIFIED: health.rs:402 matches "access denied"]
- For the value reads: `key.get_value::<String, _>(name)` returns `Err` with `ErrorKind::InvalidData` for a wrong REG_* type or bad UTF-16 — map that to `PolicyLoadFailed` (malformed). A list-subkey read that finds a value of the wrong type is also malformed → abort.

### Pattern 2: Single read → two consumers (no drift, D-04)
**What:** Read once at daemon startup; configure ProxyFilter directly; pass derived permit instructions to WFP over the existing IPC.
**Example:**
```rust
// daemon startup (agent_daemon)
let machine_policy = nono::read_machine_egress_policy()?;   // D-04 SOLE read; '?' aborts on PolicyLoadFailed (D-07)
let allowlist: Vec<String> = match &machine_policy {
    Some(p) => p.expanded_allowlist(),   // wholesale override (D-08), presets expanded (D-11/D-12)
    None    => per_user_allow_domain(),  // fall through (D-07 absent)
};
// (a) configure the in-process proxy directly:
let proxy_filter = ProxyFilter::new_strict(&allowlist);  // deny-by-default (EGRESS-01)
// (b) the SAME allowlist's presence drives the per-SID WFP proxy-only confinement;
//     WFP never sees the FQDN list — only the loopback proxy port to permit.
```
The WFP service receives ONLY `localhost_ports: [proxy_port]` + `network_mode: "proxy-only"` (no FQDNs), which is correct for D-01 (WFP does not resolve FQDNs). The single source-of-truth is the one `machine_policy` value: both the proxy allowlist and the "WFP enforcement is on" decision derive from it.

### Pattern 3: Flip the per-agent WFP request to force-through-proxy
**What:** The existing `wfp_filter_add` (launch.rs:423) sends `network_mode: "blocked"` with empty `localhost_ports` — that blocks ALL outbound for the SID (the WFP-01 model). For Phase 83's force-through-proxy, change it to permit the loopback proxy port.
**Current (launch.rs:428):**
```rust
request_kind: "activate_blocked_mode",
network_mode: "blocked",
tcp_connect_ports: vec![],
localhost_ports: vec![],          // <- blocks everything
session_sid: Some(package_sid),
```
**Phase 83 (force-through-proxy, D-02):**
```rust
request_kind: "activate_proxy_mode",
network_mode: "proxy-only",
tcp_connect_ports: vec![],
localhost_ports: vec![proxy_port], // <- PERMIT loopback:proxy_port; BLOCK all else
session_sid: Some(package_sid),
```
The service's `build_policy_filter_specs` then emits, per layer (connect-v4/v6): a `loopback_only` PERMIT filter on `IP_REMOTE_PORT == proxy_port` (weight 100, SID-keyed) PLUS a block-all filter (weight 0) — because `network_mode != "allow-all"` makes `needs_outbound_block` true (nono-wfp-service.rs:1228). The permit weight (100) beats the block weight (0) so loopback-to-proxy is permitted and everything else blocked. [VERIFIED: nono-wfp-service.rs:1186-1282, 1497-1507]

### Anti-Patterns to Avoid
- **Reading the registry from two places.** The WFP service must NEVER read HKLM for egress policy (D-04). It receives derived instructions over IPC. (It DOES read its own service config, but not the egress allowlist.)
- **Rebuilding the FQDN matcher.** `HostFilter` already passes SC-4 (D-14). Add tests; optionally harden; do not replace.
- **Rebuilding the WFP filter machinery.** `build_policy_filter_specs` + `add_policy_filter` already produce the exact permit/block shape. Only the *request* changes.
- **Falling through to per-user on a malformed key.** D-07: once the key exists, ANY error aborts. `unwrap_or_default()` / `unwrap_or_else(|_| per_user)` on the policy read is a fail-OPEN vulnerability (CLAUDE.md footgun #2).
- **Using string `starts_with` on hostnames.** The current `ends_with(".anthropic.com") && len >` is component-safe (the leading dot + length guard prevent `evilanthropic.com`). If hardening to `.split('.')`, keep the fail-secure default. (CLAUDE.md path-comparison principle applied to DNS labels.)

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| 64-bit HKLM read | Raw `RegOpenKeyExW` FFI | `winreg` 0.56 (D-10) | Safe wrapper; `io::Error` maps cleanly onto D-07; avoids unsafe + manual UTF-16. |
| Wildcard FQDN matching | New DNS-label comparator | `nono::HostFilter` (net_filter.rs) | Already passes the entire SC-4 reject matrix with existing tests; DNS-rebinding + cloud-metadata deny already handled. |
| Per-SID WFP filter add | New `FwpmFilterAdd0` calls | `nono-wfp-service` `proxy-only` request path | Loopback-permit + block-all + weight ordering + RAII cleanup + startup sweep all exist and are live-proven (WFP-01 gate). |
| Control IPC | New named pipe | `\\.\pipe\nono-agentd-control` (`ControlRequest` enum) | Medium-IL SACL already bars Low-IL agents; framing + dispatch exist. |
| Dark Factory gate scaffolding | New runner logic | `verify-dark.ps1` two-function contract + clone `wfp-egress-isolation.ps1` | Verdict shape, exit mapping, persist-before-emit, precondition skip all owned by the runner. |

**Key insight:** Phase 83's value is *connecting* proven pieces through one authoritative read, not building enforcement primitives. The dominant risk is config-shape mismatch (ADMX list vs REG_MULTI_SZ), not missing capability.

## Common Pitfalls

### Pitfall 1: ADMX `<list>` does NOT produce `REG_MULTI_SZ` (CRITICAL — shape mismatch)
**What goes wrong:** EGRESS-01 + D-13 both say the allowlist is authored as `REG_MULTI_SZ`. But the shipped Phase-82 ADMX (dist/windows/nono.admx:96-100, 131-135) uses `<list ... additive="true">`, which Group Policy materializes as **multiple separate `REG_SZ` named values inside a subkey** (`...\Policies\nono\AllowedSuffixes\nono_suffixes` → value `1`=`.anthropic.com`, `2`=`.openai.com`, ...), NOT a single `REG_MULTI_SZ` value. The winreg reader must enumerate the subkey's values, not call `get_value::<Vec<String>>` (REG_MULTI_SZ).
**Why it happens:** ADMX `<list>` and `REG_MULTI_SZ` are different GP mechanisms; the requirement text and the shipped template diverged across phases.
**How to avoid:** Decide ONE shape and make the reader match the ADMX:
- Option A (matches shipped ADMX): reader enumerates `AllowedSuffixes\` and `AllowedHosts\` subkey values (each REG_SZ) via `key.open_subkey(...).enum_values()`.
- Option B (matches requirement text): change the ADMX to write a single `REG_MULTI_SZ` value (ADMX `<list>` cannot do this directly — would need a different element type or accept the subkey-of-REG_SZ shape and update D-13's wording).
**Warning signs:** Reader returns an empty allowlist even though GP shows the policy "enabled"; or a `get_value` returns `ErrorKind::InvalidData` (wrong type). See Open Question 1 — the planner/discuss-phase must lock the shape.

### Pitfall 2: Intune 32-bit MDM extension writes to WOW6432Node
**What goes wrong:** The Intune MDM extension can run 32-bit; without `KEY_WOW64_64KEY` on the WRITE side it lands policy in `SOFTWARE\WOW6432Node\Policies\nono`, while nono reads the 64-bit hive — silent divergence.
**Why it happens:** WOW64 registry redirection.
**How to avoid:** The READ side is locked to `KEY_WOW64_64KEY` (D-09) — correct. Document in the ADMX/Intune notes (already present in nono.admx:24-28) that deployment scripts must use `[RegistryView]::Registry64`. The reader cannot fix a mis-deployed write; the gate's SKIP path should note "key not in 64-bit hive."
**Warning signs:** Policy visible under WOW6432Node but reader sees absent → falls through to per-user (looks like the override silently didn't apply).

### Pitfall 3: Malformed-key fall-through (fail-open)
**What goes wrong:** A `match`/`unwrap_or` that treats a read error as "no policy" falls through to the permissive per-user state — defeating fleet control.
**Why it happens:** Ergonomic `unwrap_or_default()` habit; D-07 is counterintuitive (malformed == unreadable == abort, unlike absent).
**How to avoid:** The reader returns `Result<Option<…>>`: `Ok(None)` ONLY for the absent case; every other error is `Err(PolicyLoadFailed)` propagated with `?`. The startup caller must `?` it (abort), never `.ok()`/`.unwrap_or`.
**Warning signs:** A corrupted key produces a clean startup instead of a non-zero exit — this is exactly what the SC-2 gate asserts must NOT happen.

### Pitfall 4: WFP loopback permit must beat the block (weight ordering)
**What goes wrong:** If the loopback PERMIT and the block-all filter have wrong relative weights, either all traffic is blocked (proxy unreachable → agent bricked) or the block is ineffective.
**Why it happens:** WFP arbitrates same-sublayer filters by weight.
**How to avoid:** This is already correct in `add_policy_filter` (nono-wfp-service.rs:1497): SID-keyed PERMIT = weight 100, BLOCK = weight 0. Do NOT change these. Verify the `proxy-only` request actually carries the proxy port in `localhost_ports` so the permit filter is emitted at all.
**Warning signs:** Agent cannot reach its own proxy (everything blocked) — permit filter missing or out-weighed.

### Pitfall 5: cross-target compile (Linux/macOS) breaks on the winreg/WFP additions
**What goes wrong:** `winreg` and `WindowsNetworkPolicy`/WFP types are Windows-only; an un-gated import or a `MachineEgressPolicy` field referencing a Windows type breaks `cargo clippy --target x86_64-unknown-linux-gnu`.
**Why it happens:** CLAUDE.md cross-target rule; the workspace CI runs `--workspace --all-targets --all-features` on Linux+macOS.
**How to avoid:** Keep `winreg` in `[target.'cfg(windows)'.dependencies]`. The `MachineEgressPolicy` TYPE (D-05) should be **platform-neutral** (just `Vec<String>` allowlist fields + preset tokens) so it compiles everywhere; only the *reader* is cfg-gated (non-Windows stub returns `Ok(None)`). Run the mandatory `cargo clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` AND `--target x86_64-apple-darwin` from the dev host (or mark the verify REQ PARTIAL per the cross-target checklist if the toolchain is absent).
**Warning signs:** Windows-host `cargo check` passes but CI Clippy(ubuntu/macos) goes red — the recurring v2.x trap (MEMORY: feedback_clippy_cross_target).

### Pitfall 6: DNS proxied — confirm no agent path resolves outside the proxy (D-02 open check)
**What goes wrong:** D-02 says "no direct DNS egress permit — DNS is proxied." If an agent does its own UDP/53 resolution (some runtimes call the system resolver directly), and WFP blocks all non-loopback outbound, DNS fails and the agent appears offline rather than filtered.
**Why it happens:** Not all HTTP clients route DNS through an HTTP proxy; some resolve locally first.
**How to avoid:** The proxy must accept hostnames in CONNECT and resolve them itself (`ProxyFilter::check_host` already does `tokio::net::lookup_host` — filter.rs:68). For agents that resolve locally, the loopback-proxy-only WFP shape will block their DNS. Surface this as Open Question 2; the default (loopback-proxy-only) is correct for HTTP-CONNECT-proxy-aware agents, which is nono's model.
**Warning signs:** Agent gets DNS-resolution failures instead of HTTP 403/deny from the proxy.

## Code Examples

### Reading an ADMX `<list>` subkey (N × REG_SZ) — matches shipped Phase-82 ADMX
```rust
// crates/nono/src/machine_policy.rs (Windows arm)
// Source: winreg 0.56 RegKey::enum_values; matches dist/windows/nono.admx <list> shape
#[cfg(target_os = "windows")]
fn read_list_subkey(parent: &winreg::RegKey, name: &str) -> std::result::Result<Vec<String>, String> {
    use winreg::enums::{KEY_READ, KEY_WOW64_64KEY};
    match parent.open_subkey_with_flags(name, KEY_READ | KEY_WOW64_64KEY) {
        Ok(sub) => {
            let mut out = Vec::new();
            for item in sub.enum_values() {
                // enum_values yields Result<(String /*value name*/, RegValue)>
                let (_vname, val) = item.map_err(|e| format!("enum {name}: {e}"))?;
                // Each list entry is REG_SZ. A non-SZ type here is MALFORMED (D-07 abort).
                let s: String = String::from_reg_value(&val)
                    .map_err(|e| format!("{name} entry not REG_SZ (malformed): {e}"))?;
                out.push(s);
            }
            Ok(out)
        }
        // Subkey absent is fine here (the parent key existing is what gates enforcement).
        Err(e) if e.raw_os_error() == Some(2) => Ok(Vec::new()),
        Err(e) => Err(format!("open {name}: {e}")),
    }
}
```

### SC-4 reject-matrix test (EGRESS-03 — add to net_filter.rs)
```rust
// Source: existing HostFilter behavior (net_filter.rs:222) — codify the SC-4 contract
#[test]
fn test_sc4_dns_component_matrix() {
    let f = HostFilter::new_strict(&["*.anthropic.com".to_string()]);
    let ip = vec![IpAddr::V4(Ipv4Addr::new(104, 18, 7, 96))];
    assert!(f.check_host("api.anthropic.com", &ip).is_allowed());      // match
    assert!(!f.check_host("anthropic.com", &ip).is_allowed());          // bare domain rejected
    assert!(!f.check_host("evilanthropic.com", &ip).is_allowed());      // no leading-dot boundary
    assert!(!f.check_host("anthropic.com.evil.com", &ip).is_allowed()); // suffix-injection rejected
}
```
(The existing `test_wildcard_does_not_match_bare_domain` + `test_host_not_in_allowlist` already cover two of these; add the explicit four-case matrix as the named EGRESS-03 contract.)

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Phase-82 `reg query` subprocess probe (health.rs) | `winreg` typed read for the authoritative egress policy | Phase 83 | Typed absent/unreadable/malformed split; the subprocess probe stays read-only in `health.rs`. |
| Per-agent WFP = `network_mode:"blocked"`, all outbound blocked (WFP-01) | `network_mode:"proxy-only"` + loopback-proxy-port permit (force-through-proxy) | Phase 83 (D-01/D-02) | Agent egresses ONLY via the proxy; proxy enforces FQDN allowlist; WFP blocks bypass. |
| Per-user `allow_domain` only | Machine policy wholesale override (D-08) | Phase 83 | Fleet admin controls egress; per-user cannot widen. |

**Deprecated/outdated:**
- Treating a machine-policy read error as "no policy" — D-07 makes malformed == abort.

## Runtime State Inventory

> Phase 83 is primarily code/config. The one runtime-state surface is the WFP filter shape change.

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | None — `MachineEgressPolicy` is read-at-startup, not persisted by nono. The allowlist lives in HKLM (admin-owned). | None |
| Live service config | `nono-wfp-service` per-SID filters: existing agents launched under the OLD `blocked` request carry a block-all filter; after the Phase-83 flip, NEW launches carry the `proxy-only` permit+block set. Filters are per-launch, reaped on agent exit + startup-swept. | No migration of existing filters needed — restart-to-apply (D-06); the startup sweep + per-agent reap reclaim old filters. |
| OS-registered state | `HKLM\SOFTWARE\Policies\nono` sentinel key (`InstalledByMsi=1`) shipped by Phase-82 MSI. Phase 83 READS the AllowedSuffixes/AllowedHosts subkeys an admin pushes there. | None (read-only; admin populates via ADMX). |
| Secrets/env vars | None — egress allowlist is not secret; no new env vars (the proxy port is already known to the daemon). | None |
| Build artifacts | ADMX template (`dist/windows/nono.admx/.adml`) is GENERATED from here-strings in `scripts/build-windows-msi.ps1` (MEMORY: windows_msi_wxs_is_generated). Editing the `.admx` directly is overwritten on rebuild. | Edit the PowerShell here-string source, not the `.admx`. |

**Nothing found in category:** Stored data, secrets/env vars — verified by inspecting the read-at-startup design (D-06) and the absence of any new persisted file.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `winreg` crate | machine-policy read (Windows) | ✓ (crates.io) | 0.56.0 | none — D-10 mandates it; gated to Windows |
| `windows-sys` | WFP FFI | ✓ (workspace) | 0.59 | none |
| `nono-wfp-service` (running, elevated) | SC-3 WFP dual-layer proof | host-gated | — | gate SKIPs if pipe `\\.\pipe\nono-wfp-control` absent |
| `nono-agentd` (running, non-elevated user ctx) | SC-3 daemon-path launch | host-gated | — | gate SKIPs if pipe `\\.\pipe\nono-agentd-control` absent |
| Admin elevation | `netsh wfp show filters` (gate inspection) | host-gated | — | gate Test-Precondition returns SKIP_HOST_UNAVAILABLE |

**Missing dependencies with no fallback:** none at build time. The SC-3 live proof is host-gated (admin + both services running) — consistent with the Dark Factory mandate: the scripted gate SKIPs cleanly on a host without the prerequisites and the milestone closes on the aggregator.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in `#[test]` (unit + integration) + PowerShell Dark Factory gates (`scripts/verify-dark.ps1`) |
| Config file | none — `cargo test` + `pwsh -File scripts/verify-dark.ps1 --gate egress-policy-deny` |
| Quick run command | `cargo test -p nono machine_policy && cargo test -p nono net_filter` |
| Full suite command | `make ci` (clippy --workspace --all-targets --all-features + fmt + tests) + `pwsh -File scripts/verify-dark.ps1 --gate egress-policy-deny` |

### Observable verification per Success Criterion
| SC | Behavior | How observed (automated) |
|----|----------|--------------------------|
| SC-1 | Reads HKLM with `KEY_WOW64_64KEY`; present overrides per-user; absent falls through | Unit: reader returns `Ok(Some(_))` when a test key exists (Windows-gated integration test seeding a HKCU-redirected or temp key), `Ok(None)` when absent. Cross-platform: non-Windows stub returns `Ok(None)`. |
| SC-2 | Unreadable/corrupt key → `NonoError::PolicyLoadFailed`, NOT fall-through; gate asserts non-zero exit | Unit: a present-but-malformed value (wrong REG_* type) → reader returns `Err(PolicyLoadFailed)`. Gate `egress-policy-deny`: seed a corrupted key (ACCESS_DENIED ACL or wrong-type value), run the startup path, assert non-zero exit. |
| SC-3 | Out-of-list domain rejected at BOTH proxy AND WFP from the same struct | Unit: `ProxyFilter::new_strict(allowlist).check_host("evil.com").is_allowed() == false`. Gate (host-gated): launch a confined agent through the daemon under a machine policy of only `*.anthropic.com`; assert (a) proxy denies a request to an out-of-list host AND (b) `netsh wfp show filters` shows the per-SID block filter (proxy-bypass blocked). Clone `wfp-egress-isolation.ps1` SID-inspection. |
| SC-4 | DNS-component matching matrix | Unit: `test_sc4_dns_component_matrix` (4 cases above) in net_filter.rs. |
| SC-5 | ADMX + Intune OMA-URI push the allowlist; AI-provider presets available | Unit/script: parse the generated ADMX, assert the preset toggles + `AllowedSuffixes`/`AllowedHosts` policies exist; assert policy.json carries the preset groups (`*.anthropic.com`, `*.openai.com`, `api.github.com`). Existing `validate-windows-msi-contract.ps1` pattern for ADMX assertions. |

### Sampling Rate
- **Per task commit:** `cargo test -p nono machine_policy` + `cargo test -p nono net_filter` (< 30 s).
- **Per wave merge:** `make ci` (workspace clippy all-targets all-features + fmt + tests).
- **Phase gate:** `pwsh -File scripts/verify-dark.ps1 --gate egress-policy-deny` PASS (or clean SKIP_HOST_UNAVAILABLE on a host without the services) before `/gsd:verify-work`. Invoke via `-File` / direct path, NEVER `pwsh -Command "<bare path>"` (swallows exit N→1; MEMORY durable lesson).

### Wave 0 Gaps
- [ ] `crates/nono/src/machine_policy.rs` — new module; needs unit tests for absent/unreadable/malformed + 64-bit-view (POLICY-01/02). Windows-gated integration test needs a way to seed a test key (consider `RegKey::predef(HKEY_CURRENT_USER)` redirection or a temp subkey under a writable hive, since HKLM\Policies needs admin).
- [ ] `crates/nono/src/error.rs` — `NonoError::PolicyLoadFailed { reason: String }` variant (`#[error(...)]` thiserror).
- [ ] `scripts/gates/egress-policy-deny.ps1` — NEW gate (two-function contract; clone `wfp-egress-isolation.ps1`).
- [ ] SC-4 four-case test in `net_filter.rs` (codify EGRESS-03 contract).
- [ ] policy.json egress/domain preset groups + an expansion unit test (token→FQDN).
- [ ] ADMX named-toggle preset policies (in the build-windows-msi.ps1 here-string) + an ADMX-contract assertion.

## Security Domain

### Applicable ASVS Categories
| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V1 Architecture | yes | Single-source-of-truth policy read (D-04); no-drift between enforcement layers |
| V4 Access Control | yes | Machine policy wholesale override of per-user (D-08); control pipe Medium-IL SACL bars Low-IL agents |
| V5 Input Validation | yes | Fail-secure parse of HKLM values (D-07); reject malformed REG_* types; DNS-component matching fails secure (D-14) |
| V8 Data Protection | partial | Allowlist is not secret; ensure no allowlist contents leak into agent-readable surfaces |
| V12 Files/Resources | yes | 64-bit hive view (D-09) prevents WOW64-redirection bypass |
| V13 Config | yes | Deny-by-default egress (EGRESS-01); presence-switches-enforcement; fail-secure on read failure |

### Known Threat Patterns for {Windows machine-policy + WFP egress}
| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Per-user config widens fleet allowlist | Elevation of Privilege | Wholesale override (D-08); per-user ignored when machine policy present |
| Malformed/corrupt key → fail-open to permissive state | Tampering / EoP | D-07: malformed == unreadable == abort with `PolicyLoadFailed` |
| WOW64 redirection (32-bit Intune) writes to wrong hive | Tampering | `KEY_WOW64_64KEY` on read (D-09); ADMX deployment note for 64-bit write |
| Proxy bypass — agent connects directly to out-of-list IP | Bypass / Spoofing | Per-SID WFP block-all-except-loopback-proxy (D-01/D-02); permit weight beats block |
| DNS rebinding / cloud-metadata SSRF through the proxy | Spoofing / Info Disclosure | Existing `HostFilter` link-local + metadata-hostname deny (net_filter.rs:79-102) — already in place |
| Low-IL agent drives the control pipe to alter policy | EoP | Control pipe SACL `S:(ML;;NW;;;ME)` (Medium-IL minimum) bars AppContainer/Low-IL callers (control_loop.rs:97) |

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `winreg` 0.56 is the legitimate `gentoo90/winreg-rs` crate | Standard Stack / Package Audit | Slopsquat risk — planner gates with checkpoint:human-verify before adding the dep |
| A2 | ADMX `<list>` materializes as N×REG_SZ subkey values, not a single REG_MULTI_SZ | Pitfall 1 / Open Q1 | If wrong, the reader shape is wrong; but this is standard GP `<list>` behavior — high confidence. Still: lock the shape in discuss-phase since D-13/EGRESS-01 say REG_MULTI_SZ. |
| A3 | The `proxy-only` request with `localhost_ports:[proxy_port]` yields permit-loopback + block-all for the SID | Pattern 3 | Verified against build_policy_filter_specs source; low risk. The proxy port value at daemon startup must be known — confirm the daemon's proxy listener port is available where `wfp_filter_add` is called. |
| A4 | Per-user `allow_domain` exists as the fall-through source for D-08 override | Pattern 2 | If the per-user allowlist plumbing differs, the "wholesale override" wiring point changes; confirm where the per-user proxy allowlist is currently sourced. |
| A5 | `winreg` errors expose `raw_os_error()` (they are `std::io::Error`) | Pattern 1 | If a different error type, the D-07 code-mapping changes shape (still achievable). High confidence — winreg returns `io::Result`. |

## Open Questions

1. **ADMX list shape vs REG_MULTI_SZ (must lock before planning the reader).**
   - What we know: shipped ADMX uses `<list additive="true">` → N×REG_SZ subkey values. D-13/EGRESS-01 say `REG_MULTI_SZ`.
   - What's unclear: whether to (A) make the reader enumerate the subkey (match shipped ADMX) or (B) change the ADMX to emit REG_MULTI_SZ.
   - Recommendation: **Option A** — reader enumerates the `<list>` subkey values (least churn; the shipped template already works for fleet push). Update D-13's wording to "list of REG_SZ entries under the AllowedSuffixes/AllowedHosts subkeys" so requirement text matches reality. Flag to discuss-phase.

2. **Agent DNS path under loopback-proxy-only (D-02 surface).**
   - What we know: the proxy resolves DNS itself (`ProxyFilter::check_host`); HTTP-CONNECT-aware clients route through it.
   - What's unclear: whether any in-scope engine resolves DNS locally (UDP/53) before connecting — which WFP would block.
   - Recommendation: keep loopback-proxy-only (D-02 default). Document that engines must be proxy-aware (already nono's model). Revisit only if a real engine fails DNS under enforcement.

3. **Proxy listener port availability at `wfp_filter_add` call site.**
   - What we know: `wfp_filter_add` (launch.rs) currently sends empty `localhost_ports`. The force-through-proxy flip needs the proxy port.
   - What's unclear: whether the daemon already knows its in-process proxy port at that call site (it configures the proxy at startup).
   - Recommendation: thread the proxy port from daemon startup (where `ProxyFilter` is configured) into the per-agent WFP request. Confirm the wiring point during planning.

## Sources

### Primary (HIGH confidence — codebase)
- `crates/nono-cli/src/bin/nono-wfp-service.rs` — WFP filter specs, loopback-permit + block-all, weight ordering, SID-keyed conditions (lines 1186-1282, 1413-1546).
- `crates/nono/src/net_filter.rs` — `HostFilter` matcher + SC-4-passing tests (lines 188-231, 274-294).
- `crates/nono-proxy/src/filter.rs` — `ProxyFilter::new_strict`, async DNS resolution (lines 30-92).
- `crates/nono-cli/src/agent_daemon/control_loop.rs` — control IPC, `ControlRequest`, Medium-IL SACL.
- `crates/nono-cli/src/agent_daemon/launch.rs` — `wfp_filter_add`/`wfp_filter_remove` current request shape (lines 423-525).
- `crates/nono-cli/src/exec_strategy_windows/network.rs` — `build_wfp_runtime_activation_request`, `WindowsNetworkPolicyMode::ProxyOnly` → request mapping (lines 490-563).
- `crates/nono-cli/src/health.rs` — Phase-82 machine-policy probe taxonomy (lines 386-417).
- `crates/nono-cli/data/policy.json` — group/profile schema (no egress groups yet — confirms D-12 needs extension).
- `crates/nono-cli/build.rs` — embedding mechanism (policy.json → `include_str!`).
- `dist/windows/nono.admx` — shipped AllowedSuffixes/AllowedHosts list policies + Intune OMA-URI notes.
- `scripts/verify-dark.ps1` + `scripts/gates/wfp-egress-isolation.ps1` — gate contract + the structural analogue.
- `.claude/skills/spike-findings-nono/references/engine-agnostic-confinement.md` — WFP SID-scoping + force-through-proxy constraints.
- `crates/nono/src/sandbox/mod.rs` — `WindowsNetworkPolicy` / `WindowsNetworkPolicyMode::ProxyOnly`.

### Secondary (MEDIUM confidence)
- `cargo search winreg` → `winreg = "0.56.0"` (crates.io, 2026-06-18).
- Project MEMORY: WFP daemon-path facts, ADMX-is-generated, verify-dark invocation rule.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — only one new crate (winreg, locked by D-10); everything else already in workspace.
- Architecture: HIGH — every enforcement substrate verified present in the codebase and live-proven (WFP-01 gate).
- Pitfalls: HIGH — Pitfall 1 (ADMX list shape) surfaced directly from the shipped template vs requirement text divergence.
- ADMX list-vs-REG_MULTI_SZ resolution: MEDIUM — needs a discuss-phase lock (Open Q1).

**Research date:** 2026-06-18
**Valid until:** 2026-07-18 (stable; the codebase substrate is unlikely to move before this phase plans)
