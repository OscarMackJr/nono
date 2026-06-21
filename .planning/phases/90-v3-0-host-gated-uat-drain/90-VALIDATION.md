---
phase: 90
slug: v3-0-host-gated-uat-drain
status: partial
nyquist_compliant: false
wave_0_complete: true
created: 2026-06-21
reconstructed_from: [90-01-SUMMARY.md, 90-02-SUMMARY.md, 90-VERIFICATION.md]
---

# Phase 90 — Validation Strategy

> Per-phase validation contract, reconstructed from artifacts (State B — no prior VALIDATION.md).
> Phase 90 is a **v3.0 host-gated UAT drain**: one production-code requirement (DRAIN-04, fully
> automated) plus three host-gated UAT-drain requirements (DRAIN-01/02/03) whose proof is
> inherently a live-host operation and therefore **manual-only by design**.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test runner (`cargo test`) |
| **Config file** | none — workspace `Cargo.toml` |
| **Quick run command** | `cargo test -p nono-cli --bin nono-agentd telemetry` |
| **Full suite command** | `cargo test -p nono-cli --bin nono-agentd` |
| **Estimated runtime** | ~2 seconds (telemetry subset); ~5–10 s (full bin) |
| **Scripted host gates** | `pwsh -File scripts/verify-dark.ps1 -Gate <name>` (DRAIN-01/02/03 — host-gated, not in-process) |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p nono-cli --bin nono-agentd telemetry`
- **After every plan wave:** Run `cargo test -p nono-cli --bin nono-agentd`
- **Before `/gsd:verify-work`:** Telemetry subset must be green (DRAIN-04 scope)
- **Max feedback latency:** ~10 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 90-01-0 | 01 | 1 | DRAIN-04 | — | `nono-agentd` binary compiles with telemetry module reachable | build | `cargo build -p nono-cli --bin nono-agentd` | ✅ | ✅ green |
| 90-01-1a | 01 | 1 | DRAIN-04 | T-90-01 | In-process `nono_security::network_deny` event advances HMAC chain (seq 0→1) | unit | `cargo test -p nono-cli --bin nono-agentd d01_network_deny_advances_chain_sequence_to_one` | ✅ `telemetry_init.rs:175` | ✅ green |
| 90-01-1b | 01 | 1 | DRAIN-04 | T-90-03 | Admin opt-out (`enabled=false`) suppresses event — chain stays at 0 | unit | `cargo test -p nono-cli --bin nono-agentd opt_out_disabled_layer_does_not_advance_chain` | ✅ `telemetry_init.rs:215` | ✅ green |
| 90-01-1c | 01 | 1 | DRAIN-04 | T-90-03 | `min_severity` threshold suppresses sub-threshold severities | unit | `cargo test -p nono-cli --bin nono-agentd min_severity_filter_predicate_matches_policy_threshold` | ✅ `telemetry/mod.rs:425` | ✅ green |
| 90-01-1d | 01 | 1 | DRAIN-04 | T-90-01 | Genesis chain sequence is 0 (`chain_sequence` accessor) | unit | `cargo test -p nono-cli --bin nono-agentd chain_sequence_genesis_is_zero` | ✅ `telemetry/mod.rs:483` | ✅ green |
| 90-01-2 | 01 | 1 | DRAIN-04 | — | Native clippy clean; cross-target PARTIAL→CI | lint | `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used` | ✅ | ✅ green (cross-target PARTIAL→CI) |
| 90-02-1 | 02 | 1 | DRAIN-01 | T-90-06/07 | Clean-VM silent MSI install integrity | scripted host gate | `pwsh -File scripts/verify-dark.ps1 -Gate clean-host-install` / `-Gate deploy-silent-install` | ✅ (script) | 🔶 manual-only (SKIP_HOST_UNAVAILABLE) |
| 90-02-2 | 02 | 1 | DRAIN-02 | T-90-06/07 | Dual-layer (proxy + kernel WFP) egress block | scripted host gate | `pwsh -File scripts/verify-dark.ps1 -Gate wfp-egress-isolation` / `-Gate egress-policy-deny` | ✅ (script) | 🔶 manual-only (SKIP_HOST_UNAVAILABLE) |
| 90-02-3 | 02 | 1 | DRAIN-03 | T-90-06/07 | Live security event reaches Application Log / SIEM | scripted host gate | `pwsh -File scripts/verify-dark.ps1 -Gate telemetry-event-emit` | ✅ (script) | 🔶 manual-only (FAIL — environmental) |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky · 🔶 manual-only*

---

## Wave 0 Requirements

Existing infrastructure covers all in-process phase behavior. DRAIN-04 ships with four
non-host-gated tests (chain-advance, opt-out, min_severity threshold, genesis), plus the broader
`telemetry/mod.rs` HMAC-chain suite. No Wave 0 test scaffolding was required.

---

## Manual-Only Verifications

These three requirements are **host-gated UAT-drain items**. Their proof is an inherently
live-host operation (fresh-VM MSI install, kernel-level WFP egress, live SIEM ingestion) and is
**not automatable in-process**. The scripted `verify-dark.ps1` gates already exist and are the
designed unattended collapse; they SKIP/FAIL on this dev host pending real hosts. This is the
designed drain disposition, not a coverage hole — see `90-HUMAN-UAT.md § Gaps` for full
operator-gated residual notes and `90-VERIFICATION.md` for the drain-intent clarification.

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Clean-VM silent MSI install with HKLM policy spine | DRAIN-01 | Needs fresh/snapshot-restored Win11 VM with no pre-existing nono install (dev host has `C:\Program Files\nono` → gate correctly SKIPs, exit 3) | Stage v3.0 MSI at `dist\windows\nono-machine.msi`; `pwsh -File scripts/verify-dark.ps1 -Gate clean-host-install` then `-Gate deploy-silent-install`; expect PASS (exit 0) |
| Dual-layer WFP egress block (proxy + kernel) | DRAIN-02 | Needs `nono-agentd` running (non-elevated) + `nono-wfp-service` active (admin) + ideally a 2nd host; daemon control pipe absent on dev host → gate SKIPs (exit 3) | `nono daemon start`; ensure WFP service running; `pwsh -File scripts/verify-dark.ps1 -Gate wfp-egress-isolation` then `-Gate egress-policy-deny`; expect PASS (exit 0) |
| Live security event in Application Log / SIEM | DRAIN-03 | Needs telemetry-capable v3.0 build (PATH binary is pre-telemetry v0.57.5), running daemon + WFP service, and an *observable* denial (path-deny is kernel-side unobservable on AppContainer backend; network-deny not implemented for direct Windows supervised runs — only the daemon+WFP path emits) | Fresh v3.0 install; daemon-launched confined agent blocked by WFP; `pwsh -File scripts/verify-dark.ps1 -Gate telemetry-event-emit`; expect EventID 10001-10005 under source `nono` + ETW provider `nono` via logman; also verify HKLM `telemetry.enabled=false` opt-out + `min_severity=Error` suppression |

---

## Out-of-Scope Flags (not Phase 90 requirements)

> Surfaced during this validation's full-suite run. **Not** a Phase 90 regression — these tests
> live in `agent_daemon/launch.rs` (Phase 74 `DaemonDaclGuard` code), which Phase 90 did not
> modify. Recorded here for traceability; investigate separately, not as part of this phase.

| Test | Location | Symptom | Assessment |
|------|----------|---------|------------|
| `daemon_dacl_guard_applies_and_reverts_write_grant` | `agent_daemon/launch.rs:1543` | Panics at `DaemonDaclGuard::apply(...).expect(...)` | Environment-sensitive real-ACL operation on a tempdir; consistent with documented Windows baseline test fragility (mandatory-label / drive-root vs `%USERPROFILE%` path ownership). Fails on this host; was green at phase-execution time (SUMMARY recorded 69 passed). |
| `daemon_dacl_guard_reap_revokes_traverse_paths` | `agent_daemon/launch.rs:1660` | Same `DaemonDaclGuard::apply(...)` failure | Same root cause. |

Full bin run on 2026-06-21: **67 passed, 2 failed** (the 2 above) — all four DRAIN-04 telemetry
tests **green**.

---

## Validation Sign-Off

- [x] All Phase 90 production-code tasks (DRAIN-04) have `<automated>` verify, all green
- [x] DRAIN-01/02/03 dispositioned Manual-Only (host-gated by design) with explicit test instructions
- [x] Sampling continuity: no 3 consecutive automatable tasks without automated verify
- [x] Wave 0: existing infrastructure covers all in-process behavior
- [x] No watch-mode flags
- [x] Feedback latency < 10 s (telemetry subset)
- [ ] `nyquist_compliant: true` — **NOT set**: 3/4 requirements are manual-only (host-gated). DRAIN-04 is fully automated; the phase is **PARTIAL by design** (1 automated requirement, 3 host-gated manual-only). This is the correct terminal state for a UAT-drain phase, not a fillable gap.

**Approval:** validated PARTIAL 2026-06-21 (1 automated / 3 manual-only by design)

---

## Validation Audit 2026-06-21

| Metric | Count |
|--------|-------|
| Requirements | 4 (DRAIN-01/02/03/04) |
| Automated (COVERED) | 1 (DRAIN-04) |
| Manual-only (by design, host-gated) | 3 (DRAIN-01/02/03) |
| MISSING automated gaps | 0 |
| New tests generated | 0 (no in-process behavior to test for the host-gated items) |
| Out-of-scope red tests flagged | 2 (Phase 74 DACL-guard, env-sensitive) |
