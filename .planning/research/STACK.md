# Stack Research: v2.10 Kernel-Driver Spike + EDR UAT + macOS Upstream Parity

**Project:** nono — v2.10 new capabilities (minifilter spike, EDR HUMAN-UAT, macOS parity)
**Researched:** 2026-06-06
**Confidence:** HIGH (codebase-confirmed baselines; WDK/MSDN verified; windows-sys docs.rs confirmed)
**Scope:** Additive changes ONLY relative to the existing baseline. Do not re-research anything already in Cargo.toml.

---

## Existing Baseline (Confirmed in Cargo.toml — Do Not Re-research)

| What | Confirmed state |
|------|----------------|
| `windows-sys = "0.59"` with features including `Win32_Storage_FileSystem`, `Win32_Security`, `Win32_System_Services`, `Win32_System_Threading`, `Win32_System_Diagnostics_Etw`, `Win32_NetworkManagement_WindowsFilteringPlatform` | `crates/nono-cli/Cargo.toml` L145 — source-confirmed |
| `ferrisetw = "1.2"` (ETW consumer — already adopted; v2.0 STACK.md was uncertain, now resolved) | `crates/nono-cli/Cargo.toml` L147 — source-confirmed |
| `nono-wfp-service` binary with real `FwpmFilterAdd0` kernel WFP enforcement (service-only model, D-05) | Phase 62 debug doc + `exec_strategy_windows/network.rs` |
| `windows-service = "0.7"` for SCM management | `crates/nono-cli/Cargo.toml` L144 |
| Rust 1.95 MSRV (workspace.package.rust-version) | `Cargo.toml` L7 |
| macOS sandbox backend: `sandbox_init()` FFI with Seatbelt profile DSL | `crates/nono/src/sandbox/macos.rs` |
| Upstream high-water mark: v0.57.0 (fork's last sync); target: v0.61.2 | git log + PROJECT.md |

---

## Feature A: Windows Kernel Minifilter Spike (Gap 6b POC)

### What Is New

The driver itself (`nono-minifilter.sys`) is C/C++ code built with the WDK, not Rust. The user-mode component that communicates with it is Rust, using the existing `windows-sys` crate with one new feature flag. This is a hard language boundary: the .sys is C, the nono-cli integration is Rust.

### A1: Driver Build Toolchain (C/C++, outside Cargo)

| Tool | Version | Purpose | Why This Version |
|------|---------|---------|-----------------|
| WDK | 28000.1761 (latest, 2026-05) | Kernel driver headers, FltMgr.lib, build templates, `inf2cat`, `signtool`, `certmgr` | Current recommended; WDK 26100.6584 is the alternative if VS 2022 is required instead of VS 2026. SDK and WDK build numbers must match. |
| Visual Studio | 2026 Community or Professional | C/C++ compiler, WDK VSIX integration, driver project templates | WDK 28000.1761 targets VS 2026; VS 2022 pairs with WDK 26100.6584. Required components: "Desktop development with C++", six Spectre-mitigated library individual components (see WDK install guide), "Windows Driver Kit" individual component. |
| Windows SDK | 10.0.28000.1 (matching WDK) | Headers + SDK tools (`signtool.exe`, `makecert.exe`, `certmgr.exe`) | SDK build number must exactly match WDK build number; install via the direct SDK download link rather than relying on VS installer. |
| FltMgr.lib | Ships with WDK | Import library for FltMgr minifilter API | Comes with WDK; no separate download. Link via `#pragma comment(lib, "FltMgr.lib")` in the driver project. |
| `inf2cat.exe` | Ships with WDK | Generates the .cat catalog file from the driver .inf | Required for the test-signing pipeline before `signtool` can sign the package. |

The EWDK (Enterprise WDK) ISO is a self-contained alternative if VS installation on the spike box is impractical: it contains VS 2026 Build Tools 18.3.0 + SDK + WDK in one mountable ISO.

### A2: Driver API Surface (FltMgr — kernel-mode C)

All APIs are in `<fltKernel.h>` (ships with WDK). No additional headers needed beyond the standard minifilter project template.

| API | Header | Purpose in Spike |
|-----|--------|-----------------|
| `FltRegisterFilter` | fltKernel.h | Register minifilter with FltMgr in `DriverEntry` |
| `FltStartFiltering` | fltKernel.h | Activate after registration |
| `FltUnregisterFilter` | fltKernel.h | Clean unload in `FilterUnload` callback |
| `FltCreateCommunicationPort` | fltKernel.h | Create kernel-side communication port for user-mode messaging |
| `FltSendMessage` | fltKernel.h | Push pre-create event data to waiting user-mode reader; use with finite `Timeout` (e.g., 500 ms as `LARGE_INTEGER`) — NEVER NULL timeout in a spike (BSOD risk if user-mode reader absent; see PITFALLS.md) |
| `FLT_CALLBACK_DATA` | fltKernel.h | Pre-create callback argument; provides file name, process info, I/O parameters |
| `FltGetFileNameInformation` | fltKernel.h | Retrieve file name from callback data; use `FLT_FILE_NAME_NORMALIZED` |
| `FltReleaseFileNameInformation` | fltKernel.h | Release name buffer after use |
| `IoGetRequestorProcess` / `PsGetProcessId` | ntddk.h | Get requestor PID for the callback event |

**INF/load-order configuration:**
- `LoadOrderGroup = "FSFilter Activity Monitor"` (same group as Sysmon, at altitude 385201)
- Spike altitude: request a test altitude in the 320000–329998 range from Microsoft, or use 370020 (nullFilter sample default) on a test-only host. Do NOT use a Sysmon-adjacent altitude on a production host.
- `StartType = SERVICE_DEMAND_START` (3) — the spike driver is loaded on demand, not at boot. This avoids the boot-start driver embedding-signature requirement.
- `ServiceType = SERVICE_FILE_SYSTEM_DRIVER` (2)
- `Dependencies = "FltMgr"`

### A3: User-Mode Communication (Rust side — existing windows-sys crate)

The user-mode component (inside `nono-cli` or a helper binary) calls FltLib APIs to connect to the driver's communication port and receive events.

**New `windows-sys` feature flag required:**

```toml
# In crates/nono-cli/Cargo.toml, under [target.'cfg(target_os = "windows")'.dependencies]:
windows-sys = { version = "0.59", features = [
    # ... existing features (DO NOT remove) ...
    "Win32_Storage_InstallableFileSystems",   # NEW for minifilter user-mode comm
] }
```

This feature exposes `FilterConnectCommunicationPort`, `FilterGetMessage`, `FilterSendMessage`, and `FilterReplyMessage` — all confirmed present in `windows_sys::Win32::Storage::InstallableFileSystems` at docs.rs.

| User-Mode API (in windows-sys) | Purpose |
|-------------------------------|---------|
| `FilterConnectCommunicationPort` | Open connection to driver's communication port by name (e.g., `L"\\NonoPocPort"`) |
| `FilterGetMessage` | Blocking read of `FILTER_MESSAGE_HEADER` + payload from driver via `FltSendMessage` |
| `FilterSendMessage` | Send a message DOWN to the driver (not needed for receive-only POC) |
| `FilterReplyMessage` | Reply to a `FltSendMessage` that requested a reply |

For the spike (prove pre-create interception only), the user-mode side only needs `FilterConnectCommunicationPort` + `FilterGetMessage` in a loop. The Rust wrapper is ~50 lines of `unsafe` code following the same RAII + `// SAFETY:` pattern used in `exec_strategy_windows/launch.rs`.

**Integration point with existing nono-wfp-service:** The minifilter communicates via a named FltMgr communication port, not via the existing WFP named-pipe IPC. These are independent channels. The spike does NOT touch `nono-wfp-service` or the existing WFP enforcement path at all — it is an additive POC component.

**Where the spike code lives:** A new standalone binary or test harness, NOT wired into production `nono run` paths. The spike is explicitly a POC; production integration is deferred pending the go/no-go ADR.

### A4: Test-Signing Pipeline

The spike requires TESTSIGNING mode on the test host. This is a one-time host setup:

```
# Step 1: Generate test certificate (in WDK Developer Command Prompt)
makecert -r -pe -ss PrivateCertStore -n CN=NonoMinifilterTest NonoTest.cer

# Step 2: Build the .sys (via VS/WDK)

# Step 3: Generate catalog file
inf2cat /v /driver:.\driver-package /os:10_x64

# Step 4: Sign the .cat
signtool sign /v /s PrivateCertStore /n NonoMinifilterTest /t http://timestamp.digicert.com nono-minifilter.cat

# Step 5: Embed-sign the .sys (required for 64-bit Windows; demand-start still needs this)
signtool sign /v /s PrivateCertStore /n NonoMinifilterTest /t http://timestamp.digicert.com nono-minifilter.sys

# Step 6: Install test cert on the host
certmgr.exe /add NonoTest.cer /s /r localMachine root
certmgr.exe /add NonoTest.cer /s /r localMachine trustedpublisher

# Step 7: Enable test-signing (requires elevation; requires reboot)
bcdedit /set testsigning on
# Reboot

# Step 8: Install driver
pnputil /add-driver nono-minifilter.inf /install
# OR: sc create + sc start for demand-start non-PnP
```

**HVCI/Memory Integrity blocker:** On Windows 11 22H2+, HVCI (Hypervisor-Protected Code Integrity) is on by default. HVCI blocks test-signed drivers unless the host has it disabled. Check via `msinfo32.exe` → "Virtualization-based security" or `System Information`. To disable: Settings → Windows Security → Device Security → Core isolation → Memory integrity → Off. Requires reboot. The dev spike host must have HVCI disabled or use a dedicated VM (Hyper-V with nested virtualization, or a bare-metal test box).

**Recommended spike host:** A dedicated VM or bare-metal machine with HVCI disabled, NOT the primary dev box. BSOD risk during driver development is real (see PITFALLS.md).

**Secure Boot interaction:** If `bcdedit /set testsigning on` returns "The value is protected by Secure Boot policy", Secure Boot must be disabled in BIOS/UEFI first. For a test VM this is straightforward; for the main dev machine, leave Secure Boot on and use a dedicated VM.

### A5: Rust-to-C Boundary Summary

| Boundary | What crosses it | How |
|----------|----------------|-----|
| Kernel driver (C) → User-mode Rust | Pre-create event data: file path + PID | `FltSendMessage` kernel-side → `FilterGetMessage` user-side; message struct defined in a shared C header (`nono_minifilter_ipc.h`), manually mirrored as a `#[repr(C)]` struct in Rust |
| User-mode Rust → Kernel driver (C) | Connection open, optional reply | `FilterConnectCommunicationPort` + `FilterReplyMessage` via `windows-sys` unsafe FFI |
| Rust Cargo build | Does NOT build the .sys | The driver project is a separate Visual Studio/MSBuild project; the Rust build produces only the user-mode component |

**No new Rust crates are needed for the user-mode side** beyond the one feature flag addition to `windows-sys`. The driver side is C/C++ + WDK, outside Cargo.

### A6: What NOT to Add

| Avoid | Why |
|-------|-----|
| Attempting to write the .sys in Rust (e.g., `windows-kernel-rs`) | No production-ready Rust kernel driver framework exists for FltMgr minifilters; the WDK C/C++ path is the only auditable, supportable route. Spike must be deliverable quickly. |
| Using the EWDK COM automation path | Only needed for CI driver builds; for a spike, VS IDE is faster to iterate with. |
| `sc create bintype= kernel` without INF | Non-PnP demand-start works for the spike but skips the filter registration needed for FltMgr altitude assignment. Use `pnputil /add-driver` with the INF. |
| Wiring the spike into `nono run` production paths | The spike is POC-only. Production integration deferred to post-ADR milestone. |

---

## Feature B: WR-02 EDR HUMAN-UAT

### B1: EDR Options for a Dev/Test Host

The UAT needs an EDR that can be installed on the dev host (Windows 11, build 26200), captures relevant telemetry (process creation, file access, network calls), and can be set up without enterprise enrollment complexity.

| Option | Practicality | What It Captures | Setup Cost |
|--------|-------------|-----------------|-----------|
| **Microsoft Defender for Endpoint (MDE) P2 — 90-day trial** | HIGH — no credit card, adds to existing Microsoft account; works on Windows 11 workstation; provides cloud-based alert portal at security.microsoft.com | Process tree, file events, network connections, Low-IL/AppContainer process launches, driver load events | Moderate: requires M365 tenant (free trial available), onboarding script download, ~10 min setup. Onboarding script is a `.cmd` that runs once. |
| **Sysmon (Sysinternals)** | HIGH — free, no license, installs in 30 seconds, self-contained, logs to Windows Event Log (Channel: `Microsoft-Windows-Sysmon/Operational`) | Process create (EID 1), network connect (EID 3), file creation (EID 11), driver load (EID 6), pipe events (EID 17/18), image load (EID 7) | Very low: `sysmon64.exe -accepteula -i sysmonconfig.xml`; use SwiftOnSecurity config as baseline |
| **Windows Defender Antivirus (built-in) + Event Log** | BASELINE — already present; zero setup | Process launches, network blocks, SmartScreen events | Zero — already running on any Windows 11 host |
| **Commercial EDR trial (CrowdStrike Falcon, SentinelOne)** | LOW — trials require sales contact or credit card; install is simple but tenant setup takes days | Full EDR visibility | High: procurement overhead makes this impractical for a developer-run UAT |

**Recommendation for WR-02:** Use **Sysmon + MDE trial** in combination. Sysmon gives immediate local event log visibility with zero tenant complexity; MDE trial gives the cloud-portal alert view that a real enterprise deployment would produce. Together they cover both the "does a real EDR see nono's activity?" question and the "what does the alert look like in a SOC console?" question.

If tenant setup for MDE is too slow for the milestone schedule, **Sysmon alone** is sufficient to close WR-02 as a valid EDR-proxy validation. Sysmon is widely used as an EDR proxy in security research and is explicitly designed to replicate EDR telemetry coverage.

### B2: Telemetry Capture for the UAT

| What to Capture | Tool | Event IDs / Method |
|-----------------|------|--------------------|
| nono process spawning Low-IL child | Sysmon | EID 1 (process create), EID 17/18 (pipe connect for supervisor) |
| nono-wfp-service loading | Sysmon | EID 6 (driver load) for any driver activity; Windows Event Log for SCM service start |
| AppContainer child process launch | Sysmon | EID 1 with IntegrityLevel field |
| WFP filter installation | MDE or Windows Security Center | Network events, firewall policy change audit |
| nono binary itself | Sysmon EID 1, MDE Process events | Confirm visibility under both |

**Tooling needed (zero new Rust/Cargo dependencies):**
- `sysmon64.exe -accepteula -i config.xml` — install Sysmon
- `Get-WinEvent -LogName 'Microsoft-Windows-Sysmon/Operational' -MaxEvents 100` — PowerShell query
- MDE onboarding `.cmd` from security.microsoft.com (if MDE trial chosen)
- Sysmon config: SwiftOnSecurity `sysmonconfig-export.xml` from GitHub (de-facto baseline config)

**No new Cargo dependencies.** This is a HUMAN-UAT (operator-run), not an automated integration test. The deliverable is a set of recorded verdicts against success criteria, not new test code.

---

## Feature C: macOS Seatbelt Upstream Parity (v0.57.0 → v0.61.2)

### C1: What's in the Upstream Window

The fork's macOS high-water mark is v0.57.0. The target is v0.61.2. Confirmed macOS-relevant commits in the window (from `git log v0.57.0..upstream/main`):

| Commit | Version | Description | Files |
|--------|---------|-------------|-------|
| `8f84d454` | ~v0.58 | `fix(macos): emit platform rules after user write allows` | `crates/nono/src/sandbox/macos.rs` |
| `fe233db4` (PR #680) | ~v0.58 | `fix(sandbox): preserve symlink path when adding CWD capability on macOS` | `crates/nono-cli/src/sandbox_prepare.rs`, `crates/nono-cli/src/profile/mod.rs` |
| `729697c2` | ~v0.60 | `feat(proxy): add --trust-proxy-ca for macOS system trust store integration` | proxy + macOS Keychain handling |
| `2f4e1a37` | ~v0.60 | `fix(proxy): clean up Keychain on trust failure and expand security docs` | proxy macOS Keychain |
| `6c472224` | ~v0.60 | `refactor(proxy): consolidate Keychain CA storage to single combined PEM entry` | proxy macOS |

The two Seatbelt-core fixes (`8f84d454` and `fe233db4`) are the highest-risk items — they touch `macos.rs` and sandbox preparation directly. The proxy/Keychain changes are macOS-relevant but touch `nono-proxy`, not the sandbox primitive.

Non-macOS commits in the window (v0.58–v0.61.2) include: session hooks, Bitwarden creds, fine-grained `allow_domain`, supervisor socket IPC, interactive denial selector, RPM artifacts, JSONC writing, registry refs in profile extends, diagnostic suppress-system-service option. These are also in scope for the UPST8 audit but are not macOS-specific.

### C2: Toolchain for macOS Re-validation

No new Rust crates are needed. The macOS Seatbelt backend uses private `libc`/`sandbox_init()` FFI which is already in place.

| Tool | Purpose | Notes |
|------|---------|-------|
| macOS host (Intel or Apple Silicon, macOS 12+) | Compile and run the macOS backend | The existing cross-target CI covers compilation; live re-validation needs a real macOS host. `sandbox_init()` is a runtime-only call; CI can compile but cannot sandbox-exec. |
| `cargo clippy --workspace --target x86_64-apple-darwin` + `--target aarch64-apple-darwin` | Cross-target clippy per CLAUDE.md rule | Required for all commits touching `#[cfg(target_os = "macos")]` code. On Windows host = PARTIAL (CI-deferred). On macOS host = full local verify. |
| `git cherry-pick` or `git diff` | Cherry-pick upstream commits into fork | Standard workflow per prior UPST cycles. DIVERGENCE-LEDGER pattern required. |
| `sandbox-exec -p '(version 1)(allow default)'` | Manual Seatbelt profile syntax test | macOS built-in; tests profile DSL syntax before the full `sandbox_init()` call |

**No new Cargo dependencies for the macOS parity work.** The commits in scope are fixes and features that use the existing Seatbelt + `nix` + `libc` surface.

### C3: macOS-Specific MSRV Note

The workspace MSRV is currently `rust-version = "1.95"` (confirmed in `Cargo.toml`). The upstream macOS commits do not introduce APIs that require a Rust version newer than 1.95. No MSRV bump is expected from the macOS parity work.

---

## Complete Additive Dependency Table

| Package / Change | Where | Feature | Why |
|---------|-------|---------|-----|
| `windows-sys` feature `Win32_Storage_InstallableFileSystems` | `crates/nono-cli/Cargo.toml` `[target.'cfg(target_os = "windows")'.dependencies]` | New feature flag on existing dep | Exposes `FilterConnectCommunicationPort` + `FilterGetMessage` for user-mode FltMgr comm port |

**That is the only Cargo.toml change across all three v2.10 features.**

All other new work is:
- A new C/C++ Visual Studio + WDK project (outside Cargo) for the minifilter `.sys`
- A new Rust source file for the user-mode spike harness (new `use` imports in the `Win32_Storage_InstallableFileSystems` module)
- A shared `#[repr(C)]` IPC struct mirroring the C-side message header
- A HUMAN-UAT runbook (no new code)
- Cherry-picks + manual replays into existing Rust files for macOS parity

---

## Driver vs. Rust-Userspace Boundary (Explicit)

```
┌─────────────────────────────────────────┐
│  Kernel (C, WDK, IRQL constraints)      │
│  nono-minifilter.sys                    │
│  - FltRegisterFilter                    │
│  - Pre-create callback                  │
│  - FltCreateCommunicationPort           │
│  - FltSendMessage → named port          │
│  Language: C                            │
│  Build: Visual Studio + WDK             │
│  Signed: test-signed .cat + embedded    │
└──────────────┬──────────────────────────┘
               │ FltMgr named communication port
               │ (\\NonoPocPort)
               │ FILTER_MESSAGE_HEADER + payload
               │ #[repr(C)] struct mirrored in Rust
               ▼
┌─────────────────────────────────────────┐
│  User-mode (Rust, safe wrapper)         │
│  Spike harness in nono-cli or binary    │
│  - FilterConnectCommunicationPort       │
│  - FilterGetMessage (blocking loop)     │
│  - windows-sys Win32_Storage_           │
│    InstallableFileSystems               │
│  Language: Rust                         │
│  Build: Cargo                           │
└─────────────────────────────────────────┘
```

The shared IPC message struct must be defined ONCE in C (the driver source) and manually replicated in Rust as a `#[repr(C, packed)]` struct with the same field types and layout. This is the single most fragile point of the C↔Rust boundary; a layout mismatch causes silent data corruption or reads from garbage memory. The spike plan must include a static size/offset assertion on the C side and a matching Rust-side `assert_eq!(size_of::<NonoMinifilterMessage>(), EXPECTED_SIZE)`.

---

## Integration with Existing nono-wfp-service

The minifilter spike is INDEPENDENT of `nono-wfp-service`. They coexist without conflict:

| Component | Mechanism | Port / Channel |
|-----------|-----------|---------------|
| `nono-wfp-service` (existing) | `FwpmFilterAdd0` at ALE_AUTH layers | Named pipe IPC (`NONO_WFP_PIPE`) |
| `nono-minifilter.sys` (new spike) | `FltSendMessage` at pre-create callback | FltMgr named comm port (`\\NonoPocPort`) |

The WFP service enforces network; the minifilter spike intercepts file opens. They operate on different kernel components (BFE/FWPM vs FltMgr) and different named channels. No code changes to `nono-wfp-service` are needed for the spike.

---

## Alternatives Considered

| Recommended | Alternative | Why Not |
|-------------|-------------|---------|
| WDK + VS 2026 for driver build | Rust kernel driver frameworks (e.g., `windows-kernel-rs`) | No production FltMgr minifilter support in Rust ecosystem; spike must be deliverable; C is the only auditable path |
| `windows-sys` `Win32_Storage_InstallableFileSystems` for user-mode | `fltlib` P/Invoke via `libloading` dynamic load | `windows-sys` is already a dependency and follows the established codebase pattern; dynamic loading adds complexity for no benefit |
| Sysmon + MDE trial for EDR UAT | CrowdStrike/SentinelOne trial | Commercial trials require sales contact; Sysmon is the de-facto EDR-proxy in security research and sufficient for WR-02 |
| TESTSIGNING + self-signed cert for spike | Attestation signing (cross-signed EV cert) | Attestation signing requires hardware EV cert purchase; out of scope for a feasibility spike |
| Demand-start driver (`SERVICE_DEMAND_START`) | Boot-start driver | Boot-start requires embedded signature AND kernel debugger discipline; demand-start is safer for spike iteration and does not require embedding |

---

## Version Compatibility Notes

| Component | Version | Compatibility Constraint |
|-----------|---------|--------------------------|
| WDK | 28000.1761 | Must match SDK build number exactly (both 28000.x) |
| Visual Studio | 2026 | WDK 28000.1761 VSIX targets VS 2026; VS 2022 requires WDK 26100.6584 |
| `windows-sys` | 0.59 (existing) | `Win32_Storage_InstallableFileSystems` feature confirmed present at this version (docs.rs). No version bump needed. |
| Rust MSRV | 1.95 (existing workspace) | No bump required; minifilter user-mode Rust code uses only stable `unsafe` + `windows-sys` |
| macOS MSRV | 1.95 | Upstream macOS commits v0.58–v0.61.2 do not introduce newer Rust requirements |

---

## Sources

- `crates/nono-cli/Cargo.toml` — existing `windows-sys 0.59` feature list, `ferrisetw = "1.2"` (source-confirmed, HIGH)
- [Microsoft Learn: Download the WDK](https://learn.microsoft.com/en-us/windows-hardware/drivers/download-the-wdk) — WDK 28000.1761 + VS 2026 recommendation (MSDN current, HIGH)
- [Microsoft Learn: Test Signing](https://learn.microsoft.com/en-us/windows-hardware/drivers/install/test-signing) — test-sign pipeline: `makecert` → `inf2cat` → `signtool` → `certmgr` → `bcdedit /set testsigning on` (MSDN, HIGH)
- [docs.rs: windows_sys::Win32::Storage::InstallableFileSystems](https://docs.rs/windows-sys/latest/windows_sys/Win32/Storage/InstallableFileSystems/index.html) — `FilterConnectCommunicationPort`, `FilterGetMessage`, `FilterSendMessage`, `FilterReplyMessage` confirmed present (docs.rs, HIGH)
- [Microsoft Learn: Communication Between User-mode and Minifilters](https://learn.microsoft.com/en-us/windows-hardware/drivers/ifs/communication-between-user-mode-and-kernel-mode) — `FltCreateCommunicationPort` + `FilterConnectCommunicationPort` pattern (MSDN, HIGH)
- [Windows driver samples: nullFilter](https://github.com/microsoft/Windows-driver-samples/blob/main/filesys/miniFilter/nullFilter/nullFilter.inf) — altitude 370020, LoadOrderGroup FSFilter Activity Monitor, `StartType = SERVICE_DEMAND_START` (Microsoft GitHub, HIGH)
- [Microsoft Learn: Load Order Groups and Altitudes](https://learn.microsoft.com/en-us/windows-hardware/drivers/ifs/load-order-groups-and-altitudes-for-minifilter-drivers) — altitude assignment for Activity Monitor group (MSDN, HIGH)
- `.planning/debug/resolved/wfp-driver-gate-placeholder.md` — confirms nono-wfp-driver.sys is a placeholder; nono-wfp-service has real FwpmFilterAdd0 (source-confirmed, HIGH)
- `git log v0.57.0..upstream/main` — macOS-relevant upstream commits confirmed: `8f84d454` (platform rules order), PR #680 `fe233db4` (symlink CWD), `729697c2`/`2f4e1a37`/`6c472224` (proxy Keychain) (git-confirmed, HIGH)
- [Microsoft Learn: MDE Trial User Guide](https://learn.microsoft.com/en-us/defender-endpoint/defender-endpoint-trial-user-guide) — 90-day trial, no credit card required (MSDN, MEDIUM)
- [Sysmon documentation](https://learn.microsoft.com/en-us/sysinternals/downloads/sysmon) — Event IDs, install procedure (MSDN, HIGH)
- HVCI default-on since Windows 11 22H2 — confirmed via Microsoft blog post on Windows 11 security defaults (MEDIUM — behavior confirmed, specific build number is 22H2 or 23H2+)

---

*Stack research for: nono v2.10 — kernel minifilter spike, EDR HUMAN-UAT, macOS Seatbelt parity*
*Researched: 2026-06-06*
