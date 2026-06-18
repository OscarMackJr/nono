# Stack Research

**Domain:** Enterprise hardening for a shipping Rust/Windows security product — silent fleet deployment, machine-policy-driven egress control, SIEM/EDR-forwardable security telemetry (nono v3.0)
**Researched:** 2026-06-18
**Confidence:** HIGH — primary findings verified against in-tree Cargo.toml, docs.rs, and crates.io; external tooling (ADMX, Intune) from official Microsoft Learn docs

---

## Executive Framing

This is a *subsequent milestone on an existing shipping product*. The dominant finding across all three themes is the same as v2.12: **almost nothing new needs to be added to the stack**. The critical question for each theme is whether `windows-sys` 0.59 already covers the Win32 surface, and whether the existing `tracing`/`tracing-subscriber` stack can be extended rather than replaced.

**Per-theme verdict:**

| Theme | New crates needed | `windows-sys` covers it? | `tracing` stack extensible? |
|-------|------------------|--------------------------|------------------------------|
| Silent MSI + GPO/SCCM/Intune | None (WiX already ships; `IntuneWinAppUtil` is external tooling, not a Rust dep) | N/A | N/A |
| Machine policy from `HKLM\SOFTWARE\Policies\nono` | `winreg` **0.56** (single new dep) OR zero new crates (use existing `windows-sys` raw calls) | YES — `Win32_System_Registry` feature needed (one line in Cargo.toml) | N/A |
| Windows Event Log + ETW + tamper-evident chain | `tracing-etw` **0.2.3** (ETW/SIEM path) + `eventlog` **0.4.0** (Event Log/`log`-compat path) — pick ONE; plus `hmac` **0.13.0** for the hash chain (reuses existing `sha2` workspace dep) | YES — `Win32_System_EventLog` and `Win32_System_Diagnostics_Etw` ALREADY in the feature list (line 149 `nono-cli/Cargo.toml`) | YES — `tracing-etw` is a `tracing-subscriber::Layer`; drop-in addition |

---

## Recommended Stack

### Theme 1 — Silent / Unattended MSI Install + GPO/SCCM/Intune Fleet Distribution

#### Recommendation: Stay on WiX MSI. Do NOT adopt MSIX this milestone.

**Why WiX MSI wins over MSIX for nono v3.0:**

MSIX explicitly does not support kernel-mode drivers or Windows services that need to run under `LocalSystem` or `NetworkService` — it only supports `session-0` services under the defined system accounts, and even those require Windows 10 2004+. nono ships `nono-wfp-service` (runs under `LocalSystem` via SCM, `Account="LocalSystem"` in the `.wxs`) and `nono-wfp-driver.sys` (kernel-mode driver, even if currently a placeholder). MSIX cannot package either. Additionally, MSIX has no equivalent of WiX's `MajorUpgrade` + `ServiceControl`/`ServiceInstall` sequence, its machine-scope story in tools like `winget` is still broken (scope=machine preference not respected without explicit `--scope machine`), and migrating the existing signed-MSI CI pipeline (`release.yml`) to MSIX would be a multi-week effort with no functional gain for the target audience (corporate IT via SCCM/Intune Win32 app deployment).

**What the WiX MSI already has (no code changes needed):**
- Machine-scope (`perMachine`, `ProgramFiles64Folder`, `ALLUSERS=1` implicitly set by `PackageScope="perMachine"`) — silent install via `msiexec /i nono-*.msi /quiet /norestart` is standard MSI behavior.
- `MajorUpgrade` element (standard WiX pattern) handles upgrades.
- `ServiceInstall` + `ServiceControl` with `Vital="no"` (Phase 67 fix — non-fatal service failure) already prevents a WFP-service startup hiccup from rolling back the install.

**Net-new WiX work for v3.0 (not a Rust dep — authoring in `build-windows-msi.ps1`):**

1. **`util:EventSource` registration** — register the custom `nono` Windows Event Log source under `HKLM\SYSTEM\CurrentControlSet\Services\EventLog\Application\nono` at install time. WiX Util extension `<util:EventSource Name="nono" Log="Application" ... />` inside a `<Component>`. This must run elevated (it does — machine MSI runs elevated). No separate `mc.exe`/manifest needed for the `RegisterEventSource` / `ReportEvent` API path; the `eventlog` crate embeds the minimal 120-byte message DLL in the binary.

2. **`HKLM\SOFTWARE\Policies\nono` key registration** — a no-op sentinel `<RegistryKey Root="HKLM" Key="SOFTWARE\Policies\nono" />` so GPO/Intune can write policy values into a key that the installer pre-creates (avoids timing issues where the service reads before the key exists). WiX `<RegistryKey>` element, no code change.

3. **Scratch-space provisioner** — `CustomAction` or `Directory`/`CreateFolder` element to create `%LOCALAPPDATA%\nono\workspaces\` with the correct ACL at install time. Alternatively, implement in nono-cli itself (preferred — avoids CustomAction elevation complexity). See ARCHITECTURE.md.

**Intune Win32 app deployment (external tooling — no Rust dep):**

Intune cannot directly use `.msi` files; it requires wrapping into `.intunewin` format via the `IntuneWinAppUtil.exe` (Microsoft Win32 Content Prep Tool, available from `github.com/microsoft/microsoft-win32-content-prep-tool`). The workflow is:
```
IntuneWinAppUtil.exe -c <folder_with_msi> -s nono-*.msi -o <output>
```
Install command: `msiexec /i nono-*.msi /quiet /norestart /l*v %TEMP%\nono-install.log`
Uninstall command: `msiexec /x {ProductCode} /quiet /norestart`
Detection rule: registry key `HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\{ProductCode}` exists.

This is IT-operator documentation, not code. No Rust dependency change.

**ADMX / GPO policy delivery:**

Hand-author a `nono.admx` + `nono.adml` XML pair (no separate tooling needed beyond a text editor; Visual Studio's ADMX editor is optional polish). The `.admx` maps each `HKLM\SOFTWARE\Policies\nono` value to a human-readable policy setting. Place in `%SystemRoot%\PolicyDefinitions\` on each endpoint (GPO push) or ingest via Intune's `./Device/Vendor/MSFT/Policy/ConfigOperations/ADMXInstall/nono/Policy/nono` OMA-URI. The registry values written by Group Policy / Intune CSP land under `HKLM\SOFTWARE\Policies\nono` — the same path the Rust policy reader consumes. No toolchain required beyond authoring the XML; the ADMX schema is documented at `learn.microsoft.com/en-us/windows/client-management/understanding-admx-backed-policies`.

---

### Theme 2 — Machine Policy from `HKLM\SOFTWARE\Policies\nono`

#### Recommendation: Use `winreg` 0.56. Do NOT add raw `windows-sys` registry calls.

**Why `winreg` over raw `windows-sys` FFI:**

`windows-sys` 0.59 already exposes `RegOpenKeyExW`, `RegQueryValueExW`, and the full `Win32_System_Registry` surface — but enabling it requires adding `"Win32_System_Registry"` to the features list (currently absent from line 149 of `nono-cli/Cargo.toml`). That alone would work, but the raw FFI is verbose and error-prone: `RegQueryValueExW` is a two-call pattern (first call to get size, second to get data), buffer management is manual, and the type dispatch (`REG_DWORD` → `u32`, `REG_SZ` → `UTF-16LE` null-terminated string, `REG_MULTI_SZ` → double-null-terminated array) requires hand-written unsafe. This is exactly the abstraction `winreg` provides.

`winreg` 0.56 is the current stable release (confirmed via docs.rs). It depends on `windows-sys` (version range `>=0.59, <=0.61`) — this is directly compatible with the workspace 0.59 pin and will NOT force a version bump. It resolves to the same workspace `windows-sys` crate via `Cargo.lock`; there is no second copy. The API is safe Rust:

```rust
use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_READ};
use winreg::RegKey;

let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
let policy = hklm.open_subkey_with_flags(
    r"SOFTWARE\Policies\nono",
    KEY_READ,
)?;
let allow_domains: String = policy.get_value("AllowDomains").unwrap_or_default();
let default_deny: u32 = policy.get_value("DefaultDenyEgress").unwrap_or(0u32);
```

The `open_subkey_with_flags` with `KEY_READ` is the correct posture for reading policy: it does not allow accidental writes, and it works for non-admin processes reading `HKLM\SOFTWARE\Policies` (the ACL on that path grants `Everyone` read by convention — Group Policy depends on this).

**Existing precedent:** `platform.rs` already reads `HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion` for OS version detection, but via a `reg.exe` subprocess (`std::process::Command`). That approach is acceptable for one-off startup detection but is too slow and fragile for a hot-path policy reader that runs at every agent launch or on policy-change notifications. `winreg` gives direct API access.

**Policy-change notifications:** For live reload when an admin pushes a GPO update mid-session, `RegNotifyChangeKeyValue` is available via `windows-sys` 0.59 (`Win32_System_Registry` feature). This is optional for v3.0 (a process restart on policy push is acceptable initially) but should be tracked. `winreg` does not wrap `RegNotifyChangeKeyValue` directly; use `windows-sys` raw for that one call.

**`windows-registry` (Microsoft's crate):** Not recommended. It requires MSRV 1.82, higher than the workspace MSRV of 1.95 — wait, the workspace MSRV is actually 1.95 (higher), so that concern does not apply. However, `windows-registry` is less widely used, has a smaller ecosystem, and `winreg` is the de-facto standard for safe registry access in the Rust Windows ecosystem with 15M+ downloads. Stick with `winreg`.

---

### Theme 3 — Structured Security Events to Windows Event Log / ETW, Tamper-Evident Chain

#### Recommendation: `tracing-etw` 0.2.3 as the primary SIEM/EDR forwarding path; `eventlog` 0.4.0 as the human-readable Event Viewer path; `hmac` 0.13.0 for the hash chain.

**Why two event emission paths:**

- **`tracing-etw`** (ETW / TraceLogging): The right channel for SIEM/EDR ingestion. ETW events flow into the kernel's real-time delivery bus where Windows Event Forwarding (WEF), Azure Monitor, Sysmon, and SIEM collectors (Splunk Universal Forwarder, Microsoft Sentinel MMA) all subscribe. TraceLogging events are self-describing (no `mc.exe` manifest needed), which is important for a product shipping to diverse corporate environments without guaranteed pre-installed manifests. `tracing-etw` 0.2.3 is a `tracing-subscriber::Layer` — it drops into the existing `tracing_subscriber::registry().with(...)` chain already initialized in `nono-cli`. The ETW provider GUID is auto-derived from the provider name string; no separate registration step at install time.

- **`eventlog`** (Windows Event Log / `ReportEvent`): The right channel for Event Viewer browsing by a local IT admin. `eventlog` 0.4.0 integrates with Rust's `log` crate, maps the five log levels to fixed Event IDs (1=Error, 2=Warn, 3=Info, 4=Debug, 5=Trace), and embeds the minimal 120-byte message DLL so Event Viewer shows message text (not `The description for Event ID N from source nono cannot be found`). Registration requires admin rights and must happen at MSI install time (via `util:EventSource` in WiX — see Theme 1). The `log` crate is NOT the same as `tracing`, but `tracing-log` (already a transitive dep in the `tracing-subscriber` tree) bridges them: `tracing_log::LogTracer::init()` routes `log::error!(...)` / `eventlog`'s emitter into the `tracing` pipeline, or you emit independently via `eventlog::log(Level::Error, "nono", "...", ...)`.

**Why NOT pick only one:**
- ETW alone: IT admins lose the familiar Event Viewer / PowerShell `Get-WinEvent` interface for quick triage on an endpoint.
- `eventlog` alone: SIEM collectors do not forward the `Application` Event Log channel by default on most corporate WEF/Sentinel configurations; they forward `Security` and `System` plus subscribed channels. ETW reaches collectors directly via real-time session subscription.

**Integration with existing `DiagnosticFormatter` deny path:**
The `DiagnosticFormatter` in `crates/nono/src/diagnostic.rs` already produces human-readable denial explanations. The v3.0 emitter wraps those into a structured `SecurityEvent` struct (serialized to JSON for the ETW field) and calls `tracing::error!(event_id=..., category=..., detail=..., "nono.deny.path")` with the `tracing-etw` layer consuming it. No change to the library boundary — the emission happens in `nono-cli` where the deny callback lands.

**Tamper-evident hash chain:**
`hmac` 0.13.0 + existing workspace `sha2` dep. The chain is: each `SecurityEvent` carries a `prev_hash: [u8; 32]` field (SHA-256 HMAC of the previous serialized event). The HMAC key is a per-session random 32-byte secret held in the supervisor process (not written to disk). This gives a within-session tamper-evident chain: any modification of a prior event breaks the chain on verification. Cross-session immutability (i.e., the "append-only" claim in SEED-003) requires persisting the chain root — scope that as an optional follow-on (SEED-005 ZT-Infra is the full answer; for v3.0, document the limitation).

`hmac` 0.13.0 is confirmed compatible with `sha2` (the `hmac` crate's own docs list `sha2` as the canonical integration dep; both implement the `digest` trait family). The workspace already has `sha2 = "0.11"`. No version conflict — `hmac 0.13.0` depends on `digest 0.10.x`, and `sha2 0.11` implements `digest 0.10`. Verified via docs.rs.

**Syslog:** Not a new crate. Syslog forwarding is handled by the IT infrastructure (Windows Event Forwarding → syslog bridge, or SIEM agent). nono does not need to speak raw syslog; emitting to ETW + Windows Event Log is sufficient for all standard enterprise SIEM pipelines.

---

## Supporting Libraries Summary

| Library | Version | Status | Purpose | Cargo.toml change |
|---------|---------|--------|---------|-------------------|
| `winreg` | **0.56** | NEW dep | Safe HKLM policy reads with typed value extraction | Add to `[target.'cfg(target_os = "windows")'.dependencies]` in `nono-cli/Cargo.toml` |
| `tracing-etw` | **0.2.3** | NEW dep | `tracing-subscriber::Layer` that emits ETW/TraceLogging events for SIEM/EDR consumption | Add to `[target.'cfg(target_os = "windows")'.dependencies]` in `nono-cli/Cargo.toml` |
| `eventlog` | **0.4.0** | NEW dep | `log`-compatible Windows Event Log emitter (Event Viewer / `Get-WinEvent`) | Add to `[target.'cfg(target_os = "windows")'.dependencies]` in `nono-cli/Cargo.toml` |
| `hmac` | **0.13.0** | NEW dep (all platforms) | HMAC-SHA256 for tamper-evident per-session event chain | Add to `[dependencies]` in `nono/Cargo.toml` (library, not CLI — chain is computed at event struct creation) OR `nono-cli/Cargo.toml` |
| `sha2` | 0.11 (existing workspace dep) | NO CHANGE | SHA-256 for HMAC; already in workspace | None |
| `windows-sys` | 0.59 (existing workspace pin) | FEATURE ADDITION ONLY | Add `"Win32_System_Registry"` feature to enable `RegOpenKeyExW`/`RegQueryValueExW` for `RegNotifyChangeKeyValue` live-reload (if implemented) | Add `"Win32_System_Registry"` to the existing feature list on line 149 |
| `tracing` / `tracing-subscriber` | 0.1 / 0.3 (existing) | NO CHANGE | Existing logging stack; `tracing-etw` drops in as an additional layer | None |
| `serde` / `serde_json` | workspace (existing) | NO CHANGE | SecurityEvent struct serialization for ETW structured fields + HMAC input | None |
| `winreg` | — | REPLACES | The `reg.exe` subprocess approach in `platform.rs` for policy reads (NOT for OS detection — leave that alone) | — |

---

## Installation

```toml
# crates/nono-cli/Cargo.toml

# Theme 2: machine policy reader
[target.'cfg(target_os = "windows")'.dependencies]
winreg = "0.56"   # NEW — safe HKLM policy reads; compatible with workspace windows-sys 0.59

# Theme 3: event emission
tracing-etw = "0.2"   # NEW — tracing-subscriber layer for ETW/SIEM path (windows-only)
eventlog = "0.4"      # NEW — log-compat Windows Event Log emitter (Event Viewer path)

# Existing windows-sys feature list (line 149): add ONE new feature
windows-sys = { version = "0.59", features = [
    # ... all existing features ...
    "Win32_System_Registry",    # ADD: enables RegOpenKeyExW/RegQueryValueExW for RegNotifyChangeKeyValue
    # Win32_System_EventLog and Win32_System_Diagnostics_Etw are ALREADY present
] }

# crates/nono/Cargo.toml OR crates/nono-cli/Cargo.toml

# Theme 3: tamper-evident hash chain
[dependencies]
hmac = "0.13"    # NEW — HMAC-SHA256 for event hash chain; works with existing sha2 workspace dep
```

---

## Alternatives Considered

| Recommended | Alternative | Why Not |
|-------------|-------------|---------|
| WiX MSI (keep) | MSIX | MSIX cannot package kernel-mode drivers or `LocalSystem` services — nono ships both. The existing signed-MSI pipeline works. MSIX adds no value for SCCM/Intune Win32 app deployment, which wraps MSI into `.intunewin` anyway. Defer MSIX to a future distribution milestone if the driver requirement ever drops. |
| `winreg` 0.56 | Raw `windows-sys` `Win32_System_Registry` calls | `winreg` wraps the same underlying APIs with safe Rust types (no buffer management, no two-call size query, typed `get_value::<u32>()`). Since `winreg` depends on `windows-sys >=0.59, <=0.61`, it resolves to the same crate instance — no version conflict, no bloat. The raw API is appropriate only if `winreg` is missing a needed call (e.g. `RegNotifyChangeKeyValue`); use raw `windows-sys` only for that specific call alongside `winreg`. |
| `winreg` 0.56 | `windows-registry` 0.6.1 (Microsoft's crate) | `windows-registry` requires MSRV 1.82; workspace MSRV is 1.95, so this concern does not apply. However, `windows-registry` is newer, less widely used, and introduces a different API surface. `winreg` has the larger community, more examples, and existing usage patterns across the Windows Rust ecosystem. `winreg` is the established standard. |
| `tracing-etw` 0.2.3 (ETW) + `eventlog` 0.4.0 (Event Log) | Single crate covering both channels | No single crate covers both ETW and the classic `ReportEvent` API. They serve different consumers: ETW → SIEM real-time bus; `ReportEvent`/`RegisterEventSource` → Event Viewer. Using both is standard enterprise Windows application practice. |
| `tracing-etw` as the SIEM path | `win_etw_tracing` | `win_etw_tracing` is explicitly superseded by `tracing-etw` per its own docs. Dead end. |
| `hmac` 0.13.0 + existing `sha2` | `hmac-sha256` (self-contained crate) | `hmac-sha256` is a no_std friendly standalone but does not integrate with the `digest` trait family used by the rest of the crypto stack. `hmac 0.13.0` + `sha2` uses the same `RustCrypto` trait foundation as the existing workspace — one consistent crypto primitive family. |
| `log`-level bridge via `tracing-log` | Separate `log` initialization | The `tracing-log` bridge (`tracing_log::LogTracer::init()`) is already a transitive dep; it routes `log::*!()` calls into the `tracing` pipeline so the `eventlog` `log`-backend appears in `tracing` spans. Avoids two separate logging initializations. |

---

## What NOT to Use

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| **MSIX packaging** | Cannot package `LocalSystem` services or kernel-mode drivers (nono-wfp-service, nono-wfp-driver.sys). Machine-scope MSI already works. | WiX MSI (keep), wrap with `IntuneWinAppUtil.exe` for Intune Win32 app. |
| **`mc.exe` / `wevtutil im` ETW manifest pipeline** | Required for strongly-typed `EventWrite` (Crimson/XML manifest) ETW events, but `tracing-etw` uses TraceLogging (self-describing events) — no manifest needed, no `mc.exe`/`rc.exe` build step, no `.man` file to distribute. | `tracing-etw` 0.2.3 (`LayerBuilder::new("nono").build()`). |
| **Writing to the `Security` channel** | Requires `SeAuditPrivilege` (only available to `LocalSystem` and the OS itself). nono runs as a normal user on the audit emission path. | `Application` channel via `eventlog` 0.4.0 + ETW via `tracing-etw`. |
| **`reg.exe` subprocess for hot-path policy reads** | Already used in `platform.rs` for one-time OS detection — acceptable there. Unacceptable for policy reads at every agent launch: spawning a subprocess per launch adds ~100ms latency, is fragile on constrained endpoints, and produces no typed values. | `winreg` 0.56 direct API. Leave `platform.rs` alone (it reads NT version, not security policy). |
| **`windows-sys` version bump (0.59 → 0.61.x)** | Gratuitous churn; cross-target-drift hazard (two cfg-gated compile errors already reached release tags). `winreg` 0.56 accepts `>=0.59, <=0.61` — the version range is already satisfied. | Stay on 0.59 pin; add `"Win32_System_Registry"` feature only. |
| **Cross-session tamper-evidence without SEED-005** | Persisting a HMAC chain root across sessions (for "append-only" guarantees) requires a trusted storage mechanism — exactly what SEED-005 (ZT-Infra signed policy) provides. Attempting it with a local file is defeatable (attacker can truncate the file). | Implement per-session tamper-evidence (in-memory HMAC chain, chain root lost on exit); document the cross-session limitation; defer immutable audit trail to SEED-005. |
| **ADMX tooling beyond a text editor** | Visual Studio's ADMX template designer, LGPO.exe etc. are convenience tools, not requirements. The ADMX XML schema is hand-authorable and must be kept in the repo as a checked-in artifact. | Author `nono.admx` + `nono.adml` by hand; validate with `xmllint`; ship in the MSI under `%SystemRoot%\PolicyDefinitions\` or document manual placement. |

---

## Stack Patterns by Variant

**If the policy reader is called on every agent launch (hot path):**
- `winreg` `open_subkey_with_flags(KEY_READ)` → `get_value()` at startup; cache in a `OnceLock<PolicySnapshot>` or `Arc<RwLock<PolicySnapshot>>`; refresh on `RegNotifyChangeKeyValue` (via `windows-sys` raw call) or on explicit `nono policy reload` command.
- Because cold-registry reads are fast (~1 µs) but the two-call pattern + error handling is cleaner through `winreg`.

**If only a handful of policy keys are needed (v3.0 scope):**
- A thin `PolicyReader` struct in `nono-cli` that reads: `DefaultDenyEgress: u32`, `AllowDomains: String` (comma-separated), `MaxSessionMinutes: u32`, `EventLogEnabled: u32`. No serialization library needed — `winreg` returns typed Rust primitives.

**If the ETW channel needs to be queryable from PowerShell:**
- `Get-WinEvent -ProviderName "nono"` works for TraceLogging providers IF there is an active ETW session subscribing to the provider. For persistent queryable events use the `eventlog` / `Application` channel path (persisted in the Windows Event Log).
- For SIEM (real-time forwarding): instruct IT to create a persistent ETW session (`logman create trace nono-audit -p {GUID} -bs 64 -mode Circular -f bincirc -max 256 -o C:\Logs\nono.etl -ets`) or configure a WEF subscription for the `Application` channel.

**If building for the Intune Win32 app path:**
- Use the machine MSI unmodified: `msiexec /i nono-machine-*.msi /quiet /norestart ALLUSERS=1`.
- `IntuneWinAppUtil.exe` wraps it: `IntuneWinAppUtil.exe -c dist\windows -s nono-machine-v*.msi -o dist\intunewin`.
- Detection rule: Registry key `HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\{D5948D55-89A4-4F09-AB43-44DBA9D25D1A}` (`DisplayName` value = `nono`). The `UpgradeCode` GUID `D5948D55-89A4-4F09-AB43-44DBA9D25D1A` is already in `build-windows-msi.ps1` for machine scope.

---

## Version Compatibility

| Package | Compatible With | Notes |
|---------|-----------------|-------|
| `winreg 0.56` | `windows-sys >=0.59, <=0.61` | Resolves to workspace 0.59 pin; confirmed from `winreg-rs` `Cargo.toml`. No second copy of `windows-sys`. |
| `tracing-etw 0.2.3` | Rust 1.80+ (MSRV); `tracing 0.1.x`; `tracing-subscriber 0.3.x` | Workspace MSRV is 1.95 — above the 1.80 requirement. Windows-only at runtime; exports a stub or feature-gates on non-Windows. |
| `eventlog 0.4.0` | Rust `log` crate (transitive via `tracing-log`) | Released 2026-03-05. Maps to `log::Level` enum. Bridge via `tracing_log::LogTracer::init()`. |
| `hmac 0.13.0` | `sha2 0.11` (workspace), `digest 0.10.x` | `sha2 0.11` implements `digest 0.10`; `hmac 0.13` requires `digest 0.10`. Compatible. Confirmed via docs.rs hmac 0.13.0. |
| `windows-sys 0.59` (feature addition) | All existing workspace deps | Adding `"Win32_System_Registry"` is additive; does not affect any existing feature or API. |

---

## Integration Points with Existing Backend

- **Policy reader → WFP/proxy enforcement:** `PolicyReader::load()` returns `PolicySnapshot { allow_domains: Vec<String>, default_deny: bool, ... }`. This feeds directly into the `CapabilitySet` builder (via `nono-cli`'s `policy.rs` group resolver) as machine-policy-level constraints that cannot be overridden by per-user profile. The per-user `allow_domain` Phase 56 logic becomes the fallback when the machine policy key is absent.

- **Event emitter → `DiagnosticFormatter`:** `DiagnosticFormatter::format()` already produces a human-readable denial string. Wrap it: `SecurityEvent::from_deny(&diagnostic)` → `tracing::error!(security_event = ?event, "nono.security.deny")` — `tracing-etw` picks up the structured field; `eventlog` sees the string via `tracing-log`. One call site, two sinks.

- **Hash chain → `SecurityEvent` struct:** Each `SecurityEvent` carries a `prev_hash: [u8; 32]`. The supervisor process holds the chain state in a `Mutex<ChainState>` (current hash + HMAC key). This is supervisor-process-local state — it belongs in `nono-cli`, not in the library.

- **WiX MSI → event source registration:** Add `<util:EventSource Name="nono" Log="Application" EventMessageFile="[INSTALLFOLDER]nono.exe" />` inside a `<Component>` in the machine-scope `$serviceComponentXml` block in `build-windows-msi.ps1`. This requires `eventlog::register("nono")` to NOT be called at runtime (the MSI registers it); or call it defensively with a `match` that ignores `AlreadyExists`.

- **Cross-target discipline:** `winreg`, `tracing-etw`, and `eventlog` are all Windows-only. They MUST be gated under `[target.'cfg(target_os = "windows")'.dependencies]`. `hmac` is cross-platform and should go in the workspace or the nono-lib `[dependencies]` without a cfg gate. Follow the CLAUDE.md cross-target clippy MUST rule: run `cargo clippy --workspace --target x86_64-unknown-linux-gnu` after any phase adding these deps — the new imports must not appear in non-Windows cfg branches.

---

## Sources

- In-tree: `crates/nono-cli/Cargo.toml` line 149 (existing `windows-sys` 0.59 feature set: `Win32_System_EventLog` and `Win32_System_Diagnostics_Etw` already present, `Win32_System_Registry` absent); `scripts/build-windows-msi.ps1` (WiX MSI generation, machine-scope `UpgradeCode`, `ServiceInstall`, `Vital="no"`); `crates/nono-cli/src/platform.rs` (existing `reg.exe` subprocess registry pattern — confirms the gap `winreg` fills) — **HIGH confidence** (authoritative, current code).
- [docs.rs/winreg/latest](https://docs.rs/winreg/latest/winreg/) — version 0.56.0 confirmed; `open_subkey_with_flags`/`get_value` API; `windows-sys >=0.59, <=0.61` dep range — **HIGH**.
- [docs.rs/tracing-etw/latest](https://docs.rs/tracing-etw/latest/tracing_etw/) — version 0.2.3 confirmed; `LayerBuilder::new("name").build()` API; MSRV 1.80; TraceLogging self-describing events — **HIGH**.
- [lib.rs/crates/tracing-layer-win-eventlog](https://lib.rs/crates/tracing-layer-win-eventlog) — version 1.0.1; alternative considered and passed over in favor of `eventlog` (which integrates with `log` crate bridge rather than requiring a separate tracing layer) — **MEDIUM**.
- [docs.rs/eventlog (latest = 0.4.0)](https://docs.rs/crate/eventlog/latest) — version 0.4.0 released 2026-03-05; `log`-compat; embeds 120-byte message DLL; fixed event IDs 1–5 — **HIGH**.
- [docs.rs/hmac/0.13.0](https://docs.rs/hmac/latest/hmac/) — version 0.13.0; `HmacSha256 = Hmac<Sha256>` pattern; `digest 0.10` trait; compatible with `sha2 0.11` workspace dep — **HIGH**.
- [learn.microsoft.com — MSIX services support](https://learn.microsoft.com/en-us/windows/msix/desktop/desktop-to-uwp-prepare) + [turbo.net MSIX limitations 2025](https://www.turbo.net/blog/posts/2025-06-16-understanding-msix-limitations-enterprise-application-compatibility) — MSIX cannot package LocalSystem services or kernel drivers — **HIGH**.
- [learn.microsoft.com — Intune Win32 app deployment](https://learn.microsoft.com/en-us/intune/app-management/deployment/win32); [microsoft/microsoft-win32-content-prep-tool](https://github.com/microsoft/microsoft-win32-content-prep-tool) — `.intunewin` wrapping workflow — **HIGH**.
- [learn.microsoft.com — ADMX ingestion via Intune OMA-URI](https://learn.microsoft.com/en-us/troubleshoot/mem/intune/device-configuration/deploy-oma-uris-to-target-csp-via-intune) — ADMX CSP path for Intune — **MEDIUM** (indirect, from search results).
- [docs.rs/windows-sys/latest/windows_sys/Win32/System/Registry](https://docs.rs/windows-sys/latest/windows_sys/Win32/System/Registry/fn.RegQueryValueExW.html) — `Win32_System_Registry` feature confirmed present in windows-sys 0.59 — **HIGH**.
- [GitHub — gentoo90/winreg-rs Cargo.toml](https://github.com/gentoo90/winreg-rs/blob/master/Cargo.toml) — `windows-sys = ">=0.59, <=0.61"` dep range confirmed — **HIGH**.

---

*Stack research for: nono v3.0 Enterprise Hardening I (Deploy · Control · Compliance)*
*Researched: 2026-06-18*
