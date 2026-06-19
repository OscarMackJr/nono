---
phase: 83-machine-policy-spine-egress-control
verified: 2026-06-18T21:00:00Z
status: human_needed
score: 7/7 must-haves verified
overrides_applied: 0
human_verification:
  - test: "Live fleet GPO push end-to-end: push ADMX to a domain-joined VM via Group Policy Management Console, set AllowedSuffixes to .anthropic.com, verify nono daemon restart reads the policy and confined agent traffic to api.anthropic.com is allowed while evil.example.com is denied at both the proxy and WFP layers"
    expected: "Daemon startup logs 'machine egress policy present — wholesale override active'; proxy allows api.anthropic.com and denies evil.example.com; netsh wfp show filters shows per-SID block filter for the agent AppContainer SID"
    why_human: "Requires a provisioned domain-joined Windows fleet VM, admin credentials, and a running nono-wfp-service + nono-agentd stack with a fresh nono.exe binary on PATH"
  - test: "Intune OMA-URI preset toggle: configure the Allow Anthropic toggle via Intune OMA-URI ADMXInstall path, verify the anthropic preset token is written to HKLM\\SOFTWARE\\Policies\\nono\\PresetTokens and that nono expands it to *.anthropic.com in the effective allowlist"
    expected: "HKLM\\SOFTWARE\\Policies\\nono\\PresetTokens contains an 'anthropic' REG_SZ value; daemon startup expands the token to *.anthropic.com and logs the allowlist; confined agent can reach api.anthropic.com"
    why_human: "Requires an Intune-enrolled test device and an Intune administrator account to push the ADMXInstall policy; cannot be automated without the cloud MDM stack"
  - test: "egress-policy-deny Dark Factory gate SC-3 on a provisioned host: run pwsh -File scripts/verify-dark.ps1 --gate egress-policy-deny on a host with the current nono.exe (with agent launch subcommand) on PATH, nono-wfp-service running, and nono-agentd running in non-elevated user context"
    expected: "Gate emits verdict PASS with sc2Pass=true, sc3WfpBlockPresent=true, sc3ProxyLayerActive=true; exit 0"
    why_human: "Dev host has stale C:\\Program Files\\nono\\nono.exe without 'agent launch' (STATE.md deferred item). SC-2 already passes (exit code 2 on malformed key verified in SUMMARY). SC-3 requires fresh binary on PATH plus running daemon and WFP service"
---

# Phase 83: Machine Policy Spine & Egress Control Verification Report

**Phase Goal:** An admin can push a deny-by-default outbound egress allowlist to a fleet via GPO ADMX or Intune OMA-URI; every confined agent's traffic is filtered at both the proxy (L7) and kernel WFP (L3/4) layers from the same deserialized source, with no allowlist drift possible between layers.
**Verified:** 2026-06-18
**Status:** human_needed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | A present-but-absent HKLM policy key returns Ok(None) (fall-through to per-user) | VERIFIED | `machine_policy.rs:257-259`: `raw_os_error()==Some(2)` maps to `Ok(None)`; 15/15 machine_policy tests pass including `non_windows_stub_returns_ok_none` |
| 2 | A present-but-unconfigured key (MSI sentinel only) returns Ok(None) (CR-02) | VERIFIED | `machine_policy.rs:277-279`: `is_unconfigured()` gate after clean parse → `Ok(None)`; tests `windows_sentinel_only_key_is_unconfigured` and `is_unconfigured_true_for_empty_policy` pass |
| 3 | A present-but-unreadable or malformed key returns Err(PolicyLoadFailed) — never falls through | VERIFIED | `machine_policy.rs:261-265` (unreadable path); `read_list_subkey` returns `Err(reason)` on wrong REG type; mapped via `.map_err` to `PolicyLoadFailed`; test `windows_wrong_reg_type_returns_policy_load_failed` passes; no `.ok()`/`.unwrap_or` on read path |
| 4 | The reader opens the 64-bit registry view (KEY_WOW64_64KEY) regardless of bitness | VERIFIED | `machine_policy.rs:255`: `open_subkey_with_flags(POLICY_PATH, KEY_READ \| KEY_WOW64_64KEY)`; same flag on all `open_subkey_with_flags` calls in `read_list_subkey` (`machine_policy.rs:188`) |
| 5 | api.anthropic.com matches *.anthropic.com; anthropic.com / evilanthropic.com / anthropic.com.evil.com are all rejected | VERIFIED | `sc4_dns_component_matrix` test in `net_filter.rs:492-519` passes: all 4 cases asserted and confirmed by `cargo test -p nono net_filter::tests::sc4_dns_component_matrix` |
| 6 | A leading-dot `.anthropic.com` suffix (ADMX-documented format) normalizes to `*.anthropic.com` and correctly allows subdomains while rejecting bare/evil domains (CR-01 fix) | VERIFIED | `machine_policy.rs:141-149`: `normalize_suffix()` converts `.x.com` → `*.x.com`; test `cr01_leading_dot_suffix_matches_via_hostfilter` passes and exercises all 4 EGRESS-03 cases through HostFilter; `raw_allowlist_normalizes_suffix_shapes` also passes |
| 7 | The daemon startup path performs exactly ONE read_machine_egress_policy() call, propagates ? on PolicyLoadFailed, and the single deserialized policy feeds both ProxyFilter (L7) and the WFP request (L3/4) with no drift | VERIFIED | `agent_daemon/mod.rs:358`: single call `nono::read_machine_egress_policy()?`; `resolve_machine_egress_policy` function at line 352; `launch.rs:444-494`: `wfp_filter_add(package_sid, tenant_id, proxy_port)` uses `request_kind: "activate_proxy_mode"`, `network_mode: "proxy-only"`, `localhost_ports: vec![proxy_port]`; nono-wfp-service.rs has zero `read_machine_egress_policy` calls (grep returned nothing); `machine_policy_handoff` tests (3/3 pass): `machine_policy_handoff_absent_falls_through_to_per_user`, `machine_policy_handoff_wholesale_override_excludes_per_user`, `machine_policy_handoff_daemon_state_proxy_port_field` |

**Score:** 7/7 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/nono/src/machine_policy.rs` | MachineEgressPolicy type + fail-secure reader | VERIFIED | 685 lines; `pub struct MachineEgressPolicy`, `read_machine_egress_policy()`, `is_unconfigured()`, `raw_allowlist()` with CR-01 normalization, Windows reader + non-Windows stub |
| `crates/nono/src/error.rs` | NonoError::PolicyLoadFailed variant | VERIFIED | Lines 252-255: `#[error("Machine policy load failed: {reason}")] PolicyLoadFailed { reason: String }` |
| `crates/nono/src/lib.rs` | pub use machine_policy re-exports | VERIFIED | Line 82: `pub use machine_policy::{read_machine_egress_policy, MachineEgressPolicy};` |
| `crates/nono/src/net_filter.rs` | sc4_dns_component_matrix test | VERIFIED | Lines 492-519; all 4 EGRESS-03 cases; test passes |
| `crates/nono-cli/src/agent_daemon/mod.rs` | Single startup read + wholesale override + ProxyFilter wiring | VERIFIED | `resolve_machine_egress_policy()` at line 352; single `read_machine_egress_policy()?` call at line 358; `DaemonState::machine_egress_proxy_port` field; `expand_preset_tokens_from_embedded()` helper; 3 machine_policy_handoff tests pass |
| `crates/nono-cli/src/agent_daemon/launch.rs` | wfp_filter_add flipped to proxy-only + proxy_port parameter | VERIFIED | Lines 444-494: `fn wfp_filter_add(package_sid, tenant_id, proxy_port: u16)`; `request_kind: "activate_proxy_mode"`, `network_mode: "proxy-only"`, `localhost_ports: vec![proxy_port]`; `wfp_filter_add_constructs_request` and `wfp_proxy_only_*` tests pass (2/2) |
| `crates/nono-cli/data/network-policy.json` | anthropic / openai / github-api preset groups | VERIFIED | Lines 111-128: `"anthropic": {"hosts": ["*.anthropic.com"]}`, `"openai": {"hosts": ["*.openai.com"]}`, `"github-api": {"hosts": ["api.github.com"]}` |
| `crates/nono-cli/src/policy.rs` | expand_egress_preset_tokens() from embedded JSON | VERIFIED | Lines 1457-1487; reads `embedded_network_policy_json()`; 6 tests pass: `policy_egress_groups_present_in_network_policy`, `expand_anthropic_token`, `expand_openai_token`, `expand_github_api_token`, `unknown_token_expands_to_empty`, `union_hosts` |
| `scripts/build-windows-msi.ps1` | ADMX named-toggle preset policies with valueName (WR-01 fixed) | VERIFIED | Lines 698-755: `AllowAnthropicPreset`, `AllowOpenAIPreset`, `AllowGitHubAPIPreset` — all have `valueName="anthropic"/"openai"/"github-api"` respectively; `enabledValue<string>` writes the token |
| `scripts/gates/egress-policy-deny.ps1` | Dark Factory gate with Test-Precondition + Invoke-Gate | VERIFIED | 441 lines; `Test-Precondition` (line 177) checks admin + WFP pipe + daemon pipe; `Invoke-Gate` (line 218) implements SC-2 (malformed key → non-zero exit) and SC-3 (dual-layer deny); never calls `exit` or `Persist-Verdict` directly; harness-internal throws use `throw` |
| `scripts/verify-dark.ps1` | egress-policy-deny gate registered | VERIFIED | Auto-discovery: line 140-145 scans `scripts/gates/*.ps1` by filename — `egress-policy-deny.ps1` is auto-included; no hardcoded ValidateSet needed; `--gate egress-policy-deny` dispatch confirmed in SUMMARY (ran and returned a verdict) |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `crates/nono/src/lib.rs` | `machine_policy.rs` | `pub mod machine_policy` + `pub use machine_policy::` | WIRED | Line 53 (`pub mod`) + line 82 (`pub use`) |
| `machine_policy.rs` Windows reader | `winreg RegKey` | `KEY_READ \| KEY_WOW64_64KEY` | WIRED | Cargo.toml line 67: `winreg = "0.56"` under `[target.'cfg(target_os = "windows")'.dependencies]` only; used at machine_policy.rs:158, 188, 255 |
| `agent_daemon/mod.rs` startup | `nono::read_machine_egress_policy()` | single `?`-propagated call | WIRED | mod.rs:358: `let machine_policy = nono::read_machine_egress_policy()?;` — exactly one occurrence confirmed by grep |
| `agent_daemon startup` | `ProxyFilter::new_strict` | effective allowlist from machine policy | WIRED | mod.rs documents and tests confirm: when `Some(policy)` present, raw_allowlist + expanded tokens build the effective allowlist, wholesale override excludes per_user_domains (test `machine_policy_handoff_wholesale_override_excludes_per_user` passes) |
| `launch.rs wfp_filter_add` | `nono-wfp-service` via IPC | `network_mode: "proxy-only"`, `localhost_ports: [proxy_port]` | WIRED | launch.rs:449-464; proxy_port is a function parameter from `daemon_state.machine_egress_proxy_port`; wfp_proxy_only tests (2/2) pass |
| WFP service | HKLM registry for egress policy | (absent — D-04 mandates WFP never reads HKLM) | VERIFIED ABSENT | grep for `read_machine_egress_policy` in `nono-wfp-service.rs` returns nothing; WFP receives derived permit instructions only via control IPC |
| `verify-dark.ps1` | `egress-policy-deny.ps1` | auto-discovery `Get-ChildItem scripts/gates/*.ps1` | WIRED | Line 140-145 of verify-dark.ps1; gate file exists at `scripts/gates/egress-policy-deny.ps1` |
| `ADMX named-toggle` | `HKLM\SOFTWARE\Policies\nono\PresetTokens` | `valueName="anthropic"/"openai"/"github-api"` | WIRED | build-windows-msi.ps1 lines 704/725/746; toggle writes token value into the PresetTokens subkey that `read_preset_subkey` enumerates |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|--------------------|--------|
| `agent_daemon/mod.rs: resolve_machine_egress_policy` | `machine_policy` (Option<MachineEgressPolicy>) | `nono::read_machine_egress_policy()` → winreg HKLM read | Yes — reads real HKLM subkeys or returns Ok(None) on absence | FLOWING |
| `agent_daemon/mod.rs: DaemonState::machine_egress_proxy_port` | `proxy_port` (Option<u16>) | Set at daemon startup from `new_with_proxy(proxy_port)` when machine policy is Some | Yes — real port number from the in-process proxy listener | FLOWING |
| `launch.rs: wfp_filter_add` | `localhost_ports: vec![proxy_port]` | `daemon_state.machine_egress_proxy_port` (threaded from startup) | Yes — real proxy port, not hardcoded | FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| 15 machine_policy tests (Plan 01) | `cargo test -p nono machine_policy` | 15 passed; 0 failed | PASS |
| sc4_dns_component_matrix (EGRESS-03) | `cargo test -p nono net_filter::tests::sc4_dns_component_matrix` | 1 passed; 0 failed | PASS |
| cr01_leading_dot_suffix_matches_via_hostfilter (CR-01) | `cargo test -p nono cr01_leading_dot` | 1 passed; 0 failed | PASS |
| machine_policy_handoff tests (Plan 02, 3 tests) | `cargo test -p nono-cli machine_policy_handoff` | 3 passed; 0 failed | PASS |
| wfp_proxy_only tests (Plan 02, 2 tests) | `cargo test -p nono-cli wfp_proxy_only` | 2 passed; 0 failed | PASS |
| policy_egress_groups tests (Plan 03, 6 tests) | `cargo test -p nono-cli policy_egress_groups` | 6 passed; 0 failed | PASS |
| egress-policy-deny gate SKIP on non-elevated host | `pwsh -File scripts/verify-dark.ps1 --gate egress-policy-deny` | Would emit SKIP_HOST_UNAVAILABLE (exit 3) when not elevated per Test-Precondition check | SKIP (host not elevated in this session) |
| SC-2: daemon exits non-zero on malformed key | Recorded in 83-04-SUMMARY.md | Gate ran on dev host (admin+pipes present): SC-2 PASS — `nono daemon start --foreground` exited code 2 on malformed REG_DWORD key | PASS (recorded) |

### Probe Execution

| Probe | Command | Result | Status |
|-------|---------|--------|--------|
| egress-policy-deny (SC-2) | `pwsh -File scripts/verify-dark.ps1 --gate egress-policy-deny` | SC-2 passed (exit code 2 on malformed key); SC-3 FAIL due to stale nono.exe on PATH (pre-existing STATE.md deferred item — not a Phase 83 defect) | PARTIAL (SC-2 PASS; SC-3 host-gated) |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| POLICY-01 | 83-01 | Reader opens HKLM with KEY_WOW64_64KEY; absent→Ok(None), present→Some | SATISFIED | machine_policy.rs:255 KEY_WOW64_64KEY; absent branch line 257-259; present branch line 270-281 |
| POLICY-02 | 83-01, 83-04 | Present-but-unreadable/malformed → Err(PolicyLoadFailed), never fall-through | SATISFIED | machine_policy.rs:261-265 (unreadable); read_list_subkey type-check; SC-2 gate assertion in egress-policy-deny.ps1 lines 225-299 |
| POLICY-03 | 83-02 | Single policy source: no allowlist drift possible between layers | SATISFIED | resolve_machine_egress_policy() is the SOLE reader (one call in nono-agentd.rs:358 via mod.rs); same allowlist feeds ProxyFilter AND wfp_filter_add; WFP service never reads HKLM |
| EGRESS-01 | 83-02 | Admin-defined deny-by-default allowlist via REG_MULTI_SZ or N×REG_SZ; policy presence switches enforcement on | SATISFIED | AllowedSuffixes/AllowedHosts enumerated as N×REG_SZ (D-13 Option A); is_unconfigured() gate ensures sentinel-only key doesn't activate enforcement (CR-02) |
| EGRESS-02 | 83-02, 83-04 | Machine-policy allowlist enforced by BOTH proxy and WFP from same deserialized source | SATISFIED (structurally wired; dual-layer live proof is host-gated) | launch.rs:729 `wfp_filter_add(..., proxy_port)` with proxy-only + localhost_ports; ProxyFilter::new_strict fed from same allowlist; WFP service receives derived instructions over IPC only |
| EGRESS-03 | 83-01 | Wildcard FQDN matching DNS-component-safe: *.x.com matches api.x.com, not x.com or evilx.com | SATISFIED | sc4_dns_component_matrix test (4 cases), cr01_leading_dot_suffix_matches_via_hostfilter test; both pass |
| EGRESS-04 | 83-03 | AI-provider presets (*.anthropic.com, *.openai.com, api.github.com) shipped | SATISFIED | network-policy.json lines 111-128; policy_egress_groups 6 tests pass; ADMX named toggles with valueName in build-windows-msi.ps1 |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `crates/nono-cli/src/policy.rs` | 1456 | `#[allow(dead_code)]` on `expand_egress_preset_tokens` | Warning | Function is uncalled (daemon uses `expand_preset_tokens_from_embedded` instead); doc-comment was stale (IN-02, fixed in commit e8748421); deferred as WR-02 (tracked in `.planning/todos/pending/20260618-phase83-codereview-deferred.md`) |
| `scripts/gates/egress-policy-deny.ps1` | 399 | SC-3 proxy-layer proof is structural (WFP block presence implies proxy-only activation) rather than a live HTTP probe | Warning | Weaker than the plan's stated "proxy denies an out-of-list host" — deferred as WR-03 (tracked) |
| `crates/nono-cli/src/agent_daemon/mod.rs` | 179-231 | `build_daemon_capability_set` shells out to `where` (WR-04) and has a silent SystemRoot fallback (WR-05) | Warning | Not introduced by Phase 83; deferred as WR-04/WR-05 (tracked) |

No TBD/FIXME/XXX debt markers found in Phase-83-modified files.

### Cross-Target Clippy Status

PARTIAL-deferred to live CI per `.planning/templates/cross-target-verify-checklist.md`. The Windows dev host cannot cross-compile to Linux/macOS because `aws-lc-sys` and `ring` require a C linker (`x86_64-linux-gnu-gcc` / `cc`) that is absent. The non-Windows stub (`#[cfg(not(target_os = "windows"))]` returning `Ok(None)`) and the platform-neutral `MachineEgressPolicy` struct (all `Vec<String>` fields, no Windows types) provide structural assurance that the workspace compiles on Linux/macOS. This is the documented acceptable outcome on a Windows-only dev host.

### Human Verification Required

#### 1. Live Fleet GPO Push — End-to-End

**Test:** On a domain-joined Windows VM with Group Policy Management Console, install the generated ADMX (from `scripts/build-windows-msi.ps1`), create a GPO that sets `AllowedSuffixes` to `.anthropic.com`, link it to a test OU, run `gpupdate /force`, restart `nono-agentd`, then launch a confined agent and observe:
  - `nono daemon status` shows `machine_egress_enforcement: active`
  - The proxy allows `api.anthropic.com:443` CONNECT (check proxy logs)
  - The proxy denies `evil.example.com:443` CONNECT with a policy-deny message
  - `netsh wfp show filters` shows the per-agent AppContainer SID in the block filter set

**Expected:** Both layers (L7 proxy deny and L3/4 WFP block) activate from the single HKLM read; no drift; per-user domains are ignored.
**Why human:** Requires provisioned domain-joined Windows VM, GPMC, running nono-wfp-service, fresh nono.exe with `agent` subcommand on PATH.

#### 2. Intune OMA-URI Preset Toggle

**Test:** In Intune, create a custom OMA-URI profile using `ADMXInstall` for the nono ADMX, then add an OMA-URI that enables the "Allow Anthropic" preset toggle. Apply to a test device, verify `HKLM\SOFTWARE\Policies\nono\PresetTokens` contains `anthropic` (REG_SZ), restart daemon, and confirm `api.anthropic.com` is reachable from a confined agent.

**Expected:** Preset token written by MDM; daemon expands token to `*.anthropic.com`; egress allowed to Anthropic endpoints.
**Why human:** Requires an Intune-enrolled test device and cloud MDM administrator access.

#### 3. egress-policy-deny Gate Full SC-3 on Provisioned Host

**Test:** On a host where a fresh `target\release\nono.exe` (with `agent launch`) is on PATH, `nono-wfp-service` is running, and `nono-agentd` is running in non-elevated user context with a machine policy of `*.anthropic.com` active in HKLM, run:
```
pwsh -File scripts/verify-dark.ps1 --gate egress-policy-deny
```

**Expected:** JSON verdict with `"verdict": "PASS"`, `"sc2Pass": true`, `"sc3WfpBlockPresent": true`, `"sc3ProxyLayerActive": true`; exit code 0.
**Why human:** Dev host has stale `C:\Program Files\nono\nono.exe` without `agent` subcommand (STATE.md deferred item). SC-2 already verified (exit code 2 on malformed key — confirmed in SUMMARY). SC-3 requires the full daemon+WFP+proxy stack with a current binary.

### Gaps Summary

No structural gaps. All 7 must-have truths are VERIFIED in the codebase by reading the source and running the named tests. The two code-review criticals (CR-01 and CR-02) were fixed before this verification and their fixes are confirmed:

- **CR-01** (leading-dot suffix normalization): `normalize_suffix()` in `machine_policy.rs:141-149` + `cr01_leading_dot_suffix_matches_via_hostfilter` test PASS.
- **CR-02** (MSI sentinel-only key falls through): `is_unconfigured()` gate in `machine_policy.rs:129-133` + `windows_sentinel_only_key_is_unconfigured` test PASS.

The human verification items are required because the phase goal explicitly calls out "an admin can push... via GPO ADMX or Intune OMA-URI" and "fleet" enforcement — these require a provisioned fleet environment that is documented as host-gated tech-debt per REQUIREMENTS.md and CONTEXT.md. The structural wiring (single-source read, no-drift proof, dual-layer request shape) is fully verified in code and tests.

---

_Verified: 2026-06-18T21:00:00Z_
_Verifier: Claude (gsd-verifier)_
