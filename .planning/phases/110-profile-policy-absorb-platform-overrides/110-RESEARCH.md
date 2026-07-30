# Phase 110: Profile/Policy Absorb + platform_overrides - Research

**Researched:** 2026-07-30
**Domain:** Rust profile-schema absorb (upstream cherry-pick) + fork-original Windows WFP kernel enforcement
**Confidence:** HIGH (all 7 upstream commits read in full diff; every cited fork surface opened and line-verified against live HEAD on `milestone/v2.13-carryforward-closeout`)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**PROF-02 — the deferred-subsystem dependency**
- **D-01 (finding):** the dependency is `cfg`-gated, and upstream already ships a non-Unix
  fallback. `d4927f95` opens `capability_ext.rs` with a `#[cfg(any(target_os = "linux",
  target_os = "macos"))] use crate::tool_sandbox::dynamic_providers::expand_dynamic_tokens;` arm
  and a `#[cfg(not(...))] fn expand_dynamic_tokens(...)` fallback returning entries unchanged. On
  Windows this absorbs with no `tool_sandbox` dependency at all; the Unix arm is the real problem
  because cross-target clippy is mandatory and a dangling `crate::tool_sandbox` path fails the gate.
- **D-02:** PORT `expand_dynamic_tokens` into a fork-owned module (alongside `capability_ext.rs`,
  or a small dedicated token-expansion module — planner's choice), so both `cfg` arms resolve
  without pulling the tool-sandbox subsystem forward. Do NOT import `crate::tool_sandbox::*` — that
  module does not exist in the fork and must not be created as a side effect of this phase.
  **MANDATORY v3.7 reconciliation note:** record in the SUMMARY and finding trail that this one
  function arrived early; v3.7 must reconcile it (adopt the fork's copy, or replace it and delete
  the fork module) rather than introducing a second copy.
- **D-03:** `$VAR` (#1296, `2cbaa9a0`) has no such dependency and absorbs normally. PROF-02 covers
  both features; only `@git:*` needed the decision above.

**PROF-03 — port ranges**
- **D-04:** ADR-86 is INTACT — `capability.rs +155` is mechanism, not policy. Adds
  `MACOS_PORT_RANGE_LIMIT` (2^14, because `sandbox_init()` SIGILLs above ~17,770 rules), a pure
  `merge_port_ranges(&[(u16,u16)]) -> Vec<(u16,u16)>` helper, and a `localhost_port_ranges` field
  with a range-allow method. No policy or enforcement decision enters the library.
- **D-05:** Phase 110 absorbs `d5803b99` WHOLE — schema + `capability.rs` + BOTH Unix emitters +
  the Windows emitter. One commit, absorbed once, by one phase. **Roadmap amendment applied
  2026-07-30:** Phase 111's CORE-01/SC1 has had the `#1398` `macos.rs` port-range emitter claim
  REMOVED — Phase 111 must not re-absorb it.
- **D-06:** the Windows emitter uses WFP-native ranges — no unrolling, no macOS-style limit. Do
  not mimic Seatbelt's per-port unroll, and do not propagate `MACOS_PORT_RANGE_LIMIT` to Windows.
- **D-07:** the Windows emitter is FORK-ORIGINAL enforcement code and gets its own threat model.
  Upstream implemented only macOS and Linux; this is the first genuinely new kernel-facing surface
  in v3.6, written not ported. Discrete-`Vec<u16>` back-compat is required (SC3).

**PROF-01 — `platform_overrides` and the flag migration**
- **D-08:** back-compat contract — BOTH forms accepted, `platform_overrides.windows` WINS, no
  deprecation warning this milestone.
- **D-09:** the migration must cover BOTH declaration sites. `windows_low_il_broker` and
  `windows_interpreters` are declared at `crates/nono-cli/src/profile/mod.rs:2391/2398` and again
  at ~2468-2475 (a second struct with its own serde handling). Migrating only one site leaves a
  silently-diverging second path.
- **D-10:** `#1380` (`719975cf`) is the `extends`-preservation fix and is required, not optional.
  The fork already has profile `extends` (#1320 is present per the parity map). Without #1380,
  `platform_overrides` is silently dropped during `extends` resolution.

**PROF-04**
- **D-11:** `bun` (#1305) and `mise` (#1387) are `policy.json` + roundtrip-test changes only. Low
  risk. Verify the presets are actually *resolvable* (SC4), not merely present in the JSON.

**Standing Rules (carried, not re-litigated)**
- **D-12:** cross-target clippy is MANDATORY for this phase. `d5803b99` alone touches
  `crates/nono/src/sandbox/macos.rs` (+150), `crates/nono/src/sandbox/linux.rs` (+28), and
  `crates/nono-cli/src/exec_strategy/supervisor_linux.rs` (+81). Both gates (`cross` linux-gnu,
  `cargo-zigbuild` apple-darwin) GREEN locally — no PARTIAL→CI. `make ci`, not clippy alone.
- **D-13:** rebuild BOTH bindings if any `nono-proxy`/`nono` struct changes — `maturin build`
  (`../nono-py`), `napi build --platform --release` (`../nono-ts`). Phase 109 proved this is
  load-bearing. `capability.rs` gains a field here, so expect drift.
- **D-14:** verify feature presence by BEHAVIOR, never by name, and confirm the fork actually HAS
  a file before dispositioning a commit `adopt`. This has now failed seven times in this milestone.

### Claude's Discretion
- Where the ported `expand_dynamic_tokens` lives (module name/placement), provided it is fork-owned
  and not under a `tool_sandbox` path.
- Plan/wave decomposition, provided `d5803b99` (the largest, cross-cutting commit) is not bundled
  into one plan with the trivial PROF-04 preset changes.
- Whether `bun` and `mise` share a plan (they almost certainly should).

### Deferred Ideas (OUT OF SCOPE)
- The 3 unmapped PROF-cluster commits → Phase 112: `0374e454` (#1400/#1402 omit inheritable
  `Option` fields when `None` on save), `9ef59181` (credential-provider doc comment), `f58c7c22`
  (#1320 CLI profile `extends` — already present in the fork).
- v3.7 reconciliation of the ported `expand_dynamic_tokens` (D-02) — must adopt-or-replace, never
  silently duplicate.
- Gray areas raised but not discussed (available if planning needs them): whether
  `platform_overrides` should also absorb the fork's other platform-divergent settings beyond the
  two named flags; whether `bun`/`mise` presets need Windows interpreter entries to be meaningful on
  this fork; and deprecation-warning timing for the legacy `windows_*` flags (D-08 defers it).
- Out of scope entirely: NET work (Phase 109, done); SPIFFE (Phase 113).
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| PROF-01 | `platform_overrides` per-OS profile patching (#1371) absorbed and preserved through `extends` resolution (#1380); `windows_low_il_broker`/`windows_interpreters` migrated into `platform_overrides.windows` with back-compat aliases. | `ae1c513e`/`719975cf` diffs read in full (see Code Examples); all 3 declaration sites for the two flags verified (Fork-Presence table, Pitfall 2); `merge_profiles`'s exhaustive-struct-literal safety net documented (Pattern 1). |
| PROF-02 | `$VAR` (#1296) and `@git:*` (#1298) token expansion in profile filesystem paths. | `2cbaa9a0` diff shows fork lacks 2 of 3 consumer sites (Pitfall 3); `d4927f95` diff shows `dynamic_providers.rs` is a self-contained ~412-line module safe to port whole (Don't Hand-Roll); D-01's cfg-gated fallback shape confirmed byte-for-byte accurate. |
| PROF-03 | Port-range profile schema (#1398) absorbed with a WFP-native remote-port-range emitter on Windows and discrete-`Vec<u16>` back-compat. | Full 17-file diff read; `capability.rs` mechanism confirmed policy-free (Architectural Responsibility Map); both Unix emitters confirmed additive; Windows WFP mechanism confirmed buildable with existing `windows-sys` pin, existing general (non-daemon-only) integration point identified and corrected from prior framing (Fork-Presence Verification). |
| PROF-04 | `bun` (#1305) and `mise` (#1387) runtime presets present and resolvable. | Both diffs read in full; fork's `policy.json`/`manifest_roundtrip.rs` shape confirmed matching; D-11 "resolvable not merely present" gap identified in Validation Architecture (no existing test loads these profiles by name). |
</phase_requirements>

## Summary

All 7 upstream commits in scope (`ae1c513e`, `719975cf`, `2cbaa9a0`, `d4927f95`, `d5803b99`,
`b620ed8e`, `f016b2d5`) are reachable in this repo's `upstream` remote (`nolabs-ai/nono`) and were
read as full diffs, not summaries. Two of CONTEXT.md's citations needed correction (see
`## Fork-Presence Verification`), and one significant scope discovery changes how PROF-03/SC3
should be planned: the fork already has a **general-purpose, non-daemon-gated** Windows WFP
network-policy path (`crates/nono/src/sandbox/windows.rs::compile_network_policy` →
`crates/nono-cli/src/exec_strategy_windows/network.rs::prepare_network_enforcement`, invoked from
the ordinary `nono run` supervised-exec dispatch), separate from the narrower
`nono agent launch`-only daemon path cited in project memory. The WFP-native range emitter should
extend the **general** path, not (only) the daemon path — this is more reachable, and more
testable without live kernel access, than the CONTEXT.md framing implied.

`platform_overrides` (PROF-01) is a clean, well-isolated absorb: a new `Option<PlatformOverrides>`
field on `Profile`, applied once in `finalize_profile` right before the group-merge step, with
merge logic that must be threaded through the fork's own `merge_profiles` (an **exhaustive struct
literal**, not `..Default::default()` — the compiler forces every call site to be updated, which
is a real safety net against the "silently dropped" failure class D-10 worries about). `$VAR`
(PROF-02a) is genuinely narrower in the fork than upstream's diff suggests — the fork lacks two of
the three consumer sites upstream's commit touches (`wiring.rs`'s pack-install `expand_vars` and
`proxy_runtime.rs`'s credential-capture-command expansion are both **absent subsystems** in this
fork); only the `capability_ext.rs` filesystem-path-expansion site applies. `@git:*` (PROF-02b)
confirms D-01/D-02 exactly: the dependency is a single self-contained ~412-line non-test module
(`dynamic_providers.rs`) with zero references to the rest of `tool-sandbox/`, safely portable
whole. Port ranges (PROF-03, `d5803b99`) is the largest commit (17 files) but decomposes cleanly:
`capability.rs` mechanism (safe — `CapabilitySet` derives `Default`, so the new field is additive,
not exhaustive-literal-forcing, a materially lower risk than the `Profile`/`NetworkConfig`
changes), two already-existing Unix emitters to extend, and a Windows emitter that is
**fork-original** kernel-facing code layered onto an existing, already-tested WFP filter-spec
builder (`crates/nono-cli/src/bin/nono-wfp-service.rs`) that already supports per-port
`FWP_MATCH_EQUAL` filters and needs a new `PortCondition::RemoteRange`/`LocalRange` variant using
`FWP_MATCH_RANGE` + `FWP_RANGE0` — both already available in the pinned `windows-sys = "0.59"`
dependency with no new crate needed. `bun`/`mise` (PROF-04) are trivial, ~30-line `policy.json` +
test-list additions matching an existing pattern exactly.

**Primary recommendation:** Plan PROF-01 (`platform_overrides` + `extends` preservation) and
PROF-02 (`$VAR` + ported `@git:*`) together as they touch the same file
(`crates/nono-cli/src/profile/mod.rs` / `capability_ext.rs`) and are both schema/mechanism work;
plan PROF-03 (port ranges) as its own wave given its size and the genuinely new Windows kernel
surface; plan PROF-04 (`bun`/`mise`) as a small trailing wave, per CONTEXT.md's own discretion
note. Do not bundle PROF-03 with PROF-04.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| `platform_overrides` schema field + merge | CLI (`nono-cli/src/profile/mod.rs`) | — | Profile parsing/merging is entirely CLI-side; the library never sees raw JSON profiles. |
| `$VAR` / `@git:*` path expansion | CLI (`capability_ext.rs`, new token module) | — | Expansion happens before capabilities are constructed; library receives only resolved paths. |
| `localhost_port_ranges` mechanism (`merge_port_ranges`, `allow_localhost_port_range`) | Library (`crates/nono/src/capability.rs`) | — | Pure, caller-supplied-list mechanism on `CapabilitySet` — ADR-86-compliant (D-04 confirms no policy enters the library). |
| macOS Seatbelt / Linux Landlock range emitters | Library (`crates/nono/src/sandbox/{macos,linux}.rs`) | — | Existing platform sandbox backends; ranges are additive to already-present per-port emit logic. |
| Windows WFP-native range emitter | CLI (`crates/nono-cli/src/bin/nono-wfp-service.rs`, `exec_strategy_windows/network.rs`) | Library (`crates/nono/src/sandbox/windows.rs::compile_network_policy`) | `compile_network_policy` (library) reads `CapabilitySet` and produces a policy-free `WindowsNetworkPolicy` value; the WFP service binary (CLI-owned, separate process) is where kernel filter construction — genuinely new enforcement code — lives. |
| `bun`/`mise` presets | CLI (`crates/nono-cli/data/policy.json`) | — | Embedded policy data, CLI-owned per existing pattern (all other runtime presets live here). |

## Fork-Presence Verification

| Claimed surface (from CONTEXT.md / ROADMAP) | Verified? | Actual location | Notes |
|---|---|---|---|
| `crates/nono-cli/src/profile/mod.rs:2391` — `windows_low_il_broker` field | **VERIFIED** | `Profile` struct field, line 2391, `#[serde(default)] pub windows_low_il_broker: bool` | Exact line match. |
| `crates/nono-cli/src/profile/mod.rs:2398` — `windows_interpreters` field | **VERIFIED** | `Profile` struct field, line 2398 | Exact line match. |
| Second declaration site `~2468-2475` | **VERIFIED** | `ProfileDeserialize` struct: `windows_low_il_broker` at 2468, `windows_interpreters` at 2475 | Confirmed distinct struct with its own serde handling and its own doc comment about `deny_unknown_fields` round-tripping, exactly as D-09 describes. A **third** site also needs updating for full round-trip parity: `impl From<ProfileDeserialize> for Profile` at line ~2518/2523 (`raw.windows_low_il_broker`/`raw.windows_interpreters`) — this wasn't separately named in CONTEXT.md but is load-bearing; omitting it means the `ProfileDeserialize→Profile` conversion silently drops the field even if both structs declare it. |
| `crates/nono-cli/src/capability_ext.rs` — `$VAR`/`@git:*` expansion lands here | **VERIFIED** | `impl CapabilitySetExt for CapabilitySet` in `capability_ext.rs`, all `fs.allow`/`fs.read`/etc. loops call `expand_vars(path_template, workdir)` today | Confirmed. Upstream's `2cbaa9a0` wraps this with a new local `expand_path(template, workdir)` helper — **naming collision risk**: `crate::policy::expand_path(path_str: &str) -> Result<PathBuf>` already exists at `policy.rs:268` with a *different* signature (no `workdir` param). The two are not directly ambiguous (module-scoped), but the identical name for two different functions is a real footgun for future readers; recommend the planner name the new local fn something distinct (e.g. `expand_profile_path`). |
| `crates/nono/src/capability.rs` — `CapabilitySet`; gains `localhost_port_ranges` + `merge_port_ranges` | **VERIFIED** | `CapabilitySet` struct at line 874, `#[derive(Debug, Clone, Default)]`, `localhost_ports: Vec<u16>` at line 894 | `merge_port_ranges`/`MACOS_PORT_RANGE_LIMIT` are wholly new (no fork equivalent) — confirmed absent via grep. Struct derives `Default`, so the new field is additive and does **not** force an exhaustive-literal update anywhere (unlike `Profile`/`NetworkConfig`, see below) — materially lower regression risk. |
| `crates/nono/src/sandbox/{macos.rs,linux.rs}` — cfg-gated Unix, trigger D-12 | **VERIFIED** | Both files exist; macOS emitter at `generate_profile`/`push_localhost_tcp_outbound_seatbelt_rules`; Linux at the Landlock ruleset-build loop in `apply_with_abi_inner` | Both currently implement only single-port `localhost_ports` handling; upstream's diff is a clean additive extension in both. |
| `crates/nono-cli/src/exec_strategy/supervisor_linux.rs` — cfg-gated Unix, trigger D-12 | **VERIFIED** | `decide_network_notification`, `SYS_BIND` arm, `SupervisorConfig.proxy_bind_ports` | Confirmed `proxy_bind_ports: Vec<u16>` exists at `exec_strategy.rs:413` and is populated from `caps.network_mode()`'s `ProxyOnly{bind_ports}` in `supervised_runtime.rs:373` (Linux-only, seccomp-notify proxy-only fallback — a **third, narrower** context distinct from the WFP path below). Upstream's diff adds a `proxy_bind_port_ranges: Vec<(u16,u16)>` sibling field, populated in `supervised_runtime.rs` from `caps.localhost_port_ranges().to_vec()` — this is a clean 2-line addition once `capability.rs`'s field exists. |
| `crates/nono-cli/data/policy.json` — `bun`/`mise` presets | **VERIFIED** | 1114-line file; existing `go-dev`/`python-dev` presets follow the exact shape upstream's `bun-dev`/`mise-dev` use | `crates/nono-cli/tests/manifest_roundtrip.rs:709` has the `AVAILABLE_GROUPS` const array upstream's diff extends — confirmed present, trivial one-line-per-commit addition. |
| `crates/nono/schema/capability-manifest.schema.json`, `crates/nono-cli/data/nono-profile.schema.json` | **VERIFIED**, both exist | `PortConfig` definition at `capability-manifest.schema.json:177` (`additionalProperties: true`); `nono-profile.schema.json` top-level has `additionalProperties: false` | **Important, previously-unstated finding**: `crates/nono/src/manifest.rs` types are **typify-generated at build time** from `capability-manifest.schema.json` (`include!(concat!(env!("OUT_DIR"), "/capability_manifest_types.rs"))`) — there is no hand-written `manifest::PortConfig` struct to edit; adding `localhost_range` to the JSON schema is sufficient and the Rust type regenerates on next `cargo build`. Separately, `nono-profile.schema.json`'s **strict** `additionalProperties: false` means `platform_overrides`, `open_port_range`, and `listen_port_range` MUST be added there too, or `validate_against_schema()` (exercised by `test_schema_validates_builtin_profiles_in_policy_json` and profile-load-time validation) will reject any profile using the new fields — this is a **third schema/struct site** (serde struct, JSON schema, typify-schema) that must stay in sync, generalizing D-09's two-site warning. |
| "WFP enforcement is service-only / daemon-path only (`nono agent launch`)" (project memory) | **PARTIALLY WRONG — corrected** | See body below | The narrow, hardcoded-single-port `wfp_filter_add()` in `crates/nono-cli/src/agent_daemon/launch.rs` (session-SID-scoped, AI-agent multi-tenant daemon) is real and daemon-path-only, as memory says. But there is a **second, general, non-daemon** path: `crates/nono/src/sandbox/windows.rs::compile_network_policy(caps)` → `crates/nono-cli/src/exec_strategy_windows/network.rs::prepare_network_enforcement()`, called from the ordinary Windows supervised-exec dispatch (`exec_strategy_windows/mod.rs:484`) for a plain `nono run`. This general path already reads `caps.localhost_ports()` today and is the correct integration point for `caps.localhost_port_ranges()`. This is a genuine correction, not a nitpick — planning the Windows emitter as daemon-only would both overscope (build unneeded multi-tenant SID plumbing) and underscope (miss the actually-reachable general path). |

## Standard Stack

No new external dependencies. All 7 commits verified `git show --stat` to touch **zero**
`Cargo.toml`/`Cargo.lock` files. `windows-sys 0.59` (already pinned in
`crates/nono-cli/Cargo.toml:157` with the `Win32_NetworkManagement_WindowsFilteringPlatform` and
`Win32_Security` features already enabled) already exposes `FWP_MATCH_RANGE`, `FWP_RANGE0`, and
`FWP_VALUE0` — confirmed present in the vendored crate source at
`windows-sys-0.59.0/src/Windows/Win32/NetworkManagement/WindowsFilteringPlatform/mod.rs:926,2374-2410`.
No `cargo add` / new package needed for the Windows emitter.

### Installation

No installation step. Package Legitimacy Audit is N/A for this phase — see below.

## Package Legitimacy Audit

**N/A — this phase installs no external packages.** All 7 in-scope commits are pure source-tree
diffs (Rust `.rs`, JSON `data/policy.json`, JSON Schema, one `.md`), independently confirmed via
`git show --stat <sha>` on each of the 7 SHAs: zero `Cargo.toml`/`Cargo.lock` hunks in any of them.
The `slopcheck` gate and registry-verification steps are skipped per the package-legitimacy
protocol's scope condition (no packages recommended, none to audit).

## Architecture Patterns

### System Architecture Diagram

```
Profile JSON (disk)
     |
     v
parse_profile_file / parse_profile_bytes   (deny_unknown_fields structs: Profile, ProfileDeserialize)
     |
     v
resolve_extends(profile, ...)              <-- platform_overrides must SURVIVE this (D-10 / #1380)
     |
     v
finalize_profile(profile)
     |  1. apply_platform_overrides(profile)   <-- NEW: merges the current-OS PlatformOverride block
     |  2. merge_implicit_default_groups(...)
     |  3. (existing) validate_profile_no_proxy(...)
     v
Resolved Profile
     |
     +--> CapabilitySet::from_profile()  (capability_ext.rs)
     |        |  for each fs.allow/read/write/... entry:
     |        |    1. expand_dynamic_tokens(entries, Some(workdir))   <-- NEW: @git:* (ported module)
     |        |    2. expand_path(template, workdir)                  <-- NEW: $VAR (policy::expand_env_vars)
     |        |                                                            then profile::expand_vars ($HOME, $WORKDIR, ...)
     |        v
     |     CapabilitySet (fs caps + localhost_ports + localhost_port_ranges[NEW])
     |
     +--> profile.network.{open_port_range,listen_port_range}  -->  profile_runtime.rs validation
                                                                     (start<=end, macOS cumulative-limit check)
                                                                          |
                                                                          v
CapabilitySet.localhost_port_ranges()
     |
     +---------------------+---------------------------+
     v                     v                           v
sandbox/macos.rs      sandbox/linux.rs      sandbox/windows.rs::compile_network_policy
(Seatbelt: unroll      (Landlock: NetPort    (produces WindowsNetworkPolicy; NEW field
 to per-port rules,     per port, per         localhost_port_ranges — NOT unrolled)
 capped at 16,384)      range, no cap)              |
                                                     v
                                       exec_strategy_windows/network.rs
                                       ::build_wfp_runtime_activation_request
                                                     |
                                                     v
                                       WfpRuntimeActivationRequest (IPC, windows_wfp_contract.rs)
                                                     |
                                                     v
                                       nono-wfp-service.rs::build_policy_filter_specs
                                       PortCondition::RemoteRange/LocalRange [NEW]
                                                     |
                                                     v
                                       add_policy_filter(): FWP_MATCH_RANGE + FWP_RANGE0
                                       (FWPM_LAYER_ALE_AUTH_CONNECT_V4/V6,
                                        FWPM_LAYER_ALE_AUTH_RECV_ACCEPT_V4/V6)
                                                     |
                                                     v
                                       FwpmFilterAdd0 (kernel, live WFP engine — host-gated)
```

### Recommended Wave Structure

```
Wave A (PROF-01): platform_overrides + extends preservation
  - crates/nono-cli/src/profile/mod.rs: PlatformOverrides/PlatformOverride types,
    apply_platform_overrides, merge_platform_overrides/merge_platform_override_slot,
    3-site windows_low_il_broker/windows_interpreters migration + back-compat aliasing
  - crates/nono-cli/data/nono-profile.schema.json: add platform_overrides (additionalProperties:false)

Wave B (PROF-02): $VAR + ported @git:* tokens
  - crates/nono-cli/src/policy.rs: substitute_vars/expand_env_vars (new, generic)
  - crates/nono-cli/src/capability_ext.rs: expand_path wrapper (RENAME to avoid policy::expand_path collision),
    expand_dynamic_tokens call per fs.* list, cfg-gated fallback per D-01
  - NEW fork-owned module (planner's choice of name/location, e.g. capability_ext/dynamic_tokens.rs):
    ported dynamic_providers.rs (parse_token, dispatch_token, git submodule, expand_dynamic_tokens)
  - SKIP: wiring.rs / proxy_runtime.rs consumer sites (absent subsystems in this fork — see Fork-Presence table)

Wave C (PROF-03): port ranges — capability.rs mechanism + 3 platform emitters
  - crates/nono/src/capability.rs: MACOS_PORT_RANGE_LIMIT, merge_port_ranges, localhost_port_ranges field
    + allow_localhost_port_range/add_localhost_port_range/localhost_port_ranges()
  - crates/nono/src/sandbox/macos.rs, linux.rs: extend existing emit loops (additive)
  - crates/nono-cli/src/exec_strategy/supervisor_linux.rs: proxy_bind_port_ranges (seccomp fallback path)
  - crates/nono-cli/src/profile/mod.rs (NetworkConfig): open_port_range/listen_port_range fields,
    3rd exhaustive-literal site in merge_profiles
  - crates/nono-cli/src/profile_runtime.rs: range validation (start<=end, macOS cumulative limit)
  - crates/nono/schema/capability-manifest.schema.json: PortConfig.localhost_range (typify regenerates)
  - crates/nono-cli/data/nono-profile.schema.json: network.open_port_range/listen_port_range
  - crates/nono/src/manifest_convert.rs: localhost_range -> allow_localhost_port_range
  - crates/nono-cli/src/output.rs, profile_cmd.rs: display-only additions
  - Windows emitter (fork-original, its own sub-wave given size/risk):
    - crates/nono/src/sandbox/windows.rs::compile_network_policy: read caps.localhost_port_ranges()
    - crates/nono-cli/src/windows_wfp_contract.rs: WfpRuntimeActivationRequest gains localhost_port_ranges
    - crates/nono-cli/src/exec_strategy_windows/network.rs: thread ranges through build_wfp_*_request fns
    - crates/nono-cli/src/bin/nono-wfp-service.rs: PortCondition::RemoteRange/LocalRange,
      FWP_MATCH_RANGE + FWP_RANGE0 in add_policy_filter, spec-building loop extension

Wave D (PROF-04): bun/mise presets
  - crates/nono-cli/data/policy.json: bun_runtime/mise_manager groups + bun-dev/mise-dev profiles
  - crates/nono-cli/tests/manifest_roundtrip.rs: AVAILABLE_GROUPS additions
```

### Pattern 1: Exhaustive struct literals as a compile-forced safety net

**What:** `Profile` (in `merge_profiles`, `crates/nono-cli/src/profile/mod.rs:3262`) and
`ProfileDeserialize`/`NetworkConfig` are built via full field-by-field struct literals with no
`..Default::default()` tail. `CapabilitySet` (`crates/nono/src/capability.rs:874`), by contrast,
derives `Default` and is built through `Self::default()`/builder methods.

**When to use:** When adding `platform_overrides` to `Profile` or `open_port_range`/
`listen_port_range` to `NetworkConfig`, the compiler will refuse to build until every literal
(`merge_profiles`, the two test-helper `Profile { .. }` literals at ~5391/5485, and any other
exhaustive literal) is updated — this is the concrete mechanism that prevents D-10's "silently
dropped" failure class for `Profile`-level fields. It does **not** apply the same way to
`CapabilitySet` — `localhost_port_ranges` is safe to add without hunting every construction site,
but that also means a forgotten "does an emitter consume it" wiring bug would NOT be caught by the
compiler the way a `Profile` field omission would.

**Example:**
```rust
// crates/nono-cli/src/profile/mod.rs:3262 (existing fork code, exhaustive literal)
fn merge_profiles(base: Profile, child: Profile) -> Profile {
    Profile {
        extends: None,
        // platform_overrides: <-- compiler forces this line to exist once the field is added
        meta: child.meta,
        ...
    }
}
```

### Pattern 2: Schema-driven codegen (typify) for `manifest.rs`

**What:** `crates/nono/src/manifest.rs` types are generated at build time from
`crates/nono/schema/capability-manifest.schema.json` via `typify`, included through
`include!(concat!(env!("OUT_DIR"), "/capability_manifest_types.rs"))`.

**When to use:** Any change to `PortConfig` (e.g. adding `localhost_range`) is made in the JSON
Schema file only — never hand-write a matching Rust struct. `cargo build` regenerates the type;
downstream consumers (`manifest_convert.rs`, `profile_cmd.rs::resolve_to_manifest`) reference the
generated field names directly (e.g. `manifest::PortConfig { localhost_range: vec![...] }`).

**Example:**
```rust
// Source: crates/nono/src/manifest.rs:18-28 (verified fork code)
#[allow(clippy::derivable_impls, clippy::incompatible_msrv, clippy::unwrap_used)]
mod generated {
    include!(concat!(env!("OUT_DIR"), "/capability_manifest_types.rs"));
}
pub use generated::*;
pub use NonoCapabilityManifest as CapabilityManifest;
```

### Pattern 3: WFP filter-spec construction is already a pure, unit-testable builder

**What:** `crates/nono-cli/src/bin/nono-wfp-service.rs::build_policy_filter_specs(request, ...)
-> Vec<PolicyFilterSpec>` is a pure function over the (de)serializable `WfpRuntimeActivationRequest`
— it does not touch the live WFP engine. `add_policy_filter(engine, spec, ...)` is the only piece
that calls `FwpmFilterAdd0` against a real kernel handle.

**When to use:** Add `PortCondition::RemoteRange(u16, u16)` / `PortCondition::LocalRange(u16, u16)`
variants; extend `build_policy_filter_specs`'s per-port loops to also emit one spec per range entry
(no per-port unrolling — D-06); extend `add_policy_filter`'s `match port { ... }` arm to build an
`FWP_RANGE0`-backed condition with `matchType: FWP_MATCH_RANGE` for the new variants. This keeps
the spec-construction logic (fully unit-testable, no admin/kernel needed) separate from the actual
kernel call (host-gated), matching the existing test pattern at
`nono-wfp-service.rs:1829-1936` (`localhost_ports: vec![8080]` fixtures).

**Example:**
```rust
// Source: crates/nono-cli/src/bin/nono-wfp-service.rs:1089-1092, :1461-1474 (verified fork code)
enum PortCondition {
    Remote(u16),
    Local(u16),
    // NEW:
    // RemoteRange(u16, u16),
    // LocalRange(u16, u16),
}
// ...
if let Some(port) = spec.port {
    let (field_key, value) = match port {
        PortCondition::Remote(value) => (FWPM_CONDITION_IP_REMOTE_PORT, value),
        PortCondition::Local(value) => (FWPM_CONDITION_IP_LOCAL_PORT, value),
    };
    conditions.push(FWPM_FILTER_CONDITION0 {
        fieldKey: field_key,
        matchType: FWP_MATCH_EQUAL,
        conditionValue: FWP_CONDITION_VALUE0 { r#type: FWP_UINT16, Anonymous: FWP_CONDITION_VALUE0_0 { uint16: value } },
    });
}
// NEW arm (sketch, not yet in fork):
// PortCondition::RemoteRange(lo, hi) => build FWP_RANGE0 { valueLow, valueHigh } (each FWP_VALUE0{ type: FWP_UINT16, ... }),
//   matchType: FWP_MATCH_RANGE, conditionValue.rangeValue = &mut range
```

### Anti-Patterns to Avoid

- **Don't propagate `MACOS_PORT_RANGE_LIMIT` to the Windows emitter.** D-06 is explicit and the
  code confirms why: the macOS limit exists solely because Seatbelt unrolls each port into its own
  rule (`sandbox_init()` SIGILLs above ~17,770 rules); WFP's `FWP_MATCH_RANGE` needs exactly one
  filter object per range regardless of width.
- **Don't build a new session-SID/multi-tenant daemon plumbing path for the Windows emitter.** The
  general `compile_network_policy` → `prepare_network_enforcement` path already exists and is
  reachable from plain `nono run`; extending the narrow `agent_daemon/launch.rs::wfp_filter_add`
  path is optional follow-on work at best, not required for SC3.
- **Don't hand-write a `manifest::PortConfig` Rust struct.** It's typify-generated; edit the JSON
  Schema.
- **Don't port `tool-sandbox/platform/{linux,macos}.rs`'s `add_policy_fs` changes.** Those are
  consumers of `expand_dynamic_tokens` *inside* the tool-sandbox subsystem, which the fork does not
  have (v3.7 scope). Only the standalone `dynamic_providers.rs` module is in scope for PROF-02.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| `$VAR`-style token substitution with a custom parser | A bespoke regex/split-based scanner | Upstream's `substitute_vars<E>(s, var_start, lookup)` (from `2cbaa9a0`, ported into `policy.rs`) | Already handles `$`-escaping (bare `$` and non-identifier-start chars pass through unchanged), is generic over the lookup closure (reused for both env-var and wiring-var expansion upstream), and ships with upstream's own test coverage to port alongside it. |
| Merging overlapping/adjacent `(u16,u16)` port ranges before rule expansion | A custom interval-merge loop | `merge_port_ranges` (from `d5803b99`, `crates/nono/src/capability.rs`) | Already correct (sort + linear merge with `saturating_add(1)` adjacency check), already unit-tested for 6 edge cases (empty, no-overlap, overlapping, adjacent, contained, unsorted, 3-way chain) — porting these tests verbatim gives free regression coverage. |
| WFP range-match filter construction | Manual `FWP_CONDITION_VALUE0` union poking without checking existing bindings | `windows-sys 0.59`'s `FWP_MATCH_RANGE`/`FWP_RANGE0`/`FWP_VALUE0` (already a transitive dependency, already feature-enabled) | No new crate or vendored binding needed; the union layout is already correct in the pinned version. |

**Key insight:** every piece of "port range" logic upstream needed (parsing, merging, per-platform
emission) already exists in a single commit that the fork can port near-verbatim for macOS/Linux;
the only genuinely novel work is the Windows kernel-facing emitter, and even that reuses an
existing, already-tested filter-spec builder rather than starting from a blank WFP integration.

## Common Pitfalls

### Pitfall 1: Treating `expand_path` naming collision as harmless
**What goes wrong:** A new local `fn expand_path(template: &str, workdir: &Path)` in
`capability_ext.rs` shadows nothing at compile time (different module, different signature from
`crate::policy::expand_path(path_str: &str)`), but is a maintenance trap — a future reader or an
IDE auto-import can silently call the wrong one if either function's signature changes to become
compatible.
**Why it happens:** Upstream's own diff introduces this exact name collision; it compiles clean in
upstream because Rust scopes function names per-module.
**How to avoid:** Name the fork's version distinctly (e.g. `expand_profile_path`).
**Warning signs:** `cargo doc` showing two `expand_path` functions with no visible module
qualification in search results.

### Pitfall 2: Forgetting the 3-way `windows_low_il_broker`/`windows_interpreters` migration
**What goes wrong:** D-09 names two declaration sites (`Profile`, `ProfileDeserialize`), but there
is a third: the `impl From<ProfileDeserialize> for Profile` conversion (~line 2518-2523) that
copies `raw.windows_low_il_broker`/`raw.windows_interpreters` field-by-field. Migrating only the
two struct declarations but not this conversion means the parsed value never reaches the resolved
`Profile`, even though both structs "have" the field.
**Why it happens:** The conversion function is exhaustive (rustc enforces it, per the doc comments
already in the file: "Exhaustively enumerated here so rustc's struct-literal completeness check
catches any future field additions") — so it WILL fail to compile if a field is added to one struct
and not mirrored in the other two, which is a safety net, but only if the planner adds the new
`platform_overrides`-derived back-compat field to all three, not just two.
**How to avoid:** Grep all three sites (`Profile` struct, `ProfileDeserialize` struct, the `From`
impl) before considering the migration complete; the compiler will catch a missing field in the
`From` impl's own literal (it's `Self { ..raw.field... }` fully enumerated) as a build error, so
this is lower-risk than it sounds — but still worth calling out explicitly in the plan's
verification steps.

### Pitfall 3: Assuming the fork's `wiring.rs`/`proxy_runtime.rs` mirror upstream's `2cbaa9a0` shape
**What goes wrong:** Upstream's `$VAR` commit touches `wiring.rs` (refactoring an existing
`expand_vars(template, ctx: &WiringContext)` pack-installer function) and `proxy_runtime.rs`
(expanding `$VAR` in `ProxyCredentialCaptureBackend`'s captured-credential command args). Neither
function/type exists in the fork: `crates/nono-cli/src/wiring.rs` has no `expand_vars` function at
all (fork's wiring.rs is a different, smaller 622-line file focused on YAML-merge directives, not
pack installation with variable substitution); `crates/nono-cli/src/proxy_runtime.rs` has no
`resolve_capture_command`/`ProxyCredentialCaptureBackend`/`ResolvedCredentialCaptureEntry` at all —
confirmed absent via whole-crate grep.
**Why it happens:** Same class of error CONTEXT.md's `<specifics>` section warns about seven times
over — upstream's file list describing what upstream's commit touches says nothing about whether
the fork has the same subsystem.
**How to avoid:** Scope PROF-02's `$VAR` work to `capability_ext.rs` + `policy.rs` only. Do not plan
tasks against `wiring.rs` or `proxy_runtime.rs` for this commit.
**Warning signs:** A task description referencing "refactor `expand_vars` in `wiring.rs`" or
"credential-capture command expansion in `proxy_runtime.rs`" for PROF-02 — both are N/A findings,
not omissions.

### Pitfall 4: Missing the third schema-sync site (`nono-profile.schema.json`)
**What goes wrong:** `nono-profile.schema.json` has strict `additionalProperties: false` at its top
level. Adding `platform_overrides`/`open_port_range`/`listen_port_range` to the `Profile` Rust
struct without adding matching properties to this schema file makes
`test_schema_validates_builtin_profiles_in_policy_json` (and any runtime profile-schema validation
call) reject profiles using the new fields, even though `serde` happily parses them.
**Why it happens:** There are effectively three parallel "profile shape" definitions in this
codebase (the `Profile`/`ProfileDeserialize` serde structs, the JSON Schema file, and — for the
unrelated `CapabilityManifest` — a typify-generated schema) and only the serde structs are directly
type-checked by the Rust compiler.
**How to avoid:** Treat `nono-profile.schema.json` as a mandatory companion edit for every new
`Profile`/`NetworkConfig` field in this phase, verified via the existing
`test_schema_validates_builtin_profiles_in_policy_json` test (which already iterates every built-in
profile — extending it with a hand-written test profile exercising the new fields would close the
gap for user-authored profiles too, since that test only covers `policy.json`'s own profiles).
**Warning signs:** `cargo test -p nono-cli` failures in schema-validation tests only after the
serde-level tests already pass.

## Code Examples

### `apply_platform_overrides` merge-order (verified upstream pattern, `ae1c513e` + `719975cf`)
```rust
// Source: git show ae1c513e -- crates/nono-cli/src/profile/mod.rs
pub(crate) fn finalize_profile(mut profile: Profile) -> Result<Profile> {
    profile = apply_platform_overrides(profile)?;   // NEW: must run before validators that need merged values
    // ... 719975cf adds: re-validate custom_credentials / env_credentials / set_vars HERE,
    //     because apply_platform_overrides can introduce values the pre-merge validators never saw.
    merge_implicit_default_groups(&mut profile)?;
    // ... existing fork validators (validate_profile_no_proxy, etc.)
    Ok(profile)
}
```

### `merge_port_ranges` (verified upstream, `d5803b99` — pure, no fork changes needed to the algorithm itself)
```rust
// Source: git show d5803b99 -- crates/nono/src/capability.rs
pub fn merge_port_ranges(ranges: &[(u16, u16)]) -> Vec<(u16, u16)> {
    if ranges.is_empty() { return Vec::new(); }
    let mut sorted = ranges.to_vec();
    sorted.sort_unstable_by_key(|&(s, _)| s);
    let mut merged: Vec<(u16, u16)> = Vec::with_capacity(sorted.len());
    let (mut cur_start, mut cur_end) = sorted[0];
    for &(start, end) in &sorted[1..] {
        if start <= cur_end.saturating_add(1) {
            if end > cur_end { cur_end = end; }
        } else {
            merged.push((cur_start, cur_end));
            cur_start = start;
            cur_end = end;
        }
    }
    merged.push((cur_start, cur_end));
    merged
}
```

### WFP filter layers already in use (verified fork code, extend — don't replace)
```rust
// Source: crates/nono-cli/src/bin/nono-wfp-service.rs:1105-1128 (verified)
fn build_wfp_layer_specs() -> [WfpLayerSpec; 4] {
    [
        WfpLayerSpec { key: FWPM_LAYER_ALE_AUTH_CONNECT_V4, label: "connect-v4", rule_name: "outbound" },
        WfpLayerSpec { key: FWPM_LAYER_ALE_AUTH_CONNECT_V6, label: "connect-v6", rule_name: "outbound" },
        WfpLayerSpec { key: FWPM_LAYER_ALE_AUTH_RECV_ACCEPT_V4, label: "recv-accept-v4", rule_name: "inbound" },
        WfpLayerSpec { key: FWPM_LAYER_ALE_AUTH_RECV_ACCEPT_V6, label: "recv-accept-v6", rule_name: "inbound" },
    ]
}
```
This matches D-06's cited layers exactly (`FWPM_LAYER_ALE_AUTH_CONNECT_V4`/`V6`); no new layer
registration is needed for the range emitter, only new condition construction within the existing
per-layer loop.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|---------------|--------|
| Single-port `localhost_ports: Vec<u16>` allowlist, unrolled per-port on every platform | `localhost_port_ranges: Vec<(u16,u16)>` alongside it; unrolled on macOS/Linux, native-range on Windows | This phase (upstream `d5803b99`, absorbed here) | Profiles needing wide port ranges (e.g. ephemeral dev-server ranges 3000-3999) no longer need thousands of discrete entries; Windows gets a materially more efficient kernel representation than the two Unix platforms. |
| Fixed `~/.gitconfig`-style literal paths for git-adjacent grants | `@git:*` dynamic-provider tokens resolved at launch time, workdir-aware | Introduced upstream pre-window (tool-sandbox, v0.65.0) for per-command sandboxes; this phase extends it to top-level `filesystem.allow`/`read`/etc. lists | Profiles can grant "whatever the user's actual git config needs" without enumerating every possible per-user dotfile location. |

**Deprecated/outdated:** None — this phase adds capability, it does not remove any existing
mechanism. The `windows_low_il_broker`/`windows_interpreters` top-level flags are explicitly kept
(D-08: both forms accepted, no deprecation warning this milestone).

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | The planner's chosen name/location for the ported `dynamic_providers.rs` module will not collide with any existing fork module name. | Architecture Patterns / Wave B | Low — this is Claude's Discretion per CONTEXT.md; a collision would surface as a compile error immediately, not a silent bug. |
| A2 | `test_schema_validates_builtin_profiles_in_policy_json` is the only schema-validation test that would catch a missed `nono-profile.schema.json` update, and it only covers `policy.json`'s own built-in profiles, not arbitrary user profiles. | Common Pitfalls / Pitfall 4 | Medium — if a broader schema-validation integration test exists elsewhere and was not found by this research's grep pass, the "must add a new test" recommendation may be redundant (harmless) rather than necessary. |
| A3 | No live WFP kernel test is feasible without Administrator privileges and a running `nono-wfp-service`; this matches the existing test posture (`SKIP_HOST_UNAVAILABLE` pattern seen elsewhere in this milestone) and is not a new constraint introduced by this phase. | Validation Architecture | Low — consistent with extensively-documented project memory (`wfp_confined_egress_and_daemon_gate`) and the existing unit-test-only pattern already present in `nono-wfp-service.rs`'s own test module. |

**If this table is empty:** N/A — see entries above. All three are low-to-medium risk and none
block planning; they are flagged for the planner's awareness, not as blocking unknowns.

## Open Questions

1. **Should the general Windows WFP path (`compile_network_policy`) or the daemon-only path
   (`agent_daemon/launch.rs::wfp_filter_add`) be the primary integration point for SC3, or both?**
   - What we know: the general path is reachable from plain `nono run` today and already reads
     `caps.localhost_ports()`; the daemon path is narrower and currently hardcodes a single port.
   - What's unclear: whether product intent wants port-range profiles to work under
     `nono agent launch` (multi-tenant AppContainer) too, which would need the narrower path
     extended as well, doubling the Windows-side surface.
   - Recommendation: plan the general path as the required SC3 deliverable (it is sufficient to
     satisfy "WFP-native remote-port-range emitter on Windows" as literally stated); treat daemon-path
     range support as optional/discretionary follow-on unless CONTEXT.md's `platform_overrides.windows`
     or a future decision explicitly calls for agent-launch port ranges.

2. **Does `nono-profile.schema.json` need a companion round-trip test for user-authored (not
   built-in) profiles using the new fields, or is schema-conformance of the 2 new built-in
   `bun-dev`/`mise-dev` profiles (which don't use `platform_overrides`/port ranges) sufficient
   coverage?**
   - What we know: `test_schema_validates_builtin_profiles_in_policy_json` only iterates
     `policy.json`'s `profiles` object; none of the existing built-ins use `platform_overrides` or
     port ranges, so this test alone would not catch a schema/struct desync for those two features.
   - What's unclear: whether existing profile-parsing tests elsewhere in `profile/mod.rs`'s test
     module already independently exercise `validate_against_schema` against ad-hoc JSON fixtures
     for new fields (a pattern seen at `test_full_profile_with_canonical_bypass_protection...`-style
     tests nearby).
   - Recommendation: the planner should add at least one hand-written schema-validation test per new
     top-level field (`platform_overrides`, `open_port_range`, `listen_port_range`) mirroring the
     existing ad-hoc-JSON test pattern already present in the file, rather than relying solely on the
     built-in-profiles iteration test.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `windows-sys` (`Win32_NetworkManagement_WindowsFilteringPlatform`, `Win32_Security` features) | Windows WFP range emitter | Yes (already a pinned dependency) | 0.59.0 | — |
| Docker + `cross` (linux-gnu cross-target clippy) | D-12 mandatory gate | Confirmed available on this dev host per `.planning/templates/cross-target-verify-checklist.md` (established in Phase 96) | `cross` 0.2.5, image `ghcr.io/cross-rs/x86_64-unknown-linux-gnu:0.2.5@sha256:9e5b39c0...` | — |
| `zig` + `cargo-zigbuild` (apple-darwin cross-target clippy) | D-12 mandatory gate | Confirmed available per the same checklist | zig 0.16.0, cargo-zigbuild 0.23.0 | — |
| Administrator privileges + live `nono-wfp-service` | Live kernel FWP filter-add verification (SC3's kernel arm) | Not available in this research/planning session (and not expected to be, per project's documented WFP test posture) | — | Unit-test the filter-spec-construction functions (`build_policy_filter_specs`, the new `PortCondition` arms) without touching the live engine; document the live-kernel arm as `checkpoint:human-verify` / host-gated, consistent with existing project posture. |

**Missing dependencies with no fallback:** None — the one host-gated dependency (live WFP kernel
access) has a documented, already-used fallback (pure-function unit testing of spec construction).

**Missing dependencies with fallback:** Administrator/live-kernel WFP verification (see above).

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test runner (`cargo test`), `proptest` for property-based cases already used in `nono-cli` |
| Config file | none — standard `#[cfg(test)] mod tests` per source file, matching every file touched in this phase |
| Quick run command | `cargo test -p nono -p nono-cli --lib` |
| Full suite command | `make ci` (clippy + fmt + tests, per CLAUDE.md) |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| PROF-01 | `platform_overrides` parses, applies for current-OS block only, survives `extends` resolution, `windows_low_il_broker`/`windows_interpreters` both forms accepted with new-form-wins precedence | unit | `cargo test -p nono-cli profile::mod::tests -- platform_overrides` | ✅ Wave A (port upstream's own `platform_overrides_*` test suite from `ae1c513e`/`719975cf`, ~10 tests, directly reusable) |
| PROF-02a | `$VAR` expands from process env in `filesystem.allow`/`read`/`write`/etc. | unit | `cargo test -p nono-cli capability_ext::tests -- expand_env_var` | ✅ Wave B (port `test_profile_fs_allow_expands_env_var` from `2cbaa9a0`) |
| PROF-02b | `@git:*` tokens expand in top-level filesystem lists on all 3 platforms (cfg-gated fallback on non-Unix) | unit | `cargo test -p nono-cli -- dynamic_tokens` (name per planner's module choice) | ✅ Wave B (port `dynamic_providers.rs`'s existing ~35-test suite wholesale) |
| PROF-03a | `merge_port_ranges` correctness (empty/no-overlap/overlap/adjacent/contained/unsorted/3-way) | unit | `cargo test -p nono capability::tests -- merge_port_ranges` | ✅ Wave C (port upstream's 6 tests verbatim) |
| PROF-03b | macOS emits per-port rules, cumulative-limit error above 16,384 | unit | `cargo test -p nono sandbox::macos::tests -- port_range` | ✅ Wave C (port upstream's 6 macOS tests) |
| PROF-03c | Linux Landlock adds `NetPort` rules per range, no cap | unit | `cargo test -p nono sandbox::linux::tests -- port_range` | ❌ Wave C must add — upstream's diff extends the ruleset-build loop but did not add a dedicated unit test in `linux.rs` itself (confirmed via diff: only `macos.rs`/`supervisor_linux.rs` gained new `#[test]` blocks) |
| PROF-03d | Windows filter-spec construction emits `FWP_MATCH_RANGE` condition for a range entry (no per-port unroll) | unit | `cargo test -p nono-cli --bin nono-wfp-service -- port_range` (or wherever the test module lives once added) | ❌ Wave C must add — fork-original code, no upstream test to port; must follow the existing `localhost_ports: vec![8080]`-style fixture pattern at `nono-wfp-service.rs:1829-1936` |
| PROF-03e | Live kernel `FwpmFilterAdd0` accepts a range filter and it is actually enforced | manual / host-gated | none automatable from this dev host | N/A — requires Administrator + live `nono-wfp-service`; document as `checkpoint:human-verify` |
| PROF-04 | `bun`/`mise` presets are schema-valid AND resolvable (group + profile both load without error) | unit | `cargo test -p nono-cli -- manifest_roundtrip` plus `cargo test -p nono-cli profile::mod::tests -- test_load_builtin_profile` (extend to cover `bun-dev`/`mise-dev` by name, not just schema shape) | ❌ Wave D should add an explicit "load and resolve `bun-dev`/`mise-dev` by name" test — the existing `test_schema_validates_builtin_profiles_in_policy_json` only proves schema conformance, not resolvability, matching D-11's explicit "resolvable, not merely present" requirement |

### Sampling Rate
- **Per task commit:** `cargo test -p nono -p nono-cli --lib` (fast, no live kernel/network)
- **Per wave merge:** `make ci` (clippy + fmt + full test suite)
- **Phase gate:** `make ci` green + both cross-target clippy gates green (D-12, mandatory — this
  phase touches `crates/nono/src/sandbox/macos.rs`, `crates/nono/src/sandbox/linux.rs`, and
  `crates/nono-cli/src/exec_strategy/supervisor_linux.rs`, all cfg-gated Unix files per the
  cross-target-verify-checklist scope) before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `crates/nono/src/sandbox/linux.rs` — no existing `#[test]` coverage for the new port-range
      Landlock rules; add alongside the ported macOS tests (PROF-03c)
- [ ] `crates/nono-cli/src/bin/nono-wfp-service.rs` — new `PortCondition::RemoteRange`/`LocalRange`
      construction needs fresh unit tests; no upstream precedent exists since this is fork-original
      code (PROF-03d)
- [ ] `crates/nono-cli/src/profile/mod.rs` or a schema-test module — add explicit
      schema-validation-plus-resolution tests for `platform_overrides`/`open_port_range`/
      `listen_port_range` beyond the built-in-profiles iteration test (Open Question 2)
- [ ] `crates/nono-cli/src/profile/mod.rs` — add "resolve `bun-dev`/`mise-dev` by name" tests
      distinct from schema-shape validation (PROF-04, D-11)

*(Framework itself is fully present — no new test-runner or fixture-infrastructure install needed.)*

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | Not touched by this phase. |
| V3 Session Management | no | Not touched by this phase. |
| V4 Access Control | yes | `platform_overrides` merge semantics are "child-wins, deny-lists union" — matches the fork's existing `merge_profiles` posture (an override can only tighten scope relative to base+extends, never silently loosen it, per `719975cf`'s own re-validation fix). |
| V5 Input Validation | yes | Port-range `start <= end` validation (upstream's `profile_runtime.rs` pattern, ported); macOS cumulative-port-count validation before Seatbelt profile generation; `@git:*` token dispatch returns an explicit error for unknown provider/query (fails closed on typos, matching upstream's own stated design goal). |
| V6 Cryptography | no | Not touched by this phase. |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| A platform-override block silently widening scope without the user noticing (e.g. an override that adds `filesystem.allow: ["/"]` for one OS) | Elevation of Privilege | Upstream's own design constrains override blocks to reject nested `extends`/`platform_overrides` (prevents a second inheritance system); merge semantics are additive-and-child-wins on a per-field basis, not a wholesale replace — port this behavior verbatim, do not simplify it to "child profile replaces base". |
| `@git:*` token resolution shelling out to `git` with attacker-influenced `workdir` | Tampering / Information Disclosure | Upstream's `parse_paths_from_stdout` already restricts git-config-derived paths to `global`/`system` scopes only, explicitly dropping `local`/`worktree` scope entries (a hostile per-repo `.git/config` cannot inject a path) — this restriction is already tested (`git_read_paths_excludes_per_repo_local_config_overrides`) and must be preserved unmodified when porting. |
| A WFP range filter accidentally matching a wider port span than intended due to `valueLow`/`valueHigh` ordering | Elevation of Privilege | `FWP_RANGE0.valueLow`/`valueHigh` must be constructed from the already-validated `(start, end)` tuple where `start <= end` is enforced upstream of the WFP layer (both in `capability.rs`'s `allow_localhost_port_range` zero-start check and `profile_runtime.rs`'s `start > end` rejection) — do not re-derive ordering inside the WFP service; trust the already-validated tuple. |
| macOS Seatbelt policy-compiler crash (SIGILL) from an oversized unrolled rule set | Denial of Service (self-inflicted, local) | Preserve `MACOS_PORT_RANGE_LIMIT` (2^14) exactly as upstream defines it, with the SIGILL rationale kept in the doc comment (per CONTEXT.md `<specifics>` — "a bare magic number invites removal"). |

## Sources

### Primary (HIGH confidence — direct repository inspection)
- `git show <sha>` (full diffs) for all 7 in-scope commits: `ae1c513e`, `719975cf`, `2cbaa9a0`,
  `d4927f95`, `d5803b99`, `b620ed8e`, `f016b2d5` — read in full, not summarized.
- Live fork HEAD (`milestone/v2.13-carryforward-closeout`) inspection via `grep`/`Read` of:
  `crates/nono-cli/src/profile/mod.rs`, `crates/nono-cli/src/capability_ext.rs`,
  `crates/nono-cli/src/policy.rs`, `crates/nono-cli/src/wiring.rs`,
  `crates/nono-cli/src/proxy_runtime.rs`, `crates/nono/src/capability.rs`,
  `crates/nono/src/sandbox/{macos.rs,linux.rs,windows.rs,mod.rs}`,
  `crates/nono-cli/src/exec_strategy.rs`, `crates/nono-cli/src/exec_strategy/supervisor_linux.rs`,
  `crates/nono-cli/src/supervised_runtime.rs`,
  `crates/nono-cli/src/exec_strategy_windows/network.rs`,
  `crates/nono-cli/src/windows_wfp_contract.rs`, `crates/nono-cli/src/bin/nono-wfp-service.rs`,
  `crates/nono-cli/src/agent_daemon/{launch.rs,mod.rs}`, `crates/nono/src/manifest.rs`,
  `crates/nono/src/manifest_convert.rs`, `crates/nono/schema/capability-manifest.schema.json`,
  `crates/nono-cli/data/nono-profile.schema.json`, `crates/nono-cli/data/policy.json`,
  `crates/nono-cli/tests/manifest_roundtrip.rs`, `crates/nono-cli/src/profile_cmd.rs`,
  `crates/nono-cli/src/output.rs`.
- `../nono-py/src/{policy.rs,sandboxed_exec.rs,windows_confined_run.rs}`,
  `../nono-ts/src/lib.rs` — binding-repo mirror check (D-13).
- `~/.cargo/registry/src/.../windows-sys-0.59.0/.../WindowsFilteringPlatform/mod.rs` — confirmed
  `FWP_MATCH_RANGE`, `FWP_RANGE0`, `FWP_VALUE0` definitions in the pinned dependency version.
- `.planning/phases/108-upst12-divergence-audit/108-DIVERGENCE-LEDGER.md` §"PROF Cluster —
  Per-Commit Table" — work-list authority, cross-checked against the live commits.
- `.planning/phases/110-profile-policy-absorb-platform-overrides/110-CONTEXT.md` — locked decisions
  D-01 through D-14, cross-verified against code.

### Secondary (MEDIUM confidence)
- `.planning/phases/108-upst12-divergence-audit/108-CONTEXT.md` — D-05/D-06/D-07 tool-sandbox
  fencing rationale, cross-checked against the `dynamic_providers.rs` self-containment finding.
- `.planning/phases/109-proxy-network-absorb/109-CONTEXT.md` — D-11 "verify by behavior, not name"
  precedent, applied here to the `wiring.rs`/`proxy_runtime.rs` absent-subsystem findings.
- `proj/ADR-86-library-boundary-convergence.md` — policy-free library boundary, applied to confirm
  `capability.rs`'s new mechanism (D-04) and `compile_network_policy`'s policy-free nature.

### Tertiary (LOW confidence)
- Project auto-memory (`wfp_confined_egress_and_daemon_gate`) — the "WFP is daemon-path only"
  framing was found to be an incomplete generalization (see Fork-Presence Verification table); the
  narrower daemon-only claim is TRUE for `agent_daemon/launch.rs::wfp_filter_add` specifically, but
  a second, general, non-daemon path also exists and is more directly relevant to this phase.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — zero new dependencies, confirmed via `git show --stat` on all 7 commits;
  `windows-sys` API surface confirmed present in the exact pinned version's vendored source.
- Architecture: HIGH — every cited file/function/line opened and read against live fork HEAD, not
  inferred from commit messages or upstream file lists.
- Pitfalls: HIGH — all four pitfalls are grounded in direct diff-vs-fork-code comparison, not
  speculation; two (the `wiring.rs`/`proxy_runtime.rs` absent-subsystem finding, and the general
  vs. daemon-path WFP correction) are genuine, previously-unrecorded corrections to prior framing.

**Research date:** 2026-07-30
**Valid until:** 30 days (stable, no external API dependency; re-verify if `upstream` remote is
re-fetched and these SHAs are rebased/squashed before Phase 110 executes).
