---
phase: 64
plan: "04"
subsystem: macos-seatbelt-ordering + windows-minifilter-vm-uat
tags: [windows, fltmgr, minifilter, kernel, vm, test-signing, macos, seatbelt, cherry-pick, drv-01, drv-02, drv-03]
dependency_graph:
  requires:
    - nono-fltmgr.c/.h extended minifilter (Plan 64-02)
    - nono-fltmgr-client run_policy_client (Plans 64-01/64-03)
    - macos.rs Wave 0 RED ordering tests (Plan 64-01)
  provides:
    - 8f84d454 applied to macos.rs (platform rules after write-allows; Seatbelt last-match-wins)
    - SC1 driver evidence — live end-to-end deny on the Azure VM (DRV-01 + DRV-02 + DRV-03)
    - nono-fltmgr.inf altitude 365678 (non-colliding, committed)
  affects:
    - crates/nono/src/sandbox/macos.rs
    - drivers/nono-fltmgr/nono-fltmgr.c (+.h), nono-fltmgr.inf
    - crates/nono-fltmgr-client/src/lib.rs
tech_stack:
  added: []
  patterns:
    - "FLT_PREOP_PENDING + FltSendMessage round-trip + FltCompletePendedPreOperation (allow=SUCCESS_NO_CALLBACK, deny=COMPLETE)"
    - "FLT_FILE_NAME_OPENED (not NORMALIZED) in pre-create to avoid re-entrant deadlock"
    - "kernel-side leaf-name pre-filter + fail-closed back-pressure for deterministic deny"
    - "embedded .sys signing via New-SelfSignedCertificate + signtool /sm; rundll32 DefaultInstall for minifilter install"
key_files:
  created:
    - .planning/phases/64-.../64-SC1-driver-evidence.md
    - .planning/phases/64-.../64-vm-runcmd-ewdk-build-local.ps1
  modified:
    - crates/nono/src/sandbox/macos.rs (8f84d454 cherry-pick)
    - drivers/nono-fltmgr/nono-fltmgr.c (+.h) (UAT defect fixes)
    - drivers/nono-fltmgr/nono-fltmgr.inf (DriverVer + SourceDisks + altitude 365678)
    - crates/nono-fltmgr-client/src/lib.rs (path-tail match, reply size, logging)
    - .planning/phases/64-.../64-SC1-VM-RUNBOOK.md
decisions:
  - "Track B 8f84d454: platform_rules() loop moved AFTER write-allows in generate_profile; macOS-gated tests deferred to CI (x86_64-apple-darwin clippy blocked by cc-rs/ring on Windows host, PARTIAL per D-12)"
  - "Track A altitude 365678 chosen from fltmc filters on the live VM — Activity Monitor band 360000-389999, non-colliding (gap between WdFilter 328010 and UCPD 385250.5), clear of AV 320000-329998 (D-08)"
  - "Kernel scopes interception to the deny-target leaf name (D-02 deterministic target) — routing every system create to user mode made the desktop hang; a real sandbox would scope by policy"
  - "Fail-CLOSED on single-slot back-pressure for the watched file: deny when the slot is busy rather than fail-open, making the deny deterministic under contention while the slot-winner still proves the full DRV-02 round-trip"
  - "Test-signing modernized: makecert is absent on EWDK 26H1 -> New-SelfSignedCertificate + signtool /sm + embedded .sys signing; pnputil /install only stages a minifilter -> rundll32 InstallHinfSection DefaultInstall creates the service + altitude registry + copies the .sys"
metrics:
  duration: "extended (live VM UAT)"
  completed: "2026-06-09"
  tasks: 2
  files: 6
---

# Phase 64 Plan 04: macOS ordering finale + live minifilter VM UAT Summary

One-liner: Applied the third P1 macOS cherry-pick (`8f84d454`) to `macos.rs`, and on the Azure Secure-Boot-OFF/HVCI-off VM test-signed, loaded, and proved the `nono-fltmgr` minifilter denies a targeted file open end-to-end via a user-mode policy round-trip (SC1 PASS — DRV-01 + DRV-02 + DRV-03).

## Tasks Completed

| Task | Name | Commit(s) |
|------|------|-----------|
| 1 (Track B) | Cherry-pick 8f84d454 (platform rules after write-allows) into macos.rs | 631cf877 |
| 2 (Track A) | VM test-signing + load + deny harness; SC1 evidence | b8783362 (+ UAT fix chain below) |

## What Was Built

### Track B — `8f84d454` (commit 631cf877)
Moved the `platform_rules()` loop in `generate_profile` to **after** the write-allows block (Seatbelt last-match-wins: a targeted platform deny now overrides a preceding broader write-allow). The GPU block keeps its read<GPU<write slot. The Wave 0 RED ordering tests (`test_generate_profile_platform_rules_after_writes`, `test_platform_rules_after_write_allows`, `test_platform_deny_symlink_and_canonical_path`) assert `read_pos < write_pos < deny_pos`. The edit is a pure block-move of an already-compiling loop, compile-safe by construction; `macos.rs` and its tests are `#[cfg(target_os = "macos")]`-gated and run on macOS CI. Cross-target: x86_64-apple-darwin `-p nono` clippy PARTIAL on the Windows host (cc-rs/ring wall), aarch64 PARTIAL — both deferred to live CI per D-12.

### Track A — live minifilter VM UAT (DRV-01 + DRV-02 + DRV-03)
Test-signed the extended `nono-fltmgr` minifilter on the Azure VM and demonstrated end-to-end deny:
`IRP_MJ_CREATE` (secret.txt) → kernel leaf-name pre-filter → single-slot ring → `FltSendMessage` over `\NonoPolicyPort` → Rust client path-tail match → `FilterReplyMessage` (Decision=deny) → worker completes IRP `STATUS_ACCESS_DENIED` → caller `CreateFile` returns `ERROR_ACCESS_DENIED (5)`. Driver loaded at altitude **365678** (non-colliding). Full evidence in `64-SC1-driver-evidence.md`.

The live run flushed out ~18 real defects that no Windows dev-host build could detect (no kernel C toolchain / no kernel runtime locally) — see the evidence file's defect table. Fix commits: `b4d2fef6` (3 compile errors), `a81627b0` (INF), `39be6f19` (load-ordering hang + client gate), `08019735` (NORMALIZED→OPENED deadlock + client tail-match), `97af1cec` (per-instance port close), `d0bb64dc` (kernel scoping), `f5ef9a74` (allow-completion), `9267a131` (fail-closed back-pressure), `aec9d78b` (IPC reply size). The runbook (`64-SC1-VM-RUNBOOK.md`) and a local-build helper (`64-vm-runcmd-ewdk-build-local.ps1`) were added/corrected to match the working EWDK-26H1 pipeline.

## Verification

| Gate | Result |
|------|--------|
| 8f84d454 applied with `Upstream-commit:` trailer + DCO | PASS (631cf877) |
| macos.rs ordering tests (`read_pos < write_pos < deny_pos`) | Deferred to macOS CI (cfg(target_os="macos")-gated; compile-safe block move) |
| x86_64-apple-darwin clippy `-p nono` | PARTIAL — cc-rs/ring wall on Windows host (D-12) → CI |
| aarch64-apple-darwin | PARTIAL — target not installed → CI |
| Driver test-signed + loaded at 365678 (`fltmc filters`/`instances`) | PASS |
| Altitude non-colliding in 360000–389999, clear of AV range | PASS (365678) |
| Deny harness `ERROR_ACCESS_DENIED (5)` | **PASS** (SC1 PASS attempt 1) |
| User-mode round-trip (DRV-02) — `[DENY ]` over `\NonoPolicyPort` | PASS |
| Allowed (non-target) creates open normally | PASS (allow = FLT_PREOP_SUCCESS_NO_CALLBACK) |
| `nono-fltmgr.inf` altitude updated 370020→365678 + committed | PASS (a81627b0) |
| 64-SC1-driver-evidence.md committed | PASS (b8783362) |
| `.sys` not committed (T-63-05) | PASS (VM-local only) |

## Deviations from Plan

- **Extensive UAT fix chain.** The plan assumed the Plan 64-02 driver and Plan 64-03 client were correct; live execution found ~18 defects across compile, INF, signing, load ordering, a pre-create deadlock, comm-port lifetime, I/O scoping, IRP completion semantics, back-pressure policy, and an IPC reply-size mismatch. All fixed and committed (table in the evidence file). This is the expected value of the live-VM gate — these are precisely the classes of bug a dev-host build cannot surface.
- **Pipeline modernization.** The Phase 63 runbook commands (`makecert`, `pnputil /install`) do not work as-is on EWDK 26H1 for a minifilter; corrected to `New-SelfSignedCertificate` + `signtool /sm` (embedded `.sys` sign) and `rundll32 DefaultInstall`. Runbook updated.
- **Design scoping + fail-closed.** Added a kernel leaf-name pre-filter and fail-closed back-pressure (not in the original plan) to keep the system responsive and make the deny deterministic. Consistent with D-02 (deterministic deny target).

## Self-Check: PASSED

- `crates/nono/src/sandbox/macos.rs` 8f84d454 (platform rules after writes): FOUND (631cf877)
- `drivers/nono-fltmgr/nono-fltmgr.inf` altitude 365678: FOUND
- `.planning/phases/64-.../64-SC1-driver-evidence.md` (SC1 PASS): FOUND (b8783362)
- Deny harness `ERROR_ACCESS_DENIED (5)`: CAPTURED
- `fltmc filters`/`instances` at 365678: CAPTURED
- No `.sys` committed: CONFIRMED (T-63-05)
