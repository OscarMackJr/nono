# Fork Windows Coverage Inventory (HEAD)

Repo: `C:\Users\OMack\Nono` @ branch `milestone/v2.13-carryforward-closeout` (based on upstream v0.66.0).
Method: Read / Grep / git only — no modifications. Verified against current code, cross-checked with durable memory notes.

**Headline:** the fork has NO `crates/nono-cli/src/tool-sandbox/` (upstream's first-class per-tool-call subsystem from v0.65.0 / PR #1105 — confirmed 0 files). Instead the fork built its own Windows-specific "sandbox-the-tools" mechanism: a Claude Code `PreToolUse` hook that shells each tool call out to `nono run`, enforced by a Low-IL primary-token broker + per-run AppContainer. It is a working prototype, not a general policy engine.

---

## 1. Windows tool/command confinement ("sandbox-the-tools")

**Mechanism (PRESENT, prototype-grade):** A Claude Code `PreToolUse` hook mediates every tool call, NOT an in-process tool-sandbox.

- Hook script: `crates/nono-cli/data/hooks/nono-tool-hook.ps1` — reads the tool JSON on stdin, pipes it to `nono claude-code-hook`, captures stdout (the JSON decision contract) on a channel separate from stderr, and **fails closed** (emits a `deny` decision) on any non-zero exit or exception.
- Install path: `crates/nono-cli/src/hooks.rs:73` `install_claude_code_hook` writes a `PreToolUse` hook with `matcher: "*"` invoking `& "$HOME/.claude/hooks/nono-tool-hook.ps1"` (see tests at `hooks.rs:665-725`). Embedded via build.rs (`hooks.rs:18-27`).
- Decision handler: `crates/nono-cli/src/claude_code_hook.rs` (1287 lines), dispatched from `app_runtime.rs:182` `Commands::ClaudeCodeHook => claude_code_hook::run()`. Per-tool policy (`pre_tool_use_response`, `claude_code_hook.rs:38-205`):
  - `Read` / `Glob` / `Grep` → **allow** (read-only, `is_read_only_tool` `:243`).
  - `Bash` → **rewritten** to run through nono (`wrapped_bash_command` `:382`); on Windows (`windows_bash_wrapper_script` `:395`) the rewrite is `& <nono> run --profile claude-code-tools-windows-runner --allow-cwd -- powershell.exe -NoProfile -NonInteractive -EncodedCommand <b64>`.
  - `Write` / `Edit` / `MultiEdit` → **deny + additionalContext** steering the model to re-issue the operation as a confined Bash/PowerShell-encoded command (`:60-183`, `build_confined_write_cmd :530`).
  - Everything else (WebFetch, WebSearch, Task, `mcp__*`, etc.) → **deny** (`:189-205`).
  - Fail-closed **self-disable guard**: refuses to wrap Bash when CWD covers `~/.claude` hook settings / agent state (`cwd_self_disable_risk_reason :272`, `canonicalize_with_existing_prefix :336`, `path_covers :330`). This guard is the SOLE mitigation for `--allow-cwd` re-granting write to `~/.claude` because `add_deny_access` is a Windows no-op.

**Enforcement backend of the confined `nono run` child (the actual OS boundary):**
- Broker: `nono-shell-broker` crate (`crates/nono-shell-broker/src/main.rs`) — a Medium-IL broker that spawns the Low-IL child. Opted-in per profile via top-level `windows_low_il_broker: true` (`ExecConfig.prefers_low_il_broker`, `exec_strategy_windows/mod.rs:184-189`) which routes to `WindowsTokenArm::BrokerLaunchNoPty`.
- Primitive: **per-run AppContainer (lowbox)** child (package SID `S-1-15-2-*`, single-source from `app_container_name`) **plus a Low-IL mandatory label (NO_WRITE_UP)**. Write-deny rides on the mandatory label, not the DACL. The AppContainer runs as a different principal with zero inherent user-profile access, so nono grants the package SID explicit read/write/traverse/read-attributes via RAII DACL guards on user-owned grant paths and cwd/binary ancestors (`exec_strategy_windows/mod.rs:436-482`, `dacl_guard.rs`).
- Filesystem restriction proper is per-path **mandatory integrity labels** applied by `AppliedLabelsGuard::snapshot_and_apply` (`mod.rs:425`), gated behind the **R-B3 WRITE_OWNER pre-check** (`mod.rs:382-406`) — workspace must be user-owned (fails on elevated/admin-owned dirs).
- Legacy `WriteRestricted` token arm also exists (`restricted_token.rs`) but `.NET`/PowerShell cannot start under WRITE_RESTRICTED (durable F-60-UAT-05) → the broker/AppContainer arm is the viable path for PowerShell-executing tools.

**Maturity:** Live-UAT PASS 5/5 on Win11 (Phase 60, 2026-06-01) for the file-confinement coding loop. Honestly labeled **defense-in-depth mediation, NOT agent isolation** — the Medium-IL `claude` process and MCP servers stay unconfined. `command_policies` (upstream tool-sandbox's executable-pinning / invocation-policy schema) is **ABSENT from code** — it appears only in `data/profile-authoring-guide.md`, with zero wiring in `crates/nono-cli/src/` (grep of `command_policies|allow_writable_executable|invocation_policy` over src = 0 hits).

---

## 2. Windows network enforcement

**WFP kernel enforcement (PRESENT, service-only, daemon-path only):**
- Real enforcement via the user-mode SYSTEM service `nono-wfp-service` (`crates/nono-cli/src/bin/nono-wfp-service.rs`): IPC `activate_blocked_mode` → `install_wfp_policy_filters` → real `FwpmFilterAdd0` per-package-SID `FWP_ACTION_BLOCK` filter conditioned on `FWPM_CONDITION_ALE_USER_ID` across 4 ALE layers (CONNECT_V4/V6 + RECV_ACCEPT_V4/V6). The kernel driver `nono-wfp-driver.sys` is an out-of-scope placeholder and is NOT required.
- **Reachability limitation:** per-SID WFP filters are installed ONLY on the daemon path (`agent_daemon/launch.rs::wfp_filter_add`, driven by `nono agent launch`). Direct `nono run --profile <block:true>` with the AppContainer broker arm does NOT install a WFP filter — the confined AppContainer has ZERO network capabilities (`SECURITY_CAPABILITIES{CapabilityCount:0}`), so egress is denied at the capability layer before WFP. Filters can leak/accumulate across service restart (tracked follow-up).
- Port-level WFP filtering exists in `WindowsNetworkPolicy` (`compile_network_policy`, `nono/src/sandbox/windows.rs:329`) with `tcp_connect_ports` / `tcp_bind_ports` / `localhost_ports`.

**Proxy (`nono-proxy`) — per upstream-feature checklist:**

| Feature | State | Evidence |
|---|---|---|
| HTTP/2 (upstream ALPN) | **PRESENT** | `config.rs:73 enable_h2`, `--allow-http2` flag, profile `network.allow_http2` (`profile/mod.rs:1736`) |
| Reverse-proxy + credential injection | **PRESENT** | `reverse.rs`, `route.rs`, `credential.rs`; `InjectMode` header/url_path/query_param/basic_auth (`config.rs:14-21`); `RouteConfig` w/ `inject_header`, `credential_format`, endpoint method+path filtering (`config.rs:96-201`) |
| Forward-proxy via `HTTP_PROXY` | **PRESENT** | `server.rs:141-150` injects `HTTP_PROXY`/`http_proxy` + token into child env |
| `no_proxy` bypass | **PRESENT** | `server.rs:51-138` computes `NO_PROXY` (loopback + non-credential allowed hosts); "smart" bypass at `:325-405` |
| OAuth2 client_credentials | **PRESENT** | `oauth2.rs`, `config.rs:180 RouteConfig.oauth2` |
| Cloud-metadata SSRF deny (link-local) | **PRESENT** | `filter.rs:5,58-72` metadata deny list + link-local range check |
| allow_domain (host allowlist model) | **PRESENT** | `profile/mod.rs:1689 allow_domain`, `AllowDomainEntry` (Plain/conditional), `merge_allow_domain` |
| `deny_domain` (deny-list model) | **ABSENT** | grep `deny_domain|blocked_domain` over profile+proxy = 0; enforcement is allowlist-only |
| Upstream (enterprise) proxy passthrough | **PRESENT** | `profile/mod.rs:1728 upstream_proxy` + `1734 upstream_bypass` |
| SPIFFE / SPIRE / SVID | **ABSENT** | grep `spiffe|spire|svid` over `nono-proxy/src` = 0 |

---

## 3. Profile/policy coverage

| Feature | State | Evidence |
|---|---|---|
| `platform_overrides` (per-OS profile patches) | **ABSENT** | grep `platform_override` over `crates/` = 0 hits |
| `extends` resolution | **PRESENT** | `cli.rs:1640 pub extends`, `profile init --extends`; resolution exercised in `capability_ext.rs:1848,2226` (`from_loaded_profile` honors `extends:"default"`/`"claude-code"`) |
| Port allowlists | **PARTIAL** | Discrete `Vec<u16>` lists only — `open_port`/`listen_port`/`connect_port` (`profile/mod.rs:1711-1719`). NO range syntax; `connect_port` documented Linux-Landlock-V4+ only |
| `command_policies` (tool-sandbox executable pinning / invocation policy) | **ABSENT (code)** | Documented in `data/profile-authoring-guide.md:83-179` but 0 code wiring; the whole `tool-sandbox/` subsystem is missing |
| `windows_low_il_broker` (fork-specific) | **PRESENT** | top-level bool → broker arm (`exec_strategy_windows/mod.rs:184`) |
| `windows_interpreters` (fork-specific) | **PRESENT** | interpreter coverage-gate resolution (`mod.rs:1091 resolve_interpreter_paths`) |
| allow_domain conditional entries, deprecated-key aliases | **PRESENT** | `AllowDomainEntry` untagged enum, serde aliases (`bypass_protection`/`override_deny`, `open_port`/`allow_port`) |

Note: profiles are policy-rich for filesystem/network/credentials/AIPC, but the per-OS override and per-command policy dimensions upstream added are not present.

---

## 4. Resource controls

**PRESENT on Windows via Job Objects** (`exec_strategy_windows/launch.rs:505-565`):
- `--max-processes` → `JOB_OBJECT_LIMIT_ACTIVE_PROCESS` + `ActiveProcessLimit` (kernel-enforced; clap range 1..=65535, `cli.rs:2799`). Readback test `launch.rs:2768`.
- `--memory` → `JOB_OBJECT_LIMIT_JOB_MEMORY` + `JobMemoryLimit` (`launch.rs:513-545`).
- `--cpu-percent` → `JobObjectCpuRateControlInformation` (`JOB_OBJECT_CPU_RATE_CONTROL_HARD_CAP`). (`--cpu-percent` is rejected on macOS — no cgroup equivalent — `cli.rs:99-118`.)
- `--timeout` wall-clock deadline computed pre-spawn (`mod.rs:915`), plus `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` / `DIE_ON_UNHANDLED_EXCEPTION` containment.

Resource-limit surface is at parity with (arguably ahead of) macOS here — Windows Job Object process/memory caps are kernel-enforced.

---

## 5. Windows sandbox backend summary

Core driver `crates/nono/src/sandbox/windows.rs` (5356 lines). Enforcement primitives available:

1. **Per-path mandatory integrity labels (NO_WRITE_UP)** — `apply()` `:52` applies a mode-derived label mask (read / write / read-write) to each compiled filesystem rule via `try_set_mandatory_label` `:77`; fail-closed. This is the filesystem-confinement primitive (analogous to Landlock/Seatbelt allow-lists but implemented as MIC labels + WRITE_OWNER gate).
2. **Low-IL primary token** — `create_low_integrity_primary_token()` `:534` (duplicate token + lower IL to `WinLowLabelSid`); `apply_low_il_label_to_token()` `:675` labels the AppContainer child post-spawn.
3. **Per-run AppContainer (lowbox)** — package-SID isolation via the broker; the validated confined-network design (WFP-matchable per-run SID). Registration requires `CreateAppContainerProfile` (derive-only SID → `CreateProcessW` ERROR_FILE_NOT_FOUND).
4. **WFP network policy compilation** — `compile_network_policy()` `:329` produces `WindowsNetworkPolicy` (connect/bind/localhost ports); actual `FwpmFilterAdd0` enforcement lives in `nono-wfp-service` (daemon path).
5. **Supervisor support classification** — `classify_supervisor_support()` `:300`; `preview_runtime_status()` `:141` and `validate_preview_entry_point()` `:249` gate ConPTY minimum build and unsupported-feature fail-closed reporting.
6. **Job Object containment** (see §4) + capability-pipe supervisor IPC (`exec_strategy_windows/supervisor.rs`, cap-pipe DACL grants for AppContainer child reachability).

The library boundary carve-out (ADR-86): Windows denial rendering stays CLI-side (`exec_strategy_windows/`), deliberately not converged with the core `diagnostic_code()`/`remediation()` surface.

---

## Biggest gaps (5-line summary)

1. **No `tool-sandbox/` subsystem** — the fork has zero of upstream's v0.65.0 per-tool-call `command_policies` engine (executable pinning, invocation policy, argv/env rules); its Windows equivalent is a Claude-Code-specific `PreToolUse` hook + `nono run` broker, honestly a defense-in-depth prototype, not a general isolation boundary.
2. **`platform_overrides` ABSENT** — no per-OS profile patching; a single profile must serve all platforms (fork bolts on `windows_low_il_broker`/`windows_interpreters` top-level flags instead).
3. **`deny_domain` ABSENT** — network filtering is allowlist-only (`allow_domain`); no deny-list model. **SPIFFE/SPIRE ABSENT** entirely.
4. **WFP network enforcement is daemon-path-only** — `nono agent launch` installs per-SID WFP filters, but direct `nono run` relies on the zero-capability AppContainer (block is at the capability layer, not a matchable WFP filter); filters can leak across service restarts.
5. **Port control is discrete lists, not ranges**, and `connect_port` is Linux-only; the tool-confinement path is Claude-Code-coupled (hardcoded `claude-code-tools-windows-runner` profile, PowerShell-encoded rewrites) rather than a generic per-command sandbox.
