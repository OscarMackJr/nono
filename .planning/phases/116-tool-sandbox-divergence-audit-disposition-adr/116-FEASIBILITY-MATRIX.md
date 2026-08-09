# Phase 116: Tool-Sandbox Feasibility Matrix

**Date measured:** 2026-08-09

**All grep commands, hit counts, and symbol citations in this document were run and verified on
the date above; every cell's evidence traces to that single date rather than being repeated
per-row (D-16 — command + hit count + date). A reader re-running any command after this date
should expect drift and re-verify against fork HEAD / upstream `v0.71.0` at that time.**

This document is factual grounding only — an inventory of what exists, not an argument for either
disposition pole. It answers "is it possible", not "should we build it". Scoring (D-09) and the
verdict (D-10/D-11) are Plan 116-05/116-06's job; this document supplies their evidence and must
not pre-argue either side (D-10). No row below states or implies a preference for Pole A (adopt +
build `platform/windows.rs`) or Pole B (formalize the fork-native PreToolUse-hook + Low-IL broker
path).

---

## Fork Primitive Inventory

**Grep pattern used (D-16 evidence discipline):**
```bash
grep -nE "^\s*pub(\(crate\)|\(super\)|\(in [^)]+\))?\s+(fn|struct|enum|const|type)" <file>
```
This pattern was tested against `dacl_guard.rs` (a file already known to export symbols) before
being trusted on `supervisor.rs`: it returned 7 hits on `dacl_guard.rs`, confirming the pattern
itself works. Per Pitfall 2 (`116-RESEARCH.md` §3a), a naive `pub|pub(crate)`-only pattern returns
**zero** hits on `supervisor.rs` despite the file being 241 KB and load-bearing, because its entire
public surface is `pub(super)`. The broadened pattern above (which includes `pub(super)` and
`pub(in ...)`) returns **23 hits** on `supervisor.rs` — confirmed nonzero, matching the Pitfall-2
finding.

### Per-file hit counts (all 10 files in scope, 2026-08-09)

| File | Grep hit count | Note |
|------|----------------|------|
| `exec_strategy_windows/mod.rs` | 27 | |
| `exec_strategy_windows/restricted_token.rs` | 4 | |
| `exec_strategy_windows/labels_guard.rs` | 2 | |
| `exec_strategy_windows/dacl_guard.rs` | 7 | pattern-validation file |
| `exec_strategy_windows/network.rs` | 54 | almost entirely `pub(super)` |
| `exec_strategy_windows/launch.rs` | 22 | |
| `exec_strategy_windows/supervisor.rs` | **23** | **all `pub(super)` — zero under a naive `pub\|pub(crate)` grep (Pitfall 2, confirmed live)** |
| `claude_code_hook.rs` | 1 | |
| `hooks.rs` | 5 | |
| `execution_runtime.rs` | 2 | |
| `nono-shell-broker/src/main.rs` | 4 | |
| **Total** | **151** | |

### Full symbol table

Visibility legend: `p` = `pub`, `pc` = `pub(crate)`, `ps` = `pub(super)`.

**`exec_strategy_windows/mod.rs`** (27 symbols — `ExecStrategy`/`ThreadingContext` enums,
`ExecConfig`/`SupervisorConfig` structs, WFP report structs, direct/supervised execute entry
points, integrity/job-object/BFE probes):

| Line | Symbol | Vis | Capability |
|------|--------|-----|------------|
| 81 | `to_u16_null_terminated()` | pc | UTF-16 arg/env encoding helper |
| 99 | `system32_exe()` | ps | System32 path resolution |
| 107 | `resolve_program()` | p | Program path resolution |
| 117 | `ThreadingContext` (enum) | p | PTY vs non-PTY dispatch context |
| 125 | `ExecStrategy` (enum) | p | Direct/Monitor/Supervised strategy selector |
| 131 | `ExecConfig<'a>` (struct) | p | Per-launch config, incl. `allowed_env_vars`/`denied_env_vars` (env scrubbing primitive, line 171/182) |
| 202 | `SupervisorConfig<'a>` (struct) | p | Supervised-launch config |
| 259 | `WindowsSupervisorDenyAllApprovalBackend` (struct) | p | Fail-closed approval backend |
| 584 | `set_windows_wfp_test_force_ready()` | pc | Test-only WFP readiness override |
| 604 | `WindowsWfpReadinessReport` (struct) | pc | WFP readiness probe result |
| 615 | `WindowsWfpInstallReport` (struct) | pc | WFP service install result |
| 621 | `WindowsWfpDriverInstallReport` (struct) | pc | WFP driver install result |
| 627 | `WindowsWfpStartReport` (struct) | pc | WFP service start result |
| 633 | `WindowsWfpDriverStartReport` (struct) | pc | WFP driver start result |
| 639 | `WindowsWfpUninstallReport` (struct) | pc | WFP uninstall result |
| 645 | `ProcessContainment` (struct) | pc | Job Object + token containment handle |
| 664 | `JOB_OBJECT_QUERY` (const) | pc | Job Object access-mask constant |
| 665 | `JOB_OBJECT_TERMINATE` (const) | pc | Job Object access-mask constant |
| 673 | `is_admin_process()` | pc | Elevation check |
| 698 | `probe_job_object_permissions()` | pc | Job Object capability probe |
| 722 | `probe_integrity_level_support()` | pc | Mandatory-label capability probe |
| 793 | `probe_bfe_service_status()` | pc | Base Filtering Engine status probe |
| 841 | `cleanup_windows_network_enforcement_artifacts()` | pc | WFP cleanup |
| 845 | `execute_direct()` | p | Direct exec strategy entry point |
| 884 | `execute_supervised()` | p | Supervised exec strategy entry point |
| 1060 | `read_distlib_shebang()` | pc | Interpreter shebang resolution |
| 1091 | `resolve_interpreter_paths()` | pc | `windows_interpreters` profile resolution |

**`exec_strategy_windows/restricted_token.rs`** (4 symbols):

| Line | Symbol | Vis | Capability |
|------|--------|-----|------------|
| 9 | `RestrictedToken` (struct) | ps | WRITE_RESTRICTED token handle |
| 21 | `generate_session_sid()` | pc | Synthetic per-session restricting SID |
| 51 | `generate_app_container_name()` | pc | AppContainer moniker (`nono.session.<uuid>`) generation |
| 55 | `create_restricted_token_with_sid()` | ps | WRITE_RESTRICTED token construction |

**`exec_strategy_windows/labels_guard.rs`** (2 symbols):

| Line | Symbol | Vis | Capability |
|------|--------|-----|------------|
| 57 | `AppliedLabelsGuard` (struct) | pc | Mandatory-label (Low-IL) grant tracking |
| 83 | `AppliedLabelsGuard::snapshot_and_apply()` | pc | Applies Low-writable mandatory labels to a path set |

**`exec_strategy_windows/dacl_guard.rs`** (7 symbols):

| Line | Symbol | Vis | Capability |
|------|--------|-----|------------|
| 72 | `AppliedDaclGrantsGuard` (struct) | pc | Per-path DACL ACE grant tracking |
| 92 | `AppliedDaclGrantsGuard::snapshot_and_apply()` | pc | Applies WRITE-class DACL ACEs to a granted path set |
| 222 | `AppliedAncestorTraverseGuard` (struct) | pc | Ancestor-directory traverse-grant tracking |
| 236 | `AppliedAncestorTraverseGuard::snapshot_and_apply()` | pc | Grants traverse on ancestor dirs so a deep grant is reachable |
| 359 | `AppliedAncestorReadAttributesGuard` (struct) | pc | Ancestor read-attributes grant tracking |
| 375 | `AppliedAncestorReadAttributesGuard::snapshot_and_apply()` | pc | Grants read-attributes on ancestor dirs |
| 401 | `AppliedAncestorReadAttributesGuard::snapshot_and_apply_targets()` | pc | Multi-target variant of the above |

**`exec_strategy_windows/network.rs`** (54 symbols — almost entirely `pub(super)`; the WFP
service/driver lifecycle + probe/status/activation-request builder surface):

| Line | Symbol | Vis | Capability |
|------|--------|-----|------------|
| 36 | `run_netsh_firewall()` | ps | netsh firewall invocation |
| 53 | `classify_netsh_firewall_failure()` | ps | netsh failure classification |
| 71 | `delete_firewall_rule()` | ps | Firewall rule cleanup |
| 77 | `unique_windows_firewall_rule_suffix()` | ps | Per-run rule naming |
| 83 | `stage_program_for_blocked_network_launch()` | ps | Network-blocked launch staging |
| 172 | `cleanup_network_enforcement_staging()` | ps | Staging cleanup |
| 176 | `cleanup_stale_network_enforcement_artifacts()` | ps | Stale-artifact cleanup |
| 191 | `current_wfp_probe_config()` | ps | WFP probe config resolution |
| 214 | `format_wfp_service_command()` | ps | Service-command formatting |
| 222 | `run_sc_query()` | ps | `sc.exe query` invocation |
| 234 | `run_sc_command()` | ps | `sc.exe` invocation |
| 246 | `sc_create_conflict_is_registered()` | ps | Service-registration conflict detection |
| 253 | `build_wfp_service_create_args()` | ps | Service-create arg builder |
| 268 | `build_wfp_service_description_args()` | ps | Service-description arg builder |
| 276 | `build_wfp_driver_create_args()` | ps | Driver-create arg builder |
| 297 | `build_wfp_driver_description_args()` | ps | Driver-description arg builder |
| 305 | `build_wfp_service_start_args()` | ps | Service-start arg builder |
| 309 | `build_wfp_driver_start_args()` | ps | Driver-start arg builder |
| 313 | `build_wfp_service_stop_args()` | ps | Service-stop arg builder |
| 317 | `build_wfp_service_delete_args()` | ps | Service-delete arg builder |
| 321 | `build_wfp_driver_stop_args()` | ps | Driver-stop arg builder |
| 325 | `build_wfp_driver_delete_args()` | ps | Driver-delete arg builder |
| 329 | `parse_windows_service_state()` | ps | Service-state parsing |
| 353 | `build_wfp_probe_status()` | ps | Probe-status assembly |
| 381 | `probe_wfp_backend_status_with_config()` | ps | Backend-status probe (injectable config) |
| 407 | `describe_wfp_runtime_activation_failure()` | ps | Activation-failure diagnostic |
| 460 | `describe_windows_network_runtime_target()` | ps | Runtime-target diagnostic |
| 496 | `build_wfp_runtime_activation_request()` | ps | Per-session activation request builder (carries `localhost_port_ranges`) |
| 541 | `build_wfp_target_activation_request()` | ps | Per-target activation request builder |
| 556 | `build_wfp_runtime_cleanup_request()` | ps | Runtime cleanup request builder |
| 575 | `cleanup_wfp_service_managed_enforcement_with_runner()` | ps | Service-managed cleanup (injectable runner) |
| 602 | `describe_wfp_probe_status_for_setup()` | ps | Setup-flow probe-status diagnostic |
| 661 | `describe_wfp_service_status_for_setup()` | ps | Setup-flow service-status diagnostic |
| 723 | `describe_wfp_driver_status_for_setup()` | ps | Setup-flow driver-status diagnostic |
| 798 | `describe_wfp_next_action_for_setup()` | ps | Setup-flow next-action diagnostic |
| 838 | `run_wfp_runtime_request()` | ps | Runtime request dispatch |
| 885 | `run_wfp_runtime_probe_with_request()` | ps | Runtime probe dispatch |
| 897 | `parse_wfp_runtime_probe_status()` | ps | Runtime probe-status parsing |
| 955 | `describe_wfp_runtime_probe_failure()` | ps | Runtime probe-failure diagnostic |
| 969 | `install_windows_wfp_service_with_runner()` | ps | Service install (injectable runner) |
| 1050 | `install_windows_wfp_service()` | pc | Service install |
| 1055 | `install_windows_wfp_driver_with_runner()` | ps | Driver install (injectable runner) |
| 1139 | `install_windows_wfp_driver()` | pc | Driver install |
| 1144 | `start_windows_wfp_driver_with_runner()` | ps | Driver start (injectable runner) |
| 1219 | `start_windows_wfp_driver()` | pc | Driver start |
| 1282 | `uninstall_windows_wfp_with_runner()` | ps | Uninstall (injectable runner) |
| 1363 | `uninstall_windows_wfp()` | pc | Uninstall |
| 1368 | `start_windows_wfp_service_with_runner()` | ps | Service start (injectable runner) |
| 1442 | `start_windows_wfp_service()` | pc | Service start |
| 1447 | `probe_windows_wfp_readiness()` | pc | Top-level readiness probe |
| 1498 | `select_network_backend()` | ps | Backend selection |
| 1613 | `install_wfp_network_backend_with_runner()` | ps | Backend install (injectable runner) |
| 1802 | `install_wfp_network_backend()` | ps | Backend install |
| 1817 | `prepare_network_enforcement()` | ps | Top-level network-enforcement prep |

**`exec_strategy_windows/launch.rs`** (22 symbols — process containment, token-arm selection,
resource limits, command-line/env-block construction, broker authenticode verification):

| Line | Symbol | Vis | Capability |
|------|--------|-----|------------|
| 140 | `DetachedStdioPipes` (struct) | ps | Detached stdio pipe handles |
| 156 | `JobObjectHandle::create()` | p | Job Object creation |
| 273 | `create_process_containment()` | pc | Top-level containment assembly |
| 356 | `assign_failure_message()` | ps | GLE-to-message mapping |
| 375 | `apply_process_handle_to_containment()` | pc | Post-spawn containment binding |
| 395 | `terminate_suspended_process()` | ps | Suspended-process teardown |
| 408 | `STATUS_TIMEOUT_EXIT_CODE` (const) | ps | Timeout exit-code constant |
| 420 | `terminate_job_object()` | pc | Job Object termination |
| 438 | `resume_contained_process()` | ps | Resume a suspended contained process |
| 475 | `apply_resource_limits()` | ps | Job Object resource-limit application |
| 575 | `prepare_runtime_hardened_args()` | ps | Hardened-args assembly |
| 681 | `build_child_env()` | ps | Child environment construction (env scrubbing site) |
| 1097 | `build_windows_environment_block()` | ps | Win32 env-block encoding |
| 1125 | `quote_windows_arg()` | ps | Win32 argv quoting |
| 1175 | `build_command_line()` | ps | Command-line construction |
| 1200 | `build_broker_command_line()` | ps | Broker command-line construction |
| 1216 | `should_use_low_integrity_windows_launch()` | ps | Low-IL launch decision |
| 1237 | `WindowsTokenArm` (enum) | ps | **The 5-arm token-construction cascade (Null / WriteRestricted / LowIlPrimary / BrokerLaunch / BrokerLaunchNoPty) — see WRITE_RESTRICTED Worked Example below** |
| 1296 | `select_windows_token_arm()` | ps | Pure decision function for the cascade above |
| 1346 | `spawn_windows_child()` | ps | Top-level child-spawn orchestration |
| 2236 | `verify_broker_authenticode()` | pc | Broker Authenticode signature verification (broker-trust gate) |
| 2304 | `is_windows_detached_launch()` | pc | Detached-launch environment check |

**`exec_strategy_windows/supervisor.rs`** (23 symbols, ALL `pub(super)` — confirms Pitfall 2 live):

| Line | Symbol | Vis | Capability |
|------|--------|-----|------------|
| 48 | `SendableHandle` (struct) | ps | Send-safe HANDLE wrapper |
| 54 | `WindowsSupervisorLifecycleState` (enum) | ps | Supervisor lifecycle state machine |
| 64 | `WindowsSupervisorLifecycleState::label()` | ps | State label |
| 77 | `WindowsSupervisedChild` (enum) | ps | Direct vs broker-mediated child handle |
| 93 | `WindowsSupervisedChild::process_handle_raw()` | ps | Raw HANDLE accessor |
| 124 | `WindowsSupervisedChild::poll_exit_code()` | ps | Non-blocking exit-code poll |
| 130 | `WindowsSupervisedChild::terminate()` | ps | Child termination |
| 204 | `WindowsSupervisorRuntime` (struct) | ps | Top-level supervisor runtime state |
| 312 | `compute_deadline()` | ps | Timeout-deadline computation |
| 327 | `WindowsSupervisorRuntime::initialize()` | ps | Runtime initialization |
| 433 | `WindowsSupervisorRuntime::attach_detached_stdio()` | ps | Detached-stdio attachment |
| 442 | `WindowsSupervisorRuntime::start_streaming()` | ps | I/O streaming start |
| 695 | `WindowsSupervisorRuntime::set_child_broker_target()` | ps | Broker-target binding |
| 1286 | `WindowsSupervisorRuntime::transport_name()` | ps | IPC transport identifier |
| 1290 | `WindowsSupervisorRuntime::run_child_event_loop()` | ps | Top-level child event loop |
| 1390 | `WindowsSupervisorRuntime::startup_failure()` | ps | Startup-failure diagnostic path |
| 1402 | `WindowsSupervisorRuntime::command_failure()` | ps | Command-failure diagnostic path |
| 1414 | `WindowsSupervisorRuntime::shutdown()` | ps | Runtime shutdown |
| 1419 | `WindowsSupervisorRuntime::pty()` | ps | PTY pair accessor |
| 1428 | `WindowsSupervisorRuntime::detached_stdio()` | ps | Detached-stdio accessor |
| 1453 | `initialize_supervisor_control_channel()` | ps | Control-channel (Named Pipe) init |
| 1462 | `open_windows_supervisor_path()` | ps | Supervisor path resolution |
| 2019 | `handle_windows_supervisor_message()` | ps | Supervisor IPC message dispatch |

**`claude_code_hook.rs`** (1 symbol):

| Line | Symbol | Vis | Capability |
|------|--------|-----|------------|
| 20 | `run()` | pc | PreToolUse hook entry point (Pole B's Claude-Code-specific dispatch) |

**`hooks.rs`** (5 symbols):

| Line | Symbol | Vis | Capability |
|------|--------|-----|------------|
| 17 | `NONO_HOOK_SH` (const) | p | Embedded Bash hook script (Unix) |
| 19 | `NONO_TOOL_HOOK_PS1` (const) | p | Embedded PowerShell hook script (Windows) |
| 34 | `HookInstallResult` (enum) | p | Hook-install outcome |
| 53 | `install_hooks()` | p | Top-level hook installation |
| 550 | `install_profile_hooks()` | p | Profile-scoped hook installation |

**`execution_runtime.rs`** (2 symbols):

| Line | Symbol | Vis | Capability |
|------|--------|-----|------------|
| 73 | `execution_start_dir()` | pc | Launch-time CWD resolution |
| 112 | `execute_sandboxed()` | pc | Top-level sandboxed-execution entry point — carries the `prefers_low_il_broker`-driven `BrokerLaunchNoPty` XOR `WriteRestricted` split (lines 540-623; see Worked Example) |

**`nono-shell-broker/src/main.rs`** (4 symbols):

| Line | Symbol | Vis | Capability |
|------|--------|-----|------------|
| 58 | `BrokerArgs` (struct) | p | Broker CLI argument shape |
| 94 | `parse_args()` | p | Broker argument parsing |
| 224 | `build_command_line()` | p | Broker's own command-line construction |
| 280 | `run()` | p | Medium-IL broker entry point; self-degrades to Low-IL and spawns the confined child — **the fork's existing "spawn a mediating trampoline binary" primitive**, the direct Windows analog of upstream's exec-shim pattern |

**Additional finding beyond the 10-file scope (D-16 "discover, don't confirm" discipline):** a
tree-wide grep for `url_shim\|open_url_runtime\|ToolSandboxOpenUrl` (not limited to the files
above) surfaces `crates/nono-cli/src/open_url_runtime.rs` and
`crates/nono-cli/src/open_url_runtime_windows.rs` — the fork already has its own url-open-shim
analog on Unix (`run_open_url_helper()`, IPC-based via `nono::supervisor::UrlOpenRequest`,
`crates/nono/src/supervisor/types.rs:451`), but the Windows arm
(`open_url_runtime_windows.rs::run_open_url_helper()`) is an explicit stub returning
`NonoError::UnsupportedPlatform("Windows delegated browser-open flows are not available yet...")`.
This is load-bearing evidence for the D-07 matrix's `url_shim` row below — not a symbol the
interfaces block named, found only because the grep searched the tree instead of confirming a
pre-named path.

### Supplementary fork primitives (outside the 10-file scope, cited by the D-07 matrix below)

Four additional fork files, not in the 10-file interfaces-block scope but directly cited by D-07
matrix rows below, so every D-07 citation is verbatim-traceable within this document (D-16). Each
was grepped live 2026-08-09:

| File | Grep command | Symbol | Line | Capability |
|------|---------------|--------|------|------------|
| `crates/nono/src/keystore.rs` | `grep -c "^pub " crates/nono/src/keystore.rs` (29 hits — public API is large; credential-scheme validators/redactors dominate) | `load_secrets()` | 266 | Multi-scheme (`op://`/`bw://`/`keyring://`/`env://`/`file://`/`cmd://`) secret loading to `Zeroizing<String>` |
| `crates/nono/src/keystore.rs` | (same) | `load_secret_by_ref()` | 305 | Single-reference secret resolution |
| `crates/nono-cli/src/dynamic_tokens.rs` | `grep -c "expand_dynamic_tokens" crates/nono-cli/src/dynamic_tokens.rs` (9 hits — includes the `pub(crate) fn` def, its internal recursive/helper call sites, and 2 unit-test names) | `expand_dynamic_tokens()` | 450 | `pub(crate) fn` — dynamic path-token expansion (`$VAR`/`@git:*`, Phase 110) |
| `crates/nono-cli/src/capability_ext.rs` | `grep -c "expand_dynamic_tokens" crates/nono-cli/src/capability_ext.rs` (11 hits — 1 cfg-gated import, 1 cfg-gated local stub def, multiple call sites) | `expand_dynamic_tokens()` (non-Unix arm) | 25-28 | **Load-bearing platform-gate finding:** `capability_ext.rs:22-23` imports the real `dynamic_tokens.rs` implementation only `#[cfg(any(target_os = "linux", target_os = "macos"))]`; the `#[cfg(not(...))]` arm at line 25-28 is a same-named, same-signature no-op stub (`Ok(entries.to_vec())`) — on Windows, dynamic-token expansion currently does nothing. `dynamic_tokens.rs`'s own module doc comment names "v3.7 (Windows Tool-Sandbox Parity)" (this milestone) as the future absorb expected to reconcile this early-arriving copy. |
| `crates/nono-cli/src/windows_wfp_contract.rs` | `grep -n "FWP_MATCH_RANGE" crates/nono-cli/src/windows_wfp_contract.rs` (1 hit) | `WfpRuntimeActivationRequest.localhost_port_ranges` | 32-35 | Native `FWP_MATCH_RANGE` port-range field (doc comment: "one filter object per range entry, never unrolled into per-port entries") |

---

## Engine-Agnosticism Grounding (from spike-findings-nono)

Neutral grounding facts for Plan 116-05's D-09 engine-agnosticism scoring — recorded here, not
scored or ranked:

- **"Sandbox-the-tools, not sandbox-the-TUI"** (`references/windows-confinement-model.md`) — the
  achievable Windows confinement model is per-tool-invocation confinement (what PR#4 already does),
  not confining an interactive agent-engine process itself. A Low-integrity client cannot register
  with the Windows console subsystem (`cmd.exe` on ConPTY dies `0xC0000142` under both raw Low-IL
  and AppContainer, Spike 001 INVALIDATED). Relevant to Pole A too: a hypothetical
  `platform/windows.rs` would also have to be a per-tool-launch mediator, not a TUI-level sandbox —
  mirroring upstream's own `platform/linux.rs`/`platform/macos.rs` shape, which are themselves
  per-command drivers, not TUI sandboxes.
- **"Daemon-as-launcher is the sound primary model"** (Spike 003, VALIDATED,
  `references/engine-agnostic-confinement.md`) — a single persistent launcher confining arbitrary
  engines (`cmd.exe`, `powershell.exe`, `python.exe`) identically via
  `nono run --profile <...> -- <engine.exe>`, proven on Win11 26200.8390. Direct evidence that the
  fork's *underlying primitive* (`nono run`) is already engine-neutral.
- **"Primitive vs. entry point" distinction** (`references/engine-agnostic-confinement.md`) — the
  Claude-specificity in the fork's current path lives only in the PreToolUse hook contract
  (`claude_code_hook.rs`), not in the confinement primitive itself (`nono run`, the broker, WFP,
  AppContainer). This reframes what "the fork's engine-agnosticism gap" means: a hook-contract
  integration problem, not a confinement-primitive problem. This distinction is offered as a
  scoring nuance for D-09, not a conclusion about either pole.

Not-yet-spiked gaps also worth D-09's awareness (per the skill's own metadata): persistent
token/job reuse across many agents + a multi-tenant `AI_AGENT` marker + one persistent multi-client
capability pipe (spike 004, never run); the `nono-py`-binding-driven real Python/LangChain agent
proof (spike 005, never run). These mean the fork-native path's own engine-agnosticism is partially
unvalidated too, not just fully proven — a fact for D-09's scoring, not a lean toward either pole.

---

## WRITE_RESTRICTED Worked Example

**The `.NET`/PowerShell-CLR-under-`WRITE_RESTRICTED` finding is the matrix's canonical worked
example of "structurally-blocked" (an OS behavior) versus a merely-higher-cost engineering path (a
build-cost judgment) — D-07 names this distinction explicitly.**

```bash
grep -n "BrokerLaunchNoPty\|WriteRestricted" crates/nono-cli/src/execution_runtime.rs
```
Hits (2026-08-09): lines 549-551, 565, 603, 616 — all inside `execute_sandboxed()` (line 112). The
load-bearing comment, `execution_runtime.rs:548-552`:

> "`session_sid` (the synthetic `S-1-5-117-*`) is RETAINED for the legacy, non-broker
> `WriteRestricted` arm only (the package SID is NOT a restricting SID). The two arms are mutually
> exclusive (BrokerLaunchNoPty XOR WriteRestricted), so the synthetic SID and package SID never
> coexist on a single child."

and at `execution_runtime.rs:614-620`, the actual arm selector reads
`loaded_profile.windows_low_il_broker` (`prefers_low_il_broker: loaded_profile.as_ref().is_some_and(|p| p.windows_low_il_broker)`)
to choose between the two arms at launch time.

**Traceability note on the original `claude_code_hook.rs:1072` citation (D-16 line-drift
discipline):** the plan's `interfaces` block cited `claude_code_hook.rs:1072` for this finding.
Re-read live on 2026-08-09, line 1072 today reads a comment about Constrained Language Mode
(`// (BrokerLaunchNoPty arm, Plan 62-12), where PowerShell is forced into Constrained Language
Mode. CLM blocks ALL .NET type/method calls.`) — a related but distinct concept (CLM restricts
*what a running PowerShell process may call*, not *whether the CLR can start at all*). The line
number has not moved off a BrokerLaunchNoPty-adjacent comment, but the **specific CLR-cannot-start
mechanism** actually lives in `exec_strategy_windows/launch.rs:1241-1278` (the `WindowsTokenArm`
enum doc comments) and `launch.rs:1365-1381` (the `spawn_windows_child()` inline comment), not in
`claude_code_hook.rs`. The `launch.rs:1241-1245` doc comment states it directly:

> "`WriteRestricted`: WRITE_RESTRICTED + per-session restricting SID. Existing non-PTY supervised
> path. Triggers STATUS_DLL_INIT_FAILED (0xC0000142) under PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE —
> hence Phase 30 D-01 routes PTY-allocating launches to BrokerLaunch instead."

and `launch.rs:1365-1368`:

> "On the Windows detached launch path, the WRITE_RESTRICTED + session-SID token combines with
> DETACHED_PROCESS + no-PTY to trigger STATUS_DLL_INIT_FAILED (0xC0000142) in console-application
> grandchildren. The only configuration that initializes the loader cleanly is a null token."

**The underlying OS fact, stated plainly:** a process holding a `WRITE_RESTRICTED` token combined
with `PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE` (ConPTY) or `DETACHED_PROCESS` fails its loader
initialization at `STATUS_DLL_INIT_FAILED` (`0xC0000142`) before the child process's own code ever
runs — this reproduces for console-hosted runtimes including the .NET/PowerShell CLR (the fork's
own `BrokerLaunch`/`BrokerLaunchNoPty` arms exist specifically to route around this by never
constructing a `WRITE_RESTRICTED` token for PTY or session-SID-driven launches, using a Low-IL
primary token via the Medium-IL broker instead). This is why the fork's token-arm split
(`WindowsTokenArm::{WriteRestricted, BrokerLaunch, BrokerLaunchNoPty}`, `launch.rs:1237-1278`,
selected by `select_windows_token_arm()`, `launch.rs:1296-1339`) exists at all — it is not an
optimization, it is a fail path around a kernel-loader OS behavior that no amount of engineering
effort inside the WRITE_RESTRICTED arm itself can avoid. That is the structural distinction the
D-07 matrix's `structurally-blocked` rating means: a capability an OS mechanism *cannot* deliver in
one configuration, forcing an entirely different mechanism (the broker arm) rather than a
higher-cost variant of the same mechanism.

---

## D-07 Feasibility Matrix

Every citation below is either a symbol name copied verbatim from the Fork Primitive Inventory
above, or the literal string "ADR-65" followed by a one-clause quote of its §6 verdict text. Rows
are phrased as capability facts, not arguments for a pole.

| # | Capability | Upstream mechanism (file:symbol @ `v0.71.0`) | Rating | Citation |
|---|------------|-----------------------------------------------|--------|----------|
| 1 | Per-command filesystem grants (`fs_read`/`fs_write`/`fs_read_file`/`fs_write_file`, expanded per invocation) | `platform/linux.rs:3721-3734` (`expand_dynamic_tokens(&policy.fs_read, ...)` etc.), enforced via Landlock ruleset built around `detect_supported_exec_gate_abi()` (`platform/linux.rs:2505`) | implementable-but-new-work | Fork already has file-grant-application primitives at the single-launch level — `AppliedDaclGrantsGuard::snapshot_and_apply()` and `AppliedLabelsGuard::snapshot_and_apply()` (both cited above, Fork Primitive Inventory) apply WRITE-class DACL ACEs / Low-writable mandatory labels to a granted path set for one child. A per-*command* (rather than per-*launch*) grant-expansion dispatch loop atop these primitives, mirroring upstream's per-command policy resolution, does not exist yet — new dispatch code over an existing enforcement primitive, not a new OS mechanism. |
| 2 | Exec shims (a copy of the sandbox binary installed as a per-command trampoline that mediates each shimmed command) | `platform/linux.rs:124` (`shims_by_command: BTreeMap<String, ShimIdentity>`), `platform/linux.rs:2934` (`materialize_shim_source()`) | implementable-on-fork-primitives | `crates/nono-shell-broker/src/main.rs::run()` (Fork Primitive Inventory) is already a mediating-trampoline binary: a separate executable spawned at Medium-IL that self-degrades to Low-IL and launches the confined child, the same "materialize a mediator process, dispatch through it" shape upstream's shim pattern uses. |
| 3 | `url_shim` (browser-open mediation for a sandboxed command that needs to open a URL) | `platform/linux.rs:335,696-697` (`crate::tool_sandbox::url_shim::{URL_OPEN_SHIM_NAME, run_url_open_shim()}`) | implementable-but-new-work | The fork already has a cross-platform IPC contract for this exact capability — `crates/nono-cli/src/open_url_runtime.rs::run_open_url_helper()` (Unix arm, live, uses `nono::supervisor::UrlOpenRequest`, `crates/nono/src/supervisor/types.rs:451`) — but the Windows arm, `crates/nono-cli/src/open_url_runtime_windows.rs::run_open_url_helper()`, is an explicit stub returning `NonoError::UnsupportedPlatform("Windows delegated browser-open flows are not available yet...")`. The wiring point and IPC message shape already exist; the Windows implementation does not. |
| 4 | `token_broker` (per-command scoped-credential grant broker, `SharedBroker`/`GrantSet::{All,Specific}`) | `platform/linux.rs:138,405-406,1671-1673` (`crate::tool_sandbox::token_broker::{SharedBroker, GrantSet}`) | implementable-but-new-work | Fork has a secure credential-loading primitive at the library tier — `crates/nono/src/keystore.rs::load_secret_by_ref()` (line 305) / `load_secrets()` (line 266) — covering `op://`, `bw://`, `keyring://`, `env://`, `file://`, `cmd://` URI schemes with `Zeroizing` memory handling. It has no per-command `GrantSet`-style scoping dispatch (which credential goes to which command invocation) — that dispatch layer is new work atop an existing, already-secure credential-loading primitive. |
| 5 | `dynamic_providers` (dynamic path-token expansion in policy fields, e.g. env-var- or git-query-driven path tokens) | `platform/linux.rs:3700,3784` (`super::dynamic_providers::expand_dynamic_tokens()`) | implementable-but-new-work | Fork already has a verbatim port of the same-shaped mechanism, `crates/nono-cli/src/dynamic_tokens.rs::expand_dynamic_tokens()` (`pub(crate) fn`, line 450) — its own doc comment (`dynamic_tokens.rs:1-15`) states it was "ported here as a fork-owned, standalone copy" specifically so a future absorb "MUST reconcile this early-arriving copy" and names "v3.7 (Windows Tool-Sandbox Parity)" — this milestone — as that future absorb. However, `capability_ext.rs:22-23` wires the real implementation in ONLY under `#[cfg(any(target_os = "linux", target_os = "macos"))]`; the non-Unix arm (`capability_ext.rs:25-28`) is a same-named no-op passthrough stub (`Ok(entries.to_vec())`) that performs zero expansion. The expansion *logic* is fork-owned and platform-portable; it is not yet wired into the Windows capability-construction path — a smaller gap than a from-scratch implementation, but real integration work, not something already running on Windows today. |
| 6 | Env scrubbing (allow/deny-list env-var filtering applied to a launched command's environment) | `platform/linux.rs:113,2432-2438` (`redaction_policy: nono::ScrubPolicy`, `argv_display()` calling `nono::scrub_argv_with_policy()`) | implementable-on-fork-primitives | `ExecConfig.allowed_env_vars` / `ExecConfig.denied_env_vars` (`exec_strategy_windows/mod.rs:171,182`, Fork Primitive Inventory) are existing fields already threaded from CLI flags through `execution_runtime.rs:536-537,612-613` into the Windows child-env construction path (`launch.rs:681`, `build_child_env()`). |
| 7 | Per-command open-port mediation (loopback bind-port / bind-range grants scoped to one command invocation) | `platform/linux.rs:3804-3817` (`network.open_port`/`open_port_range` → `caps.add_localhost_port_range()`/`caps.add_tcp_connect_port()`); `platform/macos.rs:3140-3143` (same fields, Seatbelt arm) | implementable-on-fork-primitives | The fork's WFP port-range emitter is native rather than per-port-unrolled — `crates/nono-cli/src/windows_wfp_contract.rs:32-34` (`localhost_port_ranges: Vec<(u16,u16)>`, doc comment: "expressed natively via WFP's `FWP_MATCH_RANGE` condition — one filter object per range entry, never unrolled into per-port entries") — and `exec_strategy_windows/network.rs:496` (`build_wfp_runtime_activation_request()`) carries per-session port-range data into the WFP filter today. Because the fork's sandbox-the-tools model already launches one `nono run` (one AppContainer package SID) per tool invocation, per-session WFP scoping is already structurally per-command in the current architecture — a capability-fact relevant to D-09's enforcement-depth criterion, not scored here. |
| 8 | Sealed-shim runtime dir (a per-session directory, mode `0o700`, holding materialized shim binaries, not writable by the sandboxed child beyond what's explicitly granted) | `platform/linux.rs:2921-2932` (`create_shim_dir()`, `builder.mode(0o700)`) | implementable-on-fork-primitives | `AppliedDaclGrantsGuard`/`AppliedAncestorTraverseGuard`/`AppliedAncestorReadAttributesGuard` (`exec_strategy_windows/dacl_guard.rs`, Fork Primitive Inventory) already apply scoped, path-specific DACL grants distinguishing traversal/read/write at arbitrary directory granularity — the direct Windows analog of a Unix directory-mode seal. |
| 9 | Credential resolution / secret env-var injection for a launched command (discovered capability beyond the named 8 — `platform/linux.rs`'s `resolve_credentials()`/`broker_secret_env_vars()`) | `platform/linux.rs:314` (`resolve_credentials(&plan.config.credentials, proxy_credential_env_vars)`), `platform/linux.rs:447-458` (`broker_secret_env_vars()`) | implementable-on-fork-primitives | `crates/nono/src/keystore.rs` (library tier, cross-platform) already resolves multi-scheme credential references (`op://`, `bw://`, `keyring://`, `env://`, `file://`, `cmd://`) to `Zeroizing<String>` secret values — the same class of capability upstream's `resolve_credentials()` performs, already shipping and already Windows-compatible (no Unix-only syscalls in `keystore.rs`). |
| 10 | Fine-grained per-file read-policy enforcement decided dynamically at file-open time (e.g., allow read of `src/`, deny read of a sibling `.env` in the same directory, where the *decision* — not just the ACE — is evaluated per-open rather than pre-declared) | Landlock's `PathBeneath` rule application via `landlock_abi` (`platform/linux.rs:127,277,397`) — Landlock is an LSM hook evaluated at `IRP`-equivalent open-time on every file access, allowing rules finer than what a static, pre-declared ACL can express without re-declaring an ACE per file | structurally-blocked | ADR-65 (`.planning/architecture/adr-65-minifilter-go-no-go.md`) §1 documents the only Windows mechanism capable of this class of *dynamic, per-open* interception as an `IRP_MJ_CREATE` pre-operation callback in a **minifilter** driver (`NonoPreCreate`, altitude 365678/377813.5) — DACL/mandatory-label enforcement (the fork's current Windows model) is real kernel enforcement, but it requires every allowed object's ACE to be pre-declared at grant time, not decided dynamically per open. ADR-65 §6's standing verdict: **"lean No-go / Conditional-go"** for a production minifilter — Accepted, re-affirmed at this milestone's open (PROJECT.md). This capability is therefore rated structurally-blocked under the fork's current model as a standing precondition — the ADR-65 verdict is the gating fact itself, not a build-cost judgment, and this rating does not presuppose or depend on any change to that verdict. |
| 11 | WRITE_RESTRICTED-launched console/CLR-hosted runtimes (`.NET`/PowerShell CLR under `PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE` or `DETACHED_PROCESS`) | N/A — this is a Windows-side OS-loader fact independent of upstream's Linux/macOS drivers; included as the matrix's worked example per D-07 | structurally-blocked (worked example distinguishing an OS-imposed block from a higher-cost engineering path) | `exec_strategy_windows/launch.rs:1241-1245,1365-1381` (`WindowsTokenArm::WriteRestricted` doc comment + `spawn_windows_child()` inline comment: "STATUS_DLL_INIT_FAILED (0xC0000142)... The only configuration that initializes the loader cleanly is a null token"); `execution_runtime.rs:548-552` (the `BrokerLaunchNoPty XOR WriteRestricted` mutual-exclusivity comment). See the WRITE_RESTRICTED Worked Example section above for the full OS-fact statement and the distinction: this is an OS loader behavior no amount of engineering effort inside the `WriteRestricted` arm can avoid (structurally-blocked), not a mechanism that merely costs more to harden — which is why the fork routes around it entirely via a different mechanism (`BrokerLaunch`/`BrokerLaunchNoPty`) rather than a hardened version of the same one. |

**Row count: 11** (exceeds the D-07-required minimum of 8: the 8 named capability classes, plus one
discovered capability beyond the named list — row 9 — plus the WRITE_RESTRICTED worked example —
row 11 — per the plan's explicit instruction to insert it as its own row).

---

## Neutrality Self-Check

Total rows in the D-07 Feasibility Matrix: **11**. Re-reading each row's Rating and Citation cells:
every rating is phrased as a capability fact ("X exists / does not exist", "Y requires mechanism Z
which the fork does/does not have", "ADR-65's verdict stands as a precondition") — none compares
the two disposition options, recommends adopting or rejecting either, or uses comparative or
recommending language of any kind. Row 7 (open-port mediation) notes a structural fact about the
fork's *existing* sandbox-the-tools architecture (one `nono run` per tool call) without concluding
that architecture should be kept or replaced — that conclusion is explicitly deferred to D-09's
engine-agnosticism/enforcement-depth scoring in Plan 116-05. Row 10 and row 11 both cite ADR-65's
or the OS's standing facts rather than a preference; neither suggests the blocked capability would
become newly available under a changed minifilter verdict (the row 10 citation frames the verdict
as a standing precondition, not a cost judgment, precisely to avoid that forbidden framing). Zero
rows contain comparative or recommending language toward either disposition option. This satisfies
D-10's neutral-framing requirement for this document.

---

## References

- `.planning/phases/116-tool-sandbox-divergence-audit-disposition-adr/116-CONTEXT.md` — D-07, D-08,
  D-09, D-10, D-16 (locked decisions this document satisfies).
- `.planning/phases/116-tool-sandbox-divergence-audit-disposition-adr/116-RESEARCH.md` §2f-§2h, §3,
  §4 — the pre-verified command set this document's live re-runs are anchored to.
- `.planning/architecture/adr-65-minifilter-go-no-go.md` — the standing minifilter Go/No-Go
  verdict cited by row 10.
- `.claude/skills/spike-findings-nono/SKILL.md`,
  `.claude/skills/spike-findings-nono/references/{windows-confinement-model,engine-agnostic-confinement}.md`
  — the Engine-Agnosticism Grounding section's source.
- `crates/nono-cli/src/exec_strategy_windows/{mod,restricted_token,labels_guard,dacl_guard,network,launch,supervisor}.rs`,
  `crates/nono-cli/src/{claude_code_hook,hooks,execution_runtime,open_url_runtime,open_url_runtime_windows}.rs`,
  `crates/nono-shell-broker/src/main.rs`, `crates/nono/src/keystore.rs`,
  `crates/nono-cli/src/dynamic_tokens.rs`, `crates/nono-cli/src/windows_wfp_contract.rs` — fork HEAD,
  read/grepped live 2026-08-09.
- Upstream `crates/nono-cli/src/tool-sandbox/platform/{linux,macos}.rs` at tag `v0.71.0`
  (`0055bf3c686de6fa665decd9831dc5686b282cb0`) — read via `git show v0.71.0:<path>`, live 2026-08-09.

*Plan: 116-02. Consumed by Plan 116-05's D-09 scoring and Plan 116-06's Phase 120 sizing (D-15).*
