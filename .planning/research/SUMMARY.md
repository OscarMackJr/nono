# Research Summary — nono v2.10: Kernel-Driver Spike + EDR UAT + macOS Upstream Parity

**Synthesized:** 2026-06-06 | **Confidence:** HIGH | Sources: STACK.md, FEATURES.md, ARCHITECTURE.md, PITFALLS.md (codebase + `git cherry` upstream diff + Microsoft Learn/WDK + MITRE ATT&CK)

## Executive Summary

v2.10 delivers three work streams on the v2.9 foundation.

**1. Windows FltMgr minifilter feasibility spike (Gap 6b).** A **test-signed C/C++** kernel driver in a new **out-of-workspace `drivers/nono-fltmgr/` MSBuild project** (NOT a Cargo crate — `windows-drivers-rs` is early-stage/KMDF-only and not viable) that proves `IRP_MJ_CREATE` pre-op interception and `FLT_PREOP_COMPLETE`→`STATUS_ACCESS_DENIED` deny, plus the user-mode policy roundtrip. POC only — a **go/no-go ADR** gates any production investment. ETW cannot block; FltMgr is the only path to Windows filesystem-enforcement parity with Linux seccomp. The **sole Cargo change** is one `windows-sys` feature (`Win32_Storage_InstallableFileSystems`) for a small `#[cfg(windows)]` user-mode `FilterConnectCommunicationPort`/`FilterGetMessage` client (`fltmgr_client.rs` in `exec_strategy_windows/`).

**2. WR-02 EDR HUMAN-UAT.** Closes a v2.1 deferral with **no new code** — run existing v2.9 binaries against Sysmon (free EDR-proxy) and/or an MDE trial, producing ~10 concrete pass/fail assertions. Load-bearing: whether EDR DLL-injection into Low-IL children **fails at the NO_WRITE_UP MIC boundary** as expected, and whether the broker's `CreateProcessAsUserW`+`SetTokenInformation(IntegrityLevel)` sequence (**MITRE T1134.002**) triggers quarantine.

**3. macOS Seatbelt upstream parity sync (v0.57.0 → v0.61.2).** Three P1 security/correctness commits the fork lacks: **`8f84d454`** (platform rules ordering after user write-allows — active security ordering defect), **`362ada22`** + **`8f1b0b74`** (symlink/`$PWD` CWD correctness), plus `729697c2` (`--trust-proxy-ca`, P2) and the UPST7-deferred macOS items. Follows the established UPST DIVERGENCE-LEDGER audit shape (Phase 42 template) with a `macos-only` column.

## Stack Additions

- **Minifilter driver:** WDK 28000.1761 (VS 2026) or 26100.6584 (VS 2022) + FltMgr; **C/C++ `.sys`, not Rust**. Test-signing pipeline: `makecert → inf2cat → signtool → certmgr → bcdedit /set testsigning on → reboot → pnputil`. `SERVICE_DEMAND_START` avoids boot-start signing strictness.
- **User-mode client:** add `"Win32_Storage_InstallableFileSystems"` to the existing `windows-sys = "0.59"` feature list in `crates/nono-cli/Cargo.toml`. `#[repr(C)]` struct mirroring `FILTER_MESSAGE_HEADER` with a static layout assertion on both sides.
- **EDR UAT:** Sysmon (free, ~30s install, EDR-proxy) and/or Microsoft Defender for Endpoint 90-day trial (no credit card; needs an M365 tenant). **No new Rust deps.**
- **macOS:** **no new crates / no MSRV bump** — existing `sandbox_init()` FFI + `nix` + `libc`.

## Feature Landscape (table stakes / differentiators / anti-features)

- **Spike table stakes:** pre-create interception + a real deny (`FLT_PREOP_COMPLETE`); a user-mode roundtrip proof (`FLT_PREOP_PENDING` + `FltSendMessage` with a **finite timeout**). **Anti-features (defer):** production EV/WHQL signing, kernel-version-maintenance hardening, MSI-packaging the driver, `IRP_MJ_ACQUIRE_FOR_SECTION_SYNCHRONIZATION` deny (can only return `STATUS_INSUFFICIENT_RESOURCES`, not access-denied), Rust kernel bindings.
- **EDR UAT:** ~10 assertions across supervisor/Low-IL/AppContainer visibility, false-positive risk (Job Object, WFP filter, mandatory label, broker token seq), and the MIC DLL-injection boundary. Must run **no-exclusion first** (characterize alerts) **then with-exclusion** (confirm suppression) — running only-with-exclusion proves nothing.
- **macOS:** P1 = the three correctness/security commits; P2 = `--trust-proxy-ca`. `sandbox-exec` deprecation is **not actionable** (no replacement API; keep `sandbox_init()`).

## Watch Out For (pitfalls → owning phase)

- **HVCI / Secure Boot blocks test-signing on the Win11 build-26200 host** (HVCI default-on; silently rejects HVCI-incompatible drivers at load with no BSOD/error). → A **Hyper-V Secure-Boot-OFF VM is the correct spike environment.** Check `msinfo32` before planning. (Phase 63)
- **BSOD iteration cost** (IRQL violations, own-I/O recursion via `ZwCreateFile`, blocking `FltSendMessage`, stack overflow) — each is a hard reboot. → **Design-doc-before-code gate:** prohibit driver-originated file I/O, mandate finite IPC timeouts, ring-buffer + worker-thread IPC. (Phase 63/64)
- **Altitude selection:** an altitude in the AV range (320000–329998) can fail load AND disrupt the installed EDR. → Use the Activity-Monitor/FSFilter range; **request an official altitude from Microsoft early** (`fsfcomm@microsoft.com`, ~30 business days). (Phase 63 kickoff)
- **Broker = MITRE T1134.002 signal** — modern EDRs flag the token sequence. → EDR UAT must characterize this without exclusions first. (Phase 66)
- **macOS cross-target drift is a PROVEN release blocker** — broke `v0.62.0`+`v0.62.1` (E0716 + edition-2024 let-chain in cfg-gated code that compiled fine on Windows). → **"CI macOS build leg green" is a HARD close gate, not advisory**; cherry-pick checklist must scan for let-chains/E0716 + assert Seatbelt rule ordering (last-match-wins) on a macOS host. (Phase 65) See `feedback_clippy_cross_target`.

## Suggested Build Order (Phases 63-66, continuing from 62)

1. **Phase 63** — Minifilter spike groundwork (WDK/VM verified, altitude requested, design doc) **∥** macOS DIVERGENCE-LEDGER audit (commit inventory v0.57.0..v0.61.2).
2. **Phase 64** — Minifilter spike implementation (intercept + deny + IPC roundtrip on the test VM) **∥** macOS P1 cherry-pick wave.
3. **Phase 65** — Minifilter go/no-go ADR (with latency data) + macOS re-validation HUMAN-UAT on a real macOS host (**CI macOS-green hard gate**).
4. **Phase 66** — WR-02 EDR HUMAN-UAT (code-independent; floats to whenever an EDR host is available).

## Open Questions (resolve at plan-phase)

- **HVCI/VM host state** on Win11 26200 — `msinfo32` before Phase 63 plan; nested-virt Hyper-V may add kernel-debugger scope.
- **Which EDR product** is available for Phase 66 (MDE trial needs an M365 tenant ~1 day; Sysmon-only is the fallback).
- **macOS host availability** for Phase 65 live `sandbox_init()` re-validation.
- **`FLT_PREOP_PENDING` latency budget** is unmeasured — must be quantified in the spike for the ADR.
- The **exact macOS-relevant upstream commit set** — Phase 63's audit produces it (`git log v0.57.0..v0.61.2` scoped to macOS paths).
