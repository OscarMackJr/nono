---
phase: 65
slug: minifilter-adr-macos-live-re-validation
status: approved
nyquist_compliant: true
wave_0_complete: false
created: 2026-06-09
---

# Phase 65 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test runner (`cargo test`) for `sandbox::macos`; live CLI assertions for the deny UAT; VM deny-harness re-run for driver latency |
| **Config file** | none — workspace `Cargo.toml` |
| **Quick run command** | `cargo test -p nono sandbox::macos` |
| **Full suite command** | `make test-lib` (`cargo test -p nono`) on a macOS host; `cargo test --workspace` in CI `macos-latest` |
| **Estimated runtime** | ~30 seconds (lib tests); latency re-run + live UAT are host/VM-gated and out-of-band |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p nono` (macOS-gated tests compile-skip on the Windows dev host, but the contract is in-tree)
- **After every plan wave:** Green `macos-latest` CI run
- **Before `/gsd:verify-work`:** Green `macos-latest` CI SHA captured as evidence (D-11c) + gate-65-A HUMAN-UAT checklist all-pass
- **Max feedback latency:** ~30 seconds (local lib tests); CI leg async

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 65-DRV-instrument | DRV | 1 | DRV-04 | T-63-01 / T-63-02 | QPC instrumentation adds no `Zw/Nt` I/O, preserves 500ms finite timeout, stays kernel-internal (ring entry, not the 532-byte wire struct) | manual | VM deny-harness re-run (runbook §9) — median+p99 over ~100 denied creates, both spans | ❌ W0 (new code; harness exists) | ⬜ pending |
| 65-DRV-adr | DRV | 2 | DRV-04 | — | go/no-go verdict surfaced for human-review gate (D-06), not silently locked | manual | ADR review against the six DRV-04 topics | ✅ (`.planning/architecture/`) | ⬜ pending |
| 65-MAC-ordering | MAC | 1 | MACOS-03 | T (deny-ordering regression) | deny-after-allow emission order (read<write<deny), last-match-wins | unit | `cargo test -p nono sandbox::macos::tests::test_generate_profile_platform_rules_after_writes` | ✅ `macos.rs` | ⬜ pending |
| 65-MAC-ci | MAC | 1 | MACOS-03 | T (cross-target drift) | macOS build+test+clippy green; no broken cfg-gated code reaches a tag | CI | `macos-latest` `test` + clippy jobs in `ci.yml` (capture SHA) | ✅ (capture SHA) | ⬜ pending |
| 65-MAC-uat | MAC | 2 | MACOS-03 | V4 access control | `sandbox_init()` blocks SSH key + `/etc/hosts` + `/private/etc/hosts`; dry-run shows deny-after-allow | manual HUMAN-UAT | the four live deny assertions (gate 65-A) | ✅ (CLI exists; host-gated) | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- None for the macOS ordering tests — they already exist in `crates/nono/src/sandbox/macos.rs` (`test_generate_profile_platform_rules_after_writes` and siblings).
- Net-new is the driver instrumentation code (the DRV-04 deliverable, not a test gap) and the latency-appendix file.

*Existing infrastructure covers all automatable phase requirements.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| `FLT_PREOP_PENDING` round-trip latency (both spans, median+p99 over ~100 denied creates) | DRV-04 | Requires a test-signed driver loaded on the spike VM (`nono-fltmgr-vm`, rg `rg-nono-fltmgr-spike`); cannot run in CI | Re-run the deny harness per `64-SC1-VM-RUNBOOK.md` §9 after rebuild/re-sign/reload; idempotent VM reuse-or-recreate |
| `sandbox_init()` live deny assertions (gate 65-A) | MACOS-03 | No macOS host available in CI for a real `sandbox_init()` run; host-gated | On a real macOS host: `nono run --dry-run --profile claude-code` shows deny-after-allow; `nono run --profile claude-code -- cat ~/.ssh/id_rsa` blocked; both `/etc/hosts` and `/private/etc/hosts` blocked; `make test-lib` green |
| go/no-go recommendation review | DRV-04 | Final verdict is a human-review gate (D-06) — Oscar reviews before final | Operator reviews ADR recommendation; not auto-locked |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 / manual-gate dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify (manual gates explicitly justified above)
- [ ] Wave 0 covers all MISSING references (none — ordering tests in-tree)
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s (local lib tests)
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved 2026-06-09
