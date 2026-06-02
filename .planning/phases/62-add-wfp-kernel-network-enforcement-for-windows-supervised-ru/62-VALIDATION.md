---
phase: 62
slug: add-wfp-kernel-network-enforcement-for-windows-supervised-ru
status: ready
nyquist_compliant: true
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
| 62-01-T1 | 62-01 | 1 | REQ-WFP-01 | T-62-01 (fail-open pass-through) | A `network.block:true` run with the service stopped attempts an auto-start; succeeds → enforce; fails → fail-closed error naming the remediation command; never `Ok(None)` | unit (TDD) | `cargo test -p nono-cli --lib network -- test_wfp_autostart` | ❌ W0 | ⬜ pending |
| 62-01-T2 | 62-01 | 1 | REQ-WFP-01 | — | `build_wfp_service_create_args` registers `start=auto` (manual path no longer reverts MSI posture); stale description fixed | unit | `cargo test -p nono-cli --lib network` | ✅ | ⬜ pending |
| 62-02-T1 | 62-02 | 1 | REQ-WFP-01 | T-62-SDDL (non-elevated denied / over-grant) | Control-pipe `PIPE_SDDL` grants Interactive Users connect (read+write); SYSTEM/Admin ACEs unchanged | unit (TDD) | `cargo test -p nono-cli -- pipe_sddl` | ❌ W0 | ⬜ pending |
| 62-02-T2 | 62-02 | 1 | REQ-WFP-01 | T-62-UNINST (uninstall residue) | Machine MSI `ServiceInstall Start="auto"`; `ServiceControl Start="install"`/Remove unchanged; user MSI untouched | source/file | `powershell` Start="auto" content assertion | ✅ | ⬜ pending |
| 62-03-T1 | 62-03 | 1 | REQ-WFP-01 | T-62-PA | REQ-WFP-01 present in REQUIREMENTS.md (v2.9-track section, not deferred) | source/file | `powershell` REQ-WFP-01 grep REQUIREMENTS.md | ✅ | ⬜ pending |
| 62-03-T2 | 62-03 | 1 | REQ-WFP-01 | T-62-PA | ROADMAP Phase 62 Requirements + 4-plan list + progress row | source/file | `powershell` REQ-WFP-01 grep ROADMAP.md | ✅ | ⬜ pending |
| 62-04-T6 | 62-04 | 2 | REQ-WFP-01 | — | HUMAN-UAT record (SC1–SC5) captured with PASS/FAIL verdicts | doc | `powershell` PASS-count assertion on 62-HUMAN-UAT.md | ✅ | ⬜ pending |

*62-04 Tasks 1–5 are `checkpoint:human-verify` operator steps (no automated command by design — see Manual-Only Verifications). Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

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

- [x] All tasks have `<automated>` verify or Wave 0 dependencies (62-04 T1–T5 are human-verify checkpoints, exempt by type)
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references (62-01-T1 D-03 decision-path tests; 62-02-T1 SDDL test)
- [x] No watch-mode flags
- [x] Feedback latency < 120s
- [x] Human-UAT instructions captured for the 3 manual-only behaviors above (62-04 checkpoint tasks)
- [x] `nyquist_compliant: true` set in frontmatter

*`wave_0_complete` stays false until the RED→GREEN Wave 0 tests (62-01-T1, 62-02-T1) are written and pass during execution.*

**Approval:** approved 2026-06-02
