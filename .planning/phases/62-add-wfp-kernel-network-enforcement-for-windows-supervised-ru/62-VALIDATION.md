---
phase: 62
slug: add-wfp-kernel-network-enforcement-for-windows-supervised-ru
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-06-02
---

# Phase 62 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test runner (`cargo test`) |
| **Config file** | none — workspace `Cargo.toml` |
| **Quick run command** | `cargo test -p nono-cli --lib network` |
| **Full suite command** | `make test-cli` (or `cargo test -p nono-cli`) |
| **Estimated runtime** | ~60–120 seconds |

> NOTE: This is a Windows-only (`#[cfg(windows)]` / `target_os = "windows"`) phase. Per CLAUDE.md the
> cross-target clippy rule applies — but these surfaces are Windows-only and will NOT compile under the
> Unix cross-targets, so the relevant verification REQ is the Windows-host `cargo clippy -p nono-cli` /
> `cargo test -p nono-cli`. Confirm no shared (non-cfg-gated) code is touched; if it is, run the Unix
> cross-target clippy per `.planning/templates/cross-target-verify-checklist.md`.

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p nono-cli --lib network`
- **After every plan wave:** Run `make test-cli`
- **Before `/gsd:verify-work`:** Full suite must be green AND human-UAT (machine-MSI, real Win11 host) executed
- **Max feedback latency:** ~120 seconds (automated); human-UAT is out-of-band

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 62-XX-XX | TBD | TBD | REQ-WFP-01 | T-62-01 (fail-open pass-through) | A `network.block:true` run never proceeds unenforced; service-not-running → start attempt → enforce, else fail-closed | unit | `cargo test -p nono-cli --lib network` | ❌ W0 | ⬜ pending |

*Filled concretely by the planner per task. Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] Unit tests for the D-03 "ensure service running, attempt start, else fail-closed" decision path — using the injectable-runner pattern (`install_wfp_network_backend_with_runner` + a new `start_service_fn`) so the decision logic is testable WITHOUT elevation or a running service.
- [ ] Test asserting the fail-closed error names the exact elevated remediation command.

*If the injectable-runner seam already exists, extend it rather than adding new fixtures.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Out-of-box enforced block on a real host | REQ-WFP-01 | Requires elevated machine-MSI install + real WFP kernel filters + a real outbound connection from a confined child; cannot run in CI | On a real Win11 host: install machine MSI, run a supervised `nono run` on the runner profile with `network.block:true` WITHOUT a prior `nono setup --start-wfp-service`; confirm the confined child's outbound network is denied while explicitly-allowed ports still pass. |
| Boot-start posture survives reboot | REQ-WFP-01 (D-01) | `ServiceControl Start="install"` masks `ServiceInstall Start=` until reboot | After machine-MSI install, reboot the host, then (without elevation) run the supervised `network.block:true` scenario; service must already be running. |
| Clean uninstall leaves nothing | REQ-DRN-01 (regression) | `start=auto` must not regress the Phase 53 leave-nothing invariant | `sc stop nono-wfp-service` + `msiexec /x` the machine MSI; verify service is fully removed (no orphaned registration). |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 120s
- [ ] Human-UAT instructions captured for the 3 manual-only behaviors above
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
