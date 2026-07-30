# Phase 110: Profile/Policy Absorb + platform_overrides - Pattern Map

**Mapped:** 2026-07-30
**Files analyzed:** ~17 (from RESEARCH.md §"Recommended Wave Structure")
**Analogs found:** 17 / 17 (this phase is overwhelmingly a same-file *extend* pattern — the analog for
almost every touched file is the file itself, immediately adjacent to the insertion point)

**Verification note (D-14 compliance):** every excerpt below was independently re-opened by this
agent against live fork HEAD (`milestone/v2.13-carryforward-closeout`) — not copied from RESEARCH.md's
citations unread. Two of RESEARCH.md's claims were spot-checked and found accurate on re-read:
`expand_vars` (not `expand_path`) is the actual function name threading through `capability_ext.rs`'s
fs-list loops (confirmed at `capability_ext.rs:8` import and `:463` call site); `finalize_profile`'s
current two-step body (`merge_implicit_default_groups` → `validate_profile_no_proxy`) is exactly as
cited, confirming `apply_platform_overrides` inserts as a new first step.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `crates/nono-cli/src/profile/mod.rs` (`PlatformOverrides`/`PlatformOverride` types + `apply_platform_overrides`) | model + transform | CRUD (profile-merge) | same file: `NetworkConfig` struct (1706) + `merge_profiles`'s `network:` arm (3353) | exact (same file, same struct-then-merge idiom) |
| `crates/nono-cli/src/profile/mod.rs` (3-site `windows_low_il_broker`/`windows_interpreters` → `platform_overrides.windows` back-compat) | model | CRUD | same file: existing 3 declared sites (2391/2398, 2468/2475, 2518/2523) | exact |
| `crates/nono-cli/data/nono-profile.schema.json` (`platform_overrides` property) | config/schema | transform | same file: `network` object's property block (444-575) | exact |
| `crates/nono-cli/src/policy.rs` (`substitute_vars`/generic `$VAR` helper) | utility | transform | same file: `expand_path` (268-289) | role-match (older, narrower expansion helper in the same file) |
| `crates/nono-cli/src/capability_ext.rs` (`expand_dynamic_tokens` call sites, `expand_profile_path` rename) | utility/service | transform | same file: `apply_profile_dir_allows`'s `expand_vars` call (454-474) | exact |
| NEW fork-owned token module (e.g. `capability_ext/dynamic_tokens.rs`) | utility | transform | `crates/nono-cli/src/protected_paths.rs` (405 lines, self-contained, own `#[cfg(test)]` suite) | role-match (standalone helper-module precedent) |
| `crates/nono/src/capability.rs` (`MACOS_PORT_RANGE_LIMIT`, `merge_port_ranges`, `localhost_port_ranges` field + methods) | library/model | CRUD (builder) | same file: `localhost_ports` field (894) + `allow_localhost_port` (1100) + `localhost_ports()` (1428) | exact |
| `crates/nono/src/sandbox/macos.rs` (range-aware Seatbelt emitter) | library/service | transform | same file: `push_localhost_tcp_outbound_seatbelt_rules` (464-478) + its call site (731-765) | exact |
| `crates/nono/src/sandbox/linux.rs` (range-aware Landlock emitter) | library/service | transform | same file: localhost-port `NetPort` loop (862-886) | exact |
| `crates/nono-cli/src/exec_strategy/supervisor_linux.rs` (`proxy_bind_port_ranges`) | service | event-driven (seccomp notify) | `crates/nono-cli/src/exec_strategy.rs:413` `proxy_bind_ports: Vec<u16>` field + its Linux-only populate site in `supervised_runtime.rs:373` | exact |
| `crates/nono-cli/src/profile/mod.rs` (`NetworkConfig.open_port_range`/`listen_port_range` + 3rd `merge_profiles` site) | model | CRUD | same file: `open_port`/`listen_port` field pair (1767-1771) + their `merge_profiles` arm (3365-3366) | exact |
| `crates/nono/schema/capability-manifest.schema.json` (`PortConfig.localhost_range`) | config/schema (typify source) | transform | same file: `PortConfig.localhost` (192-196) | exact |
| `crates/nono-cli/src/bin/nono-wfp-service.rs` (`PortCondition::RemoteRange`/`LocalRange`, `build_policy_filter_specs` range loop, `add_policy_filter` `FWP_MATCH_RANGE` arm) | service (kernel-facing, fork-original) | event-driven / request-response (IPC → kernel) | same file: `PortCondition::Remote`/`Local` (1089-1092) + their `build_policy_filter_specs` loops (1202-1264) + `add_policy_filter`'s `match port` arm (1461-1474) | exact |
| `crates/nono-cli/src/windows_wfp_contract.rs` (`localhost_port_ranges: Vec<(u16,u16)>` field) | model (IPC contract) | request-response | same file: `WfpRuntimeActivationRequest.localhost_ports: Vec<u16>` (15) | exact |
| `crates/nono-cli/src/exec_strategy_windows/network.rs` (`build_wfp_runtime_activation_request` threading) | service | transform | same file: `localhost_ports` handling in `build_wfp_runtime_activation_request` (498-523) | exact |
| `crates/nono/src/sandbox/windows.rs` (`compile_network_policy` gains `localhost_port_ranges`) | library | transform | same file: `localhost_ports` sort/dedup + `WindowsNetworkPolicy` literal (346-368) | exact |
| `crates/nono/src/sandbox/mod.rs` (`WindowsNetworkPolicy` struct gains field) | library/model | — | same file: struct definition (385-401) | exact |
| `crates/nono-cli/data/policy.json` (`bun_runtime`/`mise_manager` groups + `bun-dev`/`mise-dev` profiles) | config | CRUD | same file: `go_runtime` group (416-424) + `go-dev` profile (1028-1044) | exact |
| `crates/nono-cli/tests/manifest_roundtrip.rs` (`AVAILABLE_GROUPS` additions) | test | batch | same file: `AVAILABLE_GROUPS` const (709-720) | exact |

## Pattern Assignments

### `crates/nono-cli/src/profile/mod.rs` — `platform_overrides` field + `apply_platform_overrides` (PROF-01, Wave A)

**Analog:** same file — `NetworkConfig` struct shape + `merge_profiles`'s `network:` merge arm.

**`Profile` struct field declaration pattern** (verified `profile/mod.rs:2290-2398`, showing the
exact idiom every new `Profile` field already follows — `#[serde(default)]` + doc comment):
```rust
/// A complete profile definition
#[derive(Debug, Clone, Default, Serialize)]
pub struct Profile {
    #[serde(default, deserialize_with = "deserialize_extends")]
    pub extends: Option<Vec<String>>,
    ...
    /// Windows-only. When true, routes non-PTY supervised launches through
    /// `WindowsTokenArm::BrokerLaunchNoPty` instead of `WriteRestricted`.
    /// ...
    #[serde(default)]
    pub windows_low_il_broker: bool,
    #[serde(default)]
    pub windows_interpreters: Vec<String>,
```
New field: `pub platform_overrides: Option<PlatformOverrides>` goes here, `#[serde(default)]`,
doc comment citing D-08 back-compat + D-10 extends-preservation.

**The 3-site + 3-schema-site sync template — copy `windows_low_il_broker`/`windows_interpreters`
verbatim as the pattern for wiring ANY new `Profile` field through all required sites:**

Site 1 — `Profile` struct (verified, lines 2383-2398):
```rust
/// Windows-only. When true, routes non-PTY supervised launches through
/// `WindowsTokenArm::BrokerLaunchNoPty` instead of `WriteRestricted`.
/// ...
#[serde(default)]
pub windows_low_il_broker: bool,
/// Windows-only. Bare exe names of the interpreter(s) this engine's launch
/// program will spawn (e.g. `python.exe` for a console-script entry point).
/// ...
#[serde(default)]
pub windows_interpreters: Vec<String>,
```

Site 2 — `ProfileDeserialize` struct, its own doc comment explaining `deny_unknown_fields`
round-tripping (verified, lines 2467-2475):
```rust
#[serde(default)]
windows_low_il_broker: bool,
/// Windows-only. See `Profile::windows_interpreters` doc-comment.
/// `deny_unknown_fields` on `ProfileDeserialize` requires this entry for
/// round-tripping — omitting it would cause a deserialization error for
/// any profile JSON containing `windows_interpreters`. Deserializes on all
/// platforms; runtime use is Windows-only.
#[serde(default)]
windows_interpreters: Vec<String>,
```

Site 3 — `impl From<ProfileDeserialize> for Profile`, exhaustive literal (verified, lines 2518-2523):
```rust
windows_low_il_broker: raw.windows_low_il_broker,
// Phase 71 Plan 01 (D-02): forward windows_interpreters verbatim.
// Deserializes on all platforms; consumed by validate_launch_paths
// on Windows only. Exhaustively enumerated here so rustc's
// struct-literal completeness check catches any future field additions.
windows_interpreters: raw.windows_interpreters,
```
This `From` impl is itself a full struct literal (confirmed — `windows_low_il_broker`/
`windows_interpreters` are two of ~25 exhaustively-listed fields starting at line 2498) so a missed
field here is a **compile error**, not a silent drop — the safety net Pitfall 2 describes.

**`finalize_profile` insertion point** (verified live, `profile/mod.rs:2882-2893` — matches
RESEARCH.md's Code Examples section exactly):
```rust
pub(crate) fn finalize_profile(mut profile: Profile) -> Result<Profile> {
    merge_implicit_default_groups(&mut profile)?;
    \ D-06: re-run the no_proxy/allow_domain overlap check on the fully
    // extends-merged profile. ...
    validate_profile_no_proxy(&profile)?;
    Ok(profile)
}
```
`apply_platform_overrides(profile)?` becomes the new first line (per RESEARCH's cited upstream
order), before `merge_implicit_default_groups`.

---

### `crates/nono-cli/src/profile/mod.rs` — `merge_profiles` exhaustive-struct-literal sites (PROF-01/PROF-03, all waves)

**Analog:** the `network: NetworkConfig { ... }` arm inside `merge_profiles` — an existing multi-field
merge arm using `dedup_append` for `Vec` fields (verified, lines 3353-3393):
```rust
network: NetworkConfig {
    block: base.network.block || child.network.block,
    network_profile: child.network.network_profile.merge(base.network.network_profile),
    allow_domain: merge_allow_domain(&base.network.allow_domain, &child.network.allow_domain),
    deny_domain: dedup_append(&base.network.deny_domain, &child.network.deny_domain),
    no_proxy: dedup_append(&base.network.no_proxy, &child.network.no_proxy),
    open_port: dedup_append(&base.network.open_port, &child.network.open_port),
    listen_port: dedup_append(&base.network.listen_port, &child.network.listen_port),
    connect_port: dedup_append(&base.network.connect_port, &child.network.connect_port),
    ...
```
`open_port_range`/`listen_port_range` (PROF-03) follow the exact `dedup_append` pattern shown for
`open_port`/`listen_port` — same field, range-typed sibling, same merge semantics (union,
order-preserving).

`merge_profiles` itself opens with a full-`Profile` exhaustive literal (verified, lines 3262-3266):
```rust
fn merge_profiles(base: Profile, child: Profile) -> Profile {
    Profile {
        extends: None,
        meta: child.meta,
        security: SecurityConfig { ... },
```
Adding `platform_overrides` to `Profile` forces a new top-level field here too — this is the
compile-forced safety net RESEARCH's Pattern 1 describes; the planner's task must add a
`platform_overrides:` line to this literal (choose child-wins-per-OS-block semantics per D-08, not a
naive `dedup_append`/`or()` — platform overrides are keyed by OS, not a flat list).

**Existing test-helper `Profile { .. }` literals to also update** (verified present, not just cited):
`profile/mod.rs:5347` (`base_profile()`-style test fixture, full `NetworkConfig` literal incl.
`open_port: vec![3000]`) and `:5441` (child fixture, `open_port: vec![3000, 5000]`) — both are
non-`..Default::default()` exhaustive literals and will fail to compile without the new fields.

---

### NEW fork-owned token-expansion module (PROF-02b, `@git:*`, Wave B)

**Analog for placement/shape:** `crates/nono-cli/src/protected_paths.rs` — a 405-line, self-contained,
single-purpose module with its own `#[cfg(test)]` suite, imported by name from `capability_ext.rs`
(`use crate::protected_paths::{self, ProtectedRoots};` at `capability_ext.rs:9`). This is the fork's
existing precedent for "small dedicated helper module, not folded into a mega-file" — use the same
shape for the ported `dynamic_providers.rs` → e.g. `crates/nono-cli/src/dynamic_tokens.rs`.

**cfg-gated fallback idiom already present in this fork** (verified, `policy.rs:1283-1290` — a
platform-gated free function with its own doc comment, the same shape D-01 describes for
`expand_dynamic_tokens`):
```rust
/// returned unexpanded (with `~` and `$TMPDIR` intact) for caller to expand.
/// Used by learn mode.
#[cfg(any(target_os = "linux", target_os = "macos"))]
pub fn get_system_read_paths(policy: &Policy) -> Vec<String> {
    let mut result = Vec::new();
    for group in policy.groups.values() {
        if !group_matches_platform(group) {
```
CONTEXT.md's D-01 finding cites the upstream shape (`#[cfg(any(...))] use ...expand_dynamic_tokens;`
/ `#[cfg(not(any(...)))] fn expand_dynamic_tokens(...)`) — this fork precedent confirms
`#[cfg(any(target_os = "linux", target_os = "macos"))]` is already the established gate expression
used elsewhere in the CLI crate, so the ported module's gate is idiomatically consistent, not novel.

---

### `crates/nono-cli/src/capability_ext.rs` — `$VAR`/`@git:*` call sites (PROF-02, Wave B)

**Analog:** the existing `expand_vars` call inside `apply_profile_dir_allows` (verified,
`capability_ext.rs:454-474`):
```rust
fn apply_profile_dir_allows(
    path_templates: &[String],
    access: AccessMode,
    workdir: &Path,
    protected_roots: &ProtectedRoots,
    caps: &mut CapabilitySet,
    label_prefix: &str,
) -> Result<()> {
    for path_template in path_templates {
        let path = expand_vars(path_template, workdir)?;
        validate_requested_dir(&path, "Profile", protected_roots, false)?;
        ...
```
`expand_vars` is imported at the top of the file (verified, line 8):
```rust
use crate::profile::{expand_vars, Profile};
```
and defined in `profile/mod.rs:3736-3757` (verified) — it already handles `~`, `$WORKDIR`,
`$TMPDIR`, `$UID`, `$XDG_CONFIG_HOME`. This is the single existing consumer site that PROF-02a's
generic `$VAR`-from-process-env expansion (upstream's `substitute_vars`) should extend, and where
PROF-02b's `expand_dynamic_tokens(entries, Some(workdir))` call should be inserted (per RESEARCH's
architecture diagram: `expand_dynamic_tokens` runs on the raw entry list *before* `expand_vars` runs
per-entry). There are **15 total `expand_vars(path_template, workdir)?` call sites** in this file
(verified via grep: lines 463, 693, 709, 742, 758, 773, 785, 823, 850, 891, 918, 945, 972, 1005,
1097) — each is a `fs.allow`/`read`/`write`/etc. list consumer and is a candidate site for the new
`expand_dynamic_tokens` pass, matching RESEARCH's "per fs.* list" plan.

**Naming-collision note (Pitfall 1, independently confirmed):** `crate::policy::expand_path`
(verified, `policy.rs:268`, signature `fn expand_path(path_str: &str) -> Result<PathBuf>`) is a
narrower, different-signature function in a different module from `capability_ext.rs`'s `expand_vars`
re-export. Do not name any new local helper `expand_path` — use `expand_profile_path` or similar per
RESEARCH's recommendation.

---

### `crates/nono/src/capability.rs` — `localhost_port_ranges` mechanism (PROF-03, Wave C)

**Analog:** the existing `localhost_ports` field + its builder method + its accessor (verified,
lines 874-1103, 1428-1429):
```rust
#[derive(Debug, Clone, Default)]
pub struct CapabilitySet {
    fs: Vec<FsCapability>,
    ...
    tcp_connect_ports: Vec<u16>,
    tcp_bind_ports: Vec<u16>,
    /// TCP ports allowed for bidirectional IPC (connect + bind).
    /// These apply regardless of NetworkMode.
    /// ...
    localhost_ports: Vec<u16>,
```
```rust
/// Allow bidirectional localhost TCP on a specific port (builder pattern).
/// ...
#[must_use]
pub fn allow_localhost_port(mut self, port: u16) -> Self {
    self.localhost_ports.push(port);
    self
}
```
```rust
pub fn localhost_ports(&self) -> &[u16] {
    &self.localhost_ports
}
```
`localhost_port_ranges: Vec<(u16, u16)>` is a new sibling field with `allow_localhost_port_range`
(builder) and `localhost_port_ranges()` (accessor) following this exact shape. `CapabilitySet`
derives `Default` (confirmed at line 873) so this addition is additive — **not** an exhaustive-literal
site, materially lower regression risk than the `Profile`/`NetworkConfig` changes above (RESEARCH's
Pattern 1 distinction, confirmed by direct inspection).

---

### `crates/nono/src/sandbox/macos.rs` / `linux.rs` — range-aware emitters (PROF-03, Wave C)

**macOS analog** — existing per-port Seatbelt emitter, extend additively (verified, lines 462-478,
731-765):
```rust
/// Seatbelt rules: one `(remote tcp "localhost:N")` per non-zero port; `0` adds
/// a single `localhost:*` outbound rule (`localhost:0` is invalid in Seatbelt).
fn push_localhost_tcp_outbound_seatbelt_rules(profile: &mut String, localhost_ports: &[u16]) {
    let wildcard = localhost_ports.contains(&0);
    for &lp in localhost_ports {
        if lp == 0 { continue; }
        profile.push_str(&format!(
            "(allow network-outbound (remote tcp \"localhost:{}\"))\n", lp
        ));
    }
    if wildcard {
        profile.push_str("(allow network-outbound (remote tcp \"localhost:*\"))\n");
    }
}
```
A sibling `push_localhost_tcp_outbound_seatbelt_range_rules` would unroll each `(start, end)` into
per-port rules (D-06: only macOS unrolls) after `merge_port_ranges`, gated by
`MACOS_PORT_RANGE_LIMIT`, called alongside the existing function at the call sites around lines
731-765 (`caps.localhost_ports()` is read there today, non-empty check + call).

**Linux analog** — existing per-port Landlock `NetPort` loop, extend additively (verified, lines
862-886):
```rust
if !matches!(caps.network_mode(), NetworkMode::AllowAll) {
    for port in caps.localhost_ports() {
        debug!("Adding localhost TCP connect rule for port {}", port);
        ruleset = ruleset
            .add_rule(NetPort::new(*port, AccessNet::ConnectTcp))
            .map_err(|e| {
                NonoError::SandboxInit(format!(
                    "Cannot add TCP connect rule for localhost port {}: {}", port, e
                ))
            })?;
        debug!("Adding localhost TCP bind rule for port {}", port);
        ruleset = ruleset
            .add_rule(NetPort::new(*port, AccessNet::BindTcp))
            .map_err(|e| { ... })?;
    }
}
```
Landlock's `NetPort::new` takes a single `u16`, not a range — confirm at absorb time whether the
`landlock` crate (v0.4, per CLAUDE.md) exposes a range-capable rule type; if not, the Linux emitter
unrolls too (per D-05's "no cap" note — Linux "accepts the full 16-bit space", implying unrolling
without `MACOS_PORT_RANGE_LIMIT`'s ceiling, not a native range primitive). This needs confirming
against the actual `landlock` crate API during implementation, not assumed from this pattern map.

---

### `crates/nono-cli/src/exec_strategy/supervisor_linux.rs` — `proxy_bind_port_ranges` (PROF-03, Wave C)

**Analog:** existing `proxy_bind_ports: Vec<u16>` field on `SupervisorConfig`, verified present at
`crates/nono-cli/src/exec_strategy.rs:413`, populated from `caps.network_mode()`'s
`ProxyOnly{bind_ports}` in `crates/nono-cli/src/supervised_runtime.rs:373` (Linux-only seccomp-notify
fallback path, narrower/third context distinct from the WFP path — per RESEARCH's Fork-Presence
table). A sibling `proxy_bind_port_ranges: Vec<(u16,u16)>` populated from
`caps.localhost_port_ranges().to_vec()` follows the same 2-line addition shape at both sites.

---

### Windows WFP-native range emitter — FORK-ORIGINAL, no upstream analog (PROF-03/D-06/D-07, Wave C sub-wave)

This is the one surface in this phase with **no upstream code to port**. The closest analogs are the
existing single-port machinery in the same files, extended:

**`PortCondition` enum + its two consumers** (verified, `nono-wfp-service.rs:1087-1092`):
```rust
#[cfg(target_os = "windows")]
#[derive(Clone, Copy)]
enum PortCondition {
    Remote(u16),
    Local(u16),
}
```
New variants: `RemoteRange(u16, u16)`, `LocalRange(u16, u16)`.

**`build_policy_filter_specs`'s per-port loop to extend** (verified, lines 1186-1264) — four loops
exist (outbound `tcp_connect_ports`, outbound `localhost_ports` w/ `loopback_only: true`, inbound
`tcp_bind_ports`, inbound `localhost_ports` w/ `loopback_only: true`), e.g.:
```rust
for port in &request.tcp_connect_ports {
    specs.push(PolicyFilterSpec {
        key: deterministic_filter_key(base, &format!("{}-connect-{port}", layer.label)),
        layer_key: layer.key,
        action: FilterAction::Permit,
        port: Some(PortCondition::Remote(*port)),
        loopback_only: false,
    });
}
```
A parallel loop over a new `request.localhost_port_ranges: Vec<(u16,u16)>` pushes one
`PolicyFilterSpec` **per range entry** (D-06: no per-port unroll — this is the key difference from
the existing per-port loops, which push one spec per discrete port).

**`add_policy_filter`'s `match port` arm to extend** (verified, lines 1461-1474):
```rust
if let Some(port) = spec.port {
    let (field_key, value) = match port {
        PortCondition::Remote(value) => (FWPM_CONDITION_IP_REMOTE_PORT, value),
        PortCondition::Local(value) => (FWPM_CONDITION_IP_LOCAL_PORT, value),
    };
    conditions.push(FWPM_FILTER_CONDITION0 {
        fieldKey: field_key,
        matchType: FWP_MATCH_EQUAL,
        conditionValue: FWP_CONDITION_VALUE0 {
            r#type: FWP_UINT16,
            Anonymous: FWP_CONDITION_VALUE0_0 { uint16: value },
        },
    });
}
```
New arms for `RemoteRange`/`LocalRange` build an `FWP_RANGE0 { valueLow, valueHigh }` (each a
`FWP_VALUE0 { type: FWP_UINT16, ... }`) and push a condition with `matchType: FWP_MATCH_RANGE`,
`conditionValue.rangeValue = &mut range` — `FWP_MATCH_RANGE`/`FWP_RANGE0`/`FWP_VALUE0` are already
present in the pinned `windows-sys 0.59` dependency (RESEARCH confirmed, not independently
re-verified by this agent — trust RESEARCH's Standard Stack section here, it cites an exact vendored
source path).

**Existing layers, no new registration needed** (verified, lines 1105-1128 — matches RESEARCH's Code
Examples section exactly):
```rust
fn build_wfp_layer_specs() -> [WfpLayerSpec; 4] {
    [
        WfpLayerSpec { key: FWPM_LAYER_ALE_AUTH_CONNECT_V4, label: "connect-v4", rule_name: "outbound" },
        WfpLayerSpec { key: FWPM_LAYER_ALE_AUTH_CONNECT_V6, label: "connect-v6", rule_name: "outbound" },
        WfpLayerSpec { key: FWPM_LAYER_ALE_AUTH_RECV_ACCEPT_V4, label: "recv-accept-v4", rule_name: "inbound" },
        WfpLayerSpec { key: FWPM_LAYER_ALE_AUTH_RECV_ACCEPT_V6, label: "recv-accept-v6", rule_name: "inbound" },
    ]
}
```

**Existing test-fixture pattern to copy for the new range tests** (verified, lines 1820-1939 —
`sample_request()` helper builds a `WfpRuntimeActivationRequest` literal, individual tests override
specific fields via struct-update syntax):
```rust
WfpRuntimeActivationRequest {
    protocol_version: WFP_RUNTIME_PROTOCOL_VERSION,
    request_kind: "activate_blocked_mode".to_string(),
    network_mode: "blocked".to_string(),
    preferred_backend: "windows-filtering-platform".to_string(),
    active_backend: "windows-filtering-platform".to_string(),
    runtime_target: "blocked Windows network access".to_string(),
    tcp_connect_ports: Vec::new(),
    tcp_bind_ports: Vec::new(),
    localhost_ports: Vec::new(),
    target_program_path: Some(r"C:\tools\target.exe".to_string()),
    session_sid: None,
    outbound_rule_name: Some("nono-test-out".to_string()),
    inbound_rule_name: Some("nono-test-in".to_string()),
}
...
#[cfg(target_os = "windows")]
#[test]
fn proxy_policy_filter_specs_include_loopback_permits_and_block_fallback() {
    let request = WfpRuntimeActivationRequest {
        request_kind: "activate_proxy_mode".to_string(),
        network_mode: "proxy-only".to_string(),
        tcp_bind_ports: vec![8080],
        localhost_ports: vec![8080],
        ..sample_request()
    };
    let specs = build_policy_filter_specs(&request, "nono-out", "nono-in");
```
A new PROF-03d test (`localhost_port_ranges: vec![(3000, 3999)], ..sample_request()`) follows this
exact struct-update-syntax fixture pattern — construct via `build_policy_filter_specs` only (pure,
no live engine, no `#[cfg(target_os = "windows")]` gate needed for the spec-construction test itself
if the struct/enum types are made available cross-platform... but note `PortCondition`,
`PolicyFilterSpec`, and `build_policy_filter_specs` are ALL currently `#[cfg(target_os = "windows")]`-
gated (verified at lines 1072, 1080, 1087, 1094, 1104, 1130, 1135, 1142, 1147, 1162, 1175, 1185), so
the new test must carry the same `#[cfg(target_os = "windows")]` gate as
`proxy_policy_filter_specs_include_loopback_permits_and_block_fallback` above it — it will only run
in the Windows leg of `make ci` / cross-target CI, not locally on this dev host's clippy runs for
linux-gnu/apple-darwin.

**`WfpRuntimeActivationRequest` contract field** (verified, `windows_wfp_contract.rs:6-20`):
```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WfpRuntimeActivationRequest {
    pub protocol_version: u32,
    pub request_kind: String,
    pub network_mode: String,
    pub preferred_backend: String,
    pub active_backend: String,
    pub runtime_target: String,
    pub tcp_connect_ports: Vec<u16>,
    pub tcp_bind_ports: Vec<u16>,
    pub localhost_ports: Vec<u16>,
    pub target_program_path: Option<String>,
    pub session_sid: Option<String>,
    pub outbound_rule_name: Option<String>,
    pub inbound_rule_name: Option<String>,
}
```
New field `pub localhost_port_ranges: Vec<(u16, u16)>` — note this struct derives `PartialEq, Eq`
but NOT a builder/exhaustive-forced-update pattern; every construction site (verified: `sample_request()`
in the test module, and `build_wfp_runtime_activation_request` in `exec_strategy_windows/network.rs`)
IS a full literal today (no `..Default::default()` — `Serialize`/`Deserialize` structs of this shape
in this codebase are consistently built as full literals), so the compiler will force both sites to
be updated once the field is added, but there is no `#[derive(Default)]` shortcut available.

**`compile_network_policy` (library, policy-free) — the mechanism producing `WindowsNetworkPolicy`
from `CapabilitySet`** (verified, `crates/nono/src/sandbox/windows.rs:328-369`):
```rust
#[must_use]
pub fn compile_network_policy(caps: &CapabilitySet) -> WindowsNetworkPolicy {
    let mode = match caps.network_mode() { ... };
    let unsupported = Vec::new();
    let mut tcp_connect_ports = caps.tcp_connect_ports().to_vec();
    tcp_connect_ports.sort_unstable();
    tcp_connect_ports.dedup();
    let mut tcp_bind_ports = caps.tcp_bind_ports().to_vec();
    tcp_bind_ports.sort_unstable();
    tcp_bind_ports.dedup();
    let mut localhost_ports = caps.localhost_ports().to_vec();
    localhost_ports.sort_unstable();
    localhost_ports.dedup();
    let requires_backend = !matches!(mode, WindowsNetworkPolicyMode::AllowAll)
        || !tcp_connect_ports.is_empty()
        || !tcp_bind_ports.is_empty()
        || !localhost_ports.is_empty();
    let preferred_backend = if requires_backend { WindowsNetworkBackendKind::Wfp } else { WindowsNetworkBackendKind::None };
    let active_backend = preferred_backend;

    WindowsNetworkPolicy {
        mode, tcp_connect_ports, tcp_bind_ports, localhost_ports, unsupported,
        preferred_backend, active_backend,
    }
}
```
`localhost_port_ranges` follows the same sort/dedup-then-include-in-literal shape (`merge_port_ranges`
should be used here instead of plain `sort_unstable`/`dedup`, since ranges need interval merging, not
just duplicate removal — this is the one place the pattern must diverge from the discrete-port
sibling code). `requires_backend` must also check `!localhost_port_ranges.is_empty()`.

`WindowsNetworkPolicy` struct itself (verified, `crates/nono/src/sandbox/mod.rs:382-401`):
```rust
#[cfg(target_os = "windows")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowsNetworkPolicy {
    pub mode: WindowsNetworkPolicyMode,
    pub tcp_connect_ports: Vec<u16>,
    pub tcp_bind_ports: Vec<u16>,
    pub localhost_ports: Vec<u16>,
    pub unsupported: Vec<WindowsUnsupportedNetworkIssue>,
    pub preferred_backend: WindowsNetworkBackendKind,
    pub active_backend: WindowsNetworkBackendKind,
}
```
No `Default` derive — the `compile_network_policy` literal above is the only construction site found
by this pass, so adding `localhost_port_ranges` here is compile-forced at exactly one site (low risk,
easy to verify complete).

**`exec_strategy_windows/network.rs` — where `compile_network_policy`'s output is threaded onward**
(verified, lines 460-529, `describe_windows_network_runtime_target` + `build_wfp_runtime_activation_request`):
```rust
pub(super) fn build_wfp_runtime_activation_request(
    policy: &nono::WindowsNetworkPolicy,
) -> WfpRuntimeActivationRequest {
    let network_mode = match &policy.mode { ... };
    let mut tcp_bind_ports = policy.tcp_bind_ports.clone();
    let mut localhost_ports = policy.localhost_ports.clone();
    if let nono::WindowsNetworkPolicyMode::ProxyOnly { port, bind_ports } = &policy.mode {
        tcp_bind_ports.extend(bind_ports.iter().copied());
        tcp_bind_ports.sort_unstable();
        tcp_bind_ports.dedup();
        localhost_ports.push(*port);
        localhost_ports.sort_unstable();
        localhost_ports.dedup();
    }
    WfpRuntimeActivationRequest {
        protocol_version: WFP_RUNTIME_PROTOCOL_VERSION,
        request_kind: ...,
        network_mode: network_mode.to_string(),
        preferred_backend: policy.preferred_backend.label().to_string(),
        active_backend: policy.active_backend.label().to_string(),
        runtime_target: describe_windows_network_runtime_target(policy),
        tcp_connect_ports: policy.tcp_connect_ports.clone(),
        tcp_bind_ports,
        localhost_ports,
        target_program_path: None,
        outbound_rule_name: None,
        inbound_rule_name: None,
        session_sid: None,
    }
}
```
`localhost_port_ranges: policy.localhost_port_ranges.clone()` is added to this literal (a straight
clone-through; the `ProxyOnly` port doesn't affect ranges the way it affects the discrete localhost
port, so no extra push/sort/dedup is needed for the new field unless product intent says otherwise —
flag this as a planning-time judgment call, not settled by this pattern map).

**Integration point confirmed general-path, not daemon-only** (RESEARCH's correction, independently
re-verified by this agent by reading the actual code above): `compile_network_policy` and
`build_wfp_runtime_activation_request` are reachable from ordinary `nono run` (not gated behind
`nono agent launch`), matching RESEARCH's Fork-Presence Verification table finding exactly.

---

### `crates/nono/schema/capability-manifest.schema.json` — `PortConfig.localhost_range` (typify-generated, Wave C)

**Analog:** the existing `PortConfig.localhost` property (verified, lines 177-198):
```json
"PortConfig": {
  "type": "object",
  "description": "TCP port allowlists.",
  "additionalProperties": true,
  "properties": {
    "connect": {
      "type": "array",
      "items": { "type": "integer", "minimum": 1, "maximum": 65535 },
      "description": "TCP ports allowed for outbound connections."
    },
    "bind": { ... },
    "localhost": {
      "type": "array",
      "items": { "type": "integer", "minimum": 1, "maximum": 65535 },
      "description": "Localhost TCP ports allowed for bidirectional IPC (connect + bind)."
    }
  }
}
```
`localhost_range` is a new sibling property — an array of 2-element `[start, end]` tuples or an
array of `{start, end}` objects (planner's choice of JSON shape, but must match whatever
`manifest_convert.rs` expects to consume). `additionalProperties: true` here (unlike the stricter
profile schema below) means this file alone would NOT reject an unrecognized property — the
type-safety comes entirely from `manifest_convert.rs` consuming exactly the generated field name, per
RESEARCH's Pattern 2 (typify regenerates `crates/nono/src/manifest.rs` from this file at build time;
never hand-edit `manifest.rs`).

---

### `crates/nono-cli/data/nono-profile.schema.json` — `network.open_port_range`/`listen_port_range` + `platform_overrides` (Wave A + Wave C)

**Analog:** the `network` object's existing `open_port`/`listen_port` properties, inside a STRICT
`additionalProperties: false` object (verified — top-level strictness at line 7, `network` object at
line 44, properties at lines 515-534):
```json
"open_port": {
  "type": "array",
  "items": { "type": "integer" },
  "description": "Localhost TCP IPC (connect+bind). Port 0: macOS only (localhost:* outbound); Linux needs explicit ports."
},
"port_allow": {
  "type": "array",
  "items": { "type": "integer" },
  "description": "Legacy alias for open_port."
},
"allow_port": {
  "type": "array",
  "items": { "type": "integer" },
  "description": "Legacy alias for open_port."
},
"listen_port": {
  "type": "array",
  "items": { "type": "integer" },
  "description": "TCP ports the sandboxed child may listen on."
},
```
`open_port_range`/`listen_port_range` are new siblings here, each an array of `[start, end]` pairs
(or matching whatever shape the `NetworkConfig` Rust field uses). **This file's top-level
`additionalProperties: false` (verified at line 7) is the mandatory companion edit** — Pitfall 4 is
independently confirmed: omitting this edit makes `test_schema_validates_builtin_profiles_in_policy_json`
(and any runtime `validate_against_schema()` call) reject any profile actually using the new fields,
even though `serde` parses them fine. The same applies to `platform_overrides` — it needs its own new
top-level property object in this file, following the same "new named property, `additionalProperties: false`
enforced on all nested objects too" idiom visible throughout this file (16 separate
`"additionalProperties": false` occurrences at object boundaries, confirmed via grep).

---

### `crates/nono-cli/data/policy.json` — `bun`/`mise` presets (PROF-04, Wave D)

**Analog:** the existing `go_runtime` group (verified, lines 416-424) and `go-dev` profile (verified,
lines 1028-1044):
```json
"go_runtime": {
  "description": "Go toolchain paths",
  "allow": {
    "read": [
      "~/go",
      "/usr/local/go"
    ]
  }
}
```
```json
"go-dev": {
  "extends": "default",
  "meta": {
    "name": "go-dev",
    "version": "1.0.0",
    "description": "Go SDK development profile with GOPATH and module support",
    "author": "nono-project"
  },
  "security": {
    "groups": ["go_runtime"],
    "signal_mode": "isolated"
  },
  "filesystem": {},
  "network": { "block": false, "network_profile": "developer" },
  "workdir": { "access": "readwrite" },
  "interactive": false
}
```
`bun_runtime`/`mise_manager` groups and `bun-dev`/`mise-dev` profiles copy this shape exactly —
same `extends: "default"`, same `meta` block shape, same `security.groups` + `signal_mode`, same
`network`/`workdir`/`interactive` defaults. Adjust `allow.read` paths per each tool's actual install
locations (upstream's `b620ed8e`/`f016b2d5` diffs are the source of truth for the correct paths, per
D-11 — this pattern map does not re-derive those paths).

---

### `crates/nono-cli/tests/manifest_roundtrip.rs` — `AVAILABLE_GROUPS` + resolvability test (PROF-04, Wave D)

**Analog:** the existing const array (verified, lines 709-720):
```rust
const AVAILABLE_GROUPS: &[&str] = &[
    "deny_credentials",
    "deny_shell_configs",
    "deny_shell_history",
    "dangerous_commands",
    "git_config",
    "node_runtime",
    "python_runtime",
    "rust_runtime",
    "user_tools",
    "unlink_protection",
];
```
Add `"bun_runtime"`, `"mise_manager"` (or whatever names Wave D settles on) as new entries. D-11's
"resolvable, not merely present" requirement needs a SEPARATE new test beyond this array — this array
only feeds the property-based random-profile generator (per the file's own comment at line 702-703,
"randomly generated profiles must round-trip through manifest"), it does not prove `bun-dev`/`mise-dev`
resolve by name. A dedicated `profile::mod.rs`-side test analogous to
`claude_code_builtin_profile_has_windows_low_il_broker_true` (verified pattern at
`profile/mod.rs:8176-8195`, which loads a specific built-in profile by name and asserts a field) is
the right template for that resolvability test — load `"bun-dev"` by name, assert it resolves without
error and carries the `bun_runtime` group.

## Shared Patterns

### Exhaustive-struct-literal safety net (applies to every `Profile`/`NetworkConfig`/`WfpRuntimeActivationRequest` field addition)
**Source:** `crates/nono-cli/src/profile/mod.rs:3262` (`merge_profiles`), `:2496` (`impl From<ProfileDeserialize>`),
`:5347`/`:5441` (test fixtures); `crates/nono-cli/src/bin/nono-wfp-service.rs:1820` (`sample_request()`)
**Apply to:** every new field on `Profile`, `NetworkConfig`, and `WfpRuntimeActivationRequest`.
**Rule:** none of these types derive `Default` for their production-code construction paths (test
code sometimes uses `..Default::default()`, e.g. `profile/mod.rs:6400`, but the canonical merge/convert
functions do not) — so a missing field is a compile error, not a silent drop. Grep all literal sites
for the type name before considering a field-add complete; do not rely on memory of "the two sites
CONTEXT.md named."

### `Vec` field merge via `dedup_append` (applies to all new list-typed `NetworkConfig` fields)
**Source:** `crates/nono-cli/src/profile/mod.rs:3363-3367` (`deny_domain`, `no_proxy`, `open_port`,
`listen_port`, `connect_port` all merge this way)
**Apply to:** `open_port_range`/`listen_port_range` (as `Vec<(u16,u16)>`, same `dedup_append` call
shape, `dedup_append` is generic over `T: PartialEq + Clone` so tuple elements work unchanged).

### Three-schema-site sync (applies to every `Profile`/`NetworkConfig` field, generalizes D-09)
**Source:** `crates/nono-cli/src/profile/mod.rs` (serde structs) + `crates/nono-cli/data/nono-profile.schema.json`
(strict `additionalProperties: false`, verified line 7) + (for `CapabilityManifest`/`PortConfig` only)
`crates/nono/schema/capability-manifest.schema.json` (typify source, `additionalProperties: true`)
**Apply to:** `platform_overrides`, `open_port_range`, `listen_port_range` all need the serde struct
AND the `nono-profile.schema.json` companion property. Only the port-range fields also touch
`capability-manifest.schema.json`'s `PortConfig` (platform_overrides has no manifest-layer
equivalent — it is profile-only, confirmed by the absence of any `platform_overrides`-shaped
reference in `manifest.rs`/`manifest_convert.rs` during this pass's greps).

### cfg-gated Unix-vs-fallback idiom
**Source:** `crates/nono-cli/src/policy.rs:1285` (`#[cfg(any(target_os = "linux", target_os = "macos"))]`
on a free function)
**Apply to:** the new `dynamic_tokens` module's `expand_dynamic_tokens` — same gate expression already
used elsewhere in this crate, confirming it is the established idiom, not something D-01/D-02
introduces fresh.

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| Windows WFP `PortCondition::RemoteRange`/`LocalRange` construction + its unit tests | service (kernel-facing) | request-response (IPC → kernel) | Fork-original (D-07) — upstream only implemented macOS/Linux emitters. The pattern-assignment section above gives the closest possible analog (the existing single-port `PortCondition::Remote`/`Local` arms in the same file), but there is no existing range-condition code anywhere in the fork or upstream to copy directly; `FWP_MATCH_RANGE`/`FWP_RANGE0` construction must be written fresh from the `windows-sys` API surface. |
| `crates/nono/src/sandbox/linux.rs`'s range-emitter — specific Landlock range API (if `landlock` v0.4 lacks a native range rule type) | library | transform | Not verified in this pass whether `landlock` crate v0.4's `NetPort` type accepts a range; RESEARCH's D-05 note ("Linux accepts the full 16-bit space... no cap") implies unrolling without a ceiling, but this pattern map did not open the `landlock` crate's vendored source to confirm — flag for the planner to verify against the actual crate API before assuming an unroll-without-cap shape is achievable in one call. |

## Metadata

**Analog search scope:** `crates/nono/src/{capability.rs,sandbox/{macos,linux,windows,mod}.rs}`,
`crates/nono-cli/src/{profile/mod.rs,capability_ext.rs,policy.rs,protected_paths.rs,
exec_strategy.rs,exec_strategy/supervisor_linux.rs,exec_strategy_windows/network.rs,
windows_wfp_contract.rs,bin/nono-wfp-service.rs}`, `crates/nono-cli/data/{policy.json,
nono-profile.schema.json}`, `crates/nono/schema/capability-manifest.schema.json`,
`crates/nono-cli/tests/manifest_roundtrip.rs`.
**Files scanned:** 17 (all files this phase's Recommended Wave Structure names as touched)
**Pattern extraction date:** 2026-07-30
