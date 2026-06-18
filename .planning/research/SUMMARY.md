# Project Research Summary

**Project:** nono v3.0 -- Enterprise Hardening I (Deploy - Control - Compliance)
**Domain:** Enterprise-fleet hardening of a shipping Rust/Windows security product -- silent MSI deploy, machine-policy-driven egress control (HKLM), SIEM/EDR-forwardable structured telemetry
**Researched:** 2026-06-18
**Confidence:** HIGH -- all four researchers grounded findings in current in-tree source code, official Microsoft Learn docs, and verified crates.io/docs.rs data

---

## Executive Summary

nono v3.0 adds three enterprise-hardening layers -- deployment (SEED-001), egress control (SEED-002), and compliance telemetry (SEED-003) -- on top of the existing, shipping AppContainer + WFP + nono-proxy confinement stack. The domain is well-understood: these are standard Windows enterprise security product patterns (GPO ADMX, HKLM policy, Windows Event Log + ETW, WEF forwarding), and all four research threads converge on the same load-bearing insight: HKLM\SOFTWARE\Policies\nono is the shared spine. It must be built first because egress control and telemetry configuration both read from it. Build it wrong and everything downstream either fails open or produces false security guarantees.

The recommended technical approach is deliberately lean: four new crates (winreg 0.56, tracing-etw 0.2.3, eventlog 0.4.0, hmac 0.13.0) plus one windows-sys 0.59 feature addition (Win32_System_Registry). No version bumps, no new workspace members, no MSIX migration. The WiX MSI stays exactly as-is -- MSIX cannot package the LocalSystem WFP service and would require abandoning the existing signed-MSI CI pipeline for zero functional gain. The telemetry layer hooks into nono-cli as a tracing::Layer (a new telemetry/ module), not into the library DiagnosticFormatter -- preserving the library-vs-CLI boundary that is a core design invariant of this codebase.

The two load-bearing security invariants the roadmapper must enforce phase-by-phase are: (1) **fail-secure on HKLM read failure** -- a registry error must abort, never fall back to permissive defaults; and (2) **single deserialized struct, two consumers** -- proxy and WFP must read the egress allowlist from one MachineEgressPolicy struct, never from independent registry reads. Violating either invariant ships a false security claim. The tamper-evident telemetry claim is honestly scoped to Windows Event Forwarding (external SIEM copy, not a local crypto chain) -- cryptographic-local anchoring belongs to SEED-005 ZT-Infra, not this milestone.

---

## Key Findings

### Recommended Stack

The stack delta is minimal. The workspace already carries windows-sys 0.59, tracing/tracing-subscriber, serde/serde_json, and sha2 -- all reused without change. The net-new additions are four crates, all Windows-gated except hmac.

winreg 0.56 is the correct choice for HKLM reads over raw windows-sys FFI: it wraps RegQueryValueExW two-call size-then-data pattern into typed safe Rust (get_value::<u32>(), get_value::<String>()), and its windows-sys >= 0.59, <= 0.61 dep range resolves to the existing workspace pin with no second copy. The Win32_System_Registry feature must be added to the existing windows-sys feature list (currently absent from line 149 of nono-cli/Cargo.toml) to cover the one raw call needed for RegNotifyChangeKeyValue (live-reload, optional for v3.0). The existing platform.rs reg.exe-subprocess approach is acceptable for OS-version detection but unacceptable for hot-path policy reads -- winreg replaces it on the policy path only.

tracing-etw 0.2.3 (ETW/TraceLogging, the SIEM real-time bus path) and eventlog 0.4.0 (Windows Event Log, the Event Viewer/Get-WinEvent path) serve different consumers and both are needed: ETW reaches SIEM collectors directly via real-time session subscription; eventlog reaches local IT admins through the familiar Application log channel. tracing-etw is a tracing-subscriber::Layer -- it drops into the existing init_tracing() chain. eventlog bridges via tracing_log::LogTracer (already a transitive dep). hmac 0.13.0 + the existing sha2 0.11 workspace dep provide per-session HMAC chain capability without adding a new crypto family.

**Core technologies:**
- winreg 0.56 -- safe HKLM policy reads -- eliminates unsafe RegQueryValueExW buffer management; resolves to workspace windows-sys 0.59
- tracing-etw 0.2.3 -- tracing::Layer for ETW/TraceLogging -- SIEM real-time bus (Splunk UF, Azure Monitor, WEF); no mc.exe manifest needed
- eventlog 0.4.0 -- log-compat Event Log emitter -- Event Viewer / Get-WinEvent for local IT triage; registered at MSI install via WiX util:EventSource
- hmac 0.13.0 + existing sha2 -- per-session HMAC chain -- tamper-evidence scoped to within-session; cross-session requires SEED-005
- windows-sys 0.59 feature Win32_System_Registry -- additive only -- enables RegNotifyChangeKeyValue for optional live-reload; no version bump
- WiX MSI (stay, no change) -- packaging -- MSIX hard-blocked by LocalSystem WFP service + kernel driver requirement

### Expected Features

The four researchers agree on a clean MVP / v3.x / v3.1+ split.

**Must have (table stakes for v3.0):**
- Silent MSI install (/qn /norestart, correct exit codes 0/3010, ALLUSERS=1, machine-wide PATH) -- SCCM/Intune deploy dies without this
- Machine-wide HKLM\SOFTWARE\Policies\nono reader with fail-secure parse -- the spine for all other features
- Deny-by-default egress: HKLM key presence activates ProxyFilter::new_strict + WFP; machine policy overrides per-user profile
- Single MachineEgressPolicy struct consumed by both nono-proxy and WFP service (no independent reads)
- AI-provider presets (*.anthropic.com, *.openai.com, api.github.com) as named built-in groups
- GPO ADMX template (nono.admx + nono.adml) -- the delivery mechanism for HKLM policy in GPO/Intune shops
- Structured security events via eventlog (Application log, named EventData fields, EventIDs 10001-10005) + tracing-etw (ETW SIEM path) -- both registered at MSI install via util:EventSource
- Event emission wired from three existing callsites: exec-strategy post-exit (nono_security::path_deny), nono-proxy/audit.rs::log_denied() (nono_security::network_deny), hooks.rs fail-closed path (nono_security::hook_fail_closed)
- Auto-provisioned user scratch space at first-run in user context (not from MSI SYSTEM context)
- Silent root-cert install via MSI CertificateRef for both LocalMachine\Root and CurrentUser\Root
- nono health JSON verdict command (WFP service state, machine policy state, scratch space state) -- required for SCCM compliance scripts

**Should have (v3.x after validation):**
- Syslog emission (RFC 5424) via telemetry/syslog.rs -- non-WEF shops; same SecurityEvent schema
- nono verify-egress subcommand -- fleet-scale gate confirms deny-by-default is in effect at both layers
- Intune OMA-URI / CSP mapping -- MDM shops that do not run GPMC
- Tamper-evident HMAC chain layered on event emission (hmac + sha2)
- MSI UpgradeCode + MajorUpgrade for clean v3.1 upgrade path

**Defer (v3.1+):**
- Correlation token (proxy deny <-> WFP deny <-> Event Log correlation) -- high wiring complexity; phase after telemetry is stable
- Policy-change live reload via RegNotifyChangeKeyValue -- snapshot-at-launch is the v3.0 model; live-reload needs ADR before implementation
- nono audit security-log verify <session-id> -- depends on tamper-evident chain shipping first
- SEED-005 ZT-Infra immutable ledger -- its own milestone; the local HMAC chain is explicitly out of scope for v3.0

### Architecture Approach

The architecture is three concentric layers sharing one data source. A new crates/nono-cli/src/config/machine.rs module reads HKLM\SOFTWARE\Policies\nono once at startup, produces a MachinePolicy struct, and fails secure on any parse error. This struct feeds two paths: (1) the egress control path, where MachinePolicy.egress_allowlist is injected into ProxyConfig construction in nono-cli and into the daemon CapabilitySet builder for WFP -- neither nono-proxy nor nono-wfp-service reads the registry independently; and (2) the telemetry path, where a new crates/nono-cli/src/telemetry/ module implements a SecurityEventLayer (tracing::Layer) that is registered in init_tracing() and routes nono_security::* tracing events to the Windows Event Log and ETW sinks. The library (crates/nono/) is untouched -- no policy, no telemetry, no registry reads. All additions land in nono-cli.

**Major components:**
1. config/machine.rs (NEW) -- MachinePolicy struct + winreg-backed HKLM reader; fail-secure on ReadError; NotConfigured falls through to per-user profile normally
2. telemetry/ module (NEW, 4 files) -- SecurityEventLayer (tracing::Layer), SecurityEvent schema (hashed paths, no raw PII), windows.rs emitter (cfg(windows)), syslog.rs emitter (cfg(unix))
3. scripts/build-windows-msi.ps1 (MODIFIED) -- add util:EventSource registration, HKLM\SOFTWARE\Policies\nono sentinel key, scratch-root ProgramData\nono\, machine-wide PATH Environment element, WiX CertificateRef
4. ProxyConfig construction site in nono-cli (MODIFIED) -- inject machine_policy.egress_allowlist so proxy reads from the same source as WFP
5. nono-agentd capability-set builder (MODIFIED) -- inject allowlist into per-agent NetworkCapability for WFP enforcement
6. Three existing emit callsites (MODIFIED) -- exec-strategy post-exit, nono-proxy/audit.rs, hooks.rs

### Critical Pitfalls

1. **Fail-OPEN on HKLM read failure** -- unwrap_or_default() or a fallback to per-user profile on registry error silently makes the machine appear policy-governed while running with no network restriction. Fix: model the result as PolicyReadResult::Enforced | NotConfigured | ReadError; only ReadError is fatal; add a dark-factory gate that injects a permission-denied key and asserts non-zero exit.

2. **Proxy and WFP allowlist drift from independent HKLM reads** -- wiring only proxy to HKLM and leaving WFP on its old per-user path ships false security: WFP continues to allow everything the user configured. Fix: deserialize HKLM once into MachineEgressPolicy; pass the same struct to both enforcement layers; the dark-factory gate must verify both layers reflect the HKLM list, not just one.

3. **Wildcard suffix matching via string ends_with** -- host.ends_with(.anthropic.com) matches anything.anthropic.com.evil.com. Fix: DNS component comparison (split on ., compare right-to-left); this is the exact pattern that produced WR-01 in Phase 56 (is_loopback_domain DNS-component fix) and must be applied to the machine-policy allowlist integration.

4. **SYSTEM-context MSI provisioning writes scratch space to SYSTEM %LOCALAPPDATA%** -- SCCM/Intune runs MSI as SYSTEM; %LOCALAPPDATA% resolves to C:\Windows\system32\config\systemprofile\...; every user R-B3 ownership guard fails. Fix: provision scratch space at first-run from the user own process; MSI creates only machine-global C:\ProgramData\nono\.

5. **Tamper-evident telemetry without external anchor** -- a local HMAC key stored in HKLM\SOFTWARE\nono\ can be deleted and re-signed by a local admin, defeating the claim. Fix: v3.0 tamper-evidence = Windows Event Forwarding to SIEM (external copy out of local attacker reach); local HMAC chain is a v3.x addition after SEED-005 ZT-Infra defines the key-storage model. Write the ADR first.

6. **Event Log custom channel silent-drop without manifest registration** -- RegisterEventSourceW returns NULL if wevtutil im was never run; the code continues silently and no events are emitted. Fix for v3.0: use the existing Application log source pattern (no manifest needed, already proven in nono-wfp-service.rs); defer custom structured manifest + channel to a future SIEM-schema hardening phase; treat RegisterEventSourceW returning NULL as NonoError::TelemetryUnavailable logged to stderr.

---

## Implications for Roadmap

Four researchers reached explicit consensus on a three-phase build order. The dependency is strict: machine-policy spine must be complete before either egress control or telemetry can be wired. Egress control and telemetry can be built in parallel once the spine is done.

### Phase 82: Silent MSI + Fleet Deployment Infrastructure

**Rationale:** Deployment is the physical prerequisite for everything else. Without a machine-wide MSI that runs correctly under SYSTEM, there is no HKLM\SOFTWARE\Policies\nono key to read, no Event Log source to write to, no service to start, and no scratch space to provision. This phase has the most invisible failure modes -- SYSTEM-context PATH/scratch pitfalls, root-cert TLS trust-path matrix -- and must be dark-factory verified before Phase 83 builds on top of it.

**Delivers:** Silent msiexec /qn /norestart (exit 0 / 3010); machine-wide PATH; HKLM\SOFTWARE\Policies\nono sentinel key; C:\ProgramData\nono\ machine-global root; util:EventSource registration for Application log; WiX CertificateRef for both cert stores; nono health JSON verdict command.

**Addresses:** SEED-001 table stakes (silent install, SYSTEM-context safety, non-interactive service start, correct exit codes, Intune Win32 detection rule, nono setup --provision-fleet verb).

**Avoids:** Pitfall 4 (SYSTEM-context scratch space), Pitfall 5 (non-atomic service install + health verdict), Pitfall 13 (root-cert TLS trust path matrix across PowerShell/Node/rustls).

**Dark-factory gate:** verify-dark.ps1 --gate DEPLOY-01 -- silent install under New-LocalUser SYSTEM context; workspace path owned by user not SYSTEM; nono health exits non-zero on degraded service; TLS through proxy verified from three client types.

**Research flag:** Standard WiX + MSI patterns, well-documented. Skip deep research-phase. Use existing build-windows-msi.ps1 as base.

---

### Phase 83: Machine Policy Spine + Egress Reconciliation

**Rationale:** The HKLM policy reader is the single shared dependency for egress control AND telemetry channel configuration. It must land as one atomic phase that wires BOTH nono-proxy and nono-wfp-service to the same MachineEgressPolicy struct. Splitting proxy and WFP across separate phases produces the allowlist-drift false-security state that is the highest-severity pitfall in the codebase.

**Delivers:** crates/nono-cli/src/config/machine.rs with MachinePolicy struct and winreg-backed reader; fail-secure PolicyReadResult enum; ProxyConfig construction site updated to inject egress_allowlist; nono-agentd capability-set builder updated for WFP; AI-provider built-in group (*.anthropic.com etc.) in network-policy.json; GPO ADMX template (nono.admx + nono.adml); nono diagnose-egress <hostname> subcommand.

**Addresses:** SEED-002 table stakes (machine-policy-managed allowlist, deny-by-default posture, wildcard subdomain syntax, machine policy precedence over per-user profile, WFP + proxy from same source, AI-provider presets).

**Avoids:** Pitfall 1 (fail-OPEN on HKLM read failure), Pitfall 2 (proxy/WFP drift), Pitfall 3 (wildcard suffix matching footgun), Pitfall 6 (env-var propagation lag), Pitfall 7 (WOW6432Node redirection -- KEY_WOW64_64KEY required), Pitfall 8 (default-deny blocks AI provider), Pitfall 9 (TOCTOU policy reload widening -- snapshot-at-launch model).

**Stack additions this phase:** winreg = 0.56 in [target.cfg(target_os = windows).dependencies] of nono-cli/Cargo.toml; Win32_System_Registry added to windows-sys feature list.

**Dark-factory gate:** verify-dark.ps1 --gate EGRESS-01 -- inject HKLM allowlist, launch agent, assert proxy rejects out-of-list domain AND WFP blocks unlisted SID; inject permission-denied key, assert non-zero exit; inject WOW6432Node key, assert NOT read; mid-session policy push, assert running session unaffected.

**Research flag:** Standard winreg + Group Policy ADMX patterns are well-documented; the KEY_WOW64_64KEY detail and the PolicyReadResult fail-secure enum design need careful implementation review but no additional research-phase.

---

### Phase 84: SIEM/EDR Telemetry (SEED-003)

**Rationale:** Telemetry is structurally independent from egress control once the machine policy spine is done (it reads MachinePolicy.telemetry_channel for the channel name). It can proceed in parallel with Phase 83 if the MachinePolicy struct API is agreed first, or sequentially after Phase 83 completes. The critical design decision -- Application log source (no manifest, proven in nono-wfp-service.rs) vs custom channel (manifest + wevtutil im in MSI) -- must be made in an ADR before implementation begins. The recommended v3.0 path is Application log source, deferring custom manifest to a future phase.

**Delivers:** crates/nono-cli/src/telemetry/ module (4 files: mod.rs, event.rs, windows.rs, syslog.rs); SecurityEventLayer registered in init_tracing(); SecurityEvent schema with hashed paths and named EventData fields (no raw PII); EventIDs 10001-10005; three emit callsites wired (nono_security::path_deny, nono_security::network_deny, nono_security::hook_fail_closed); ADR recording tamper-evidence scope (external SIEM forwarding, not local crypto chain).

**Addresses:** SEED-003 table stakes (structured events, named EventData fields, defined EventID space, WEF-compatible channel, events wired from DiagnosticFormatter deny path).

**Avoids:** Pitfall 10 (Event Log manifest not registered -- use Application source for v3.0), Pitfall 11 (full paths/PII in SIEM events -- SecurityEvent uses hashed path + category), Pitfall 12 (false tamper-evident claim -- ADR + external-SIEM-forwarding model explicitly documented).

**Stack additions this phase:** tracing-etw = 0.2 and eventlog = 0.4 in [target.cfg(target_os = windows).dependencies] of nono-cli/Cargo.toml; hmac = 0.13 in [dependencies] (cross-platform, for optional HMAC chain extension in v3.x).

**Dark-factory gate:** verify-dark.ps1 --gate TELEMETRY-01 -- clean-host MSI install (no prior wevtutil im); trigger sandbox denial; assert Event Log entry appears in Application log under nono source with correct EventID and named fields; assert NO raw file path in event body; assert ETW provider emits via logman.

**Research flag:** The tracing-etw TraceLogging API and eventlog log-bridge are both well-documented. The SecurityEvent schema design (what to hash, what to categorize, PII policy) warrants a short design review within the phase, not a separate research-phase.

---

### Phase Ordering Rationale

- **Phase 82 first** because the MSI provisions the HKLM sentinel key, registers the Event Log source, and establishes the ProgramData/nono/ machine-global root -- all physical prerequisites for Phases 83 and 84 to test against.
- **Phase 83 before Phase 84** because MachinePolicy.telemetry_channel is read in the SecurityEventLayer initialization, and the fail-secure contract tests in Phase 83 establish the registry-reader pattern that Phase 84 reuses.
- **Phase 83 and Phase 84 are candidates for parallel execution** by two engineers if the MachinePolicy struct API is locked at the start of Phase 83 and Phase 84 works against a stub implementation.
- **Never split proxy and WFP egress wiring across phases** (Pitfall 2): both must be wired in Phase 83 or the dark-factory gate must explicitly block Phase 83 from passing until both layers verify.
- **The Dark Factory pattern** (unattended verify-dark.ps1 --gate verdicts) must accompany each phase as a first-class deliverable, not an afterthought. Each theme has a distinct gate: DEPLOY-01, EGRESS-01, TELEMETRY-01.

### Research Flags

Phases needing deeper research during planning:
- **Phase 83 (HKLM spine + egress):** The KEY_WOW64_64KEY WOW6432Node behavior, the PolicyReadResult enum fail-secure contract, and the exact merge semantics between machine policy and per-user profile (especially the lock_egress flag design) warrant a sub-spike or design doc within the phase before implementation. The wildcard suffix matching audit of all filter.rs call sites needs a dedicated checklist.
- **Phase 84 (telemetry):** The SecurityEvent schema PII policy (what constitutes a path category, what the default TelemetryDetailLevel is) and the ADR on tamper-evidence scope must be written as the first deliverable of the phase before any wiring begins.

Phases with standard patterns (skip research-phase):
- **Phase 82 (MSI):** WiX Environment, CertificateRef, util:EventSource, and ServiceInstall patterns are all well-established. The in-tree build-windows-msi.ps1 and Phase 53/61 signed-MSI history provide sufficient context. No research-phase needed.

---

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | All four crates verified against docs.rs; version compatibility matrix confirmed via Cargo.toml dep-range inspection; MSIX anti-feature confirmed via Microsoft Learn official docs |
| Features | HIGH | Grounded in SEED-001/002/003 seeds + in-tree ProxyFilter, DiagnosticFormatter, AuditRecorder; enterprise deployment patterns from official Microsoft Learn and Intune docs |
| Architecture | HIGH | Every integration point mapped to current in-tree files (config/user.rs, policy.rs, nono-proxy/config.rs, nono-proxy/audit.rs, diagnostic.rs); no greenfield components |
| Pitfalls | HIGH | 13 pitfalls grounded in in-tree code (nono-wfp-service.rs, network_policy.rs, diagnostic.rs), project memory (v2.12 Phase 56 WR-01, Phase 60 R-B3), and Phase 53/61 signed-MSI history |

**Overall confidence:** HIGH

### Gaps to Address

- **winreg vs raw windows-sys for RegNotifyChangeKeyValue:** The live-reload call is not wrapped by winreg; the one raw windows-sys call alongside winreg is confirmed acceptable but the exact async integration pattern (tokio task + HANDLE wait) needs a small code sketch before Phase 83 implementation. This is a v3.x concern if live-reload is deferred.
- **Exact SecurityEvent PII policy:** Which path components are safe to include at TelemetryDetailLevel=1 (category) vs TelemetryDetailLevel=0 (hash-only) is not fully enumerated. This must be a Phase 84 ADR deliverable before the schema is wired.
- **Intune 32-bit MDM Extension WOW6432Node write path:** Confirmed as a risk (Pitfall 7) but the specific Intune Management Extension version bitness on Windows 10/11 was not directly verified. The KEY_WOW64_64KEY fix is correct regardless; the gap is whether the Intune CSP deployment scripts need RegistryView::Registry64 explicitly. Flag for the Phase 83 ADMX/OMA-URI deliverable.
- **rustls_native_certs configuration in nono-proxy:** Pitfall 13 identifies that rustls may be initialized with webpki roots rather than the Windows cert store. The exact initialization code path in nono-proxy was not inspected to confirm; this must be a Phase 82 acceptance criterion.

---

## Sources

### Primary (HIGH confidence)

- In-tree crates/nono-cli/Cargo.toml line 149 -- existing windows-sys 0.59 feature set; Win32_System_Registry absence confirmed
- In-tree scripts/build-windows-msi.ps1 -- WiX MSI generation, machine-scope UpgradeCode, ServiceInstall, Vital=no
- In-tree crates/nono-cli/src/platform.rs -- existing reg.exe subprocess pattern (gap winreg fills)
- In-tree crates/nono-cli/src/bin/nono-wfp-service.rs -- Application Event Log EVENT_LOG_SOURCE, RegisterEventSourceW/ReportEventW pattern
- In-tree crates/nono-cli/src/network_policy.rs -- ResolvedNetworkPolicy, build_proxy_config, WR-01 DNS-component fix
- In-tree crates/nono/src/diagnostic.rs -- DiagnosticFormatter, DenialRecord (library boundary; re-use for SIEM = Pitfall 11)
- In-tree crates/nono-proxy/src/config.rs, filter.rs, audit.rs -- ProxyConfig, ProxyFilter::new_strict, log_denied()
- docs.rs/winreg/latest -- v0.56.0; windows-sys >= 0.59, <= 0.61 dep range
- docs.rs/tracing-etw/latest -- v0.2.3; MSRV 1.80; TraceLogging self-describing events
- docs.rs/eventlog/latest -- v0.4.0; fixed EventIDs 1-5; 120-byte embedded DLL
- docs.rs/hmac/0.13.0 -- compatible with digest 0.10 / sha2 0.11
- learn.microsoft.com -- MSIX services limitations -- MSIX cannot package LocalSystem services or kernel drivers
- learn.microsoft.com -- Intune Win32 app deployment
- learn.microsoft.com -- ADMX-backed policies in Intune
- learn.microsoft.com -- Writing manifest-based ETW events
- Project memory: windows_mandatory_label_write_owner, windows_msi_wxs_is_generated, windows_appcontainer_wfp_validated, windows_wfp_enforcement_is_service_only
- Phase history: 53/61 (signed MSI), 56 (allow_domain + WR-01), 60 (R-B3 ownership guard), v2.11 (WFP service non-fatal start), v2.13 (Dark Factory verify-dark.ps1 gate pattern)
- SEED-001, SEED-002, SEED-003 (.planning/seeds/)
- CLAUDE.md -- library-vs-CLI boundary, Configuration load failures must be fatal, path-component-comparison footgun

### Secondary (MEDIUM confidence)

- Splunk Lantern -- Windows event log for Enterprise Security -- WEF channel forwarding defaults
- Microsoft TechCommunity -- Windows Events in Sentinel -- Application channel forwarding behavior
- learn.microsoft.com -- ADMX Intune OMA-URI -- CSP path for ADMX ingestion
- GitHub anthropics/claude-code issue #51400 -- wildcard FQDN matching as industry expectation
- turbo.net -- MSIX limitations 2025 -- MSIX service limitations corroborating Microsoft Learn

---
*Research completed: 2026-06-18*
*Ready for roadmap: yes*
