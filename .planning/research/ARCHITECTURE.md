# Architecture Research

**Domain:** Enterprise Hardening integration into an existing Rust/Windows 5-crate workspace — HKLM machine policy spine, unified egress, and structured security-event telemetry (nono v3.0)
**Researched:** 2026-06-18
**Confidence:** HIGH — every integration point grounded in current in-tree source code (config/user.rs, policy.rs, nono-proxy/config.rs, nono-proxy/audit.rs, diagnostic.rs); no greenfield components.

---

## Core Constraint

The three themes (Deployment, Control, Compliance) are NOT independent silos. They share one data source: `HKLM\SOFTWARE\Policies\nono`. The machine policy layer is the shared prerequisite for the other two. Build it first or the egress and telemetry features have nowhere to read from.

---

## System Overview

```
  ┌──────────────────── POLICY SPINE ─────────────────────────┐
  │  HKLM\SOFTWARE\Policies\nono  (pushable via GPO / Intune) │
  │  new: crates/nono-cli/src/config/machine.rs               │
  │                                                            │
  │  egress_allowlist: ["*.anthropic.com", "corp.internal"]   │
  │  telemetry_channel: "Application"                          │
  │  scratch_root: "C:\\ProgramData\\nono\\workspaces"         │
  └────────────────────────────┬───────────────────────────────┘
                               │ read at startup (fail-secure)
           ┌────────────────────┼─────────────────────────────┐
           │                   │                              │
           ▼                   ▼                              ▼
  ┌─────────────────┐  ┌─────────────────────┐  ┌────────────────────┐
  │  Deployment     │  │  Egress Control      │  │  Telemetry         │
  │  (SEED-001)     │  │  (SEED-002)          │  │  (SEED-003)        │
  │                 │  │                      │  │                    │
  │  MSI silent     │  │  nono-proxy:         │  │  new: crates/      │
  │  install flags  │  │  ProxyConfig gets    │  │  nono-cli/src/     │
  │  Scratch-space  │  │  allowed_hosts from  │  │  telemetry/        │
  │  provisioner    │  │  machine policy      │  │  mod.rs            │
  │  GPO ADMX       │  │                      │  │                    │
  │  Intune CSP     │  │  nono-wfp-service:   │  │  hooks into        │
  └─────────────────┘  │  WFP per-SID filter  │  │  DiagnosticFormatter
                       │  reads same list     │  │  tracing Layer     │
                       └─────────────────────┘  │  → Windows Event Log
                                                 │  → Syslog          │
                                                 └────────────────────┘
```

---

## Feature 1: Machine Policy Layer

### Where It Lives

New file: `crates/nono-cli/src/config/machine.rs`

Added to `config/mod.rs` as `pub mod machine;`.

This is the only new Rust module in the crate. Everything else is modified existing files.

### What It Does

Reads `HKLM\SOFTWARE\Policies\nono` on Windows via `windows-sys` (already a workspace dep). On non-Windows it returns `Ok(None)` so the codebase compiles cross-platform. Fails secure: a registry read error is fatal (abort), not a fallback to permissive defaults.

```rust
// crates/nono-cli/src/config/machine.rs — sketch, not final code
#[derive(Debug, Default)]
pub struct MachinePolicy {
    pub egress_allowlist: Option<Vec<String>>,   // REG_MULTI_SZ
    pub telemetry_channel: Option<String>,        // REG_SZ
    pub scratch_root: Option<PathBuf>,            // REG_SZ
    pub strict_egress: Option<bool>,              // REG_DWORD 0/1
}

pub fn load_machine_policy() -> Result<Option<MachinePolicy>>
```

### Precedence / Merge Rule

The merge order is: machine policy > per-user profile > built-in defaults.

The existing precedence stack at startup is:
1. CLI flags (highest)
2. Per-user profile (`~/.config/nono/config.toml` loaded via `config/user.rs`)
3. Embedded `policy.json` defaults (via `policy.rs` group resolver)

Machine policy inserts between CLI flags and the per-user profile. The user can still override with explicit CLI flags (principle: machine policy sets the floor, not a ceiling on per-run use). For security-relevant settings like `egress_allowlist`, machine policy MAY be configured to reject user relaxations (a `lock_egress: true` flag in the registry).

The merge is performed in `crates/nono-cli/src/main.rs` (or its `app_runtime` dispatcher) immediately after argument parsing, before profile loading:

```
1. parse CLI args
2. load_machine_policy()     <-- NEW, fail-secure
3. load_user_config()        <-- existing
4. load_embedded_policy()    <-- existing
5. resolve capabilities      <-- existing policy.rs
```

### Fail-Secure Invariants

- If the registry key exists but a value is malformed, abort with `NonoError::ConfigParse`. Never silently use empty/permissive defaults for security values.
- If the registry key does not exist at all, return `Ok(None)` — machine is not centrally managed, fall through to per-user config normally.
- Configuration load failures must be fatal (CLAUDE.md § Permission Scope: "Configuration load failures must be fatal").

### New vs Modified: Feature 1

| Component | Status | Change |
|-----------|--------|--------|
| `crates/nono-cli/src/config/machine.rs` | NEW | Registry reader, `MachinePolicy` struct |
| `crates/nono-cli/src/config/mod.rs` | MODIFIED | `pub mod machine;` + re-export |
| `crates/nono-cli/src/main.rs` (or `app_runtime.rs`) | MODIFIED | Call `load_machine_policy()` at startup |
| `crates/nono-cli/Cargo.toml` | MODIFIED | No new deps — `windows-sys` already present |
| `scripts/build-windows-msi.ps1` | MODIFIED | Add silent-install flags (`/quiet /norestart`) + scratch-space provisioner custom action |
| `crates/nono-cli/data/policy.json` | NOT MODIFIED | Machine policy is a separate layer, not a policy group |

---

## Feature 2: Unified Egress Control

### The Two Enforcers

nono currently has two orthogonal egress enforcement mechanisms:

| Enforcer | Layer | Mechanism | What It Controls |
|----------|-------|-----------|-----------------|
| `nono-proxy` | User-space L7 | HTTP CONNECT intercept + domain filter | Domain + method + path; handles credential injection; runs only when proxy is active |
| `nono-wfp-service` | Kernel (WFP) | `FwpmFilterAdd0` per-SID filter | Raw TCP/UDP by AppContainer SID; enforces for ALL processes with that SID regardless of proxy |

These are NOT duplicates. They are defense-in-depth layers with different granularities. The enterprise story is: WFP provides the hard kernel-enforced boundary (agent cannot bypass even if it ignores the proxy), while nono-proxy provides L7 method/path filtering and credential injection on top.

### The Problem With Duplicating the Allowlist

Today `nono-proxy`'s `ProxyConfig.allowed_hosts` is populated from the per-user profile JSON. The WFP service has its own separate allowlist fed by the daemon's `capability_set`. If machine policy adds a third copy, the system has three allowlists to keep in sync, with no single source of truth.

### Solution: One Allowlist, Two Consumers

Machine policy owns the allowlist (`HKLM\...\EgressAllowlist` REG_MULTI_SZ). Both enforcers read from it at startup through the machine policy layer. Neither gets its own copy of the list.

```
HKLM\...\EgressAllowlist
    │
    ▼
MachinePolicy.egress_allowlist: Vec<String>
    │                           │
    ▼                           ▼
ProxyConfig.allowed_hosts   WFP allowlist passed to
(via nono-proxy filter.rs)  nono-wfp-service at daemon launch
```

### Integration Point: nono-proxy

`ProxyConfig` is built in `nono-cli` (not inside `nono-proxy` itself — the proxy is a library whose config is constructed by the caller). The call site that constructs `ProxyConfig` is in `nono-cli`'s execution strategy / supervisor setup code.

**Modified file:** wherever `ProxyConfig { allowed_hosts: ..., strict_filter: ..., }` is constructed in `crates/nono-cli/src/`. This is a read-at-construction-time injection, not a runtime reload. The machine policy list replaces or augments `allowed_hosts`.

If `MachinePolicy.strict_egress == true`, set `ProxyConfig.strict_filter = true` so an empty list is deny-all rather than allow-all (the `ProxyConfig.strict_filter` field already exists in `nono-proxy/src/config.rs`).

**No changes to `nono-proxy/src/`** — the proxy already has the data model needed. The caller just supplies different values.

### Integration Point: nono-wfp-service

The WFP service receives its per-agent rules from the daemon via the control pipe. The daemon's `launch_agent` path builds the `CapabilitySet` (which encodes the network capability). Machine policy allowlist entries translate into `NetworkCapability` entries in the capability set before the daemon passes them to the WFP service.

**Modified file:** the daemon's capability-set builder (in `nono-agentd`), where it constructs the network capability for a new agent. The machine policy allowlist is merged in here before the `CapabilitySet` is sent to the WFP service.

### Reconciliation Summary

There is no new "enterprise egress story" that replaces the existing ones. The architecture is:

1. Machine policy is the allowlist source.
2. `nono-cli` reads machine policy at startup and injects it into `ProxyConfig`.
3. `nono-agentd` reads machine policy at daemon startup and injects it into the `CapabilitySet` it builds for each agent.
4. WFP enforces the kernel boundary; proxy enforces the L7 boundary. Both use the same domain list from the same source.

### New vs Modified: Feature 2

| Component | Status | Change |
|-----------|--------|--------|
| `crates/nono-proxy/src/config.rs` | NOT MODIFIED | `ProxyConfig` already has `allowed_hosts` + `strict_filter` |
| `crates/nono-proxy/src/filter.rs` | NOT MODIFIED | `ProxyFilter` already accepts `allowed_hosts` slice |
| `crates/nono-cli/src/` (ProxyConfig construction site) | MODIFIED | Inject `machine_policy.egress_allowlist` into `ProxyConfig` at construction |
| `nono-agentd` (capability set builder) | MODIFIED | Inject machine policy allowlist into per-agent `CapabilitySet` for WFP |
| `crates/nono-cli/src/config/machine.rs` | NEW (Feature 1 deliverable) | Provides `MachinePolicy.egress_allowlist` |

---

## Feature 3: Structured Security-Event Telemetry

### Event Sources (What to Capture)

Four deny/violation event types need to become structured security signals:

1. **Path deny** — when `DiagnosticFormatter` records a `DenialRecord` (path + access mode + reason)
2. **Network deny** — when `nono-proxy/audit.rs::log_denied()` fires (already emits `tracing::info!` with structured fields)
3. **Label violation** — when the Windows mandatory-label (Low-IL) enforcement blocks an operation (currently goes to stderr as an error exit)
4. **Hook fail-closed** — when `hooks.rs` / session hooks trigger a fail-closed response (Phase 58 hooks path)

### Where to Hook

The cleanest integration point is a `tracing::Layer` that listens on a specific target prefix (`nono_security::*`) and forwards structured events to the OS sink. This approach:
- Requires no changes to the library (`crates/nono/`) — stays within CLI
- Reuses the existing `tracing` initialization in `crates/nono-cli/src/cli_bootstrap.rs` (`init_tracing()`)
- Lets existing callsites emit `tracing::warn!(target: "nono_security::path_deny", ...)` without importing a new crate
- Is testable in isolation (replace the subscriber in tests)

The alternative of wrapping `DiagnosticFormatter` directly is worse: `DiagnosticFormatter` is in the library, which must stay policy-free. Adding event emission to it would violate the library-vs-CLI boundary.

### New Module: `crates/nono-cli/src/telemetry/`

```
crates/nono-cli/src/telemetry/
├── mod.rs          # SecurityEventLayer (tracing::Layer impl), SecurityEvent enum
├── event.rs        # SecurityEvent schema: event_type, severity, session_id,
│                   # agent_pid, path/host/method, timestamp_unix_ms, nonce
├── windows.rs      # Windows Event Log emitter (cfg(target_os = "windows"))
│                   # uses windows-sys ReportEventW (evntprov.h alternative)
└── syslog.rs       # Syslog emitter (cfg(unix)) via RFC 5424 UDP/TCP
                    # (can use the `syslog` crate or roll minimal UDP)
```

This is a new module in `nono-cli`, not a new crate. The CLAUDE.md boundary rule is clear: policy, UX, and output live in the CLI. Telemetry emission is output, and it depends on machine policy config (channel names), so it belongs in `nono-cli`.

A separate crate would be premature — the emitter has no consumers other than `nono-cli` and `nono-agentd`, and the tracing Layer approach lets both binaries register it at their respective init paths.

### How Existing Emit Points Are Wired

**Path deny — DiagnosticFormatter path:**
`DiagnosticFormatter::format_footer()` is called in `nono-cli` after child exit. The CLI already has the `DenialRecord` slice at that point. The telemetry wiring goes in the CLI caller (the exec strategy's post-exit handler), not inside `DiagnosticFormatter` itself:

```
// In nono-cli exec strategy post-exit handler (MODIFIED):
for denial in &denials {
    tracing::warn!(
        target: "nono_security::path_deny",
        path = %denial.path.display(),
        access = %denial.access,
        reason = ?denial.reason,
        session_id = session_id,
    );
}
```

The `SecurityEventLayer` picks this up and routes it to Event Log / Syslog.

**Network deny — nono-proxy audit.rs:**
`audit::log_denied()` already emits `tracing::info!(target: "nono_proxy::audit", decision = "deny", ...)`. Two options:
- Add a second `tracing::warn!(target: "nono_security::network_deny", ...)` call in `log_denied()` — clean, but adds a second emit.
- Have `SecurityEventLayer` filter on `nono_proxy::audit` events where `decision == "deny"` — avoids touching proxy source.

Preferred: option 1 (explicit dual-emit in `log_denied`), because it keeps the security event schema independent of the proxy audit schema and avoids stringly-typed field scraping.

**Hook fail-closed:**
`crates/nono-cli/src/hooks.rs` — add a `tracing::warn!(target: "nono_security::hook_fail_closed", ...)` call at the fail-closed code path. This is the most targeted change.

### Windows Event Log Specifics

Custom Application log channel: `nono-security` (or `nono/Security`). Registered via registry key at MSI install time (part of the deployment work in Feature 1). Uses `windows-sys::Win32::System::EventLog::ReportEventW` or the modern ETW path via `evntprov`. The `DWORD` event IDs map to event types (1000=path_deny, 1001=network_deny, 1002=label_violation, 1003=hook_fail_closed).

The MSI custom action (or a `nono setup --register-event-source` sub-command) registers the source. Failing to register is non-fatal for the sandbox; telemetry emission silently no-ops if the source is not registered (this is the one place where a silent fallback is acceptable — losing a telemetry event is not a security failure).

### Tamper Evidence

The SEED-003 "tamper-proof" claim maps to two lightweight mechanisms (SEED-005 ZT ledger is out of scope):
1. Append-only channel: Windows Event Log Application channel is append-only by OS design; events cannot be deleted by non-admin processes.
2. Per-event sequence counter: include an incrementing `sequence_id` in each event. Gaps in the sequence visible to the SIEM indicate dropped events.

A cryptographic event chain (hash-chained events) is SEED-005 territory — defer it.

### New vs Modified: Feature 3

| Component | Status | Change |
|-----------|--------|--------|
| `crates/nono-cli/src/telemetry/mod.rs` | NEW | `SecurityEventLayer`, `SecurityEvent` schema |
| `crates/nono-cli/src/telemetry/event.rs` | NEW | Event enum + schema types |
| `crates/nono-cli/src/telemetry/windows.rs` | NEW | `cfg(windows)` Event Log emitter |
| `crates/nono-cli/src/telemetry/syslog.rs` | NEW | `cfg(unix)` Syslog emitter |
| `crates/nono-cli/src/cli_bootstrap.rs` | MODIFIED | Register `SecurityEventLayer` with the `tracing` subscriber at `init_tracing()` |
| `crates/nono-cli/src/` (exec strategy post-exit) | MODIFIED | Emit `nono_security::path_deny` events per denial |
| `crates/nono-proxy/src/audit.rs` | MODIFIED | Add dual-emit `nono_security::network_deny` in `log_denied()` |
| `crates/nono-cli/src/hooks.rs` | MODIFIED | Emit `nono_security::hook_fail_closed` at fail-closed path |
| `crates/nono/src/diagnostic.rs` | NOT MODIFIED | Library stays policy-free; emit happens in CLI caller |
| `scripts/build-windows-msi.ps1` | MODIFIED | Register Event Log source at install time |

---

## Data Flow: Policy to Enforcement

```
Startup:
  load_machine_policy()
       │
       ├─► egress_allowlist ─────────────────────────┐
       │                                              │
       │   ProxyConfig construction (nono-cli)        │    CapabilitySet construction (nono-agentd)
       │        │                                     │              │
       │        ▼                                     │              ▼
       │   ProxyFilter::new_strict(allowlist)         │    NetworkCapability { allowed_domains }
       │        │                                     │              │
       │        ▼                                     │              ▼
       │   nono-proxy: filter each CONNECT req        │    nono-wfp-service: FwpmFilterAdd0 per-SID
       │
       ├─► telemetry_channel ──► SecurityEventLayer init ──► Event Log / Syslog
       │
       └─► scratch_root ──► scratch-space provisioner (setup.rs / MSI custom action)

Runtime deny events:
  exec_strategy post-exit handler
       │
       ├─ DenialRecord[] ──► tracing::warn!(target: "nono_security::path_deny", ...)
       │                          │
       │                     SecurityEventLayer ──► Event Log (1000)
       │
  nono-proxy audit::log_denied()
       │
       ├─ deny event ──► tracing::warn!(target: "nono_security::network_deny", ...)
       │                      │
       │                 SecurityEventLayer ──► Event Log (1001)
       │
  hooks.rs fail-closed path
       │
       └─ hook failure ──► tracing::warn!(target: "nono_security::hook_fail_closed", ...)
                                │
                           SecurityEventLayer ──► Event Log (1003)
```

---

## Component Boundaries

| Component | Owns | Does NOT own |
|-----------|------|-------------|
| `crates/nono/` (library) | Sandbox primitives, `CapabilitySet`, `DiagnosticFormatter`, `DenialRecord` types | Policy, telemetry emission, machine registry reads |
| `crates/nono-cli/src/config/machine.rs` | HKLM read, `MachinePolicy` struct, fail-secure parse | Egress enforcement, event emission |
| `crates/nono-cli/src/telemetry/` | `SecurityEventLayer`, schema, OS emitters | Policy loading, sandbox enforcement |
| `crates/nono-proxy/` | Domain filter, credential injection, `ProxyConfig` data model | Building `ProxyConfig` from policy (CLI does that) |
| `nono-agentd` | Per-agent capability set construction, WFP handoff | Machine policy parsing (reads via machine.rs API) |

The library-vs-CLI boundary from CLAUDE.md is preserved throughout: the library gets new types at most (if `DenialRecord` needs a new field), never policy logic or platform-specific emission.

---

## Build Order (Dependency-Respecting)

The machine policy layer is the shared prerequisite. Neither egress injection nor telemetry configuration can proceed without it.

**Phase A — Policy Spine (blocks everything else)**
1. `crates/nono-cli/src/config/machine.rs` — `MachinePolicy` struct + HKLM reader
2. Wire `load_machine_policy()` into startup in `main.rs`
3. `scripts/build-windows-msi.ps1` — silent install flags + scratch provisioner custom action + Event Log source registration

**Phase B — Egress Control (requires A)**
4. Inject `MachinePolicy.egress_allowlist` into `ProxyConfig` at construction site in nono-cli
5. Inject allowlist into `nono-agentd` capability-set builder for WFP
6. Dark-factory gate: `verify-dark.ps1` gate confirms proxy rejects out-of-allowlist domain + WFP blocks unlisted SID

**Phase C — Telemetry (requires A; independent from B)**
7. `crates/nono-cli/src/telemetry/` module (schema, Layer, Windows emitter, Syslog emitter)
8. Register `SecurityEventLayer` in `init_tracing()` using `MachinePolicy.telemetry_channel`
9. Emit `nono_security::*` events at the three callsites (exec-strategy, audit.rs, hooks.rs)
10. Dark-factory gate: `verify-dark.ps1` gate confirms Event Log entry appears after a sandbox denial

B and C can proceed in parallel once A is complete.

---

## Anti-Patterns to Avoid

### Anti-Pattern: Three Separate Allowlists

**What people do:** add a machine-policy allowlist field to `MachinePolicy`, add another field to `ProxyConfig`, and pass a third copy to the WFP service — each sourced independently from the registry.

**Why it's wrong:** when IT updates the GPO, only one of the three lists updates. The other two retain stale entries. The security boundary becomes whatever the strictest stale list happens to be, which is not the policy the IT admin intended.

**Do this instead:** `MachinePolicy.egress_allowlist` is the single source. `ProxyConfig.allowed_hosts` is constructed from it. The WFP capability set is constructed from it. No copy-paste.

### Anti-Pattern: Emit Telemetry From the Library

**What people do:** add a `telemetry_emitter: Option<Box<dyn Fn(SecurityEvent)>>` callback to `DiagnosticFormatter` so denials can be emitted inline.

**Why it's wrong:** the library is a pure sandbox primitive with no security policy (CLAUDE.md design decision #4). Adding an emission callback leaks policy concerns into the library, makes the library non-embeddable without a telemetry context, and breaks the clean fuzz/test surface.

**Do this instead:** the CLI caller already has the `DenialRecord` slice after child exit. Emit from there. The `DenialRecord` type is already a clean data struct with no behavior.

### Anti-Pattern: New Crate for Telemetry

**What people do:** create `crates/nono-telemetry/` as a new workspace member to share telemetry logic between `nono-cli` and `nono-agentd`.

**Why it's wrong:** the only consumers are `nono-cli` and `nono-agentd`. Both already depend on `nono-cli` indirectly via the shared type ecosystem. A new crate adds a Cargo.toml, CI build target, and version pin for marginal benefit. The `tracing::Layer` approach achieves the sharing goal — both binaries register the same layer type at their respective `init_tracing()` calls.

**Do this instead:** put the `SecurityEventLayer` and schema in `nono-cli/src/telemetry/` and re-export it for `nono-agentd` to use via a path dep if needed. Promote to a crate only if a third consumer appears.

### Anti-Pattern: Silent Fallback on Machine Policy Parse Error

**What people do:** `machine_policy.unwrap_or_default()` so a corrupted registry value silently returns empty/permissive policy.

**Why it's wrong:** an empty `egress_allowlist` with `strict_filter = false` means `ProxyFilter::allow_all()` — the opposite of the intended deny-by-default posture.

**Do this instead:** a malformed registry value is a fatal error. The operator must fix the GPO. CLAUDE.md § Permission Scope: "Configuration load failures must be fatal."

---

## Scaling Considerations

This is a single-machine tool deployed fleet-wide. The scaling concern is the number of machines the GPO applies to, not concurrent users on one machine.

| Scale | Architecture Adjustment |
|-------|------------------------|
| 1–100 machines | HKLM registry via GPO is sufficient; no central server needed |
| 100–10K machines | Add ADMX template for Group Policy; Intune CSP for cloud-managed fleets; no architectural change |
| 10K+ machines | SEED-005 ZT-Infra signed policy overrides (separate milestone); the registry spine remains but gets supplemented by a policy distribution service |

The Windows Event Log → Windows Event Forwarding (WEF) → SIEM pipeline handles scale on the telemetry side without any code changes — that is the OS-native fan-out.

---

## Sources

- Current in-tree: `crates/nono-cli/src/config/user.rs` — `UserConfig` struct (precedence model)
- Current in-tree: `crates/nono-cli/src/config/mod.rs` — startup config load sequence
- Current in-tree: `crates/nono-cli/src/policy.rs` — `Policy`, `Group` (embedded policy.json schema)
- Current in-tree: `crates/nono-proxy/src/config.rs` — `ProxyConfig`, `strict_filter`, `allowed_hosts`
- Current in-tree: `crates/nono-proxy/src/filter.rs` — `ProxyFilter::new_strict()`
- Current in-tree: `crates/nono-proxy/src/audit.rs` — `log_denied()`, `NetworkAuditEvent`, tracing targets
- Current in-tree: `crates/nono/src/diagnostic.rs` — `DiagnosticFormatter`, `DenialRecord` (library boundary)
- CLAUDE.md § Library vs CLI Boundary, § Key Design Decisions #4
- SEED-001, SEED-002, SEED-003 (`.planning/seeds/`)
- PROJECT.md `## Current Milestone: v3.0` (egress reconciliation, policy spine decisions)

---
*Architecture research for: nono v3.0 Enterprise Hardening I (Deploy · Control · Compliance)*
*Researched: 2026-06-18*
