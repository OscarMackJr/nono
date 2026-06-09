---
status: passed
phase: 64-minifilter-spike-implementation-macos-p1-cherry-pick-wave
verified: 2026-06-09
method: inline (orchestrator) — live SC1 UAT + build/test gates
requirements: [DRV-01, DRV-02, DRV-03, MACOS-02]
---

# Phase 64 Verification

**Verdict: PASSED.** Phase goal — implement the Windows minifilter spike (DRV-01/02/03)
and absorb the third P1 macOS Seatbelt ordering cherry-pick (MACOS-02) — achieved and
proven by live execution on the Azure test VM.

## Requirement traceability

| Req | Goal | Evidence | Status |
|-----|------|----------|--------|
| DRV-01 | Test-signed minifilter denies a targeted file open end-to-end on a Secure-Boot-OFF/HVCI-off VM | `64-SC1-driver-evidence.md`: harness `ERROR_ACCESS_DENIED (5)`; `fltmc filters/instances` at altitude 365678 | ✅ |
| DRV-02 | Minifilter does a user-mode policy round-trip over `\NonoPolicyPort`; Rust `#[cfg(windows)]` client receives + replies | `[DENY ]` logged by `nono_fltmgr_client.exe` via FltSendMessage/FilterGetMessage/FilterReplyMessage; driver completes IRP from the reply | ✅ |
| DRV-03 | Full test-signing pipeline executed (sign → install → load) | EWDK-26H1 pipeline run live (New-SelfSignedCertificate → inf2cat → signtool /sm → rundll32 DefaultInstall → fltmc load); documented in `drivers/README.md` + runbook | ✅ |
| MACOS-02 | Absorb P1 macOS cherry-picks (8f1b0b74, 362ada22, 8f84d454) with Upstream-commit trailers; ordering tests assert deny-after-allow | 8f1b0b74+362ada22 in `sandbox_prepare.rs` (Plan 64-03), 8f84d454 in `macos.rs` (Plan 64-04, commit 631cf877); ordering tests assert read<write<deny (macOS-gated → CI) | ✅ |

## Must-haves

- All 5 plans have committed SUMMARYs (64-01..64-05). ✅
- 8f84d454 applied with verbatim `Upstream-commit:` trailer + DCO. ✅
- INF altitude 370020 → 365678 (non-colliding, committed). ✅
- `drivers/README.md` documents both pipelines + `nono-wfp-driver.sys`-untouched note. ✅
- `.sys` not committed (VM-local, T-63-05). ✅ (`git ls-files drivers/nono-fltmgr/*.sys` empty)

## Gates

- `cargo build --workspace`: PASS. `cargo build -p nono-fltmgr-client`: PASS.
- `cargo test -p nono` / `-p nono-cli`: only the 5 documented pre-existing Windows baseline
  failures (`try_set_mandatory_label`, `profile_cmd` init, 3× `protected_paths`) — present at
  the phase-base commit, NOT regressions.
- macOS ordering tests + apple-darwin clippy: cfg-gated / cc-rs(ring)-blocked on the Windows
  host → PARTIAL, deferred to live CI per D-12.

## Human-verification / follow-ups (non-blocking)

- macOS CI must run the Seatbelt ordering tests (`sandbox::macos`) and apple-darwin clippy
  (x86_64 + aarch64) — deferred from the Windows dev host (D-12).
- Official Microsoft minifilter altitude (request to fsfcomm@microsoft.com) pending; spike
  uses the temporary non-colliding 365678.
- The ~18 UAT defect fixes (see `64-SC1-driver-evidence.md`) live in the driver/client source;
  a future re-run on a fresh VM should reproduce SC1 PASS from a clean stage.

**Phase 64 goal achieved.**
