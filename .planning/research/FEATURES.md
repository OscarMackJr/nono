# Feature Research

**Domain:** Enterprise-fleet hardening for an AI-agent sandbox (nono v3.0 Windows) — Deployment, Egress Control, Compliance telemetry
**Researched:** 2026-06-18
**Confidence:** HIGH for deployment/egress (well-established GPO/Intune/WFP conventions); MEDIUM for telemetry schema specifics (custom ETW provider conventions are documented but SIEM field normalization varies by product)

---

## Framing: What "enterprise-ready" actually requires

This milestone adds three orthogonal hardening layers ON TOP of the existing nono primitives. The right framing is:

- **Deployment (SEED-001):** IT can push nono to 500 machines without touching any of them manually.
- **Control (SEED-002):** Security can enforce a deny-by-default egress policy from HKLM — no user can loosen it.
- **Compliance (SEED-003):** CISO can see every blocked action as a structured security event in their SIEM, not a lost stderr line.

Everything below is evaluated as *net-new work* only. The existing WFP/proxy/MSI/DiagnosticFormatter surfaces are the integration points, not features to rebuild.

---

## Theme 1: Deployment (SEED-001)

### Table Stakes — Deployment

Features enterprise IT admins assume work before they'll evaluate the product.

| Feature | Why Expected | Complexity | Existing nono Hook |
|---------|--------------|------------|-------------------|
| **Silent/unattended MSI install** (`msiexec /i nono-machine.msi /qn /norestart`) | Every enterprise deployment tool (SCCM/MECM, Intune Win32 app, GPO software dist) requires zero UI. The `/qn` flag is the universal silent install contract — if it pops a dialog it's dead on arrival. Logged with `/L*v`. | LOW | Existing WiX machine MSI already built (Phase 53/61). Net-new: verify `/qn /norestart` exits 0 or 3010, no dialog shown. |
| **Correct MSI exit codes** (0 = success, 3010 = success-reboot-required, 1603 = fatal) | SCCM/Intune mark the deployment as failed if the installer exits with anything else. Missing or wrong exit codes mean the tool reports 100% failure rate and the admin gives up. | LOW | Need to verify WiX `ErrorCondition` → correct MSI return mapping. |
| **SYSTEM-context install** (`ALLUSERS=1`, writes to `%ProgramFiles%`, not `%APPDATA%`) | Intune Win32 apps run as SYSTEM by default. If the installer writes to user profile paths it fails silently under SYSTEM. Machine MSI already targets `Program Files` — needs a test under SYSTEM. | LOW | Machine MSI already per-machine. Need dark-gate test: install under `New-LocalUser` → SYSTEM run. |
| **Machine-wide service registration** (`nono-wfp-service` and `nono-agentd` as `SERVICE_AUTO_START` or on-demand, startable without an interactive session) | A service that only works when someone is logged in is not a machine-wide service. Admins expect `sc start` to work from a remote session. | LOW | Services exist. Need to verify they start under SCM without a logged-in user (non-interactive). |
| **Machine-wide `PATH` registration** (nono binary visible to all users without manual profile edit) | A tool in `%ProgramFiles%` that isn't on `PATH` requires a manual user step on every machine. The MSI MUST add `%ProgramFiles%\nono` to the SYSTEM PATH during install. | LOW | Absent from current MSI — net-new WiX `Environment` element. |
| **Intune Win32 app detection rule** (registry key or file presence at install path for Intune compliance checks) | Intune requires a detection rule to know the app is already installed. Without it, every compliance cycle re-deploys. Standard: check `HKLM\SOFTWARE\nono` version key OR `%ProgramFiles%\nono\nono.exe` exists. | LOW | Net-new registry write in MSI. |
| **Silent root-certificate install** (broker signing cert trusted on all machines without manual click-through) | The broker requires a trusted Authenticode cert. An enterprise MSI MUST silently import it to `Cert:\LocalMachine\Root` — the same mechanism Intune's Trusted Certificate Profile uses (`.cer` in Base-64 X.509 pushed to Computer Store – Root). | MEDIUM | `nono setup --trust-broker` is interactive. Net-new: silent import via MSI `CustomAction` using `certutil -addstore -f ROOT`. |
| **Auto-provisioned user scratch space** with `WRITE_OWNER` inheritance (eliminates manual profile-owned-CWD setup per user) | Admins cannot run `takeown` on each user's machine. The service or first-run provisioner must create `%LOCALAPPDATA%\nono\workspaces\<user>` with the correct DACL + WRITE_OWNER. | MEDIUM | Phase 60 `grant_sid_write_on_path`/`AppliedDaclGrantsGuard` exists. Net-new: provisioner called at login or by daemon on first agent launch. |

### Differentiators — Deployment

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| **GPO ADMX template** (`nono.admx` + `nono.adml`) for pushing machine policy from Group Policy Management Console | Admins at GPO-managed shops (large enterprise, government) expect to configure security tools through GPEDIT/GPMC like they configure Edge, Office, Chrome. An ADMX drops nono into the existing toolchain with no custom scripts. Policy writes to `HKLM\SOFTWARE\Policies\nono` under Computer Configuration. | MEDIUM | No ADMX template exists. Net-new XML authoring. Pairs with SEED-002 policy spine. |
| **Intune OMA-URI / CSP mapping** for the same `HKLM\SOFTWARE\Policies\nono` keys | MDM-managed shops use Intune settings catalog or custom OMA-URI (`./Device/Vendor/MSFT/Policy/Config/<scope>/<setting>`). ADMX-backed policies can be ingested via `Policy/ConfigOperations/ADMXInstall`. Intune-only shops don't run GPMC — they need this second path. | MEDIUM | Requires publishing the ADMX to Intune or authoring direct OMA-URI settings. Pairs with ADMX. |
| **`nono setup --provision-fleet` CLI verb** (non-interactive, idempotent service config + scratch space + trust anchor from a single admin command) | Enables scripted provisioning in environments that don't use GPO/Intune (startup scripts, Ansible, golden image baking). One idempotent command replaces the current manual 5-step checklist. | MEDIUM | New CLI verb; wraps existing `nono setup` interactive path; must be safe to run repeated times. |
| **MSI upgrade path** (version detect + in-place upgrade without uninstall/reinstall) | SCCM/Intune upgrade deployments fail if the MSI requires an uninstall step first. A versioned `UpgradeCode` in the WiX template plus a `MajorUpgrade` action gives admins clean upgrades. | LOW | WiX supports this with two XML elements; existing MSI needs `UpgradeCode` GUID + `MajorUpgrade` element. |

### Anti-Features — Deployment

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|-----------------|-------------|
| **MSIX packaging** instead of MSI | MSIX is the "modern" Windows packaging format and has Intune-native support. | MSIX's virtualized registry/filesystem layer conflicts with nono's kernel-level WFP service registration and SYSTEM PATH writes — services can't be registered from within an MSIX app container boundary without MSIX service extensions (requires signed Store submission or enterprise sideload infra). MSI + Win32 Intune app is the correct path for security tools with kernel services. | Stick with WiX MSI + Intune Win32 app deployment (.intunewin wrapper). MSIX defer to a packaging milestone if ever justified. |
| **Per-user install as the default** | Seems simpler; no elevation required. | A per-user install means no machine-wide PATH, no shared `nono-wfp-service`, and each user gets a different binary version. WFP requires SYSTEM-level service registration (elevated). The current machine MSI is correct — don't regress. | Machine MSI (`ALLUSERS=1`) with per-user scratch-space provisioning at first run. |
| **Reboot required for all installs** | Some tools require a reboot to register services. | A mandatory reboot during a fleet rollout is a fleet disruption event. The WFP service can be started post-install without a reboot if the MSI `CustomAction` calls `sc start` after WiX `ServiceInstall`. | Target exit code 3010 (reboot optional, not forced). Non-mandatory reboot is table stakes. |

---

## Theme 2: Egress Control (SEED-002)

### Table Stakes — Egress Control

| Feature | Why Expected | Complexity | Existing nono Hook |
|---------|--------------|------------|-------------------|
| **Machine-policy-managed allowlist** sourced from `HKLM\SOFTWARE\Policies\nono\EgressAllowlist` (REG_MULTI_SZ) | Security architects expect *machine policy to be immutable by the user*. If the allowlist lives only in a per-user JSON profile, a user can add `*.evil.com` and disable the control. HKLM policy must win over per-user profile, just as HKLM Group Policy wins over HKCU in all Windows security tools. | MEDIUM | No HKLM policy reader exists. Net-new registry reader in the proxy/WFP startup path. |
| **Deny-by-default posture** when the machine policy key is present (empty `EgressAllowlist` = total deny) | Enterprise expectation: presence of the machine policy key opts the machine into deny-by-default egress. Absence = "not yet governed" (current permissive behavior preserved). This matches how Windows SRP/AppLocker work — the policy key's presence switches the enforcement mode. | LOW | `ProxyFilter::new_strict([])` already exists. Needs a mode gate triggered by the HKLM key's presence. |
| **Wildcard subdomain syntax** (`*.anthropic.com`, `*.corp.example.com`) | Every enterprise egress tool (Zscaler, Netskope, Cloudflare One) supports wildcard FQDN matching. Admin expectation: `*.anthropic.com` allows `api.anthropic.com` and `claude.ai` without enumerating every subdomain. Exact-host-only matching is a non-starter for AI providers. | LOW | `HostFilter` exists. Check whether current wildcard logic covers `*.x.com` vs `x.com` subdomains — ensure exact-root match is NOT included by `*.x.com` (security: `*.evil.com` should not match `evil.com`). |
| **Machine policy takes precedence over per-user profile** | An enterprise admin who sets a machine policy must know users cannot override it. The read order must be: HKLM policy → user profile (additive) → runtime flags. A user's `allow_domain` additions in their JSON profile must not be honored if HKLM is in deny-by-default mode. | MEDIUM | Per-user profile reader (`policy.rs`) is the only current path. Net-new: merge logic that ignores user-profile `allow_domain` entries when HKLM policy key is present (fail-secure). |
| **AI-provider presets** (built-in allowlist entries for `*.anthropic.com`, `*.openai.com`, `api.github.com`, `models.github.com`) | Admins don't know Claude's CDN hostnames. A named preset (e.g. `allow_preset = "ai-providers"` in the policy) that expands to the known AI provider FQDN set removes the per-hostname research burden and prevents misconfiguration. Anthropic, OpenAI, and GitHub Copilot are the three the market expects. | LOW | A static preset map in the WFP/proxy startup code. Needs a maintenance process to update as providers add CDN domains. |
| **`nono-wfp-service` + `nono-proxy` reading from the same policy source** | Today proxy reads per-user profile; WFP reads WFP filter rules set at daemon-launch time. An admin who sets a machine policy must get enforcement at both layers simultaneously, without wiring them separately. The single source is HKLM. | MEDIUM | Two separate readers today. Net-new: policy-spine adapter that both consumers call at startup and that both refresh on policy change (or restart). |

### Differentiators — Egress Control

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| **Policy-change live reload** (service detects HKLM key change via `RegNotifyChangeKeyValue` and reloads allowlist without restart) | Admins pushing a policy update to 500 machines don't want to restart services manually. Live reload means the updated allowlist takes effect within seconds via GPO propagation, matching how security tools like Windows Firewall policy updates work. | HIGH | `RegNotifyChangeKeyValue` in an async loop in `nono-wfp-service` + `nono-proxy`. Carefully handle mid-reload state: existing connections must be killed or allowed to drain (fail-secure default: kill). |
| **`nono verify-egress` subcommand** (machine-level gate: tests that deny-by-default is actually in effect, confirms AI-provider hosts resolve through the proxy, confirms WFP blocks an off-allowlist host) | An auditor or IT admin who just pushed the policy needs to confirm it's working without reading source code. This is the fleet-scale equivalent of `nono setup --check-only` for the network layer. Outputs a verdict per enforcement layer. | MEDIUM | New CLI verb; internally calls `ProxyFilter::check_host` + a WFP filter query. Pairs well with the Dark Factory `verify-dark.ps1` gate pattern. |
| **`nono-proxy` + WFP deny-event correlation** (when a connection is blocked, link the proxy-level deny reason to the WFP filter rule that blocked it) | Separate proxy-deny logs and WFP filter events make incident investigation slow. Correlation lets the SIEM show "denied by policy `EgressAllowlist`: `*.internal.corp.com` not matched" as a single event instead of two disconnected log lines. | HIGH | Needs a shared event ID / correlation token emitted at both layers. Pairs with SEED-003 telemetry. |
| **Corporate HTTPS proxy chaining** (`NONO_UPSTREAM_PROXY` or HKLM policy key for environments where all egress traverses Zscaler/Netskope) | Enterprises running a mandatory corporate proxy must be able to chain nono-proxy → corporate proxy, or the allowlist has no value (traffic bypasses nono entirely). | MEDIUM | `--upstream-proxy` flag exists in nono-proxy. Net-new: honor an HKLM policy key so the proxy chain is set fleet-wide without per-user config. |

### Anti-Features — Egress Control

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|-----------------|-------------|
| **IP-address allowlisting instead of FQDN** | "We know the AI provider IPs, just allowlist them." | AI provider CDNs have dynamic IP pools that change frequently. IP-based allowlists break silently and require constant maintenance. All enterprise-grade egress tools (Zscaler, Cloudflare One, Netskope) moved to FQDN/wildcard matching for exactly this reason. | FQDN + wildcard matching is the correct primitive. IP-based blocking as a belt-and-suspenders for known malicious IP ranges is acceptable but NOT as the primary allowlist mechanism. |
| **User-configurable machine policy** (surfacing HKLM keys as user-editable settings in a GUI or per-user JSON) | "Make it easy for users to add domains." | If users can write to the machine policy, the machine policy provides no security guarantee. HKLM keys are ACL'd to SYSTEM/Administrators; user write access defeats the entire threat model. | Per-user `allow_domain` in the user profile is the user's additive channel — ONLY when the machine policy is not in strict mode. |
| **Proxy-only enforcement without WFP** | "The proxy is easier to configure." | `nono-proxy` is a user-mode process that an agent can bypass by creating a direct TCP socket (CONNECT_TUNNEL, raw socket). WFP kernel enforcement is the layer that makes bypass structurally impossible. Both layers together are the enterprise claim. Either alone is defense-in-depth but not the full story. | Ship both layers wired to the same HKLM policy spine. Document that proxy = application-layer visibility + credential injection; WFP = kernel enforcement. |

---

## Theme 3: Compliance / Telemetry (SEED-003)

### Table Stakes — Compliance

| Feature | Why Expected | Complexity | Existing nono Hook |
|---------|--------------|------------|-------------------|
| **Structured security events in Windows Event Log** (custom Application channel, e.g. `nono/Security`, with an ETW provider GUID, registered manifest `.man` + resource DLL) | Every Windows SIEM agent (Splunk UF, Azure Monitor Agent, MMA, WEF subscription) forwards from Windows Event Log channels. A tool that writes only to stderr is invisible to the SIEM estate. Custom Application channel (not Security channel — that requires SeAuditPrivilege and is reserved for the OS) is the correct tier for application-level security events. | HIGH | No Event Log channel exists. Net-new: author `nono.man` ETW manifest, compile with `mc.exe`, embed resource DLL, register on install via MSI `CustomAction`. |
| **Structured event schema with named `EventData` fields** (not free-text messages) | Splunk's `XmlWinEventLog` input and Sentinel's Windows Event Log connector parse `<Data Name="...">` fields as structured columns. A free-text message string requires regex extraction in every SIEM deployment; named fields are parsed automatically. Schema must include at minimum: `EventId`, `TimeCreated`, `Computer` (hostname), `ProcessId`, `SessionId`, `UserId` (SID), `EventType` (path_deny / network_deny / label_violation / hook_fail_closed), `Resource` (path or host), `PolicySource` (group name or HKLM key), `Outcome` (denied / allowed). | MEDIUM | `DiagnosticFormatter` has `DenialRecord` + `PolicyExplanation` structs with most of these fields. Net-new: map to named `EventData` elements in the ETW write call. |
| **Defined EventID space** (distinct IDs for each event type; range reserved in the manifest) | SIEM alert rules reference specific EventIDs. If all events use the same ID, every alert fires on every event type. The standard is: one EventID per event class. Proposed: `10001` = path deny, `10002` = network deny, `10003` = integrity violation (hook fail-closed), `10004` = label violation, `10005` = policy reload. Range `10000–10099` reserved for nono Security events. | LOW | Design-time decision; zero implementation cost if done before manifest authoring. |
| **Defined severity levels** (EventLog `Level`: 1=Critical, 2=Error, 3=Warning, 4=Info) | SIEM alert triage uses severity levels. Path denials during normal agent operation = Warning. Network deny for an off-allowlist host = Error (active exfiltration attempt blocked). Hook fail-closed (policy enforcement failure) = Critical. Info = normal operation events (agent started, policy loaded). | LOW | Design-time; mapped at ETW write call. |
| **WEF-compatible channel configuration** (proper channel type = `Operational`, max size, retention policy in manifest) | Windows Event Forwarding subscriptions pull from `Operational` channels, not `Debug` or `Analytic` (the latter require explicit subscription configuration). The manifest must declare `channel type="Operational"` for WEF to forward automatically via standard subscriptions. | LOW | Manifest design choice; zero runtime cost. |
| **Events wired from existing `DiagnosticFormatter` deny path** | The deny decision already happens; it must also emit a structured event. The wiring point is the supervisor's post-execution diagnostic path (`format_footer` / `format_supervised_footer`) and the real-time denial recording path (`with_denials`). Net-new: an `emit_security_event()` call at each `DenialRecord` capture site. | MEDIUM | `DiagnosticFormatter` is the source-of-truth for denials. Proxy filter denials and WFP denials need separate wiring points. |
| **Syslog emission** (RFC 5424 structured-data format, UDP/TCP to a configurable endpoint) | Not every shop runs Windows-native WEF. Linux-majority shops, cloud-native SIEMs, and shops running Splunk HEC or a syslog aggregator need RFC 5424 structured syslog. The structured-data block (`[nono@12345 type="path_deny" resource="C:\secret" policy="ssh"]`) maps directly to the same event schema. | MEDIUM | No syslog emitter exists. Net-new: a `syslog` crate integration in the supervisor binary, configured via HKLM policy key `SyslogEndpoint` (host:port) or per-user profile. |

### Differentiators — Compliance

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| **Tamper-evident event chain** (each Event Log event includes a SHA-256 HMAC over `prev_hash ‖ event_content`, with a per-session chain key derived from the session ID) | A sophisticated attacker who gains admin rights can clear the Event Log (`wevtutil cl`). An HMAC chain means the SIEM can detect a gap (missing prev_hash sequence) or log tampering even after the fact. This is the lightweight SEED-003 tamper-evidence — not an immutable ledger (that's SEED-005 ZT-Infra), but enough for "tamper-evident append-only chain" as the v3.0 claim. | HIGH | Extends the existing `AuditRecorder` hash-chain pattern (v2.2 AUD-01: per-event leaf hash + chain head + Merkle root). New: same pattern applied to Event Log event payloads. Chain-head included as a named `EventData` field `ChainHead` in every emitted event. |
| **`nono audit security-log verify <session-id>`** (recompute chain over all events for a session and report any gap or hash mismatch) | An auditor can confirm the event chain is intact for a specific session. This is the SIEM-facing companion to `nono audit verify` (AUD-02) for the structured event stream. | MEDIUM | Reads events from the custom channel by EventRecordID range (session scope), recomputes chain. |
| **Sentinel / Splunk pre-built query templates** (KQL workbook for Sentinel; SPL search for Splunk, targeting the custom channel by `Channel` and EventIDs) | Admins don't write SIEM queries from scratch. A shipped KQL workbook or Splunk saved search that tiles "blocked path denials last 24h", "top 10 agents by network-deny count", and "policy reload events" gives immediate time-to-value. | LOW | Shipped as `docs/siem/` — a KQL file + SPL file referencing the `nono/Security` channel and EventID range 10000–10099. |
| **Correlation token** linking proxy deny → WFP deny → Event Log entry (single `CorrelationId` UUID per agent request that shows up in all three places) | For a blocked exfiltration attempt, today an admin sees three unrelated log entries. A shared `CorrelationId` (set at the supervisor level, passed to proxy and WFP event emission) lets a single SIEM join produce the full story: "Agent X tried to reach evil.com, proxy denied it, WFP filter also dropped the packet, event emitted." | HIGH | Requires correlation ID threading from the agent's network request down through both enforcement layers. Major wiring work; consider deferring to v3.1 if scope is tight. |

### Anti-Features — Compliance

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|-----------------|-------------|
| **Writing to the Windows Security channel** (`Security` log, EventIDs 4xxx) | Sounds authoritative — the Security channel is what SIEMs monitor most closely. | Writing to the Security channel requires the `SeAuditPrivilege`, which is granted only to LSASS and a few OS components. A user-mode application trying to write there will either be silently dropped or require elevation that defeats the principle of least privilege. Custom Application channel + WEF forwarding is the correct mechanism. | Custom `nono/Security` channel (Application tier) + WEF subscription that forwards to the SIEM's Security index. |
| **Storing raw policy content in events** (emit the full ADMX policy dump or full allowlist in every event) | "More context = easier investigation." | Bloats every event. A 500-entry allowlist repeated in 10,000 events/day = significant SIEM ingest cost. Field-name references (e.g. `PolicySource = "EgressAllowlist[*.anthropic.com]"`) give traceability without bloat. | Reference the triggering policy entry by name/key, not its full value. |
| **Real-time SIEM push via API** (have nono call Splunk HEC or Sentinel API directly) | "Skip the forwarding agent." | Requires credentials stored on the machine, a direct outbound connection to the SIEM endpoint (which conflicts with the deny-by-default egress policy), and per-SIEM integration maintenance. WEF → SIEM agent is the correct architecture: one integration maintained by the SIEM vendor, not N maintained by nono. | Emit structured events to the Windows Event Log; let the existing SIEM agent (Splunk UF, AMA) forward them. That's the contract enterprises have with Windows applications. |
| **Verbose Info-level events for every allowed operation** | "Log everything so we can audit compliance retroactively." | High-cardinality Info events (every allowed file read, every allowed network connection) flood the SIEM, increase ingest costs, and trigger noise fatigue. SIEM teams invariably suppress verbose info events within weeks. | Log denied/blocked events and policy-change events. Allowed operations are the success path — they don't need telemetry unless the session is in an explicit audit mode. |

---

## Feature Dependencies

```
DEPLOYMENT
  Silent MSI (/qn)
      └──requires──> ALLUSERS=1 machine install
      └──requires──> Correct exit codes (0, 3010, 1603)
      └──enables──> SYSTEM-context install (Intune)

  Machine-wide service
      └──requires──> Silent MSI (service registered at install)
      └──requires──> Non-interactive SCM start (no logged-in user)

  ADMX template / OMA-URI
      └──requires──> HKLM policy spine (HKLM\SOFTWARE\Policies\nono)
      └──enables──> Egress policy push via GPO/Intune
      └──enables──> Syslog endpoint config push via GPO/Intune

  Auto-provisioned scratch space
      └──requires──> Machine-wide service (provisioner runs at service start or login)
      └──requires──> grant_sid_write_on_path (Phase 60, exists)

CONTROL (SEED-002)
  HKLM policy reader
      └──requires──> HKLM policy spine (Deployment prerequisite)
      └──enables──> Machine-policy allowlist enforcement
      └──enables──> AI-provider presets

  Machine policy precedence over user profile
      └──requires──> HKLM policy reader
      └──requires──> Per-user policy.rs reader (exists)
      └──conflicts──> user-configurable machine policy (anti-feature)

  WFP + proxy reading from same HKLM source
      └──requires──> HKLM policy reader
      └──requires──> nono-wfp-service (exists)
      └──requires──> nono-proxy ProxyFilter::new_strict (exists)

COMPLIANCE (SEED-003)
  ETW manifest + custom channel
      └──requires──> MSI CustomAction for manifest registration (mc.exe / wevtutil im)
      └──requires──> Deployment (Silent MSI) — channel registered at install time

  Structured event emission
      └──requires──> ETW manifest (defines EventIDs and named fields)
      └──requires──> DiagnosticFormatter DenialRecord → EventData field mapping
      └──requires──> Proxy filter deny hook (net-new wiring point)
      └──requires──> WFP deny hook (net-new wiring point)

  Tamper-evident chain
      └──requires──> Structured event emission
      └──requires──> AuditRecorder hash-chain pattern (AUD-01, exists)
      └──enhances──> SEED-005 ZT-Infra ledger (future milestone)

  Syslog emission
      └──requires──> Structured event schema (same fields as EventLog path)
      └──requires──> HKLM SyslogEndpoint policy key (Deployment prerequisite)

  Correlation token
      └──requires──> Structured event emission
      └──requires──> Proxy deny hook
      └──requires──> WFP deny hook
      └──HIGH complexity — consider deferring to v3.1
```

### Dependency Notes

- **HKLM policy spine is the shared prerequisite for Control and Compliance.** The ADMX template, the egress policy reader, and the Syslog endpoint config all read from `HKLM\SOFTWARE\Policies\nono`. Build the registry reader once, make it available to all three consumers.
- **The ETW manifest registration must happen at MSI install time.** The `mc.exe`-compiled `.man` + resource DLL must be present and registered (`wevtutil im`) before any event emission attempt, or the `ReportEvent` / ETW `EventWrite` call silently fails. This makes the ETW manifest a hard dependency of the MSI.
- **Deployment is the foundation for both Control and Compliance.** Without a machine-wide install, there is no HKLM spine to read and no Event Log channel to write to.
- **The tamper-evident chain extends AUD-01 (shipped), not a greenfield design.** The hash-chain pattern is already proven in the `AuditRecorder`. The net-new part is applying it to Event Log events and exposing `ChainHead` as a named `EventData` field.
- **Correlation token is the highest-complexity item in SEED-003.** It requires threading a UUID from the agent request context into both the proxy and WFP deny paths. Consider phasing: emit without correlation first, add correlation in v3.1.

---

## MVP Definition

### Launch With (v3.0)

- [ ] **Silent MSI install** (`/qn /norestart`, correct exit codes, ALLUSERS=1, machine-wide PATH) — SEED-001 table stakes, unblocks all fleet deployment.
- [ ] **HKLM policy reader** (`HKLM\SOFTWARE\Policies\nono`, REG_MULTI_SZ `EgressAllowlist`, binary policy presence = strict mode) — the shared spine for SEED-002 and SEED-003.
- [ ] **Machine-policy-managed deny-by-default egress** (HKLM → `ProxyFilter::new_strict` + WFP; machine policy overrides user profile; AI-provider presets) — SEED-002 primary claim.
- [ ] **Custom ETW provider + Application channel registration** (`nono/Security` channel, manifest, named `EventData` fields, EventIDs 10001–10005) — SEED-003 prerequisite for all SIEM integration.
- [ ] **Structured event emission from DiagnosticFormatter denial path** (path deny, network deny, hook fail-closed → named EventData → Windows Event Log) — SEED-003 minimum viable.
- [ ] **ADMX template** (`nono.admx` + `nono.adml`, pushes `EgressAllowlist` to HKLM via GPMC) — SEED-001 differentiator + SEED-002 deployment mechanism.
- [ ] **Auto-provisioned scratch space** (WRITE_OWNER, user-owned DACL, provisioner in service/daemon) — eliminates the last manual setup step.
- [ ] **Silent root-cert install via MSI** (silently trusts the broker cert at install, no click-through) — closes the last clean-host friction.

### Add After Validation (v3.x)

- [ ] **Syslog emission** — trigger: Event Log emission working; add RFC 5424 path for non-WEF shops.
- [ ] **`nono verify-egress` subcommand** — trigger: HKLM policy and WFP both wired; add the fleet verification gate.
- [ ] **Intune OMA-URI / CSP mapping** — trigger: ADMX template stable; add the MDM path as a second delivery mechanism.
- [ ] **Tamper-evident HMAC chain for Event Log** — trigger: basic event emission working; layer on chain verification.
- [ ] **MSI upgrade path** (`UpgradeCode` + `MajorUpgrade`) — trigger: v3.0 MSI stable; add for v3.1 upgrade deployment.

### Future Consideration (v3.1+)

- [ ] **Correlation token** (proxy deny ↔ WFP deny ↔ Event Log correlation) — high complexity, major wiring; phase after basic telemetry is stable.
- [ ] **Policy-change live reload** (`RegNotifyChangeKeyValue`) — defer until policy reload semantics are well-understood (especially mid-session kill vs drain behavior).
- [ ] **`nono audit security-log verify`** — defer until tamper-evident chain ships.
- [ ] **SEED-005 ZT-Infra immutable ledger** — its own standalone milestone; depends on SEED-003 audit pipeline.

---

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| Silent MSI `/qn` + correct exit codes | HIGH | LOW | P1 |
| Machine-wide PATH registration in MSI | HIGH | LOW | P1 |
| HKLM policy reader (registry) | HIGH | LOW | P1 |
| Machine-policy deny-by-default egress | HIGH | MEDIUM | P1 |
| WFP + proxy from same HKLM source | HIGH | MEDIUM | P1 |
| ETW manifest + channel registration | HIGH | HIGH | P1 |
| Structured event emission (path deny, net deny) | HIGH | MEDIUM | P1 |
| ADMX template (GPMC push) | HIGH | MEDIUM | P1 |
| Auto-provisioned scratch space | HIGH | MEDIUM | P1 |
| Silent root-cert install via MSI | HIGH | MEDIUM | P1 |
| AI-provider presets | MEDIUM | LOW | P1 |
| SYSTEM-context install verification | MEDIUM | LOW | P1 |
| Syslog emission | MEDIUM | MEDIUM | P2 |
| `nono verify-egress` gate | MEDIUM | MEDIUM | P2 |
| Intune OMA-URI/CSP mapping | MEDIUM | MEDIUM | P2 |
| Tamper-evident HMAC chain | MEDIUM | HIGH | P2 |
| MSI upgrade path | MEDIUM | LOW | P2 |
| Policy-change live reload | LOW | HIGH | P3 |
| `nono audit security-log verify` | LOW | MEDIUM | P3 |
| Correlation token | LOW | HIGH | P3 |

**Priority key:** P1 = must have for v3.0 · P2 = should add in v3.x · P3 = future consideration

---

## Comparable Product Feature Analysis

| Approach | Standard enterprise security tool (e.g., CrowdStrike Falcon sensor, Sysmon) | Ad-hoc script-based AI agent controls | nono v3.0 |
|----------|-----------------------------------------------------------------------------|---------------------------------------|-----------|
| Silent MSI with SYSTEM-safe install | Yes (table stakes) | No | **Yes** (P1) |
| ADMX / GPO / Intune push | Yes (table stakes) | No | **Yes** (P1, net-new) |
| Machine-policy overrides user profile | Yes (table stakes) | No | **Yes** (P1, net-new) |
| Structured Windows Event Log events (named fields, EventID space) | Yes (table stakes) | No | **Yes** (P1, net-new) |
| WEF-forwardable events | Yes | No | **Yes** (P1, design choice) |
| Kernel-enforced network deny (WFP) | Varies | No | **Yes** (existing, wired to policy) |
| Tamper-evident event chain | Varies (some tools) | No | **P2** (HMAC over existing hash-chain) |
| Live policy reload | Varies | No | **P3** |

nono's edge in the enterprise context: uniquely combines OS-enforced agent confinement (AppContainer + WFP) with a unified HKLM policy spine that drives both the network enforcement layer and the compliance telemetry layer from a single registry source pushable via standard GPO/Intune tooling. No other tool in this space enforces at the kernel level while also emitting structured SIEM-ready events from the same policy decision.

---

## Sources

- Milestone scope: `.planning/PROJECT.md` "Current Milestone: v3.0 Enterprise Hardening I" — HIGH
- SEED files: `.planning/seeds/SEED-001/002/003-*.md` — HIGH
- [msiexec /qn silent install enterprise patterns — AdvancedInstaller](https://www.advancedinstaller.com/silent-install-exe-msi-applications.html) — HIGH
- [Intune Win32 app deployment — Microsoft Learn](https://learn.microsoft.com/en-us/intune/app-management/deployment/add-win32) — HIGH (official)
- [Policy CSP — Microsoft Learn](https://learn.microsoft.com/en-us/windows/client-management/mdm/policy-configuration-service-provider) — HIGH (official)
- [ADMX-backed policies in Intune — Microsoft Learn](https://learn.microsoft.com/en-us/intune/intune-service/configuration/administrative-templates-windows) — HIGH (official)
- [Understanding ADMX policies — Microsoft Learn](https://learn.microsoft.com/en-us/windows/client-management/understanding-admx-backed-policies) — HIGH (official)
- [Implementing Registry-based Policy — Microsoft Learn](https://learn.microsoft.com/en-us/previous-versions/windows/desktop/policy/implementing-registry-based-policy) — HIGH (official)
- [Writing Manifest-based ETW Events — Microsoft Learn](https://learn.microsoft.com/en-us/windows/win32/etw/writing-manifest-based-events) — HIGH (official)
- [Writing an Instrumentation Manifest — Microsoft Learn](https://learn.microsoft.com/en-us/windows/win32/wes/writing-an-instrumentation-manifest) — HIGH (official)
- [Custom Windows Event Forwarding Logs — Microsoft Learn](https://learn.microsoft.com/en-us/archive/blogs/russellt/creating-custom-windows-event-forwarding-logs) — HIGH (official)
- [Configuring Windows event logs for Enterprise Security use — Splunk Lantern](https://lantern.splunk.com/Security/Product_Tips/Enterprise_Security/Configuring_Windows_event_logs_for_Enterprise_Security_use) — HIGH
- [Windows Events collection in Sentinel — Microsoft TechCommunity](https://techcommunity.microsoft.com/blog/fasttrackforazureblog/windows-events-how-to-collect-them-in-sentinel-and-which-way-is-preferred-to-det/3997342) — MEDIUM
- [Egress Gateway Allowlisting — JumpCloud](https://jumpcloud.com/it-index/what-is-egress-gateway-allowlisting) — MEDIUM
- [Control which domains your AI agents can access — AWS ML Blog](https://aws.amazon.com/blogs/machine-learning/control-which-domains-your-ai-agents-can-access/) — MEDIUM
- [Claude Code allow network egress wildcard matching inconsistency — GitHub Issue #51400](https://github.com/anthropics/claude-code/issues/51400) — MEDIUM (confirms wildcard FQDN is the industry expectation)
- [Set event log security via Group Policy — Microsoft Learn](https://learn.microsoft.com/en-us/troubleshoot/windows-server/group-policy/set-event-log-security-locally-or-via-group-policy) — HIGH (official)
- [Palantir Windows Event Forwarding reference — GitHub](https://github.com/palantir/windows-event-forwarding/blob/master/windows-event-channels/README.md) — MEDIUM
- Internal: `crates/nono-proxy/src/filter.rs` (ProxyFilter, HostFilter, `new_strict` method) — HIGH (codebase)
- Internal: `crates/nono/src/diagnostic.rs` (DiagnosticFormatter, DenialRecord, PolicyExplanation structs) — HIGH (codebase)
- Internal: AUD-01 hash-chain pattern (v2.2 Phase 22, `AuditRecorder` in `crates/nono/src/undo/`) — HIGH (codebase)

---
*Feature research for: enterprise fleet deployment, egress control, and compliance telemetry (nono v3.0 Windows)*
*Researched: 2026-06-18*
