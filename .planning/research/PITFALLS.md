# Pitfalls Research

**Domain:** Enterprise hardening of a Windows security product — silent MSI fleet deploy, machine-policy HKLM egress allowlist, and SIEM/EDR telemetry (nono v3.0)
**Researched:** 2026-06-18
**Confidence:** HIGH (grounded in in-tree `nono-wfp-service.rs`, `network_policy.rs`, `diagnostic.rs`, Phase 53/61/62 signed-MSI history, Phase 56 `allow_domain`, project memory `windows_mandatory_label_write_owner`, `windows_appcontainer_wfp_validated`, `windows_wfp_enforcement_is_service_only`, `windows_msi_wxs_is_generated`, SEED-001/002/003 seeds, and verified Win32/WFP/Event-Log semantics)

> Scope note: this file covers pitfalls that are NEW with the v3.0 enterprise features — silent-install/GPO, HKLM policy spine, egress reconciliation, and telemetry. The per-invocation confinement traps banked in prior milestones carry forward unchanged. Cross-references to known codebase landmines are made explicit where the v3.0 work amplifies them.

---

## Critical Pitfalls

### Pitfall 1: Fail-OPEN on HKLM policy read failure (security hole disguised as robustness)

**What goes wrong:**
The machine policy reader (`HKLM\SOFTWARE\Policies\nono`) returns an error — key absent on a pre-provisioned host, ACL mismatch, registry-service hiccup, or WOW6432Node redirection confusing the reader. The code catches the error and falls back to an empty allow-list or the per-user profile. The agent runs with a permissive user-configured allowlist (or no network restriction at all) while the operator believes machine policy is enforced. From a fleet CISO's perspective this is a silent security regression: every new deploy default-denies nothing while the IT team waits for GPO replication.

**Why it happens:**
The instinct is "policy absent = not yet configured = don't break the user's workflow." That instinct is correct for feature configuration (e.g., update-check URLs) but catastrophically wrong for a security boundary. The HKLM key is the deny-by-default egress allowlist spine — absence means the enterprise policy has not been applied to this machine, which is a reason to restrict MORE aggressively, not less. Developers who grew up writing user-facing software treat config-read errors as "use defaults"; security products must treat them as "fail closed."

**How to avoid:**
- Codify the policy-read failure contract in Rust as an enum: `PolicyReadResult::Enforced(AllowList)`, `PolicyReadResult::NotConfigured`, `PolicyReadResult::ReadError(NonoError)`. Only `NotConfigured` falls back to per-user profile; `ReadError` must be fatal (deny-all or abort launch).
- At the call site: `match read_machine_policy() { ReadError(e) => return Err(NonoError::PolicyLoadFailed(e)), NotConfigured => load_user_profile(), Enforced(list) => use list }`.
- Add a dark-factory gate: run the policy reader against a deliberately permission-denied registry key and assert the process exits non-zero with a `PolicyLoadFailed` error — not a permissive launch.
- CLAUDE.md already mandates: "Configuration load failures must be fatal. If security lists fail to load, abort." Apply that rule explicitly to the registry reader.

**Warning signs:**
- `RegQueryValueExW` return value is checked with `if err != ERROR_SUCCESS { return Ok(Default::default()) }` rather than `if err != ERROR_SUCCESS { return Err(...) }`.
- `unwrap_or_default()` anywhere in the registry-read path.
- A test that intentionally corrupts the policy key and asserts that nono still launches with some allow-list.
- No test for the `ReadError` → abort path.

**Phase to address:**
Phase 83 (machine-policy HKLM reader). Make the fail-secure contract the first acceptance criterion, verified by a dark-factory gate that injects an unreadable key.

---

### Pitfall 2: proxy allowlist and WFP allowlist drifting out of sync (false sense of security)

**What goes wrong:**
The existing `nono-proxy` (Layer 7) and `nono-wfp-service` (kernel Layer 3/4) are separately configured. When v3.0 adds the HKLM policy spine, both layers must read from it. The pitfall is wiring only one layer to the new HKLM source while leaving the other on its old per-user profile path. The visible symptom is that one policy is enforced but the other is not: an agent blocked at the proxy can still open a raw TCP socket that bypasses the proxy entirely (WFP would have blocked it, but WFP was never updated); or conversely, WFP blocks the socket but the proxy's allow-list was never consulted for methods/paths. The worst case: both layers are wired to HKLM but through independent deserialization — an admin makes one registry change that the proxy reads correctly but WFP interprets as "allow all" due to a missing key or a version mismatch.

**Why it happens:**
The proxy and WFP service were built at different milestones (Phases 56 and 62) with separate configuration paths. When a cross-cutting policy source is added, the natural instinct is "update the proxy first, WFP is harder, do it later." Later never arrives before the feature ships, and the partial wiring is invisible until a penetration test.

**How to avoid:**
- Define a single `MachineEgressPolicy` struct that both `nono-proxy` and `nono-wfp-service` consume. Deserialize once from HKLM at startup; pass the same struct to both enforcement layers. Never let each layer parse independently.
- In the dark-factory gate for egress (SEED-002): establish a test that injects a corpus of domain entries into the HKLM key, launches an agent, and asserts BOTH layers reflect the policy — proxy filter (`/proxy-allowed-hosts` debug endpoint or log line) AND WFP filter-state proof (see `verify-dark.ps1 --gate WFP-01` pattern from v2.13). A pass on one layer only is not a pass.
- Treat the proxy as L7 defense-in-depth, not the sole boundary. WFP is the structural kernel enforcement; the proxy adds method/path filtering. Documented in the risk model so no one believes "proxy-only" ships the feature.
- Version-stamp the policy schema in the registry key (`HKLM\SOFTWARE\Policies\nono\PolicyVersion = DWORD`). If WFP service starts and reads a version higher than it understands, it must fail-safe (block-all or refuse to install filters) rather than silently use a stale allow-list.

**Warning signs:**
- `nono-proxy` and `nono-wfp-service` have separate `load_config()` functions that each open `HKLM\SOFTWARE\Policies\nono` independently.
- The SEED-002 dark-factory gate tests proxy-side blocking but does not assert WFP filter state.
- A changelog entry says "proxy now reads machine policy" with no corresponding WFP entry.
- `network_policy.rs`'s `ResolvedNetworkPolicy` struct has no path to the WFP service's filter-install routine.

**Phase to address:**
Phase 83 (machine-policy spine) must include a single `MachineEgressPolicy` reader used by BOTH layers. The dark-factory gate must verify both. Do not split this across two phases without a failing integration test that enforces consistency.

---

### Pitfall 3: Wildcard matching footgun — proxy suffix matching and WFP bypass via unlisted subdomain

**What goes wrong:**
The existing `network_policy.rs` suffix list (e.g., `.anthropic.com`) is converted to a `*.anthropic.com` wildcard for the proxy filter via `build_proxy_config`. The proxy correctly blocks `evil.com`. However, the matching code uses a string suffix comparison, not a DNS component comparison. An attacker-controlled domain `anthropic.com.evil.com` could in theory match a naively-written `endswith(".anthropic.com")` check. More practically: a corporate allow-list entry for `.corp.internal` added via GPO ADMX accidentally allows `anything.corp.internal` including `exfil.corp.internal` — the wildcard is far broader than intended.

The WFP side has the complementary problem: WFP filters must specify exact remote IP ranges (v3) or remote FQDNs (v4+, limited). A suffix wildcard in the HKLM allow-list cannot be directly expressed as a WFP ALE condition; the implementation might resolve the wildcard to a snapshot of IP addresses at policy-load time, which goes stale as CDN IPs rotate. An agent can then reach a newly-rotated `api.anthropic.com` IP that is no longer in the WFP filter.

**Why it happens:**
The proxy wildcard matching bug is well-understood in the codebase (CLAUDE.md: "String `starts_with()` on paths is a vulnerability"; `network_policy.rs` already uses proper `starts_with('.')` guards for the loopback case). But the same discipline must be applied to the suffix matching in the proxy filter layer itself — not just nono's config parsing. The WFP IP-snapshot problem is inherent to mixing a DNS-name policy with an IP-layer enforcement mechanism and is easy to underestimate.

**How to avoid:**
- For proxy suffix matching: use DNS component comparison, not string suffix. A function `matches_suffix(host: &str, suffix: &str) -> bool` should split both on `.` and compare components from right to left, never use `host.ends_with(suffix)` directly. This already partially exists in the codebase; audit every site that calls into the proxy `filter.rs` matching logic after HKLM integration.
- For WFP and CDN-rotated IPs: the correct architecture is to rely on the proxy layer for FQDN-level enforcement (the proxy can verify the SNI/Host header) and use WFP only to enforce "only traffic through the nono proxy port is allowed" rather than enumerating every remote IP. This is the AppContainer + proxy architecture that SEED-002 implies is already in place from Phase 62 — preserve it; do not try to enumerate AI-provider IPs in WFP directly.
- Ensure the machine-policy schema documents which entries are exact-host vs suffix, and that GPO ADMX template only allows suffix entries in the `AllowedSuffixes` multi-string key, not the `AllowedHosts` exact-match key. Mixing them in one flat list invites operator mistakes.

**Warning signs:**
- `host.ends_with(&format!(".{}", suffix))` without first stripping a leading `.` from `suffix`.
- A test showing `anthropic.com.evil.com` is blocked is missing.
- WFP filter-install code resolves `*.anthropic.com` to a list of IPs via `GetAddrInfoW` and hard-codes them into `FwpmFilterAdd0` conditions.
- GPO ADMX template uses a single `REG_MULTI_SZ` for both exact hosts and wildcard suffixes.

**Phase to address:**
Phase 83 (HKLM policy reader + egress reconciliation). The suffix-matching review is a pull-request gate item — all callers of `filter.rs` suffix matching must use component comparison before the HKLM-backed allow-list ships.

---

### Pitfall 4: Silent MSI per-user vs per-machine context confusion — WRITE_OWNER scratch-space and service install

**What goes wrong:**
A silent push via SCCM/Intune runs the MSI in the SYSTEM context (per-machine install, `MSIINSTALLPERUSER=""` or `ALLUSERS=1`). The nono MSI's custom action that provisions the WRITE_OWNER scratch space (`%LOCALAPPDATA%\nono\workspaces\`) resolves `%LOCALAPPDATA%` as `C:\Windows\system32\config\systemprofile\AppData\Local` — the SYSTEM account's local app data, not the target user's. The result: scratch workspaces are provisioned for SYSTEM and every subsequent `nono run` for a real user fails the R-B3 user-ownership guard (the directory is SYSTEM-owned, not user-owned), producing a cryptic access-denied that looks like a sandbox bug.

**Why it happens:**
`%LOCALAPPDATA%` is a per-user environment variable that resolves differently under SYSTEM than under the install-target user. WiX MSI custom actions that resolve env vars during the install do so in the SYSTEM token context for per-machine installs. The Phase 60 scratch-space provisioner (`grant_sid_write_on_path`, `AppliedDaclGrantsGuard`) was designed and tested under a user-context launch, not a SYSTEM-context MSI custom action.

**How to avoid:**
- Provision scratch space at first-run (when the user actually launches nono), not at MSI install time. The MSI should only create `C:\ProgramData\nono\` (machine-global, SYSTEM/Admins-writeable) and `C:\Program Files\nono\` (binaries). User-specific scratch space (`%LOCALAPPDATA%\nono\workspaces\`) must be created by nono itself on first use, in the user's security context. This eliminates the SYSTEM-context resolution problem entirely.
- If the MSI must pre-provision for "zero-touch first run" experience: use a deferred custom action running as the logged-in user (WiX `Impersonate="yes"` on the custom action) rather than in the SYSTEM context.
- The machine MSI should install a Group Policy object or registry entry at `HKLM\SOFTWARE\nono\ScratchRoot` pointing to `%ProgramData%\nono\workspaces\<USERNAME>` as a pattern, with a note that nono-cli expands `<USERNAME>` at runtime in the user's token. Never let the MSI expand per-user paths under SYSTEM.
- Dark-factory gate: run the MSI install with `msiexec /i ... /quiet ALLUSERS=1` under a non-admin test account and verify the workspace path is owned by the test account, not SYSTEM.
- Cross-references to known landmines: `feedback_windows_mandatory_label_write_owner` (WRITE_OWNER not implicit for Owner; drive-root user dirs fail), Phase 60 `AppliedDaclGrantsGuard`.

**Warning signs:**
- MSI custom action creates `%LOCALAPPDATA%\nono\workspaces\` without `Impersonate="yes"`.
- The provisioner is called from within a `<CustomAction>` that runs `Execute="deferred"` without explicitly setting `Impersonate="yes"`.
- First-run on a machine-installed nono shows the workspace path is `C:\Windows\system32\config\systemprofile\...`.
- Tests only cover the case where the provisioner is called from the user's own process, never from a SYSTEM-context script.

**Phase to address:**
Phase 82 (silent MSI install flags). The scratch-space provisioner must be re-evaluated against the SYSTEM-context install path before the machine-install gate ships.

---

### Pitfall 5: Service install in MSI is non-atomic — rollback leaves the service half-registered

**What goes wrong:**
The WiX MSI uses the built-in `ServiceInstall` and `ServiceControl` elements to register and start `nono-wfp-service`. If the service start fails (e.g., the WFP driver is blocked by a corporate policy, or the service binary is not yet signed and fails a code-integrity check), Windows MSI rolls back the installation but leaves artifacts:
- The service registration in SCM may or may not be cleaned up (depends on MSI rollback sequencing).
- The `HKLM\SYSTEM\CurrentControlSet\Services\nono-wfp-service` registry key may persist.
- A partial machine MSI install with no user-visible error is reported as "success" on some MDM platforms because `msiexec` exits 0 if the rollback itself succeeds.

The v2.11 milestone explicitly made service start non-fatal for this reason (`nono-wfp-service` start is non-fatal so a service hiccup doesn't roll back the product). But the v3.0 machine-policy spine depends on the service being reliably present — if the service is half-registered, machine-policy egress enforcement is silently absent.

**Why it happens:**
MSI rollback is a best-effort mechanism; it reverses registered actions in reverse order but custom error states (SCM partial-register) are not well-tested. "Non-fatal service start" is the right call for end-user UX but creates a grey area for enterprise fleet where the IT desk needs a hard binary: "installed and enforcing" or "not installed." The nuance is invisible to SCCM/Intune compliance checks.

**How to avoid:**
- Separate service-install from service-start in the WiX sequences. The service should always be registered (the binary is already in `Program Files`); start failure is a deferred error that `nono setup` checks and surfaces via Event Log.
- Ship a `nono health` command that emits a machine-readable JSON verdict: `{"wfp_service": "running|stopped|not_installed", "machine_policy": "enforced|not_configured|read_error", "scratch_space": "ok|error"}`. This is what SCCM/Intune compliance scripts call, not `sc query`.
- The dark-factory gate for the MSI must simulate a service-start failure and assert: (a) `msiexec` exits non-zero or emits an Event Log entry; (b) `nono health` reports the degraded state; (c) nono-cli refuses to launch a confined agent when `wfp_service` is `not_installed` (fail-secure, deny egress).
- Do not rely on MSI `ServiceControl/@Wait` semantics for compliance evidence — they are unreliable under SCM.
- Cross-reference: `windows_msi_wxs_is_generated` (the `.wxs` is regenerated from `build-windows-msi.ps1`; edit the script, not the `.wxs`).

**Warning signs:**
- The WiX `ServiceControl` element has `Wait="yes"` and the install sequence continues regardless of start outcome.
- `msiexec /quiet` exits 0 on a machine where `sc query nono-wfp-service` returns `FAILED`.
- No `nono health` verb exists.
- SCCM compliance script just checks `(Get-Service nono-wfp-service).Status -eq "Running"` without testing nono-cli's own assessment.

**Phase to address:**
Phase 82 (silent MSI). The `nono health` verdict command is a phase deliverable, not a nice-to-have. The dark-factory gate must verify the degraded-service path.

---

### Pitfall 6: Machine-wide env-var propagation lag — new PATH or NONO_ env vars not visible until re-logon

**What goes wrong:**
The MSI registers nono in the system PATH (`HKLM\SYSTEM\CurrentControlSet\Control\Session Manager\Environment\Path`) and sets machine-wide env vars like `NONO_POLICY_PATH`. These changes are broadcast to running processes via `WM_SETTINGCHANGE`. However:
- Sessions already logged in at install time will not see the new PATH until they log out and back in (or open a new process tree). On a machine where an engineer keeps a terminal open all day, nono is installed by SCCM but `nono run` gives "command not found" for hours.
- The `WM_SETTINGCHANGE` broadcast is delivered only to windows (message-loop processes). Console-only tools (cmd.exe, PowerShell without a window, CI agents) never receive it.
- If `NONO_POLICY_PATH` is set machine-wide to point at the HKLM policy file path, but the user's process was spawned before the machine env-var was set, nono reads an old/absent policy path and silently uses defaults — the fail-OPEN risk from Pitfall 1 via a different mechanism.

**Why it happens:**
Windows env-var propagation semantics are widely misunderstood. Developers test the install on a fresh shell opened after the install and it works; they never test the "install while sessions are open" case, which is the common fleet scenario.

**How to avoid:**
- Do not rely on env-var PATH for the machine-policy path at runtime. Machine policy should ALWAYS be read from a fixed, well-known registry key (`HKLM\SOFTWARE\Policies\nono`) regardless of env vars. The registry is always current; env vars are snapshotted at process start.
- The HKLM policy reader must call `RegOpenKeyExW` at every policy-evaluation point (or on a short TTL cache), never read an env-var path that was captured at startup.
- Document the post-install re-logon requirement in the SCCM deployment note; add a check in `nono health` that warns if the current process's PATH does not include the nono install dir (detects the "installed but old session" case).
- If machine env-vars are used (e.g., `NONO_POLICY_OVERRIDE` for escape-hatch testing), they are advisory only and must not affect the security path. The security path reads from the registry directly.

**Warning signs:**
- `std::env::var("NONO_POLICY_PATH")` in the policy-loading code path.
- Policy is loaded once at startup and cached for the process lifetime without a TTL.
- The MSI's install test runs in a fresh shell opened after install (never validates the already-open-session case).
- A test sets an env var to simulate machine policy rather than writing to a temporary registry key.

**Phase to address:**
Phase 83 (machine-policy HKLM reader). The reader must be registry-direct, not env-var-mediated.

---

### Pitfall 7: WOW6432Node registry redirection silently reads wrong policy

**What goes wrong:**
On 64-bit Windows, a 32-bit process reads `HKLM\SOFTWARE\Policies\nono` as `HKLM\SOFTWARE\WOW6432Node\Policies\nono` unless the key open uses `KEY_WOW64_64KEY`. nono-cli is always 64-bit, so it reads the 64-bit hive. BUT the Group Policy client writes to `HKLM\SOFTWARE\Policies\nono` (the 64-bit hive), while some Intune MDM service agents (e.g., the Intune Management Extension, which can be 32-bit) write to `HKLM\SOFTWARE\WOW6432Node\Policies\nono` via redirected writes. If the writing agent and the reading agent read from different hives, the policy appears absent to nono (fail-OPEN per Pitfall 1), or the hives diverge (stale policy in one, current in the other).

**Why it happens:**
WOW6432Node redirection is automatic and silent for 32-bit processes on 64-bit Windows; developers on 64-bit dev hosts never observe it because their test processes are 64-bit. The problem surfaces only when the Intune or SCCM delivery agent happens to be 32-bit.

**How to avoid:**
- Always open the machine-policy key with `RegOpenKeyExW(..., KEY_READ | KEY_WOW64_64KEY)` to explicitly request the 64-bit hive regardless of nono-cli's own bitness.
- Document in the ADMX template and Intune CSP OMA-URI that the policy path is the 64-bit hive; the deployment package must deploy-via-64-bit-capable mechanism (PowerShell `[Microsoft.Win32.RegistryView]::Registry64` when setting keys via script).
- Dark-factory gate: write a test that explicitly creates the key under `HKLM\SOFTWARE\WOW6432Node\Policies\nono` and verifies nono does NOT read it (i.e., the 64-bit path is authoritative). This catches the "accidentally reads WOW hive" regression.

**Warning signs:**
- `RegOpenKeyExW` calls without `KEY_WOW64_64KEY`.
- Policy integration test uses a PowerShell script without `[Registry]::LocalMachine.OpenSubKey(..., [RegistryView]::Registry64)`.
- No WOW64 redirection coverage in tests.

**Phase to address:**
Phase 83 (machine-policy HKLM reader). One-liner fix, but must be explicitly tested.

---

### Pitfall 8: Default-deny that accidentally blocks the AI provider and bricks the agent

**What goes wrong:**
The machine-policy allow-list is configured with corporate domains and `api.anthropic.com`. The GPO template ships the list without entries for Anthropic's CDN, OAuth endpoints, or the streaming endpoint (`api2.anthropic.com`, `*.anthropic.com` for streamed completions). An agent that runs an LLM call sees a WFP/proxy block on the streaming connection. The agent silently fails (no error dialog, the stream just hangs then times out), and the user reports "nono broke Claude." Because default-deny is correct, this is an operator-configuration problem, but nono's diagnostics do not attribute the failure to the policy block — the error is just a timeout in the AI SDK.

**Why it happens:**
The AI provider's outbound requirements are not static: Anthropic uses multiple subdomains and CDN prefixes that change over maintenance windows. Corporate IT uses the documented `api.anthropic.com` entry but misses the streaming-origin variant. The deny happens silently at the kernel (WFP BLOCK) and the AI SDK retries until timeout without surfacing a clear reason.

**How to avoid:**
- Ship a vetted `anthropic-claude-code` group entry in the embedded `network-policy.json` (already present from Phase 56) that includes ALL required subdomains for Claude Code operation (`api.anthropic.com`, `statsig.anthropic.com`, `sentry.io` subset, etc.). The HKLM policy schema should support a `"use_builtin_group": ["claude-code"]` key that references the vetted embedded list, so enterprise admins can say "allow Claude Code traffic" without enumerating IPs.
- The `DiagnosticFormatter` must surface WFP and proxy blocks when the AI provider endpoint is blocked. Currently it formats path-deny explanations; extend it to format network-deny explanations with the blocked host name and the matching policy rule (or "not in allowlist").
- Provide a `nono diagnose-egress <hostname>` subcommand that tests whether a given host passes the machine-policy + proxy filter without launching a full agent session.
- The machine-policy schema's `AllowedHosts` must validate that entries are syntactically valid hostnames/wildcards at policy-load time, not at enforcement time. An ADMX-pushed value of `api .anthropic.com` (space included) silently denies and is impossible to debug without the validator.

**Warning signs:**
- The only diagnostic for a blocked network connection is a timeout in the AI SDK with no nono-attributed error.
- The embedded `claude-code` network profile in `network-policy.json` does not include all outbound endpoints required by Claude Code.
- No `nono diagnose-egress` or equivalent.
- An IT admin can push an empty `AllowedHosts` list via GPO and nono accepts it without warning (silently blocks everything).

**Phase to address:**
Phase 83 (egress reconciliation). The AI-provider group entries in `network-policy.json` must be reviewed and a `nono diagnose-egress` or dry-run mode must be a deliverable.

---

### Pitfall 9: TOCTOU on machine-policy reload — allow-list widens mid-session

**What goes wrong:**
The machine-policy reader caches the allow-list in memory for the lifetime of a supervised session (typically minutes to hours for a long agentic run). An admin pushes a new HKLM policy — typically a widening (adds a new corporate SaaS domain) — while the agent is running. The running session is still using the old (more restrictive) allow-list; the new request gets blocked; the operator assumes nono is broken because the policy was updated. Conversely, if nono refreshes the policy mid-session, a malicious insider could push a widened policy that permits exfiltration targets just before a compromised agent makes the request.

**Why it happens:**
Caching vs. live-reload is a classic tradeoff. Security products almost always prefer caching (the session's security context is fixed at launch); operational products prefer live-reload. Enterprise customers will ask for live-reload because their IT teams push policy changes during business hours without wanting to bounce sessions.

**How to avoid:**
- Fix: the allow-list is fixed at session start (launch-time snapshot). The "no escape hatch" principle (nono core invariant) applies to the network layer: you cannot widen the allowed scope of a running session. Narrowing mid-session is acceptable (add a policy-change watcher that can only REMOVE entries from the running cache, never add them).
- Document this explicitly in the GPO ADMX template tooltip: "Policy changes apply to new sessions. Running sessions retain the policy in effect at launch."
- If live-reload is demanded: it must be widen-only through an explicit operator command (`nono update-policy`), not automatic — and even then the design needs ADR treatment.
- The dark-factory gate should test: push a widened policy mid-session; assert the running session still uses the old (more restrictive) list; assert a newly launched session picks up the new list.

**Warning signs:**
- The allow-list is stored in an `Arc<RwLock<AllowList>>` that is written by a `tokio::fs::watch` on the registry.
- Policy reload is triggered by `WM_SETTINGCHANGE` delivery (which an unprivileged process can forge via `SendMessage`).
- No test for "policy changed while session running."

**Phase to address:**
Phase 83 (HKLM policy reader). Codify the snapshot-at-launch model in the ADR. The widen-only-narrow mid-session rule is a security invariant.

---

### Pitfall 10: Event Log custom-channel registration requires admin and manifest deployment

**What goes wrong:**
Creating a custom Event Log channel (e.g., `nono/Security`) requires:
1. A `.man` event manifest compiled to a `.mc` file.
2. `wevtutil im nono.man` run as admin during installation to register the provider GUID and channel in `HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\WINEVT\Publishers\<GUID>`.
3. The compiled `.dll` resource file (or the nono binary itself if resources are embedded) on a stable path.

If step 2 is skipped (manifest not installed) or the resource DLL path changes (e.g., nono is moved from `C:\Program Files\nono\` after install), Event Log writes fail. The failure mode is: `RegisterEventSourceW` returns NULL, `ReportEventW` is never called, and the security event is silently dropped. From the CISO's perspective, the telemetry pipeline appears to work (no error) but emits nothing.

**Why it happens:**
The MSI's custom action that runs `wevtutil im` requires elevation and runs in the SYSTEM context — it must succeed during the install, not later. Developers test `ReportEventW` from a dev machine where they ran `wevtutil im` manually; the automated CI/MDM install never does this, so the registration is absent in production.

The nono-wfp-service already uses the Application Event Log (`EVENT_LOG_SOURCE: &str = SERVICE_NAME`) and writes `ReportEventW` for sweep operations — this is the existing pattern, but it relies on Windows auto-registering the Application log source at first write (which works for the generic Application log but NOT for a custom channel with structured schema).

**How to avoid:**
- Use the existing **Application event log source** pattern (`RegisterEventSourceW(SERVICE_NAME)`) for v3.0 rather than a custom channel. This works without a manifest and is already proven in `nono-wfp-service.rs`. Add nono-specific event IDs to the existing source. Defer custom channel + structured manifest to a future "SIEM schema hardening" milestone.
- If a custom channel IS needed: the manifest compilation (`mc.exe`), resource compilation (`rc.exe`), and `wevtutil im` must all be wired into the WiX MSI's install sequence via `CustomAction`/`ExePackage`, and `wevtutil um` must be in the uninstall sequence. The manifest path must be pinned to the `Program Files` install root; use `[INSTALLDIR]` in the manifest XML, not a hard-coded path.
- Dark-factory gate: run the MSI on a clean machine (no prior `wevtutil im`), emit a security event via `nono run`, and assert the event appears in Event Viewer under the expected log/source. A missing event is a test failure.
- The emission code must treat `RegisterEventSourceW` returning NULL as a `NonoError::TelemetryUnavailable` (logged to stderr), NOT silently ignored. Silent drop of security events is not acceptable.

**Warning signs:**
- `RegisterEventSourceW` return value is not checked.
- The MSI has no `wevtutil im` or `EventManifest` component.
- Security events are tested only on the dev machine where `wevtutil im` was run manually.
- `ReportEventW` fails silently (returns false) and the code continues without logging the failure.

**Phase to address:**
Phase 84 (SIEM/EDR telemetry — SEED-003). Use the Application log source approach for v3.0; defer custom manifest to a later hardening phase.

---

### Pitfall 11: Telemetry events leaking secrets and file paths into the Windows Event Log

**What goes wrong:**
The `DiagnosticFormatter` generates human-readable denial messages that include the full path of the denied file (e.g., `C:\Users\alice\Projects\secret-project\src\main.rs denied write access`). When this is emitted to the Windows Event Log — which is forwarded to Splunk/Sentinel via Windows Event Forwarding — every denied-access event includes:
- The full path of the corporate codebase being protected.
- Possibly credential-adjacent paths (e.g., `C:\Users\alice\.ssh\id_rsa` denied read — this path is now in the SIEM and tells an attacker what to target).
- In the network-deny case, the request URL may include query parameters containing tokens or session IDs if the AI SDK embeds them in the URL.

The "tamper-evident" claim for the telemetry is then undermined: the SIEM becomes itself a target for correlation-attack reconnaissance.

**Why it happens:**
The `DiagnosticFormatter` was designed for a human engineer reading stderr, where full paths are helpful for debugging. Re-using it verbatim for SIEM emission treats a debugging tool as a structured security signal. The distinction between "debug detail" and "security event schema" is not enforced at the API boundary.

**How to avoid:**
- Define a separate `SecurityEvent` struct that is distinct from `DiagnosticOutput`. The `SecurityEvent` carries:
  - An opaque path hash (`sha256(canonical_path)[0..8]`) rather than the full path — sufficient for correlation, safe for SIEM forwarding.
  - A path category tag (`workspace_file`, `system_path`, `user_profile`, `temp`) rather than the raw path.
  - For network denials: the hostname only (no URL path, no query string).
- The `DiagnosticFormatter` continues to emit full paths to stderr (for debugging). `SecurityEvent` is the SIEM-safe schema. These are two separate emission paths; they share the `DenialReason` enum but diverge on detail level.
- Mark any `SecurityEvent` field that could be PII-adjacent (full path, full URL) as `#[zeroize]` or `#[sensitive]` with a note that it MUST NOT be placed in the Event Log payload.
- The ADMX template should include a `TelemetryDetailLevel = DWORD` key: `0 = hashed`, `1 = category`, `2 = full-path` (requires explicit admin consent + elevated audit-log permissions). Default must be `0`.
- Review the nono-wfp-service's existing `ReportEventW` calls for the sweep events — they currently log object GUIDs and error codes, which is safe. Ensure the new denied-action events follow the same pattern.

**Warning signs:**
- `SecurityEvent` is a type alias for `DiagnosticOutput`.
- A network-deny event includes the full request URL in the Event Log message string.
- The Event Log emission code calls `diagnostic_formatter.format_human_readable()` and passes the result as the Event Log message.
- No `TelemetryDetailLevel` configuration; all emitted events include full paths.
- Tests emit events and assert on substrings of full paths (which confirms the path is in the event).

**Phase to address:**
Phase 84 (SIEM/EDR telemetry). The `SecurityEvent` schema is a phase deliverable. Define it before wiring to the emission backend.

---

### Pitfall 12: "Tamper-evident" telemetry claims that are not — append-only with no cryptographic anchor

**What goes wrong:**
SEED-003 specifies "tamper-evident append-only event chain." The common implementation mistake is to add an incrementing sequence number and a per-record HMAC signed with a key stored in `HKLM\SOFTWARE\nono\TelemetryKey`. An attacker with local admin (the threat model for a compromised agent that escaped confinement to admin-level) can delete or overwrite `HKLM\SOFTWARE\nono\TelemetryKey`, re-sign any sequence of events with a new key, and the "tamper-evident" chain no longer detects the gap.

A more subtle version: events are written to the Windows Event Log (which is append-only at the OS level) and the "tamper evidence" claim is that Event Log is protected by Windows ACLs. This is true for the Security log (SYSTEM-only write, `SeSecurityPrivilege` to clear) but NOT for the Application log (any process running as the event's registered source can write to it, and an admin can clear the Application log with zero audit trail).

**Why it happens:**
"Tamper-evident" is an easy claim to make and a hard property to actually provide in user-space. The shortcut is to protect the log with the same ACLs that protect the OS — which is fine for the Windows Security log but is a weaker claim for Application log or file-based logs. SEED-003 correctly cross-references SEED-005 (ZT-Infra signed policy overrides) as the immutable-audit angle; the pitfall is shipping a v3.0 "tamper-evident" claim without the ZT-Infra backing.

**How to avoid:**
- Do NOT claim full tamper-evidence in v3.0. Claim: "structured security events emitted to Windows Event Log; forwarding to SIEM provides external copy beyond local attacker's reach."
- The actual tamper-evidence story is: events are written to the Application log + forwarded via Windows Event Forwarding (WEF) to a SIEM collector. The SIEM's copy is out-of-reach for a locally-compromised host. Document this architecture in the SEED-003 deliverable explicitly: tamper-evidence = remote forwarding, not local crypto chain.
- If a local crypto chain is required: the HMAC key must be stored in a hardware-backed TPM PCR-sealed secret or in a remote key-management service, not in a registry key a local admin can delete. Defer this to the SEED-005 ZT-Infra milestone (it is scope-appropriate there).
- The ADR for Phase 84 should explicitly record: "Local HMAC chain rejected — attacker with local admin can forge. External SIEM forwarding is the tamper boundary."

**Warning signs:**
- `TelemetryHmacKey` is stored in `HKLM\SOFTWARE\nono\`.
- The "tamper-evident" claim in the user documentation is not qualified with "via external SIEM forwarding."
- The tamper-detection test does not simulate an admin deleting the HMAC key and verify the chain is broken.
- SEED-005 features (signed policy, remote attestation) are implemented inside Phase 84 instead of in their own milestone.

**Phase to address:**
Phase 84 (SIEM/EDR telemetry). Write the ADR first; be explicit about the tamper-evidence scope. Reference SEED-005 as the future work.

---

### Pitfall 13: Silent root-cert install via GPO modifying the wrong store — proxy TLS interception fails fleet-wide

**What goes wrong:**
The nono proxy intercepts HTTPS traffic (for domain filtering and credential injection) using a machine-local CA. For enterprise fleet deploy, this CA cert must be in the user's `Trusted Root Certification Authorities` store so the AI SDK's TLS stack accepts the proxy's re-issued certificates. The GPO/Intune deployment pushes the cert to `HKLM\SOFTWARE\Microsoft\SystemCertificates\Root\Certificates\` (the machine root store). However, .NET-based tools and Electron apps (like Claude Code) use the `CurrentUser\Root` store first, falling back to `LocalMachine\Root`. The cert installed in `LocalMachine\Root` is visible to Win32 `CertVerifyCertificateChainPolicy` calls but may NOT be visible to LibreSSL/OpenSSL-bundled runtimes (common in Rust binaries) that do not consult the Windows cert store at all unless explicitly configured.

**Why it happens:**
Windows has five root cert stores (CurrentUser\Root, LocalMachine\Root, LocalMachine\AuthRoot, service stores, and others). "Install via GPO" lands in `LocalMachine\Root`. Rust binaries using `rustls` or `native-tls` with `webpki` roots do not consult the Windows cert store by default — they use the compiled-in root set. Claude Code (Electron+Node) uses Node's `tls` module which DOES consult the Windows cert store. The matrix is: some clients pick it up, some don't, with no visible error — just TLS handshake failure appearing as a network-deny.

**How to avoid:**
- Map the TLS trust paths for every client process that will go through the nono proxy: `nono-proxy` itself (Rust/rustls), Claude Code (Electron/Node), the AI SDK (check if it ships its own cert bundle). Document which stores each client uses.
- For nono-proxy: ensure `rustls` is configured with `rustls_native_certs` (which reads the Windows cert store) rather than the default `webpki` roots. This is a single configuration change but must be verified.
- For the machine-CA cert: install to BOTH `LocalMachine\Root` (for Win32/CryptoAPI clients) AND `CurrentUser\Root` for the installing user. The WiX `CertificateRef` element handles this.
- GPO/Intune cert deployment: use the "Computer Configuration > Policies > Windows Settings > Security Settings > Public Key Policies > Trusted Root Certification Authorities" GPO path, which lands in `LocalMachine\Root` and is replicated by Group Policy to all machine stores.
- Dark-factory gate: after MSI install with the CA cert, open a TLS connection to a known endpoint through the proxy from (a) a PowerShell script (uses CryptoAPI), (b) a Node.js script (uses Windows cert store via Node), and (c) from within nono-cli itself. All three must succeed without TLS errors.

**Warning signs:**
- `rustls` is initialized with `RootCertStore::empty()` + `load_native_certs()` missing.
- The CA cert is installed only via `certutil -addstore Root` (adds to LocalMachine\Root only).
- TLS tests pass in dev environments (where the engineer's machine already has the cert in CurrentUser\Root from manual testing) but fail on fleet deploy.
- No test for the Rust/rustls trust path independently from the Node/CryptoAPI path.

**Phase to address:**
Phase 82 (silent MSI) — the cert install must be part of the MSI's `CertificateRef` component, and the trust paths for all client types verified in the dark-factory gate.

---

## Technical Debt Patterns

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Fall back to per-user profile on HKLM read error | Doesn't break developer setups | Fail-OPEN: machine policy appears enforced but isn't (Pitfall 1) | Never for the egress allowlist |
| Use `std::env::var("NONO_POLICY_PATH")` for machine policy path | Easy to test in CI | Env var snapshotted at process start; stale after machine env-var update (Pitfall 6) | Never for security config; OK for override in test-only code with explicit comment |
| Wire only proxy to HKLM; do WFP later | Faster first pass | Proxy/WFP allowlist drift; false sense of security (Pitfall 2) | Never; must be a single atomic phase |
| Re-use `DiagnosticFormatter` output verbatim for SIEM events | Zero new code | Full paths and URLs in SIEM; PII leakage (Pitfall 11) | Never |
| Store HMAC telemetry key in HKLM | Simple to implement | Local admin can delete and re-sign; tamper-evidence claim is false (Pitfall 12) | Never; use external SIEM forwarding instead |
| Provision scratch space from the MSI SYSTEM context | No first-run provisioning code needed | Workspace owned by SYSTEM not user; R-B3 guard fails fleet-wide (Pitfall 4) | Never; provision at first-run in user context |
| Use Application log source without `wevtutil im` manifest | Works on dev host | Silent event-drop in production if registration was never run (Pitfall 10) | OK only if using the generic Application source (no manifest needed); NOT OK for custom channels |
| Single flat `REG_MULTI_SZ` for hosts and wildcard suffixes | Simple registry schema | Operator can't distinguish exact vs suffix entries; wildcard creep (Pitfall 3) | Never; use separate keys for hosts and suffixes |

## Integration Gotchas

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| HKLM registry reader | `RegOpenKeyExW` without `KEY_WOW64_64KEY` | Always pass `KEY_READ \| KEY_WOW64_64KEY` to read the 64-bit hive (Pitfall 7) |
| WFP + nono-proxy allowlist | Each layer parses HKLM independently | Deserialize once into `MachineEgressPolicy`; pass to both layers (Pitfall 2) |
| MSI custom action for scratch space | Resolve `%LOCALAPPDATA%` in SYSTEM context | Create scratch space at first-run from user context; MSI creates only machine-global `ProgramData\nono\` (Pitfall 4) |
| Event Log custom channel | Skip `wevtutil im` manifest registration | Use existing Application source pattern for v3.0; defer custom manifest (Pitfall 10) |
| Root CA cert install | `certutil -addstore Root` only | WiX `CertificateRef` for both `LocalMachine\Root` and `CurrentUser\Root`; verify `rustls_native_certs` in nono-proxy (Pitfall 13) |
| GPO ADMX + Intune MDM | Intune MDM Extension (32-bit) writes to WOW6432Node | All readers must use `KEY_WOW64_64KEY`; deployment scripts use `[RegistryView]::Registry64` (Pitfall 7) |
| Policy reload | `Arc<RwLock<AllowList>>` updated by file watcher | Snapshot policy at session start; no mid-session widening (Pitfall 9) |
| `DiagnosticFormatter` → SIEM | Pass `format_human_readable()` output as Event Log message | Define separate `SecurityEvent` struct with hashed paths; `DiagnosticFormatter` = stderr only (Pitfall 11) |

## Performance Traps

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| Per-request `RegOpenKeyExW` + full policy deserialization | Latency spike on every network connection attempt | Cache the policy at session start; re-read only on explicit policy-change notification | Any workload with > ~100 network requests/session |
| WFP filter bulk-install on every session start | Startup latency; WFP filter-churn; potential race with existing filters | Install WFP filters once in the service at policy-load time; individual session start just verifies the filter is active | Fleet with frequent short sessions (e.g. CI agents) |
| Event Log writes on every denied action | High-frequency agents generate thousands of events/min; Event Log fills; forwarding backlog | Deduplicate repeated deny events with a time-window bucket; emit a "N events suppressed" aggregate | Agents with tight retry loops hitting a blocked host |

## Security Mistakes

| Mistake | Risk | Prevention |
|---------|------|------------|
| `unwrap_or_default()` on HKLM registry read | Fail-OPEN: machine policy appears enforced but isn't | `ReadError` must be fatal; fail-secure return (Pitfall 1) |
| HMAC telemetry key in local registry | Attacker with local admin forges event chain; "tamper-evident" claim is false | External SIEM forwarding is the tamper boundary; local crypto deferred to SEED-005 (Pitfall 12) |
| Full path and URL in Event Log security event | PII/reconnaissance in SIEM forwarding; leaked corporate codepath structure | `SecurityEvent` schema with hashed path + category only; configurable detail level (Pitfall 11) |
| String suffix matching `host.ends_with(".corp")` | `evil.corp.com` bypasses the rule with `notcorp.com` crafted hostname | DNS component comparison; use `Path::starts_with` analogy for domain components (Pitfall 3) |
| Mid-session policy widening via registry watcher | Malicious insider pushes wider policy mid-session for exfiltration window | Snapshot-at-launch; narrowing only mid-session (Pitfall 9) |
| Service-start failure silently treated as success | Fleet machine not enforcing WFP egress; appears healthy to MDM | `nono health` verdict command; fail-secure on WFP service absent (Pitfall 5) |
| Per-machine MSI custom action writes to SYSTEM `%LOCALAPPDATA%` | Scratch space owned by SYSTEM; every user's R-B3 guard fails | First-run provisioner in user context; MSI writes only to `ProgramData` (Pitfall 4) |

## "Looks Done But Isn't" Checklist

- [ ] **HKLM policy reader:** Often missing the `ReadError` → abort path — verify a permission-denied key causes a non-zero exit, not a permissive launch.
- [ ] **Egress reconciliation:** Often wires only proxy, not WFP — verify both layers reflect the same HKLM-sourced allow-list via the dark-factory gate.
- [ ] **Wildcard suffix matching:** Often uses string `ends_with` — verify `anthropic.com.evil.com` is correctly rejected as NOT matching `.anthropic.com`.
- [ ] **Scratch space provisioner:** Often tested only in user context — verify the per-machine MSI install (ALLUSERS=1) creates workspace paths owned by the target user, not SYSTEM.
- [ ] **WOW64 registry reads:** Often missing `KEY_WOW64_64KEY` — verify nono reads the 64-bit hive even when tested from a 32-bit host script.
- [ ] **Event Log manifest registration:** Often only works on dev host where `wevtutil im` was run manually — verify a clean-host MSI install emits detectable events without prior manual setup.
- [ ] **Security event schema:** Often re-uses `DiagnosticFormatter` output — verify the Event Log event body does NOT contain a full file path or URL.
- [ ] **TOCTOU policy reload:** Often missing the mid-session widening test — verify a policy pushed while a session is running does not widen the running session's allow-list.
- [ ] **Service health verdict:** Often missing `nono health` command — verify the command emits a machine-readable JSON verdict and exits non-zero if WFP service is absent.
- [ ] **Root cert trust paths:** Often tested only for one client type — verify TLS through nono-proxy works from PowerShell (CryptoAPI), Node.js, AND from nono-cli (rustls/native-certs).

## Recovery Strategies

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| Fail-OPEN on HKLM read shipped to fleet | HIGH | Emergency GPO push + registry key creation; hotfix nono-cli to treat read error as fatal; re-deploy MSI |
| Proxy/WFP allowlist drift discovered post-deploy | HIGH | Audit WFP filter-state on fleet machines; push corrected policy; hotfix WFP reader to use shared struct |
| Scratch space owned by SYSTEM, R-B3 failing fleet-wide | MEDIUM | Run `nono setup --fix-scratch` (new command) in user context via SCCM run-as-user job; hotfix MSI custom action |
| Event Log events silent-dropped (no manifest registered) | MEDIUM | Push manifest registration as a SCCM/Intune remediation script; hotfix the emission code to treat RegisterEventSource NULL as an error |
| Full paths in SIEM events discovered in audit | MEDIUM | Redact/purge affected SIEM events (coordination with SIEM admin); deploy patched nono with `SecurityEvent` schema; configure `TelemetryDetailLevel=0` via GPO |
| Fake tamper-evidence claim in docs/compliance | LOW (docs) / HIGH (compliance audit) | Amend documentation to clarify external-SIEM-forwarding model; remove local crypto chain claims; expedite SEED-005 milestone |
| AI provider blocked by default-deny on first deploy | LOW-MEDIUM | `nono diagnose-egress api.anthropic.com`; GPO add the missing host group; session restart |

## Pitfall-to-Phase Mapping

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| 1. Fail-OPEN on HKLM read failure | Phase 83 (HKLM policy reader) | Dark-factory gate: permission-denied key → non-zero exit |
| 2. Proxy/WFP allowlist drift | Phase 83 (egress reconciliation) | Dark-factory gate: both layers reflect HKLM list |
| 3. Wildcard suffix matching footgun | Phase 83 (HKLM reader + proxy integration) | Unit test: `anthropic.com.evil.com` rejected |
| 4. MSI per-machine SYSTEM scratch-space | Phase 82 (silent MSI) | Dark-factory gate: `ALLUSERS=1` install → workspace owned by user |
| 5. Non-atomic service install / health verdict | Phase 82 (silent MSI) | Dark-factory gate: service-start failure → `nono health` reports degraded |
| 6. Env-var propagation lag | Phase 83 (HKLM reader) | Code review: no `std::env::var` in security policy path |
| 7. WOW6432Node registry redirection | Phase 83 (HKLM reader) | Unit test: 32-bit-simulated write to WOW hive → NOT read by nono |
| 8. Default-deny blocks AI provider | Phase 83 (egress + diagnostics) | `nono diagnose-egress api.anthropic.com` passes; diagnostic on block includes hostname |
| 9. TOCTOU policy reload widening | Phase 83 (HKLM reader) | Dark-factory gate: mid-session policy push → running session unaffected |
| 10. Event Log manifest not registered | Phase 84 (SIEM/EDR telemetry) | Dark-factory gate: clean-host install → detectable event emitted |
| 11. Secrets/paths in telemetry events | Phase 84 (SIEM/EDR telemetry) | Unit test: `SecurityEvent` serialized body contains no raw path strings |
| 12. False "tamper-evident" claim | Phase 84 (SIEM/EDR telemetry) | ADR records external-SIEM-forwarding model; no local crypto chain in v3.0 |
| 13. Root-cert TLS trust path | Phase 82 (silent MSI) | Dark-factory gate: TLS through proxy passes for PowerShell + Node + nono-cli |

## Sources

- In-tree code (HIGH — authoritative, current):
  - `crates/nono-cli/src/bin/nono-wfp-service.rs` — Application Event Log source pattern (`EVENT_LOG_SOURCE = SERVICE_NAME`), `RegisterEventSourceW` and `ReportEventW` usage, WFP control pipe SDDL `PIPE_SDDL`.
  - `crates/nono-cli/src/network_policy.rs` — `ResolvedNetworkPolicy`, `build_proxy_config`, `expand_proxy_allow`, suffix wildcard → `*.` conversion, `partition_allow_domain`, `is_loopback_domain` DNS-component fix (WR-01 precedent).
  - `crates/nono/src/diagnostic.rs` — `DiagnosticFormatter`, `DenialReason` enum (existing denial path; the re-use risk for SIEM emission is identified here).
- Project memory (HIGH):
  - `windows_mandatory_label_write_owner` — WRITE_OWNER not implicit; drive-root user dirs fail; `%USERPROFILE%` works; paired with `path_is_owned_by_current_user` + `GetEffectiveRightsFromAclW`.
  - `windows_msi_wxs_is_generated` — `.wxs` is generated from `build-windows-msi.ps1`; always edit the script.
  - `windows_appcontainer_wfp_validated` — per-run AppContainer + WFP kernel-blockable; must `CreateAppContainerProfile` (derive-only → ERROR_FILE_NOT_FOUND).
  - `windows_wfp_enforcement_is_service_only` — WFP = service-path only; `nono agent launch` → `wfp_filter_add`; not direct `nono run`; WFP-01 dark gate = structural filter-state proof.
- Phase history (HIGH):
  - Phase 53/61 — signed machine+user MSI pipeline; `release.yml` signing-order fix.
  - Phase 56 — fine-grained `allow_domain`; WR-01 `is_loopback_domain` DNS-component bug fix (string prefix hazard directly analogous to Pitfall 3).
  - Phase 60 — `grant_sid_write_on_path` / `AppliedDaclGrantsGuard` / R-B3 user-ownership guard.
  - v2.11 — `nono-wfp-service` start made non-fatal (Pitfall 5 background).
- SEED documents (HIGH): SEED-001, SEED-002, SEED-003.
- CLAUDE.md security principles (HIGH): "Configuration load failures must be fatal. If security lists fail to load, abort." + path string comparison footgun + `unwrap_or_default()` footgun.

---
*Pitfalls research for: nono v3.0 Enterprise Hardening I (Windows fleet deploy, machine-policy egress, SIEM telemetry)*
*Researched: 2026-06-18*
