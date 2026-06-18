---
phase: 83
plan: 02
subsystem: egress-policy
tags: [egress, wfp, proxy, daemon, machine-policy, POLICY-03, EGRESS-01, EGRESS-02]
dependency_graph:
  requires: [83-01, 83-03]
  provides: [resolve_machine_egress_policy, wfp_filter_add_proxy_only, daemon_proxy_startup]
  affects: [83-04]
tech_stack:
  added: []
  patterns:
    - "D-04 SOLE read: one read_machine_egress_policy() at daemon startup feeds both ProxyFilter (L7 FQDN) and WFP request (L3/4 per-SID kernel filter); no drift possible"
    - "D-08 wholesale override: machine policy completely replaces per-user allow_domain; per-user cannot widen fleet allowlist"
    - "D-01/D-02 force-through-proxy: WFP proxy-only + localhost_ports=[proxy_port] yields PERMIT-loopback(weight 100) + BLOCK-all(weight 0) per SID"
    - "EGRESS-01 deny-by-default: ProxyFilter::new_strict with machine allowlist; empty allowlist denies every host structurally"
    - "D-07 fail-secure taxonomy: absent key -> Ok(None) fall-through; present-but-broken key -> Err(PolicyLoadFailed) abort startup via ?"
    - "D-06 restart-to-apply: startup snapshot held for daemon process lifetime; GPO changes apply on next restart"
    - "Pitfall 3 avoided: no .ok()/.unwrap_or on policy read; ? propagation aborts daemon on corrupt key"
    - "Inline expand_preset_tokens_from_embedded in mod.rs: mirrors policy.rs logic via include_str! of network-policy.json (crate:: resolution precludes calling policy::expand_egress_preset_tokens from nono-agentd binary context)"
    - "std::mem::forget(handle): proxy handle leaked for daemon lifetime (proxy and daemon exit together; no separate shutdown ordering needed)"
key_files:
  created: []
  modified:
    - crates/nono-cli/src/agent_daemon/mod.rs
    - crates/nono-cli/src/agent_daemon/launch.rs
    - crates/nono-cli/src/bin/nono-agentd.rs
decisions:
  - "D-04 SOLE read implemented in resolve_machine_egress_policy(); single call propagated with ? in nono-agentd.rs startup path"
  - "expand_preset_tokens_from_embedded added inline in mod.rs (not policy.rs) because nono-agentd crate:: resolves to binary root, not nono-cli module tree"
  - "Proxy handle leaked via std::mem::forget (not stored in DaemonState): daemon and proxy share the same process lifetime; no shutdown ordering benefit"
  - "build_daemon_state async fn added to nono-agentd.rs windows_impl: separates startup wiring from loop invocation; allows rt.block_on to start proxy before accept/control loops"
  - "nono-agentd.rs uses ..ProxyConfig::default() for unset fields (external_proxy, direct_connect_ports, max_connections) to survive future ProxyConfig field additions without breaking the struct literal"
metrics:
  duration: 45m
  completed: "2026-06-18"
  tasks: 2
  files: 3
---

# Phase 83 Plan 02: Single-Source Egress Wiring (POLICY-03 / EGRESS-01 / EGRESS-02) Summary

**One-liner:** Single `read_machine_egress_policy()` call at daemon startup feeds both the in-process ProxyFilter (L7 deny-by-default) and the per-agent WFP `proxy-only` request (L3/4 kernel block) from the same deserialized struct, closing the allowlist-drift false-security state (D-04).

## Tasks Completed

| Task | Name | Commit | Key Files |
|------|------|--------|-----------|
| Task 1 | Single startup read + wholesale override + ProxyFilter wiring | `f67c57a3` | crates/nono-cli/src/agent_daemon/mod.rs |
| Task 2 | Flip wfp_filter_add to force-through-proxy + nono-agentd startup wiring | `71324d62` | crates/nono-cli/src/agent_daemon/launch.rs, crates/nono-cli/src/bin/nono-agentd.rs |

## What Was Built

### Task 1: Single Startup Read + Wholesale Override + ProxyFilter Wiring

`crates/nono-cli/src/agent_daemon/mod.rs`:

1. **`EMBEDDED_NETWORK_POLICY_JSON` constant**: `include_str!(concat!(env!("OUT_DIR"), "/network-policy.json"))` — mirrors the pattern already used for EMBEDDED_POLICY_JSON, making the preset group map accessible from the nono-agentd binary context where `crate::config::embedded` is unreachable.

2. **`expand_preset_tokens_from_embedded(tokens: &[String]) -> Result<Vec<String>, String>`**: Parses the embedded JSON via `serde_json::Value`, looks up each token in the `groups` object, returns the `hosts` array. Unknown tokens expand to empty (T-83-token-widen fail-secure). Deduplicates results. Returns `Err(String)` on JSON parse failure.

3. **`resolve_machine_egress_policy(per_user_domains: &[String]) -> nono::Result<(Vec<String>, bool)>`**:
   - Calls `nono::read_machine_egress_policy()?` — exactly ONE call, propagated with `?` (D-04/D-07 Pitfall 3: never `.ok()`/`.unwrap_or`)
   - `Some(policy)`: builds effective allowlist = `policy.raw_allowlist()` + `expand_preset_tokens_from_embedded(&policy.preset_tokens)` → wholesale override (D-08), per-user `per_user_domains` ignored entirely
   - `None`: returns `(per_user_domains.to_vec(), false)` (D-07 fall-through; no enforcement)
   - Documents D-06 restart-to-apply in code comment

4. **`machine_egress_proxy_port: Option<u16>` field** added to `DaemonState`

5. **`DaemonState::new_with_proxy(proxy_port: u16) -> Self`** constructor: sets `machine_egress_proxy_port: Some(proxy_port)`

6. **3 unit tests** added: `machine_policy_handoff_absent_falls_through_to_per_user`, `machine_policy_handoff_wholesale_override_excludes_per_user`, `machine_policy_handoff_daemon_state_proxy_port_field`

### Task 2: Flip wfp_filter_add + nono-agentd Startup Wiring

`crates/nono-cli/src/agent_daemon/launch.rs`:

- **`wfp_filter_add` signature**: Added `proxy_port: u16` parameter
- **Request shape flipped** (D-01/D-02 force-through-proxy):
  - `request_kind`: `"activate_blocked_mode"` → `"activate_proxy_mode"`
  - `network_mode`: `"blocked"` → `"proxy-only"`
  - `localhost_ports`: `vec![]` → `vec![proxy_port]`
  - `session_sid`, rule names, fail-secure status handling: VERBATIM (unchanged)
- **Step 6.5 in `launch_agent`**: Resolves `proxy_port` from `daemon_state.machine_egress_proxy_port`; `inspect_err` terminates suspended process before returning `Err` if `None` when WFP scoping required (D-05 fail-secure); passes `proxy_port` to `wfp_filter_add`
- **3 tests updated/added**: `wfp_proxy_only_constructs_proxy_mode_request` (SC per plan spec), `wfp_proxy_only_port_is_parameterised` (no hardcoded port), `wfp_filter_add_constructs_request` (updated to proxy-mode shape)
- `nono-wfp-service.rs` weights UNTOUCHED (Pitfall 4: PERMIT weight 100 / BLOCK weight 0 already correct)

`crates/nono-cli/src/bin/nono-agentd.rs`:

- **`build_daemon_state` async fn** added to `windows_impl`:
  - `!machine_policy_active` → `DaemonState::new()` (D-07 fall-through)
  - `machine_policy_active` → `nono_proxy::server::start(ProxyConfig { strict_filter: true, allowed_hosts: egress_domains, bind_port: 0, ..default() })` → `DaemonState::new_with_proxy(port)` (EGRESS-01: deny-by-default structural; ephemeral OS-assigned port)
  - Proxy handle held for daemon lifetime via `std::mem::forget(handle)`
  - Any proxy startup error returns `Err(String)` → caller aborts daemon (fail-secure)
- **Both `run_service` and `run_foreground_mode`** wired to:
  1. Call `resolve_machine_egress_policy(&[])` — D-04 SOLE read at startup; abort on `Err`
  2. Call `rt.block_on(async { build_daemon_state(...).await })` — start proxy if active
  3. Use the returned `DaemonState` (with or without proxy port) for accept/control loops

## Verification Results

```
cargo test -p nono-cli machine_policy_handoff
running 3 tests
test agent_daemon::tests::machine_policy_handoff_absent_falls_through_to_per_user ... ok
test agent_daemon::tests::machine_policy_handoff_wholesale_override_excludes_per_user ... ok
test agent_daemon::tests::machine_policy_handoff_daemon_state_proxy_port_field ... ok
test result: ok. 3 passed; 0 failed
```

```
cargo test -p nono-cli wfp_proxy_only
running 2 tests
test agent_daemon::launch::tests::wfp_proxy_only_constructs_proxy_mode_request ... ok
test agent_daemon::launch::tests::wfp_proxy_only_port_is_parameterised ... ok
test result: ok. 2 passed; 0 failed
```

```
grep -n read_machine_egress_policy crates/nono-cli/src/agent_daemon/mod.rs
=> line 258 (comment), 330 (doc), 358 (SOLE CALL: nono::read_machine_egress_policy()?)
=> Exactly one live call, propagated with `?` — no .ok()/.unwrap_or
```

```
cargo clippy --bin nono-agentd -- -D warnings -D clippy::unwrap_used
=> Finished (no errors)

cargo clippy --bin nono -- -D warnings -D clippy::unwrap_used
=> Finished (no errors)
```

```
cargo build --bin nono-agentd
=> Finished (clean build)
```

Full nono-cli test suite: 1292 passed, 4 pre-existing baseline failures (profile_cmd init + 3 protected_paths — documented in `nono_cli_windows_baseline_test_failures` memory; not regressions from this plan), 2 ignored.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] nono-agentd.rs startup not wiring proxy or resolve_machine_egress_policy**
- **Found during:** Task 1 — `mod.rs` adds `resolve_machine_egress_policy` and `DaemonState::new_with_proxy`, but both `run_service` and `run_foreground_mode` still called `DaemonState::new()` (no policy read, no proxy start). Without this wiring, `machine_egress_proxy_port` is always `None`, causing every WFP-scoped agent launch to fail-secure with an error.
- **Fix:** Added `build_daemon_state` async fn in `windows_impl`; wired both startup paths to call `resolve_machine_egress_policy` then `build_daemon_state`; start nono-proxy with `strict_filter=true` when machine policy active; use `DaemonState::new_with_proxy(port)` to thread the port through.
- **Files modified:** `crates/nono-cli/src/bin/nono-agentd.rs`
- **Commit:** `71324d62`

**2. [Rule 1 - Bug] `crate::policy::expand_egress_preset_tokens` inaccessible from nono-agentd binary context**
- **Found during:** Task 1 — `nono-agentd.rs` loads `agent_daemon/mod.rs` via `#[path]`, so `crate::` resolves to the binary root (not nono-cli's module tree). Existing comments in `mod.rs` document this constraint explicitly ("cannot reach `crate::windows_wfp_contract`").
- **Fix:** Added `EMBEDDED_NETWORK_POLICY_JSON` constant via `include_str!` and implemented `expand_preset_tokens_from_embedded` inline in `mod.rs`, mirroring the embedding pattern already in use.
- **Files modified:** `crates/nono-cli/src/agent_daemon/mod.rs`
- **Commit:** `f67c57a3`

**3. [Rule 1 - Bug] `map_err` should be `inspect_err` (clippy::manual_inspect)**
- **Found during:** Task 2 — clippy -D warnings caught a `map_err(|e| { ... e })` pattern in step 6.5 of `launch_agent` that passes the error through unchanged.
- **Fix:** Changed to `inspect_err(|_|  { ... })` (run side-effects without transforming the error).
- **Files modified:** `crates/nono-cli/src/agent_daemon/launch.rs`
- **Commit:** `71324d62`

## Known Stubs

None. All code paths are wired:
- `resolve_machine_egress_policy` is called at daemon startup in both service mode and foreground mode
- `build_daemon_state` starts the proxy and returns a wired `DaemonState` when policy is active
- `wfp_filter_add` threads the proxy port through to the WFP request

The only runtime gap is the WFP service call itself (requires `nono-wfp-service` to be installed and running) — this is the existing pre-Plan-83 gate, not a stub introduced here.

## Threat Flags

No new threat surface introduced. All changes are internal to the daemon startup path:
- No new named pipes or sockets exposed externally
- The in-process proxy binds on `127.0.0.1:0` (OS-assigned loopback-only port)
- The WFP control pipe is unchanged (`\\.\pipe\nono-wfp-control`, pre-existing)
- `resolve_machine_egress_policy` is read-only (HKLM read, no write)

All five STRIDE threats in the plan's threat register are mitigated:
- T-83-layer-drift: single read at startup feeds both layers (f67c57a3)
- T-83-proxy-bypass: proxy-only + localhost_ports=[proxy_port] (71324d62)
- T-83-peruser-widen: wholesale override, per_user_domains ignored when policy present (f67c57a3)
- T-83-malformed-failopen: `?` propagation; no .ok()/.unwrap_or (f67c57a3)
- T-83-lowil-ipc: existing control pipe Medium-IL SACL unchanged (not touched)

## Self-Check: PASSED

- `crates/nono-cli/src/agent_daemon/mod.rs` — FOUND (contains resolve_machine_egress_policy, expand_preset_tokens_from_embedded, DaemonState::new_with_proxy)
- `crates/nono-cli/src/agent_daemon/launch.rs` — FOUND (contains proxy-only, wfp_proxy_only tests)
- `crates/nono-cli/src/bin/nono-agentd.rs` — FOUND (contains build_daemon_state, resolve_machine_egress_policy call)
- Commit `f67c57a3` — FOUND in git log (feat(83-02): wire machine egress policy into DaemonState)
- Commit `71324d62` — FOUND in git log (feat(83-02): flip wfp_filter_add to proxy-only + wire proxy startup)
