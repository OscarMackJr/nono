# Phase 117: Fail-Direction Contract + Startup Self-Attestation - Research

**Researched:** 2026-08-09
**Domain:** Windows kernel-composite confinement (restricted token / mandatory label / AppContainer /
DACL / WFP), suspended-process attestation, Rust struct-drop semantics, fault-injection seam design.
**Confidence:** MEDIUM-HIGH — the layer derivation and gate-point findings are grounded in direct
file:line citations verified by symbol (not file presence); a small number of items are genuinely
open and flagged `UNRESOLVED`.

## Summary

The five things that most change how this phase should be planned:

1. **There are at least THREE independent `CREATE_SUSPENDED` spawn-and-gate implementations, not
   one.** `crates/nono-cli/src/exec_strategy_windows/launch.rs` (`spawn_windows_child`,
   ~launch.rs:2075-2145), `crates/nono-shell-broker/src/main.rs` (~:536-661), and
   `crates/nono-cli/src/agent_daemon/launch.rs` (`launch_agent`, ~:600-860) each independently call
   `CreateProcess{W,AsUserW}` with `CREATE_SUSPENDED` and `ResumeThread`. The daemon module's own
   doc comment states this is deliberate: *"This module intentionally does NOT depend on
   `exec_strategy_windows/`... We inline the job/process creation using raw Windows APIs (the same
   calls `exec_strategy_windows` makes)"* (`agent_daemon/launch.rs:26-30`). D-21's "the gate" is
   therefore not a single code location — it is a **pattern that must be independently
   re-implemented (or newly shared) in three call sites**, which is exactly the DRAIN-02/DRAIN-03
   "hand-maintained list drifts from source of truth" failure class the milestone invariants warn
   about. This was not in CONTEXT.md's canonical_refs and should be the first thing the planner
   accounts for in wave-sizing.

2. **On the broker arm, nono-cli's own suspended-spawn window never sees the real confined child.**
   `WindowsSupervisedChild::Native` (`exec_strategy_windows/supervisor.rs:77-95`) wraps whichever
   process `CreateProcess{W,AsUserW}` returned in `launch.rs`. For `WindowsTokenArm::BrokerLaunch` /
   `BrokerLaunchNoPty`, that process is `nono-shell-broker.exe` itself (Medium-IL, unconfined) — the
   actual Low-IL / AppContainer shell child is spawned *inside the broker's own process*, a second,
   separate `CREATE_SUSPENDED` window (`nono-shell-broker/src/main.rs:536-661`). An attestation gate
   placed only at nono-cli's spawn site cannot observe the broker arm's real confinement; the probe
   must live inside `nono-shell-broker` too.

3. **WFP may already be effectively "confirmed" pre-spawn, not "unobservable"** — contradicting
   CONTEXT.md D-18's stated accepted-consequence premise. `install_wfp_network_backend` →
   `assert_wfp_activation_installed_filters` (`network.rs:1730`, defined `network.rs:1775-1800`)
   already aborts the whole launch (`NonoError::UnsupportedPlatform`) if the elevated
   `nono-wfp-service` reports `installed_filter_count == 0` or absent, for the *specific* session's
   package SID / policy, over the existing named-pipe IPC. This happens **before** `CREATE_SUSPENDED`
   even fires, inside `prepare_network_enforcement` (called from `prepare_live_windows_launch`,
   `mod.rs:484`). See §C and §G for the exact framing this changes.

4. **AppContainer is NOT categorically absent from "the broker arm"** as CONTEXT.md D-08 states —
   it is absent specifically from `WindowsTokenArm::BrokerLaunch` (the PTY arm) and present on
   `WindowsTokenArm::BrokerLaunchNoPty` (`ExecConfig.app_container_name` doc comment,
   `mod.rs:139-145`: *"Present on the `BrokerLaunchNoPty` (no-PTY supervised) arm; `None` on other
   arms"*). This is exactly the kind of matrix nuance D-08 asks the registry to carry — the correction
   itself is evidence D-08's per-arm-matrix requirement is load-bearing, not decorative.

5. **A shipped, reviewed precedent for D-30's compiled-out fault-injection seam already exists in
   this crate**: `#[cfg(feature = "test-trust-overrides")]` gates trust-root override code in
   `trust_cmd.rs` / `trust_keystore.rs` (Cargo.toml `crates/nono-cli/Cargo.toml:44`), with
   `#[cfg(not(feature = ...))]` production fallback arms throughout. This is a closer, more
   copy-pasteable model than inventing a new pattern, and it lives in the exact crate D-02 assigns
   the registry to.

Also worth flagging up front: the `PreparedWindowsLaunch` struct-field comments in `mod.rs`
(~:298-333) claim "reverse-of-declaration drop order" for its guard fields. **Struct fields in Rust
drop in forward declaration order, not reverse** (verified — see §D). The code's actual behavior is
still correct (declaration order top-to-bottom already matches the intended
labels→dacls→ancestors→network unwind sequence), but the comment's stated mechanism is wrong. This
matters for D-22 planning because a future edit that reorders fields "to fix the drop order" per the
comment's stated (incorrect) rule would silently break the unwind sequence.

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| CINT-01 | Fail-direction contract naming every layer + its cited enforcing call site | §A (layer inventory), §B (arm matrix), §I (blast radius) |
| CINT-02 | Startup self-attestation that aborts or downgrades rather than over-claiming | §C (probe availability), §D (gate point), §F (surfaces), §E (fault injection precedent for verifying the abort/degrade path itself) |
| CINT-03 | Per-layer forced-unavailable test + discovery meta-test | §E (seam to generalize), §H (house test pattern) |

</phase_requirements>

## A. Layer Inventory (D-10 derivation)

Derived by following `prepare_live_windows_launch` (`crates/nono-cli/src/exec_strategy_windows/mod.rs:348-499`),
`spawn_windows_child` (`exec_strategy_windows/launch.rs:1346` onward), and the two structurally-independent
sibling spawn sites (`nono-shell-broker/src/main.rs`, `agent_daemon/launch.rs`).

| Candidate layer | Enforcing call site (file:line) | Established how | What happens TODAY when it can't be established | Applies to which arms |
|---|---|---|---|---|
| **Mandatory integrity label** (NO_WRITE_UP, ± NO_READ_UP/NO_EXECUTE_UP) | `labels_guard::AppliedLabelsGuard::snapshot_and_apply` (`labels_guard.rs:83`), called `mod.rs:425` | `SetNamedSecurityInfoW(LABEL_SECURITY_INFORMATION)` per compiled fs-policy rule path | Fail-closed abort: `snapshot_and_apply` returns `Err` via `?` at `mod.rs:425`, which propagates out of `prepare_live_windows_launch` before any spawn. Also gated by the WRITE_OWNER precheck (R-B3, `mod.rs:382-406`) which aborts earlier with a named `SandboxInit` error if the workspace owner lacks `WRITE_OWNER`. | All arms — `prepare_live_windows_launch` runs once, before arm selection even happens (arm selection is inside `spawn_windows_child`, called after). |
| **Restricted token (WRITE_RESTRICTED + session SID)** | `restricted_token::create_restricted_token_with_sid` (`restricted_token.rs:55`), called from `launch.rs:1403` inside the `WindowsTokenArm::WriteRestricted` match arm | `CreateRestrictedToken` with a per-session synthetic SID (`generate_session_sid`, `restricted_token.rs:21`) | Fail-closed: the `?` on `create_restricted_token_with_sid(sid)?` (`launch.rs:1403`) propagates `Err` out of `spawn_windows_child` before `CreateProcess*` is ever called. No fallback token. | `WindowsTokenArm::WriteRestricted` only (non-PTY, non-broker-preferred, `session_sid.is_some()` — the "existing non-PTY supervised path", `launch.rs:1324-1328`). Structurally absent on `BrokerLaunch`/`BrokerLaunchNoPty`/`LowIlPrimary`/`Null`. |
| **AppContainer profile + package SID** | `nono::create_app_container_profile` + `nono::derive_app_container_sid`, called from `nono-shell-broker/src/main.rs:322-336`; consumed as `SECURITY_CAPABILITIES` at `main.rs:425-459` and applied via `PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES` at `main.rs:536-556`. A **second, independent** copy of the same primitives is used daemon-side: `agent_daemon/launch.rs` doc comment steps 2-3 (`launch.rs:9-12`). | `derive_app_container_sid` (library, policy-free) → `SECURITY_CAPABILITIES{ AppContainerSid, CapabilityCount: 0 }` attribute on `CreateProcessW` | Fail-closed at parse time: `parse_args` (`main.rs:94`) calls `nono::derive_app_container_sid(name)?` and fails closed on empty/invalid names (`main.rs:191-195`). If `--no-pty` is set without `app_container_name`, the broker refuses (`main.rs:202`, fail-closed doc comment `main.rs:84-88`). | `WindowsTokenArm::BrokerLaunchNoPty` only (`ExecConfig.app_container_name` doc, `mod.rs:139-145`: "Present on the BrokerLaunchNoPty... arm; `None` on other arms") **and** the daemon `nono agent launch` path (`agent_daemon/launch.rs`, always AppContainer — `DMON-01`, doc `launch.rs:1-5`). **Absent on `BrokerLaunch` (PTY)**, contradicting CONTEXT.md D-08's flat claim "AppContainer is absent on the broker arm" — see Summary point 4. |
| **DACL grants (session-SID write / package-SID write / ancestor traverse / ancestor read-attrs)** | `dacl_guard::AppliedDaclGrantsGuard::snapshot_and_apply` (`dacl_guard.rs:92`, applied `mod.rs:436-440`); `AppliedAncestorTraverseGuard::snapshot_and_apply` (`dacl_guard.rs:236`, applied `mod.rs:449-455`); `AppliedAncestorReadAttributesGuard::snapshot_and_apply_targets` (`dacl_guard.rs:401`, applied `mod.rs:473-482`). Daemon-side reimplementation: `DaemonDaclGuard` (`agent_daemon/launch.rs:73-90`, explicitly NOT imported from `dacl_guard.rs` — module-independence). | `SetNamedSecurityInfoW`/ICACLS-equivalent ACE grants keyed to `package_sid` (or `session_sid` on WriteRestricted) | Fail-closed via `?` on each `.transpose()?` at `mod.rs:439-440`, `:454-455`, `:481-482` — any grant failure aborts `prepare_live_windows_launch` before spawn. `None` (no-op, immediately reverted on Drop per the doc comment) when `config.package_sid` is `None`. | Session-SID grant: `WriteRestricted` only (`_applied_dacls` comment, `mod.rs:308-311`: "OPERATIVE on the WriteRestricted arm; inert but harmless elsewhere"). Package-SID variants: any arm where `config.package_sid.is_some()` — i.e. `BrokerLaunchNoPty` and the daemon path. |
| **WFP egress filters** | `install_wfp_network_backend` / `prepare_network_enforcement` (`network.rs`, dispatched via the `WindowsNetworkBackend` trait, `mod.rs:501-510`); pre-spawn confirmation `assert_wfp_activation_installed_filters` (`network.rs:1775-1800`, called `network.rs:1730`) | Named-pipe IPC to the elevated `nono-wfp-service.exe`, which calls the real WFP API and reports back `installed_filter_count` | **Already fail-closed and pre-confirmed**, not merely "configured": `WfpRuntimeActivationProbeStatus::AcceptedButNotEnforced` / `NotImplemented` / stray `Ready` / `CleanupSucceeded` / `FilteringProbeSucceeded` all map to `Err` (`network.rs:1698-1752`); only `EnforcedPendingCleanup` + `installed_filter_count > 0` succeeds. Daemon path has its own copy: `wfp_filter_add` (`agent_daemon/launch.rs:444`), gated before `ResumeThread` at "step 6.5" (`agent_daemon/launch.rs:698-735`). | Per-SID WFP: daemon path (`nono agent launch`) and `BrokerLaunchNoPty` (package-SID-scoped). AppID/firewall-rule fallback (`FirewallRulesNetworkBackend`) is program-path-scoped, not session-scoped — see §G for the legacy-vs-WFP backend selection open question. |
| **Minifilter (absence)** | N/A — no driver exists in this tree. ADR-65 stands (D-33). | N/A | Structurally absent; `crates/nono-cli/src/exec_strategy_windows/` contains no driver-lifecycle code. Per-file read policy within one directory is explicitly not claimed (BOUND-02, Phase 119). | All arms, uniformly — this is the one row with no per-arm variation. |
| **Job Object containment** | `create_process_containment` (`launch.rs:273-348`); `apply_process_handle_to_containment` (`launch.rs:375-393`), called `launch.rs:2134` BEFORE `resume_contained_process` (`launch.rs:2145`). Daemon: `assign_process_to_agent_job` (`agent_daemon/launch.rs:1048-1066`). | `AssignProcessToJobObject` on the suspended child, with an explicit deny-DACL on the job object itself (`build_job_security_attributes`, referenced `launch.rs:282-292`) | Fail-closed: `apply_process_handle_to_containment` failure → `terminate_suspended_process` (`launch.rs:2135`) then `Err` propagates. `assign_failure_message` (`launch.rs:356-373`) distinguishes the nested-job-collision case (GLE=5) with an explicit fail-secure refusal message. | **IN — recommend adding as a full registry row.** It is load-bearing (kill-group guarantee, `--timeout` enforcement) and already has an existing, well-tested fail-closed gate immediately adjacent to the `CREATE_SUSPENDED`→`ResumeThread` window — natural to co-locate with D-21's attestation gate rather than treat as out of scope. |
| **Env sanitization** (`allowed_env_vars` / `denied_env_vars`) | `env_sanitization::{is_env_var_allowed, is_env_var_denied, should_skip_env_var}` (imported `mod.rs:79`), consumed inside `build_child_env` (`launch.rs`, called `launch.rs:1355`) | Pure filtering logic over the env block before `CreateProcess*`; no OS enforcement primitive, no probe surface | Fail-closed by construction for the empty-allow case (`test_windows_empty_allow_denies_all_env_vars`, cited in `mod.rs` doc comment ~:178-181) — an empty allow-list denies all, never falls through to inherit-all. | **OUT — recommend excluding from the registry as a distinct row, or folding as a footnote under the process-identity layer.** Reasoning: it is data hygiene applied before spawn, not a *kernel-enforced confinement guarantee* with a post-hoc probe; CINT-01's six named layers and WFP/DACL/labels are all things a compromised child cannot unilaterally defeat from inside the process. A wrong env var, if it leaked, is a confidentiality concern, not a containment-escape one — different threat class from the rest of the contract. State this exclusion reasoning in the contract per D-10's "every exclusion is reasoned on the record." |
| **`validate_windows_launch_paths` interpreter coverage gate** | `Sandbox::validate_windows_launch_paths` (`crates/nono/src/sandbox/mod.rs:925-932`, delegates to `windows::validate_launch_paths`), called `mod.rs:357-362` | Static policy-coverage check: does the compiled fs-policy already cover `program`, `current_dir`, and every resolved interpreter path? | Fail-closed today already: returns `Err` before any label/DACL/spawn work happens if an interpreter is uncovered (doc comment `mod.rs:194-198`: "fail-secure: an uncovered interpreter causes the gate to refuse with a named diagnostic"). | **IN, but as a pre-flight gate row, not a "confirm the child actually got it" attestation row.** This is a *build-time-of-the-launch-plan* check, not a runtime kernel-state probe — CINT-02's self-attestation (D-21, "against the real child while it is suspended") doesn't apply to it the same way. Recommend registry carries it as a distinct `outcome: abort` row with `probe: n/a — validated pre-spawn, not re-checked post-spawn` to keep D-18's vocabulary honest. |
| **Broker Authenticode trust gate** | `broker_authenticode` machinery exercised via `nono setup --check-only` (test harness: `crates/nono-cli/tests/broker_authenticode.rs`); production dispatch gate referenced at `launch.rs` around the `BrokerLaunch`/`BrokerLaunchNoPty` dispatch (D-32-11..14, Phase 32) | Authenticode signature comparison between `nono.exe` and the sibling `nono-shell-broker.exe`, with a dev-layout bypass keyed off a compile-time-baked path (`is_dev_build_layout`, `launch.rs:2190-2192`) | Fail-closed in production layouts (per the module doc in `broker_authenticode.rs:1-10`); explicitly bypassed only under the compile-time-baked dev-target-root check, never a runtime env var (R-B4 hardening, `launch.rs:2170-2189`). | **IN — recommend adding as a registry row scoped to the broker arms only** (`BrokerLaunch`/`BrokerLaunchNoPty`). It gates a structurally distinct failure mode (supply-chain / binary-substitution) that none of the other six rows cover, and CINT-01 asks for "every layer the backend composes," not only the six named as a floor. |
| **Proxy egress (nono-proxy)** | Not exercised inside `exec_strategy_windows/`; proxy activation is orchestrated at a higher layer (`command_runtime.rs`/`supervised_runtime.rs`) and is cross-platform, not Windows-specific plumbing. | Separate process (`nono-proxy`), TLS-terminating HTTP(S) proxy with host-allowlist filtering | UNRESOLVED — this research did not trace the proxy's own fail-direction; it is a cross-platform component, not a Windows-composite layer in the CINT-01 sense. | **OUT of the CINT-01 Windows layer registry** — recommend excluding with the reasoning that proxy egress is BOUND-01's subject (Phase 119: "what nono governs... destination, credential, containment... not payload"), a cross-platform mechanism outside "the Windows backend composes," not a Windows-kernel confinement primitive. Flag this exclusion explicitly in the contract per D-10. |

## B. Execution Arm Matrix (D-08)

**Answer: it is genuinely two axes that collapse to a small number of realized combinations, not one
flat "arm" enum.** The two axes are:

- **Axis 1 — token-construction arm**, decided by `select_windows_token_arm`
  (`launch.rs:1296-1339`, pure function, 5 inputs: `is_detached`, `has_pty`, `has_session_sid`,
  `caps_demand_low_il`, `prefers_low_il_broker`). Produces `WindowsTokenArm::{Null, WriteRestricted,
  LowIlPrimary, BrokerLaunch, BrokerLaunchNoPty}` (`launch.rs:1237-1278`).
- **Axis 2 — entry path / process topology**, decided by which *binary* handles the spawn:
  `nono.exe` direct (`exec_strategy_windows/launch.rs`), `nono-shell-broker.exe` (spawned BY
  `nono.exe`, which then itself spawns the real child), or `nono-agentd.exe` (`agent_daemon/launch.rs`,
  structurally independent implementation, §Summary point 1).

These are not orthogonal — `WindowsTokenArm::BrokerLaunch`/`BrokerLaunchNoPty` *are* the point at
which axis 1 hands off to axis-2's broker topology. The four D-23 gated paths map onto this matrix as:

| D-23 path | Token arm (axis 1) | Entry-path binary (axis 2) | Suspended-spawn site(s) traversed |
|---|---|---|---|
| direct `nono run` (no PTY, no low-IL-broker profile pref) | `WriteRestricted` (or `Null`/`LowIlPrimary` on edge configs) | `nono.exe` only | `launch.rs:2075-2145` once |
| `nono run --shell`-equivalent / PTY-allocating interactive session | `BrokerLaunch` | `nono.exe` (spawns broker suspended) → `nono-shell-broker.exe` (spawns real child suspended) | `launch.rs:2085-2107`/`:2134-2145` (broker.exe as the "child") **then** `main.rs:536-661` (the real child) — two suspended windows in series |
| daemon `nono agent launch` | N/A — daemon path does not go through `select_windows_token_arm` at all; it always builds `SECURITY_CAPABILITIES` directly (`agent_daemon/launch.rs` step 2-5) | `nono-agentd.exe` | `agent_daemon/launch.rs` steps 5-8 (~:600-860), **independent implementation, no shared code with `launch.rs`** |
| per-tool-call hook (`claude_code_hook.rs`) | Whatever the rewritten `nono run ...` command line resolves to (same cascade as row 1, since the hook rewrites `Bash` calls into a `nono run` invocation string — `claude_code_hook.rs:268`, `:383-406` resolve `nono.exe`'s own path to build the command) | `nono.exe` (spawned fresh by Claude Code as a new process, not by nono-cli itself) | Same as direct `nono run` — this path does **not** have its own spawn site; it re-enters the standard CLI entry point. **Unless** the active profile sets `windows_low_il_broker` (`prefers_low_il_broker`), in which case it also traverses `BrokerLaunchNoPty` → broker.exe's own suspended window. |

**Recommendation for the registry type (Claude's Discretion per CONTEXT.md, informing not
overriding):** model expectancy as `(entry_path, token_arm) → Expectancy` rather than either axis
alone. A flat `WindowsTokenArm`-only matrix would incorrectly imply the daemon path is representable
by one of the five `WindowsTokenArm` variants when it structurally bypasses that enum entirely.

`WindowsTokenArm` cascade ordering, verified against current source (`launch.rs:1296-1339`):
`is_detached → Null` / `has_pty → BrokerLaunch` / `prefers_low_il_broker && has_session_sid →
BrokerLaunchNoPty` / `has_session_sid → WriteRestricted` / `caps_demand_low_il → LowIlPrimary` /
else `Null`. Matches CONTEXT.md's description; line range (~1237-1278 for the enum, not exactly the
cascade fn, which starts at `:1296`) is close enough not to need correction.

## C. Probe Availability (D-17/D-18)

Verified by symbol, not file presence, per the project's standing lesson (`feedback_disposition_confidence_needs_symbol_level.md`).

| Layer | Live probe exists? | API + import path | Notes |
|---|---|---|---|
| Mandatory integrity label | Partial — `TokenIntegrityLevel` is already imported in `mod.rs:45` (`windows_sys::Win32::Security::TokenIntegrityLevel`) and used in `probe_integrity_level_support` (`mod.rs:722-791`), but that function probes the **supervisor's own** token, not the child's. `GetTokenInformation` is imported (`mod.rs:44`) and used for the same self-probe (`mod.rs:683-689`). **No existing call queries the suspended child's token integrity level.** Adding one is straightforward: `OpenProcessToken(child_process_handle, TOKEN_QUERY, ...)` + `GetTokenInformation(token, TokenIntegrityLevel, ...)`, both already-imported symbols. | `windows_sys::Win32::Security::{GetTokenInformation, TokenIntegrityLevel}` — CONFIRMED already imported at `mod.rs:44-45`. | CONTEXT.md's claim is directionally right but imprecise: the symbols are imported and used, but only against the supervisor's own process, never the child's. |
| Restricted token (session SID) | Partial — `GetTokenInformation(TokenRestrictedSids, ...)` is used in a **test-only** module of `restricted_token.rs` (`restricted_token.rs:147-211`, `#[cfg(test)]`), not in production code, and `TokenRestrictedSids` is NOT imported in `mod.rs`. | `windows_sys::Win32::Security::{GetTokenInformation, TokenRestrictedSids, TOKEN_GROUPS}` — imported only inside `restricted_token.rs`'s test module. A production probe would need a new top-level import. | This is precedent that the API call shape works (it's already exercised in a test), but it is not "already imported in `exec_strategy_windows/mod.rs`" as CONTEXT.md states — correct citation is `restricted_token.rs:147`, test-scoped. |
| AppContainer package SID | No existing probe found (production or test) for reading a token's AppContainer SID back out. | `TokenAppContainerSid` (a `TOKEN_INFORMATION_CLASS` value) + `GetTokenInformation` — NOT currently imported anywhere in the crate (grep found zero hits for `TokenAppContainerSid`). This is the class named in the objective's example; confirmed absent from the tree, so it is genuinely new FFI surface, not a reuse. | `windows_sys` does expose `TokenAppContainerSid` under `Win32::Security` (same enum as `TokenIntegrityLevel`); adding it is a one-line import, but it IS new, unlike D-17's "largely already-available calls" framing suggests for this specific row. |
| DACL grants (session/package SID write, ancestor traverse/read-attrs) | No live re-read-back probe; these are supervisor-applied filesystem ACEs, not token state. A probe would mean re-querying the DACL of the granted paths (`GetNamedSecurityInfoW`, already imported in `crates/nono/src/sandbox/windows.rs:22-25`) and confirming the expected ACE is present. Structurally different shape from the token probes — it is a filesystem-state re-read, not a process-state re-read, and it targets a set of *paths* (which the child may not even exist relative to yet at the `CREATE_SUSPENDED` moment), not the child handle itself. | `windows_sys::Win32::Security::Authorization::GetNamedSecurityInfoW` — imported in `crates/nono/src/sandbox/windows.rs`, not in `exec_strategy_windows/`. | Because the DACL grants are applied by the supervisor itself immediately before spawn (not something the child does), "established" here plausibly means "the apply call succeeded" (already fail-closed, §A) rather than "re-observed independently" — recommend the registry treat DACL grants as `established, not independently observable` **by design** (the apply-time `Result` IS the confirmation; there is no separate kernel object to re-query that would tell you anything the apply call didn't already tell you), distinct from WFP's unobservable case which stems from an elevation boundary. |
| WFP egress filters | **Confirmed pre-spawn** via IPC with the elevated service (`installed_filter_count`, §A/§G) — this is NOT the "unobservable" case CONTEXT.md pre-declares. What genuinely IS unobservable from the non-elevated supervisor: re-querying the *live* WFP filter engine state (`FwpmFilterGetById0`/`FwpmFilterEnum0`) directly, which requires `FWPM_SESSION` access typically gated to elevated/SYSTEM callers — nono.exe (non-elevated) never calls these APIs directly; it always goes through the elevated service's IPC response. | No direct WFP query API is called from nono-cli (`crates/nono-cli/src/exec_strategy_windows/network.rs` — confirmed via grep, zero `Fwpm*` symbols in that file); all WFP interaction is proxied through the `nono-wfp-service` IPC protocol (`windows_wfp_contract.rs`). | **This is the load-bearing correction to CONTEXT.md D-18.** The honest framing: WFP's install-time confirmation (`installed_filter_count > 0`) is real and already fail-closed, but it is a *report from the elevated service*, not an independent kernel-state observation by the (untrusted-relative-to-the-service, but here actually the *more*-trusted, non-elevated) supervisor. Whether that counts as `confirmed` or `established, not independently observable` under D-18's vocabulary is a genuine judgment call for the planner — see Open Questions. |
| Job Object containment | Yes — `IsProcessInJob` is already imported (`mod.rs:62-65`, comment: "consumed only by Plan 31-03's `broker_dispatch_tests`... gated under `#[cfg(all(test, target_os = "windows"))]`") and exercised in a test (`launch.rs:3054, ~3088-3100`). Production code never calls it; `AssignProcessToJobObject`'s own return code (§A) is the only production-path confirmation today. | `windows_sys::Win32::System::JobObjects::IsProcessInJob` — imported, test-only today. | Cheap, well-understood API; promoting the test-only call into a production D-21 probe is low-risk and matches D-24's "cheap local calls" framing. |
| Minifilter (absence) | N/A by construction (ADR-65) — no probe target exists. | N/A | The registry row's `probe` field should read something like `n/a — no such layer exists; row documents structural absence`, not `unconfirmed`. |

## D. Attestation Gate Point (D-21/D-22/D-23)

**The nono-cli suspended-spawn site** is inside `spawn_windows_child`
(`crates/nono-cli/src/exec_strategy_windows/launch.rs:1346`). The two `CreateProcess*` calls are at
`launch.rs:2085` (`CreateProcessAsUserW`, non-null token) and `:2095` (`CreateProcessW`, null token) —
both reached from the same `if !h_token.is_null() { ... } else { ... }` branch (`:2075-2108`), and the
broker's own suspension (when `arm == BrokerLaunch`) happens earlier in the same function at
`launch.rs:2002-2013`. **All three converge on the same post-spawn block**
(`launch.rs:2111-2153`): `created == 0` check → wrap `process`/`thread` in `OwnedHandle` (`:2118-2119`)
→ close inherited pipe ends (`:2126-2132`) → `apply_process_handle_to_containment` (`:2134`, Job
Object assign) → `apply_resource_limits` (`:2141`) → `resume_contained_process` → `ResumeThread`
(`:2145`, defined `launch.rs:438-450`). **What is in scope at the gate point:** `process` (owned
`HANDLE`), `thread` (owned `HANDLE`), `containment.job` (owned `HANDLE`, already assigned by this
point), `config` (full `ExecConfig`, including `package_sid`/`session_sid`), and `arm` (the selected
`WindowsTokenArm`). This is the natural D-21 insertion point for the nono-cli-side probes (token
integrity, restricted SIDs, `IsProcessInJob`) — insert between `apply_resource_limits` (`:2141`) and
`resume_contained_process` (`:2145`), matching the existing "everything must be true BEFORE
`ResumeThread`" discipline already used for job assignment and resource limits at this exact site.

**If a gate returned `Err` between them:** the existing pattern is `terminate_suspended_process(process.raw(), "<reason>")` (`launch.rs:395-402`, wraps `TerminateProcess(process, 1)`) then propagate `Err` — this is literally the same shape used for the two adjacent gates (`apply_process_handle_to_containment` failure at `:2135`, `apply_resource_limits` failure at `:2142`). D-21/D-22 slot into this exact idiom without inventing new control flow.

**`PreparedWindowsLaunch` guard set and Drop order** (struct at `mod.rs:296-333`): fields declared in
this order — `_applied_labels` → `_applied_dacls` (`Option`) → `_applied_ancestor_traverse`
(`Option`) → `_applied_ancestor_read_attrs` (`Option`) → `_network_enforcement` (`Option`) →
`launch_program` (not a guard). **Correction to the code's own comments:** the comments at
`mod.rs:298-333` repeatedly say fields are ordered so "reverse-of-declaration drop order" reverts
labels first and network last. **Rust struct fields drop in forward declaration order, not
reverse** — the reverse-of-declaration rule applies to *local stack variables* (LIFO), not struct
fields, which the Rust reference confirms was a deliberate, documented design choice distinct from
C++ (see Sources). The code's *actual* runtime behavior is still correct — because the fields happen
to be declared in exactly the order the authors want them dropped (labels first, network last), drop
proceeding in true forward order produces the same sequence the comments describe. This is purely a
documentation-accuracy defect, not a functional one, but D-22 planning should not "fix" the drop
order based on the comment's stated (wrong) mechanism if a future edit reorders these fields — flag
this as a candidate SC4 discrepancy row (see §G) so it gets corrected rather than propagated.

**Broker arm and daemon path: do they share the gate, or need their own?**
Confirmed **four independent gate insertion points are needed**, not one:
1. `launch.rs:2141-2145` (nono-cli's own spawn — covers direct `nono run` and, for the broker arms,
   the broker.exe process itself, but NOT the broker's grandchild).
2. `nono-shell-broker/src/main.rs:645-660` (the broker's own `ResumeThread` for the real Low-IL /
   AppContainer child — already has an inline gate today: `OpenProcessToken` + `apply_low_il_label_to_token`
   between `CREATE_SUSPENDED` and `ResumeThread`, `main.rs:615-644`, demonstrating the pattern is
   already proven in this exact spot).
3. `agent_daemon/launch.rs:848` (`ResumeThread(thread_handle_raw)`) — already has WFP (`:698-735`) and
   DACL (`:755-799`) gates immediately before it ("step 6.5"/"step 6.6" per the module's own numbered
   doc comment), same idiom, structurally independent code.
4. The hook path re-enters gate #1 (it does not have its own spawn site) — **so D-23's "four gated
   paths" resolve to THREE distinct code locations to instrument**, with the hook path automatically
   covered once #1 is done, and the broker arm requiring BOTH #1 and #2 to be gated for full coverage.

## E. Fault-Injection Seam (D-29/D-30)

**Current shape**, verified: `WINDOWS_WFP_TEST_FORCE_READY: AtomicBool` (`mod.rs:558`, deliberately
promoted out of `#[cfg(debug_assertions)]` per Phase 41 REQ-CI-02 so it is available in release
builds too) + `set_windows_wfp_test_force_ready(force_ready: bool)` (`mod.rs:584-597`) + CLI flag
`--dangerous-force-wfp-ready` (`cli.rs:2211-2214`, `hide = true`) wired in `command_runtime.rs:89-92`.
**Storage mechanism:** a process-global `AtomicBool`, `Ordering::Relaxed`. **Gate mechanism:** the
*setter itself* checks `std::env::var_os("NONO_TEST_HARNESS").is_none()` and silently no-ops (with a
`tracing::warn!`) if unset (`mod.rs:588-596`) — this is a **runtime** gate on an otherwise
**always-compiled** flag; the flag exists and is settable in every build, production included, it
just requires the env var to take effect. **How a test drives it:** set `NONO_TEST_HARNESS=1` in the
test process environment, then pass `--dangerous-force-wfp-ready` to the `nono` subprocess under
test.

**D-30's concern is precise and correct**: the toggle is compiled into every release binary; the only
thing standing between an attacker and flipping it is an env var they fully control if they can spawn
the process at all. A `#[cfg(feature = "...")]`-gated seam removes the code from release binaries
entirely.

**A shipped, reviewed precedent for exactly this compiled-out shape already exists in the same
crate**: `test-trust-overrides` (`crates/nono-cli/Cargo.toml:43-44`: *"Enables test-only trust root
overrides used by the integration harness"*), consumed via `#[cfg(feature = "test-trust-overrides")]`
throughout `trust_cmd.rs` (`:30`, `:415`, `:1678`, `:2101`) and `trust_keystore.rs` (11 call sites,
`:10` through `:571`), each paired with a `#[cfg(not(feature = "test-trust-overrides"))]` production
fallback (e.g. `trust_cmd.rs:432`). **This is the closer model to generalize from than inventing a new
pattern**: a new `layer-fault-injection` (or similarly named) feature, default-off, added to
`crates/nono-cli/Cargo.toml`'s `[features]` block, with each per-layer force-unavailable hook gated
`#[cfg(feature = "layer-fault-injection")]` and a `#[cfg(not(...))]` arm that either doesn't compile
the hook at all or hard-no-ops it.

**Workspace `[features]` inventory** (every crate the plan may need to touch for D-30):

| Crate | `[features]` block | Notes |
|---|---|---|
| `crates/nono-cli` | `default = ["system-keyring"]`; `system-keyring`; `test-trust-overrides` (`Cargo.toml:37-44`) | Home of the existing precedent; also D-02's registry home — natural place for the new feature. |
| `crates/nono` | `default = ["system-keyring"]`; `system-keyring` (`Cargo.toml:28-32`) | No test-only feature exists here today; if any library-side probe needed a compiled-out test hook, a new feature would be needed here too — but D-02 keeps policy CLI-side, so this is likely unnecessary. |
| `crates/nono-proxy` | `default = ["system-keyring"]`; `system-keyring` (`Cargo.toml:31-35`) | Out of scope (proxy egress excluded from the registry, §A). |
| `crates/nono-shell-broker` | **No `[features]` block at all.** | If the broker's own suspended-spawn gate (§D point 2) needs a fault-injection hook, this crate needs a brand-new `[features]` section added — not an extension of an existing one. |
| `bindings/c` | **No `[features]` block at all.** | Out of scope for this phase (no C FFI surface touched). |

**Migrate vs. defer:** D-30 explicitly requires this be addressed one way or the other. Given the
precedent already exists and the blast radius is one crate (`nono-cli`) for the primary WFP toggle,
recommend **migrate in-phase** for the WFP toggle (small, well-precedented) and **treat the broker's
own future force-unavailable hooks as new `[features]` additions to `nono-shell-broker`**, sized as
part of whichever plan implements the broker-side per-layer tests (CINT-03) — flag that crate's
current featureless state as a small but real addition, not a zero-cost reuse.

## F. Integration Surfaces (D-27)

**Telemetry (`crates/nono-cli/src/telemetry/mod.rs` / `event.rs`):** `SecurityEventLayer` struct
confirmed at `mod.rs:218` (matches CONTEXT.md exactly). `advance_chain` at `mod.rs:126`.
`TELEMETRY_CHAIN_DOMAIN` at `mod.rs:75` (also confirmed exact). **Adding a new structured event
requires:** (1) a new `SecurityEventType` variant in `event.rs:67-90` (currently 10 variants,
EventIDs 10001-10010, documented in a table at `event.rs:16-29`); (2) a new `EVENT_ID_*` constant
(`event.rs:38-56`) and an `event_id_for` match arm (tested exhaustively, `event.rs:286-291`); (3) no
changes needed to `SecurityEvent`'s struct shape (`event.rs:253-274`) unless the new event needs a
field none of the existing ones carry — the existing shape (`event_type`, `agent_pid`, optional
`path_hash`/`path_category`/`host`, `session_id`, `chain_head`, `timestamp_unix_ms`) plausibly covers
a layer-attestation-downgraded event with `path_hash`/`path_category`/`host` all `None` and a new
field for "which layer" would need to be added or the layer name folded into an existing free-text
path via `nono::scrub_value`. **This is a genuine open design question** — see Open Questions.

**Diagnostic codes (`crates/nono/src/diagnostic/codes.rs` + `crates/nono/src/error.rs`):**
`NonoDiagnosticCode` is `#[non_exhaustive]` (`codes.rs:20`) — new variants are additive-safe from
outside the crate. **`NonoError::diagnostic_code()`** (`error.rs:383-448`) is an **exhaustive match
with no wildcard arm** — adding a new `NonoError` variant (e.g. for "a required layer could not be
attested") requires updating this match, or reusing an existing variant. Existing precedent for a
narrowly-scoped new error: `LabelApplyFailed`, `DaclApplyFailed`, `PolicyLoadFailed` (all mapped to
`NonoDiagnosticCode::ConfigurationError`, `error.rs:431-435`) — a new `LayerAttestationFailed { layer:
String, reason: String }`-shaped variant fits this exact pattern and would need its own
`NonoDiagnosticCode` variant (recommend a new one, e.g. `LayerAttestationFailed`, rather than
overloading `ConfigurationError` or `SandboxDeniedPath`, since D-22 explicitly wants the operator to
learn *which layer* failed, and a distinct code lets `DiagnosticFormatter`/remediation surfaces
branch on it precisely). ADR-86 carve-out confirmed still standing: Windows denial rendering stays
CLI-side (per CLAUDE.md's table), so the formatting of this new code's message can live in
`nono-cli`, only the code/variant needs to live in the library per the existing split.

**Machine policy (`crates/nono/src/machine_policy.rs`):** `MachineEgressPolicy` (`:150-183`) is the
extension precedent — `telemetry: TelemetryConfig` (`:171-182`) was added as a nested sub-struct with
`#[serde(default)]`, read from a sibling HKLM subkey during the same single registry read (module doc
says this shape is deliberate: "designed to be extended... without re-architecting the single startup
read", `:145-148`), and explicitly excluded from `is_unconfigured()`'s counting (`:178-180`, "MUST
NOT be counted"). A new `required_layers: RequiredLayersPolicy` field would follow the identical
shape: new sub-struct, `#[serde(default)]`, new HKLM subkey, and an explicit decision on whether it
counts toward `is_unconfigured()` (recommend: it should NOT count, matching the telemetry precedent —
an admin who has only set required-layers policy should not accidentally flip the daemon to
strict-deny-all egress). Abort-vs-degrade precedent confirmed: "D-07 egress aborts, D-14 telemetry
degrades" language appears literally in the module (`:513-514`, `:594`, `:605`) — this is the
existing house vocabulary D-25/D-26 should reuse rather than invent new terms.

**Banner (`crates/nono-cli/src/output.rs::print_banner`, confirmed exact at `:32-46`):** prints
**only** `"nono v{version}"` — zero enforcement-claim text exists in the banner today. **There is no
existing banner wording that would become a lie under a downgraded claim** — this is good news for
D-27's scope (nothing pre-existing to correct), but it also means the downgrade text is 100% new
copy, not a retrofit. The more relevant existing "claim surface" precedent is `print_capabilities`
(`output.rs:57-...`) and, for a genuinely close 3-state precedent, the **Linux-only**
`format_scope_status` (`output.rs:492-500`, `#[cfg(target_os = "linux")]`): `(requested, enforced,
supported)` → `"requested, enforced"` / `"requested, not enforced"` / `"requested, unsupported by
detected ABI"` / `"not requested"` / `"not requested; detected ABI has no scope support"`. This is a
close cousin of D-27's "confirmed / not expected / unconfirmed" vocabulary, though it is Linux-only
code today and would need a Windows-side analog, not a reuse (the function is behind
`#[cfg(target_os = "linux")]`).

## G. Candidate SC4 Discrepancies

Evidence discipline per D-16: every command below was actually run against this working tree on
**2026-08-09**, against the full tree (not a pre-decided path), and hit counts are the literal tool
output.

| # | Finding | Command run | Hit count | Severity (D-13) | Disposition |
|---|---|---|---|---|---|
| SC4-1 | `crates/nono/src/sandbox/windows.rs::apply()` (the original honesty-gap site, `:52`) still returns `Ok(())` for a `CapabilitySet` requesting network restriction without checking that the CLI's separate WFP runtime activation actually happened — the library-level `apply()` validates *shape* only (`compile_network_policy`, unsupported-shape check `:82-88`), never confirms *runtime* enforcement. This is the ADR-86-documented carve-out (library applies only what's in `CapabilitySet`; CLI owns the runtime WFP pipeline) — **not a bug**, but the contract must state it explicitly so a reader of `apply()` alone does not assume network enforcement happened. | `grep -n "fn apply" crates/nono/src/sandbox/windows.rs` | 1 (`:52`) | Low — already correctly carved out by ADR-86; risk is *documentation*, not *behavior*. Record as a `fail-open` (justified) row, not `fail-open-defect`. | Record in contract with ADR-86 citation; no code change needed. |
| SC4-2 | The `PreparedWindowsLaunch` struct-field comments (`mod.rs:290-291`, `:298-333`) assert "Rust drop order is reverse-of-declaration" for **struct fields**, which is factually wrong per the Rust reference (structs drop fields in forward declaration order; only local stack bindings are LIFO). Current behavior is coincidentally correct (see §D) but the stated *mechanism* is wrong and could mislead a future editor who reorders fields "to preserve" the (wrong) rule. | `grep -n "reverse-of-declaration" crates/nono-cli/src/exec_strategy_windows/*.rs` | 2 (`mod.rs:291`, `launch.rs:291` — the latter is the LOCAL-variable case in `create_process_containment`, which IS correctly reverse-of-declaration since it's a stack binding, not a struct field) | Low functional risk today, Medium latent risk (a future refactor could silently break unwind order trusting the wrong comment). Not a `fail-open-defect` (nothing fails open today) — a documentation-accuracy defect. | Fix the comment in-phase (cheap); note in the SPEC's discrepancy section per D-15 with "fixed here." |
| SC4-3 | `NetworkEnforcementGuard::FirewallRules` (legacy AppID/netsh-rule backend, `mod.rs:280-286`, Drop impl `network.rs:3-15`) still exists as a live code path alongside `WfpServiceManaged`. Its selection logic (`WindowsNetworkBackend` trait, `mod.rs:501-510`; concrete impls `FirewallRulesNetworkBackend`/`WfpNetworkBackend`) was **not fully traced** in this research pass — it is unclear from the code read so far whether `FirewallRulesNetworkBackend` is still reachable in production or is dead/fallback-only. If reachable, it is a SEPARATE fail-direction path from the WFP row's fail-closed `assert_wfp_activation_installed_filters` gate, and its own abort/degrade contract is unverified. | `grep -n "FirewallRulesNetworkBackend" crates/nono-cli/src/exec_strategy_windows/*.rs` | 3 (struct decl `mod.rs:512`, and 2 more in `network.rs` — exact selection-dispatch site not confirmed in this pass) | **UNRESOLVED severity — needs a dedicated trace in planning.** If reachable and its fail-direction is weaker than WFP's, this could be a real `fail-open-defect`. | Planner must trace `WindowsNetworkBackend` backend-selection dispatch before sizing the WFP contract row; flag as a Wave-0 investigation task, not assumed-safe. |
| SC4-4 | `set_windows_wfp_test_force_ready`'s runtime `NONO_TEST_HARNESS` gate (D-29/D-30, §E) is a live attack-surface pattern per D-30's own framing — this is CONTEXT.md's own pre-flagged discrepancy, confirmed real by this research (§E) and not yet fixed. | `grep -n "NONO_TEST_HARNESS" crates/nono-cli/src/exec_strategy_windows/mod.rs` | 2 (`:589`, `:592`, i.e. the check + the warn message referencing it) | Medium — it is a real, always-compiled toggle in every release binary, gated only by an attacker-settable env var; but it currently only affects the WFP row (not silently widening a boundary the operator didn't already control via other flags), and requires local process-spawn control to exploit (already a strong position for an attacker). | D-30 already requires disposition: migrate to `#[cfg(feature = ...)]` (recommended, §E) or record as a deferred SC4 row with successor named. |

## H. Meta-Test Precedent (D-32)

**Correction to CONTEXT.md:** the house pattern uses `env!("CARGO_MANIFEST_DIR")` +
`std::fs::read_to_string` (a **test-run-time** read, always fresh), not `include_str!` (a
**compile-time** embed). Zero hits for `include_str!` in `crates/nono-cli/tests/*.rs` — the one
`include_str!` use in the crate is in production code (`hooks.rs`), unrelated to the test pattern.

The clearest example is `crates/nono-cli/tests/resl_supervisor_drain.rs` (full pattern shown,
`:1-80`): reads `src/exec_strategy.rs` via a `CARGO_MANIFEST_DIR`-joined path
(`read_exec_strategy`, `:27-34`), extracts the region between two literal sentinel strings
(`sentinel_region`, `:38-66`, with `//`-comment stripping to avoid false-positives from prose), and
asserts a structural property over that region (`region.matches("return Ok((").count() == 0`,
`:71-77`) with a message naming exactly what regressed and where to fix it. This is
**self-enforcing** in the sense CONTEXT.md means: it fails loudly the moment new code violates the
invariant, without needing a live runtime environment (works on the Windows dev host even though the
subject is Linux supervisor code).

**How a D-32 meta-test could enumerate registry rows:** two real options given what this tree
supports:

1. **Compile-time enum iteration.** If the registry's layer-identity type is a small closed `enum
   LayerId { ... }` with a `const ALL: &[LayerId] = &[...]` (or a `strum`-style derive, not currently
   a workspace dependency — would need adding), the meta-test iterates `LayerId::ALL` at test-run time
   and, for each variant, checks a **derivable test-function name** (e.g. `format!("force_unavailable_{}",
   layer.snake_case_name())`) exists — which then requires a source-scan (option 2) to actually confirm
   the function is present, since Rust has no runtime reflection over test functions.
2. **Source-scan cross-reference** (the pattern this tree already uses, extended). Read the relevant
   test file(s) via `CARGO_MANIFEST_DIR`, extract all `#[test] fn <name>` declarations with a simple
   line-scan (mirroring `resl_supervisor_drain.rs`'s sentinel-region technique, or a regex — `regex` is
   already a workspace dependency per `jsonschema`'s transitive use, worth confirming it's a direct dep
   before relying on it), and assert every name in `LayerId::ALL.iter().map(name)` appears as a
   substring of some discovered test function name.

**Recommendation:** combine both — compile-time enumeration for the source-of-truth list (cheap,
type-checked, can't drift), source-scan for confirming test-function existence (the only mechanism
this tree has proven for "assert something exists in source"). This matches D-32's own framing
("discovery-based... consistent with D-01's registry-as-source-of-truth") and requires no new
workspace dependency if a plain substring/line-scan is used instead of `regex`.

## I. Blast Radius & Cross-Target Gates (D-11/D-35)

Per-file cfg verification (files the plan will plausibly touch), confirmed by direct grep against
each file, not assumed from CONTEXT.md:

| File | Contains `#[cfg(target_os = "linux")]` / `"macos"` / `any(...)` / under `exec_strategy/` or `bindings/c/src/`? | Cross-target clippy MUST applies? |
|---|---|---|
| `crates/nono-cli/src/execution_runtime.rs` | YES — `#[cfg(unix)]` (`:1`, `:332`, `:851`), `#[cfg(target_os = "linux")]` (`:28`, `:419`, `:466`, `:479`, `:530`, `:532`, `:534`), `#[cfg(target_os = "macos")]` (none found directly but `#[cfg(any(target_os = "linux", target_os = "macos"))]` at `:703`, `:705`), `#[cfg(target_os = "windows")]` (10+ sites) | **YES, confirmed.** Matches CONTEXT.md D-11. |
| `crates/nono-cli/src/cli.rs` | YES — `#[cfg(target_os = "macos")]` (`:100`, `:2070`, `:2776`, `:5504`), `#[cfg(target_os = "windows")]` (12+ sites) | **YES, confirmed.** |
| `crates/nono-cli/src/output.rs` | YES — `#[cfg(target_os = "linux")]` at `:357`, `:429`, `:491`, `:502` (the `format_scope_status` family, §F). No macOS/any-combined cfg found in this file specifically. | **YES, confirmed** — the file-level trigger in CLAUDE.md is any of the three listed patterns; `target_os = "linux"` alone qualifies. |
| `crates/nono-cli/src/exec_strategy_windows/*.rs` (mod.rs, launch.rs, network.rs, dacl_guard.rs, labels_guard.rs, restricted_token.rs, supervisor.rs) | Lives under `exec_strategy_windows/`, not literally `exec_strategy/` — **the CLAUDE.md rule names `exec_strategy/` (Unix) explicitly, not `exec_strategy_windows/`.** These files are `#[cfg(target_os = "windows")]`-only content; they don't carry the reverse (Unix) cfg branches CLAUDE.md's rule is protecting against. | **UNRESOLVED — literal reading says NO** (the directory name doesn't match `exec_strategy/`), but the *spirit* of the rule (protect Unix cfg-gated code from an untested-on-this-host regression) doesn't apply the same way to Windows-only files edited on a Windows host. Recommend the planner treat this as "N/A — Windows-only files, verified live on this Windows dev host instead," and confirm this reading explicitly rather than silently assuming either way. |
| `crates/nono/src/sandbox/windows.rs`, `crates/nono/src/sandbox/mod.rs` | `sandbox/mod.rs` gates the Windows facade functions behind `#[cfg(target_os = "windows")]` per-function (confirmed throughout §A citations) but the file also compiles the Linux/macOS facades — **need to verify presence of `#[cfg(target_os = "linux")]`/`"macos"` literally in `sandbox/mod.rs` before ruling in/out.** Not fully verified in this pass. | UNRESOLVED for `sandbox/mod.rs` specifically — flag as a Wave-0 grep task. `sandbox/windows.rs` itself is Windows-only content (like `exec_strategy_windows/`), same reasoning as above. |
| `crates/nono/src/machine_policy.rs` | Contains a non-Windows stub (`:701`: "Non-Windows stub: returns `Ok(None)`") implying `#[cfg(not(target_os = "windows"))]` or `#[cfg(target_os = "windows")]` branching exists, but the literal `#[cfg(target_os = "linux")]`/`"macos"` pattern was not directly grepped in this pass. | UNRESOLVED — needs a direct grep before the plan finalizes its cross-target-gate task list. |
| `crates/nono/src/diagnostic/codes.rs`, `crates/nono/src/error.rs` | Confirmed: `error.rs` has `#[cfg(target_os = "linux")]` at the tail of `diagnostic_code()`'s match (`:445-446`, the `Landlock`/`LandlockPath` arm). `codes.rs` — not grepped for cfg directly, but it's a plain data-type file with no `windows_sys` imports; unlikely to carry platform cfg. | **`error.rs`: YES, confirmed.** `codes.rs`: likely no, but not directly verified — low risk given its content (pure enums/structs). |
| `crates/nono-cli/src/telemetry/mod.rs`, `event.rs` | Not grepped for cfg in this pass; `telemetry/windows.rs` (a submodule, `mod.rs:30`) is presumably the Windows-specific half, implying the split already isolates platform code away from `mod.rs`/`event.rs`. | UNRESOLVED — recommend a direct grep in Wave 0 if telemetry changes end up non-trivial; low risk given the module is already split by platform (`syslog`, `windows` submodules per `mod.rs:29-31`). |

**Net assessment:** CONTEXT.md D-11's three named files (`execution_runtime.rs`, `cli.rs`,
`output.rs`) are all confirmed correct — the cross-target clippy MUST genuinely applies to all three.
The `exec_strategy_windows/` and `sandbox/windows.rs` files are a **judgment call** the planner
should make explicitly rather than silently assume either way, since they don't literally match
CLAUDE.md's named `exec_strategy/` directory pattern.

## Validation Architecture

### Test Framework

| Property | Value |
|---|---|
| Framework | Rust built-in test runner (`cargo test`), `#[cfg(all(test, target_os = "windows"))]`-gated modules for Windows-only assertions, plus `crates/nono-cli/tests/*.rs` integration tests that spawn `env!("CARGO_BIN_EXE_nono")` subprocesses |
| Config file | none — no `pytest.ini`/`jest.config`-equivalent; test discovery is Cargo-native (`[[test]]` implicit from `tests/*.rs`, `#[test]` fns in-crate) |
| Quick run command | `cargo test -p nono-cli --lib exec_strategy_windows -- --test-threads=1` (Windows-only unit tests are single-process-state-sensitive per the crate's env-var save/restore discipline in CLAUDE.md) |
| Full suite command | `make test` (workspace) / `make ci` (clippy + fmt + tests) per CLAUDE.md |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|---|---|---|---|---|
| CINT-01 | The registry compiles and every row cites a real, current call site | unit (registry self-check: does each cited call-site string exist in the named file?) | `cargo test -p nono-cli registry_call_sites_exist -x` | ❌ Wave 0 — new file, e.g. `crates/nono-cli/src/exec_strategy_windows/layer_registry.rs` + its test module |
| CINT-01 | SPEC drift check: `proj/SPEC-windows-fail-direction-contract.md` matches the registry (D-01) | unit or `tests/*.rs` source-scan | `cargo test -p nono-cli spec_matches_registry -x` | ❌ Wave 0 — depends on whether SPEC is generated or drift-checked (Claude's discretion, CONTEXT.md) |
| CINT-02 | A forced-unavailable layer causes abort or downgrade, never a silent "enforcing" claim | integration (spawn `nono run` under `NONO_TEST_HARNESS`/fault-injection feature with one layer forced off; assert exit behavior + stderr/event content) | `cargo test -p nono-cli --features layer-fault-injection attestation_downgrades_on_forced_unavailable -x` | ❌ Wave 0 — needs the generalized seam from §E to exist first (D-29/D-30 land before or alongside CINT-02's own tests) |
| CINT-02 | Downgraded claim reaches all three D-27 channels (banner, diagnostic code, telemetry event) | unit × 3 (one per channel) | `cargo test -p nono-cli downgrade_reaches_banner downgrade_reaches_diagnostic_code downgrade_reaches_telemetry_event -x` | ❌ Wave 0 |
| CINT-03 | Every registry row has a forced-unavailable test (discovery meta-test) | unit, source-scan per §H | `cargo test -p nono-cli every_registry_row_has_a_test -x` | ❌ Wave 0 |
| CINT-03 | A row that cannot be tested on an ordinary (non-elevated, non-domain-joined, etc.) host is a loud, named gap (D-31) | unit — assert the meta-test's "host-gated, manual-verification" list is non-silent (e.g. printed, not `#[ignore]`d without a reason string) | `cargo test -p nono-cli host_gated_rows_are_loud -x` | ❌ Wave 0 |

### Sampling Rate

- **Per task commit:** the relevant `#[cfg(all(test, target_os = "windows"))]` unit module for the
  file just touched (fast, single-file scope, matches the existing convention throughout
  `exec_strategy_windows/`).
- **Per wave merge:** `cargo test -p nono-cli --features layer-fault-injection` (full crate, with the
  new fault-injection feature enabled so the forced-unavailable tests actually run) + `cargo test -p
  nono --lib` (library-side diagnostic-code/error additions).
- **Phase gate:** `make ci` (clippy `-D warnings -D clippy::unwrap_used`, `cargo fmt --all -- --check`,
  full workspace tests) green, PLUS the two local cross-target clippy gates for every file flagged
  `YES` in §I, PLUS a manual pass on real Windows hardware for the D-31 host-gated rows (see below).

### Automatable on this host vs. host-gated (D-31 loud-gap candidates)

| Validation | Automatable here? | Reason |
|---|---|---|
| Registry/SPEC drift check, diagnostic-code additions, telemetry event shape | YES — pure Rust, no elevation/hardware needed | Runs in any `cargo test` invocation on this dev host |
| Restricted token / mandatory label / DACL forced-unavailable tests | YES — `CreateProcess*` + token/ACE manipulation works non-elevated on this dev host (existing `#[cfg(all(test, target_os = "windows"))]` modules already do this) | Confirmed by the existing `rb3_gate_tests`/`interpreter_resolve_tests` modules already passing on this host |
| AppContainer forced-unavailable test | Likely YES, non-elevated (AppContainer profile registration doesn't require admin per the durable lesson `windows_appcontainer_wfp_validated.md`) | Existing precedent: broker's own AppContainer spawn path already has test coverage patterns (`broker_dispatch_tests`) |
| WFP forced-unavailable test | **PARTIALLY host-gated** — requires the `nono-wfp-service` to be installed/running (admin-only install step, per `wfp_confined_egress_and_daemon_gate.md`); the *toggle* (`set_windows_wfp_test_force_ready`/its D-30 successor) can be exercised without a live service, but confirming the *contracted outcome* (abort vs. degrade) end-to-end needs the real service. Recommend: automate the toggle-level unit test here; name the full end-to-end WFP-forced-unavailable-under-live-service test as a **named manual-verification item** per D-31, not a silent `#[ignore]`. |
| Job Object forced-unavailable test | YES — `IsProcessInJob` and `AssignProcessToJobObject` both work non-elevated | Already proven by `broker_dispatch_tests` |
| Minifilter row | **Structurally untestable — no driver exists.** Per D-31, this must be a named, explicit "not applicable / structurally absent" entry, never silently skipped, citing ADR-65. | N/A by construction |
| Daemon-path (`nono-agentd`) forced-unavailable tests | **UNRESOLVED — likely host-gated or requiring a running daemon process**, since `agent_daemon/launch.rs`'s own test module already shows patterns like `launch_agent_inserts_into_daemon_state` (`:1992`) that may or may not need live Windows kernel objects vs. mockable state. Planner must verify in Wave 0 whether the existing daemon test harness can drive a forced-unavailable scenario without admin/service dependencies. |

### Observable signal proving each success criterion

- **SC1** (one document names every layer): `proj/SPEC-windows-fail-direction-contract.md` exists,
  parses, and the drift-check test (`spec_matches_registry`) is green.
- **SC2** (forced-unavailable → abort or visible downgrade, never silent over-claim): the
  `attestation_downgrades_on_forced_unavailable`-family integration tests pass for every non-host-gated
  row, AND the D-31 loud-gap list is non-empty-but-explicit for the WFP/daemon rows still pending
  manual verification.
- **SC3** (every contract row has a test): `every_registry_row_has_a_test` meta-test is green.
- **SC4** (contract/code disagreements resolved or recorded in-phase): the §G candidate table above is
  either closed out (code fixed) or promoted into the SPEC's "Contract vs. code discrepancies" section
  with a named successor, per D-13/D-14/D-15 — observable as a non-empty, cited section in the SPEC
  document itself.

## Open Questions for the Planner

1. **Does WFP's install-time `installed_filter_count` confirmation (§C, §Summary-3) satisfy D-18's
   `confirmed` state, or does it still belong in `established, not independently observable`?**
   - What we know: the elevated service reports a count back over IPC, and the supervisor already
     fails closed on a bad count, before spawn.
   - What's unclear: whether a report *from* the enforcing component (rather than an independent
     third-party observation of kernel state) satisfies the spirit of D-19 ("the supervisor attests;
     the confined process never does") — the WFP *service* isn't the confined process, but it also
     isn't a neutral third party the way `IsProcessInJob` querying the child's own kernel object is.
   - Recommendation: treat it as its own named state if the four-value D-18 vocabulary doesn't fit
     cleanly, OR explicitly document "confirmed-by-report-from-the-enforcing-component" as a
     sub-case of `confirmed` with a citation to this distinction, so a future reader doesn't conflate
     it with `IsProcessInJob`-style independent observation.

2. **Is `FirewallRulesNetworkBackend` (legacy AppID/firewall-rule fallback) still reachable in
   production, and if so, what is its own fail-direction?** (SC4-3, §G)
   - What we know: the type and its `Drop` impl exist and compile.
   - What's unclear: the exact dispatch logic selecting it over `WfpNetworkBackend` was not traced to
     completion in this research pass.
   - Recommendation: Wave-0 task — trace `WindowsNetworkBackend` backend selection before writing the
     WFP contract row, since this may be a second WFP-adjacent row entirely (or dead code worth
     removing, which would itself be a small SC4 fix).

3. **Where does the daemon path (`nono-agentd`) fit the D-30 fault-injection seam and D-32 meta-test
   given `agent_daemon/launch.rs`'s explicit non-dependency on `exec_strategy_windows/`?**
   - What we know: it's structurally independent code with its own WFP/DACL gates already in place.
   - What's unclear: whether the registry's per-row "enforcing call site" citation should point at
     BOTH `launch.rs` and `agent_daemon/launch.rs` sites for layers present on both paths (dual
     citation), or whether the registry needs an explicit per-entry-path call-site field, not just a
     single citation column — this is a real shape question for the registry's column design (D-08's
     "Claude's Discretion: registry table layout... provided expectancy-per-arm... are representable"
     — the entry-path axis needs the same treatment as the token-arm axis).
   - Recommendation: extend the registry schema to carry (entry_path, call_site) pairs per layer,
     not a single call_site column, given the confirmed 3-independent-implementation reality (§Summary-1).

4. **Does the new `SecurityEvent` (or a reused `TelemetryDegraded` variant) need a new field to name
   the specific layer, and if so, does that violate D-28's "layer-specific downgrade detail stays off
   channels the confined process can read"?**
   - What we know: `TelemetryDegraded` (EventID 10005) already exists and is a plausible reuse
     candidate; `SecurityEvent`'s struct has no generic "detail" free-text field beyond `host`
     (network-only) and `path_hash`/`path_category` (path-only).
   - What's unclear: whether telemetry events are considered part of "the operator's channel" (D-28
     says specific layer names are fine there) or could theoretically be read by a sufficiently
     privileged confined process (they're written to ETW/Application Event Log, both of which a
     Medium-IL or higher process could plausibly read depending on ACLs — this was not verified).
   - Recommendation: verify ETW/Application-Log ACLs are NOT readable by a Low-IL/AppContainer child
     before deciding the telemetry channel can safely carry the specific layer name; if unverified,
     default to coarse wording on this channel too and reserve specifics for a channel confirmed
     operator-only (e.g. a local audit-events file the child cannot open).

5. **`sandbox/mod.rs`, `sandbox/windows.rs`, `machine_policy.rs`, and `telemetry/mod.rs`/`event.rs`
   cross-target cfg status** (§I) — several UNRESOLVED cells in the blast-radius table need a direct
   grep before the plan finalizes which cross-target gates apply to which new/touched files.

## Sources

### Primary (HIGH confidence — direct file:line reads of this repository)
- `crates/nono-cli/src/exec_strategy_windows/mod.rs` (full read)
- `crates/nono-cli/src/exec_strategy_windows/launch.rs` (targeted reads: token arm, spawn function, drop comments, ~lines 150-460, 1230-1490, 2000-2160)
- `crates/nono-cli/src/exec_strategy_windows/network.rs` (targeted reads: WFP probe/activation status enums and dispatch, ~lines 340-450, 1650-1800)
- `crates/nono-cli/src/exec_strategy_windows/{restricted_token,labels_guard,dacl_guard}.rs` (symbol greps + targeted reads)
- `crates/nono-cli/src/exec_strategy_windows/supervisor.rs` (WindowsSupervisedChild, ~lines 60-146)
- `crates/nono-shell-broker/src/main.rs` (targeted read, ~lines 460-670)
- `crates/nono-cli/src/agent_daemon/launch.rs` (targeted reads, module doc + gate-ordering greps)
- `crates/nono-cli/src/execution_runtime.rs`, `cli.rs`, `output.rs` (cfg-branch greps, targeted reads of `print_banner`/`format_scope_status`)
- `crates/nono/src/sandbox/windows.rs`, `crates/nono/src/sandbox/mod.rs` (targeted reads: `apply()`, `validate_windows_launch_paths`, `windows_filesystem_policy`)
- `crates/nono/src/sandbox/linux.rs` (targeted reads/greps: `RulesetStatus` match, `SeccompOpts` doc, ABI degrade)
- `crates/nono-cli/src/telemetry/{mod,event}.rs` (targeted reads)
- `crates/nono/src/diagnostic/codes.rs` (full read), `crates/nono/src/error.rs` (targeted read, `diagnostic_code`/`remediation`)
- `crates/nono/src/machine_policy.rs` (targeted reads/greps)
- `crates/nono-cli/tests/{broker_authenticode,resl_supervisor_drain,trust_policy_template}.rs` and 6 other `CARGO_MANIFEST_DIR`-using test files (grep + targeted reads)
- `crates/nono-cli/Cargo.toml`, `crates/nono/Cargo.toml`, `crates/nono-proxy/Cargo.toml`, `crates/nono-shell-broker/Cargo.toml`, `bindings/c/Cargo.toml` (`[features]` grep)
- `.planning/phases/117-.../117-CONTEXT.md`, `.planning/REQUIREMENTS.md`, `.planning/ROADMAP.md` (full/targeted reads)
- `.planning/templates/cross-target-verify-checklist.md` (existence confirmed)

### Secondary (MEDIUM confidence)
- Rust struct-field drop-order semantics — WebSearch cross-referencing `rust-lang/rfcs#744`,
  `rust-lang/rust#16661`, and `blog.bryantluk.com`'s explainer, consistent across all sources with the
  code-observed behavior in this tree.

### Tertiary (LOW confidence / not independently verified)
- Whether ETW/Application Event Log entries are readable by a Low-IL/AppContainer confined process
  (Open Question 4) — not checked in this session; flagged for planner verification.
- Exact `WindowsNetworkBackend` selection dispatch logic (`FirewallRulesNetworkBackend` vs.
  `WfpNetworkBackend`) — traced partially, not to completion (Open Question 2 / SC4-3).
- Whether `agent_daemon/launch.rs`'s existing test module (`launch_agent_inserts_into_daemon_state`
  etc., `:1992`+) can drive forced-unavailable scenarios without live Windows service dependencies
  (Validation Architecture, daemon-path row) — not verified.

## Metadata

**Confidence breakdown:**
- Layer inventory / D-10 derivation (§A): HIGH — every row grounded in direct file:line citations,
  cross-checked against the actual `?`/`Err` control flow, not just doc comments.
- Execution arm matrix / D-08 (§B): HIGH — the two-axis finding is a direct consequence of reading
  `agent_daemon/launch.rs`'s own module doc, not inference.
- Attestation gate point / D-21-23 (§D): HIGH for the nono-cli and broker sites (read in full); MEDIUM
  for exact daemon-path line numbers (grepped, not fully read line-by-line).
- Fault-injection seam / D-29-30 (§E): HIGH — both the current shape and the precedent are directly
  read, not inferred.
- SC4 discrepancy hunt (§G): MEDIUM — SC4-1/2/4 are HIGH confidence (fully traced); SC4-3 is
  explicitly flagged UNRESOLVED/needs-Wave-0-work, per the evidence-discipline rule that "I don't know
  yet" is itself valid, cited output.
- Cross-target blast radius (§I): MEDIUM — the three CONTEXT.md-named files are HIGH confidence
  (directly grepped); several additional files are flagged UNRESOLVED rather than guessed.

**Research date:** 2026-08-09
**Valid until:** ~14 days (fast-moving phase touching actively-edited Windows exec-strategy code;
re-verify line numbers before planning if this research is consumed more than two weeks stale).
