---
phase: 80
slug: clean-host-install-uat
status: approved
nyquist_compliant: true
wave_0_complete: false
created: 2026-06-17
---

# Phase 80 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in (build change) + PowerShell (the gate + MSI contract) |
| **Config file** | `Cargo.toml` (Rust); none for the PS gate (self-contained) |
| **Quick run command** | `pwsh scripts/validate-windows-msi-contract.ps1 -BinaryPath <nono.exe> -BrokerPath <broker.exe> -ServiceBinaryPath <svc.exe> -DriverBinaryPath <sys>` |
| **Full suite command** | `pwsh scripts/verify-dark.ps1 --gate clean-host-install` (on a clean Win11 VM) |
| **Estimated runtime** | ~120 seconds (MSI install + version probe + uninstall on the clean VM); contract check ~5s |

---

## Sampling Rate

- **After every task commit:** Run `pwsh scripts/validate-windows-msi-contract.ps1 ...` (contract assertions) where the change touches the MSI; for the `.cargo/config.toml` change rely on `cargo build --release` succeeding.
- **After every plan wave:** Run the MSI contract validation + a local `build-windows-msi.ps1 -EmitOnly` (or full build) to confirm the `.wxs` still emits.
- **Before `/gsd:verify-work`:** `pwsh scripts/verify-dark.ps1 --gate clean-host-install` must emit a non-error verdict (PASS on a clean VM, or SKIP_HOST_UNAVAILABLE on the dirty dev host — both are non-FAIL).
- **Max feedback latency:** ~120 seconds (full gate on the VM); ~5 seconds for the contract assertion on the dev host.

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 80-01-xx | 01 | 1 | INST-01 | — | `+crt-static` applies to `x86_64-pc-windows-msvc` only; Linux/macOS linkage unchanged | compile | CI Clippy/Test on ubuntu-latest + macos-latest (no new test; existing CI proves no regression) | ✅ | ⬜ pending |
| 80-01-xx | 01 | 1 | INST-01 | T-80 (MSI path injection) | Service start failure is non-fatal (`vital="no"`); install does not roll back | unit/contract | `pwsh scripts/validate-windows-msi-contract.ps1 ...` (new `vital="no"` assertion) | ✅ (existing; needs new assertion) | ⬜ pending |
| 80-02-xx | 02 | 2 | INST-01 | T-80 (MSI/log path injection) | MSI install exits 0 (or 3010), `nono --version` runs from a NEW pwsh session; clean uninstall | integration/gate | `pwsh scripts/verify-dark.ps1 --gate clean-host-install` | ❌ W0 (new gate file) | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*
*Wave numbers are indicative — the planner sets authoritative waves in the PLAN.md frontmatter.*

---

## Wave 0 Requirements

- [ ] `scripts/gates/clean-host-install.ps1` — new gate file covering INST-01 (the main Phase 80 deliverable; `Test-Precondition` + `Invoke-Gate`)
- [ ] New `Assert-Equal` for `Vital="no"` on the **ServiceInstall** element (PascalCase — verified against the WiX v4 XSD; `ServiceControl` has no `Vital` attribute) in `scripts/validate-windows-msi-contract.ps1` — locks the D-04 contract
- [ ] Static-CRT wiring (D-03) across every clean-host-tested build path: `.cargo/config.toml` `[target.x86_64-pc-windows-msvc]` `rustflags = ["-C", "target-feature=+crt-static"]` (covers local + release.yml where no `RUSTFLAGS` env is set) PLUS appended `-C target-feature=+crt-static` in the Windows compile steps of `.github/workflows/ci.yml` and `.github/workflows/release.yml` (where the config-file rustflags would be silently dropped by an active `RUSTFLAGS` env — Cargo's flag sources are mutually exclusive, first-match-wins). Ground-truth proof: `dumpbin /imports nono.exe` shows no `vcruntime140.dll` import.

*Existing infrastructure (verify-dark.ps1 runner, reference gates, MSI contract validator) covers the rest.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Clean-host PASS verdict | INST-01 | Requires a deliberately-prepared fresh Win11 VM/snapshot (no prior nono) — the dark gate self-detects and SKIPs on the dirty dev host (D-01) | On a fresh Win11 VM: stage `dist\windows\nono-machine.msi`, run `pwsh scripts/verify-dark.ps1 --gate clean-host-install`, confirm verdict `PASS` and that `nono --version` printed |

*The clean-host PASS is "manual" only in that it needs operator-provided fresh-VM provisioning; the gate itself runs fully unattended once invoked on that VM.*

---

## Validation Sign-Off

- [ ] All tasks have automated verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references (new gate file, new contract assertion, `.cargo/config.toml`)
- [ ] No watch-mode flags
- [ ] Feedback latency < 120s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
