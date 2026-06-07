# Architecture Research — nono v2.10

**Domain:** Capability-based OS sandbox — kernel driver spike, EDR UAT, macOS upstream sync
**Researched:** 2026-06-06
**Confidence:** HIGH — all integration points derived from direct source inspection of the v0.62.2 codebase and prior milestone planning artifacts

---

## Overview

This document answers three architecture questions for the v2.10 milestone, building on the shipped v2.9 base. It does NOT re-describe the existing architecture; it maps only what is new or modified.

---

## Theme A: Windows FltMgr Minifilter — Feasibility Spike

### What exists today (the placeholder)

`crates/nono-cli/data/windows/nono-wfp-driver.sys` is a 156-byte ASCII text file:

```
Placeholder nono Windows WFP driver artifact.
This is not a real kernel driver.
Its purpose is to establish the expected install and registration contract.
```

It is embedded at build time alongside the `nono-wfp-service` binary and copied to the output directory by `build.rs`. The `nono setup --install-wfp-driver` plumbing (`install_windows_wfp_driver`) already registers it as a `type=kernel demand` SCM service (`nono-wfp-driver`). The `build_wfp_probe_status` function was updated in Phase 62 plan 62-06 to NOT require the driver for `WfpProbeStatus::Ready` — the driver slot is explicitly out of the enforcement path.

The current clean-uninstall invariant (REQ-DRN-01, Phase 53) covers the driver service: `uninstall_windows_wfp_with_runner` calls `remove_single_windows_service` for both `nono-wfp-service` and `nono-wfp-driver`. The MSI `CaUninstallWfpServices` custom action invokes this with `Return=ignore` so uninstall is never blocked.

### Where a real minifilter `.sys` lives

A real FltMgr minifilter cannot be a Rust Cargo crate; the WDK-based driver model requires C/C++ or WDK-C with a specialized build environment. The correct workspace placement is:

```
drivers/
  nono-fltmgr/
    nono-fltmgr.c         # Filter entry, FltRegisterFilter, callbacks
    nono-fltmgr.h
    nono-fltmgr.vcxproj   # MSBuild project targeting WDK
    sources               # (optional; WDK 7/8 legacy)
    inf/
      nono-fltmgr.inf     # INF for test-signed install
    nono-fltmgr.sln
  README.md               # Build + signing + test-install instructions
```

This is a **separate MSBuild sub-project** outside the Cargo workspace. The Cargo workspace `Cargo.toml` does NOT gain a new member. The spike deliverable is a compiled, test-signed `.sys` that can be installed on a test-signing-enabled VM — it is NOT bundled into the MSI for v2.10.

The existing placeholder `crates/nono-cli/data/windows/nono-wfp-driver.sys` remains the MSI-packaged artifact for v2.10. The spike driver lives in `drivers/nono-fltmgr/` and is built and installed manually on the test VM. A follow-on milestone (Gap 6b production) would replace the placeholder with the real `.sys` in the MSI.

**Confidence:** HIGH — WDK minifilters cannot be expressed as Rust/Cargo targets; the `drivers/` top-level directory is the standard pattern for mixed-language Windows driver repos (confirmed from WDK samples and WDF driver repository conventions).

### Communication: FilterCommunicationPort vs WFP control pipe

The existing WFP user-mode service uses a named pipe (`\\.\pipe\nono-wfp-control`) with a JSON request/response protocol (`WfpRuntimeActivationRequest` / `WfpRuntimeActivationResponse`). The client is `run_wfp_runtime_request` in `exec_strategy_windows/network.rs`.

An FltMgr minifilter communicates with user mode through a **FilterCommunicationPort** (`FltCreateCommunicationPort` / `FilterConnectCommunicationPort`). This is a completely separate channel from the WFP named pipe — it is not layered on top of it.

For the spike, the minifilter should use a **dedicated FilterCommunicationPort** rather than reusing the WFP control pipe, for these reasons:

1. The WFP service and the minifilter are different kernel subsystems. Conflating them into one pipe would make it impossible to independently stop/start the minifilter service without breaking WFP enforcement.
2. The `WfpRuntimeActivationRequest` protocol is scoped to WFP ALE layer concepts (session SID, package SID, AppContainer); extending it with file-trust concepts would pollute a well-tested protocol.
3. FilterCommunicationPort is the Microsoft-documented API for minifilter user-kernel IPC and is the only channel that supports the required altitude-gated kernel callbacks.

**Recommended spike IPC shape:**

```
user-mode side (new nono-fltmgr-client.rs in nono-cli, Windows-only):
  FilterConnectCommunicationPort(L"\\NonoPolicyPort")
  send: { "request_kind": "check_trust", "path": "...", "pid": N }
  recv: { "decision": "allow" | "block" | "unknown" }

kernel side (nono-fltmgr.c):
  FltCreateCommunicationPort(filter, L"\\NonoPolicyPort", ...)
  IRP_MJ_CREATE pre-operation callback → message to user-mode, wait, apply decision
```

The user-mode client is a small new Rust module — `crates/nono-cli/src/exec_strategy_windows/fltmgr_client.rs` — gated `#[cfg(target_os = "windows")]`. It uses `windows-sys` `FilterConnectCommunicationPort` / `FilterSendMessage` / `FilterGetMessage`. It does NOT depend on the existing WFP pipe, the nono-wfp-service, or the nono library; it is nono-cli-only policy infrastructure.

### Driver lifecycle vs existing WFP service + MSI packaging

The WFP service (`nono-wfp-service`) runs as LocalSystem and starts `start=auto` (Phase 62, 62-01). The minifilter driver would run as a kernel-mode driver with a separate SCM entry (`type=kernel`). The existing placeholder SCM registration (`nono-wfp-driver`, `start=demand`) already models the right shape. For the spike this stays `demand` — the driver is started manually on the test VM, not boot-started.

**Lifecycle composition:**

```
Boot:
  SCM start=auto → nono-wfp-service.exe (SYSTEM)   [existing, Phase 62]
  SCM start=demand → nono-wfp-driver.sys           [spike: manual start on test VM]
  FilterManager loads nono-fltmgr.sys on demand    [gap 6b spike only]

nono run --profile runner:
  → network.rs probe_wfp_backend_status             [unchanged, driver NOT required for Ready]
  → (future) fltmgr_client: FilterConnectCommunicationPort

msiexec /x:
  CaUninstallWfpServices (Return=ignore)
    → run_backend_purge (WFP objects)
    → remove_single_windows_service(nono-wfp-service)  [unchanged]
    → remove_single_windows_service(nono-wfp-driver)   [unchanged — still placeholder .sys]
```

**Clean-uninstall invariant (REQ-DRN-01) is fully preserved.** The spike driver is not bundled in the MSI; therefore MSI uninstall cannot break from the spike. The `nono-wfp-driver` SCM service is already removed by the uninstall path. If a follow-on milestone bundles the real `.sys`, the MSI `RemoveFiles` + `ServiceControl` elements for the driver service need to be added to `build-windows-msi.ps1` — that is out of scope for v2.10.

### Spike component boundary

The spike has a clean surface: one new out-of-workspace MSBuild project (`drivers/nono-fltmgr/`) and one small new Rust module (`fltmgr_client.rs`). Nothing in the nono library, nono-proxy, nono-shell-broker, or nono-ffi is touched.

```
Modified:
  crates/nono-cli/data/windows/nono-wfp-driver.sys  [no change for v2.10 spike]
  crates/nono-cli/src/exec_strategy_windows/mod.rs  [add fltmgr_client mod, cfg-gated]

New (Rust):
  crates/nono-cli/src/exec_strategy_windows/fltmgr_client.rs

New (C/MSBuild, outside Cargo workspace):
  drivers/nono-fltmgr/nono-fltmgr.c
  drivers/nono-fltmgr/nono-fltmgr.vcxproj
  drivers/nono-fltmgr/inf/nono-fltmgr.inf
  drivers/nono-fltmgr/README.md

New (documentation):
  .planning/adr/ADR-NNN-fltmgr-spike-go-no-go.md
```

The `nono-wfp-driver.sys` placeholder in `data/windows/` is NOT replaced during the spike — the spike binary lives only in `drivers/` and is installed manually on the test VM.

**Cross-target clippy note:** `fltmgr_client.rs` contains `#[cfg(target_os = "windows")]` code. Per CLAUDE.md § Coding Standards, this triggers the cross-target clippy verification requirement. Because the WDK headers required to call `FilterConnectCommunicationPort` via `windows-sys` may not be available in the Linux/macOS cross-compile environment, the cross-target verification for this file is likely to be PARTIAL and must be documented per `.planning/templates/cross-target-verify-checklist.md`.

---

## Theme B: WR-02 EDR HUMAN-UAT

### Nature of the phase

This is a **validation phase only** — no code changes. The deliverable is a HUMAN-UAT document recording verdicts. The existing codebase is the test subject.

### Host/runner setup

The EDR UAT requires:

1. A **real Windows 10/11 host** (not CI VM, not Azure Hosted runner) with a commercial EDR product installed and actively monitoring. Representative options: CrowdStrike Falcon, Microsoft Defender for Endpoint, SentinelOne. The host can be the existing Win11 26200 dev machine if an EDR agent can be installed, or a dedicated test VM.

2. The **machine MSI** installed (signed v0.62.2 or later), so `nono-wfp-service` is registered and running at boot, and the full supervised run + AppContainer path is exercised.

3. **Specific test scenarios** covering the behaviors most likely to trigger EDR alerts:
   - `nono run --profile runner -- cmd /c echo hi` (Low-IL AppContainer child spawn — CreateProcessW with SECURITY_CAPABILITIES)
   - `nono run --profile claude-code -- claude --version` (heavy-runtime child with Low-IL token)
   - `nono setup --install-wfp-service` + `nono setup --start-wfp-service` (SCM service registration from a standard-user session — FwpmFilterAdd0 via LocalSystem service)
   - A blocked-network run: `nono run --profile runner --network-block -- curl https://example.com` (WFP ALE_AUTH kernel filter + AppContainer SID)
   - Session hook execution (`session_hooks` with a PowerShell hook script — Phase 58 surface)

4. **Verdict criteria** (what the HUMAN-UAT document records):
   - Was a **false-positive alert** raised? For each scenario: yes/no, alert type, alert text.
   - Did the EDR **block** any nono operation? (E.g., quarantine nono.exe, block WFP filter install, kill the Low-IL child.)
   - Is nono's behavior **visible** to the EDR in a useful way? (Useful for defenders — this is a positive outcome.)
   - Are there **remediation steps** if the EDR interferes? (Signing exceptions, policy exclusions.)

5. **Artifact:** `.planning/phases/NNN-edr-human-uat/NNN-HUMAN-UAT.md` following the established Phase 62 and Phase 53 HUMAN-UAT document shape.

### Integration points

No new integration surface. The EDR UAT exercises:

- `exec_strategy_windows/launch.rs` — AppContainer spawn (`SECURITY_CAPABILITIES`, `CreateAppContainerProfile`)
- `exec_strategy_windows/network.rs` — WFP service control-pipe IPC (`run_wfp_runtime_request`)
- `crates/nono-cli/src/bin/nono-wfp-service.rs` — FwpmFilterAdd0 from LocalSystem
- `crates/nono-cli/src/hooks.rs` + Phase 58 hook runtime — session hook PowerShell execution

No files are modified. The EDR runner setup and teardown are operator steps documented in the HUMAN-UAT plan.

---

## Theme C: macOS Seatbelt Upstream Parity Sync (through v0.61.2)

### Audit shape: mirrors DIVERGENCE-LEDGER

The established DIVERGENCE-LEDGER audit shape (YAML frontmatter + cluster table + per-cluster rationale) applies here, mirroring Phase 42 (UPST5 audit) and Phase 47 (UPST6 audit). The file lives at:

```
.planning/phases/NNN-upst8-macos-audit/DIVERGENCE-LEDGER.md
```

### Scope of the audit

- **Range:** upstream `v0.57.0..v0.61.2` (the fork's confirmed high-water mark is v0.57.0)
- **Filter:** macOS-relevant commits only — commits touching `macos.rs`, `sandbox/`, Seatbelt DSL, `learn.rs` macOS branches, macOS-only profile fields, macOS-specific policy groups in `policy.json`, and macOS CI lanes
- **Also pick up:** the two UPST7-deferred macOS items:
  - `$PWD` symlink-CWD capture — adds `symlink-canonicalize-cwd` logic before `sandbox_init`; affects `crates/nono/src/sandbox/macos.rs` and `crates/nono-cli/src/exec_strategy.rs` (or equivalent Unix exec path)
  - `platform-rules-after-user-write-allows` ordering — the rule that platform deny rules must be placed after user-granted write-allows to avoid defeating user intent; currently `generate_profile` in `macos.rs` positions platform rules between reads and writes (verified in source), so this may already match; diff-inspect required before concluding

### Surfaces the audit touches

| File | What changes | Risk |
|------|-------------|------|
| `crates/nono/src/sandbox/macos.rs` | `generate_profile` — new rules, ordering fixes, new capability fields | MEDIUM — profile generation is security-critical; every change needs a test |
| `crates/nono-cli/src/exec_strategy.rs` (or Unix exec path) | `$PWD` symlink-CWD capture before sandbox apply | LOW — additive; `std::env::current_dir()` + `canonicalize` before `Sandbox::apply` |
| `crates/nono-cli/data/policy.json` | New macOS-only groups or deny rules | LOW — additive group additions |
| `crates/nono-cli/src/profile/builtin.rs` or `policy.rs` | macOS-specific profile fields from upstream | LOW — additive |
| `crates/nono/src/capability.rs` | New `CapabilitySet` fields if upstream adds new macOS capability types | MEDIUM — library API; check C FFI + nono-ffi impact |

The `nono-ffi` C bindings (`bindings/c/src/`) are impacted only if `crates/nono/src/capability.rs` adds public types. If it does, `types.rs` and `capability_set.rs` in the FFI crate need corresponding updates and `cbindgen` re-run. This must be flagged in the audit ledger per cluster.

### Diff-inspect approach

The macOS sync follows the same diff-inspect discipline as UPST5/UPST6:

1. `git fetch upstream --tags && git log v0.57.0..v0.61.2 --no-merges --oneline` scoped to macOS-relevant paths
2. For each candidate commit: read the diff, classify into: `will-sync` / `fork-preserve` / `won't-sync` / `split`
3. The `windows-touch` column equivalent here is a `macos-only` column — flag commits that add Seatbelt DSL constructs or macOS-only FFI that have no Linux equivalent, because these need lib-boundary review (library vs CLI) before cherry-picking
4. Per D-42-C2 pattern: flag commits touching `capability.rs` or `lib.rs` as potentially affecting the C FFI surface

### Re-validation requirement

After cherry-picks land, the Seatbelt layer must be re-validated on a **real macOS host**. This is the equivalent of the Phase 62 HUMAN-UAT for the Windows path. The re-validation runs `nono run` with the `claude-code` profile on macOS and confirms:

- `sandbox_init` succeeds with the updated profile
- Previously-passing tests still pass (focus: `test_generate_profile_*` in `macos.rs`, `make test-lib` on macOS)
- The UPST7-deferred items (`$PWD` CWD capture, rule ordering) behave correctly

---

## Component Summary: New vs Modified

| Component | Status | Owner crate | Notes |
|-----------|--------|-------------|-------|
| `drivers/nono-fltmgr/` (MSBuild) | **NEW** | outside Cargo workspace | Spike only; not bundled in MSI |
| `exec_strategy_windows/fltmgr_client.rs` | **NEW** | nono-cli | `#[cfg(windows)]`; FilterCommunicationPort client |
| `exec_strategy_windows/mod.rs` | **MODIFIED** | nono-cli | Add `fltmgr_client` mod declaration |
| EDR HUMAN-UAT doc | **NEW** | planning artifact | No code changes |
| `sandbox/macos.rs` | **MODIFIED** | nono (library) | Upstream cherry-picks; tests required |
| `capability.rs` | **POSSIBLY MODIFIED** | nono (library) | Only if upstream adds macOS capability types |
| `policy.json` | **POSSIBLY MODIFIED** | nono-cli (embedded) | New macOS-only groups |
| `bindings/c/src/` | **POSSIBLY MODIFIED** | nono-ffi | Only if capability.rs gains new public types |
| `nono-wfp-driver.sys` (placeholder) | **UNCHANGED** | nono-cli data | Spike driver lives in `drivers/`, not here |
| `nono-wfp-service.rs` | **UNCHANGED** | nono-cli bin | EDR UAT exercises but does not change |
| `exec_strategy_windows/network.rs` | **UNCHANGED** | nono-cli | WFP pipe protocol unchanged |

---

## Build Order Recommendation

The three themes are mostly independent, but the minifilter spike has a hard dependency on a Windows test VM with test signing enabled and a WDK build environment — setup lead time is significant. The macOS audit requires a macOS host for re-validation.

**Recommended phase ordering:**

```
Phase 63 (parallel-safe):
  63-A: macOS DIVERGENCE-LEDGER audit (any host; read-only git log + diff)
  63-B: Minifilter spike groundwork:
        - WDK environment setup documentation
        - drivers/ directory scaffold + .vcxproj + .inf
        - test-signing VM setup + TESTSIGNING boot flag
        - fltmgr_client.rs skeleton (compiles, no real calls yet)

Phase 64 (sequential):
  64-A: macOS cherry-pick wave (depends on 63-A audit)
  64-B: Minifilter spike implementation (depends on 63-B groundwork):
        - IRP_MJ_CREATE pre-callback
        - FilterCommunicationPort message exchange
        - test-signed .sys install on test VM
        - end-to-end file-open interception proof

Phase 65 (sequential):
  65-A: macOS re-validation HUMAN-UAT on macOS host (depends on 64-A)
  65-B: Minifilter spike ADR + go/no-go recommendation (depends on 64-B)

Phase 66 (parallel-safe; no code dependencies):
  66:   WR-02 EDR HUMAN-UAT (EDR runner available; depends on no code changes)
```

**Parallelism rationale:**

- Phase 63-A and 63-B have no file overlap — the audit is read-only and the `drivers/` scaffold is entirely new
- Phase 66 (EDR UAT) has no code dependencies; it can run as soon as the test host is available, even before the macOS sync lands; however, if the macOS sync introduces new profile content the EDR UAT may want to test, scheduling it after 64-A is preferable
- macOS audit (63-A → 64-A → 65-A) is the critical path for macOS host availability; if a macOS host is not available, 65-A blocks
- The minifilter spike (63-B → 64-B → 65-B) is the critical path for WDK environment availability; the spike ADR is a gate before any production driver decision

**Phase numbering:** continues from Phase 62 (Phase 63+).

---

## Anti-Patterns to Avoid

### Bundling the spike `.sys` in the MSI

**What:** Replacing `crates/nono-cli/data/windows/nono-wfp-driver.sys` with the spike binary and shipping it in the v2.10 MSI.
**Why not:** A test-signed driver requires the machine to have TESTSIGNING enabled (boot-time flag). Installing it on a standard production machine would fail silently or require user action. The spike is a de-risking artifact, not a production component. The Phase 53 clean-uninstall invariant was tested against the placeholder — replacing it changes the uninstall surface mid-milestone without a re-UAT.
**Instead:** Keep the placeholder in `data/windows/`; ship the spike binary only in `drivers/nono-fltmgr/` for manual test-VM use.

### Reusing the WFP named-pipe for minifilter IPC

**What:** Extending `\\.\pipe\nono-wfp-control` and `WfpRuntimeActivationRequest` to carry minifilter decisions.
**Why not:** The WFP pipe protocol is `serde_json`-based JSON over a tokio async named pipe. FilterCommunicationPort uses a synchronous kernel APC-based message system with fixed-size message buffers — it cannot be layered on a tokio pipe. Conflating the two makes both harder to reason about and test independently.
**Instead:** Dedicated `\\NonoPolicyPort` FilterCommunicationPort in `fltmgr_client.rs`.

### Skipping the DIVERGENCE-LEDGER for macOS commits and blind cherry-picking

**What:** Running `git cherry-pick` on upstream macOS commits without a per-commit diff-inspect audit first.
**Why not:** Phase 43 Plan 43-01 proved that cluster-isolation assumptions fail when upstream commits have cross-cluster re-export dependencies. The `capability.rs` surface is cross-platform; a macOS-only upstream commit may touch shared types and break the C FFI. The Phase 48 close also showed that cherry-picks on Windows-host without cross-target clippy can ship cfg-gated Unix compile errors.
**Instead:** Run the DIVERGENCE-LEDGER audit (Phase 63-A), diff-inspect every candidate, classify, then cherry-pick in disposition order.

### Running the EDR UAT on a CI Azure Hosted runner

**What:** Substituting a CI runner for a real EDR-instrumented host.
**Why not:** Azure Hosted runners do not have commercial EDR agents installed, and any EDR agent would be in an unknown configuration state. The point of WR-02 is to validate against a representative enterprise EDR posture, not a clean VM.
**Instead:** A dedicated test VM (or the dev machine) with an EDR agent actively enforcing policy.

---

## Sources

- Source inspection: `crates/nono-cli/src/exec_strategy_windows/network.rs` (v0.62.2, direct read)
- Source inspection: `crates/nono-cli/src/bin/nono-wfp-service.rs` (v0.62.2, lines 1-80)
- Source inspection: `crates/nono/src/sandbox/macos.rs` (v0.62.2, lines 1-1304)
- Source inspection: `crates/nono-cli/data/windows/nono-wfp-driver.sys` (placeholder, 156 bytes)
- Planning artifact: `.planning/milestones/v2.9-ROADMAP.md` — Phase 62 WFP architecture, AppContainer/package-SID design, clean-uninstall invariant
- Planning artifact: `.planning/PROJECT.md` — v2.10 milestone scope (Gap 6b, WR-02, macOS sync through v0.61.2)
- Planning artifact: `.planning/phases/42-upst5-audit/DIVERGENCE-LEDGER.md` — DIVERGENCE-LEDGER audit shape, diff-inspect methodology, windows-touch column
- Microsoft FltMgr docs: FilterCommunicationPort / FltCreateCommunicationPort API (WDK, HIGH confidence — standard minifilter IPC pattern since WDK 2003)
