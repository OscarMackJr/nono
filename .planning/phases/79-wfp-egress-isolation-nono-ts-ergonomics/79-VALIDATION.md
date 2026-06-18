---
phase: 79
slug: wfp-egress-isolation-nono-ts-ergonomics
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-06-18
---

# Phase 79 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

This phase has two independent deliverables that exercise two distinct test surfaces:
- **WFP-01** — an unattended PowerShell gate run via the dark-factory harness (`scripts/verify-dark.ps1 --gate wfp-egress-isolation`). Host-gated: requires a live Win11 host with `nono-wfp-service` running (else SKIP_HOST_UNAVAILABLE).
- **TSRG-01** — a napi integration test in the sibling `nono-ts` repo (`npm test`), plus Rust unit/clippy gates in this repo for the `policy.json` profile additions and the `windows_confined_run.rs` wiring.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework (Rust)** | Cargo test runner (existing workspace) |
| **Framework (nono-ts)** | `node tests/*.js` (no jest/vitest; plain node scripts) |
| **Gate runner** | `scripts/verify-dark.ps1` (PASS=0 / FAIL=2 / SKIP_HOST_UNAVAILABLE=3 / HARNESS_ERROR=4) |
| **Config file** | none — profiles embedded via `crates/nono-cli/build.rs` from `data/policy.json` |
| **Quick run command (Rust)** | `cargo test -p nono-cli` |
| **Quick run command (gate)** | `pwsh scripts/verify-dark.ps1 --gate wfp-egress-isolation` |
| **Quick run command (nono-ts)** | `cd C:\Users\OMack\nono-ts && napi build --platform --release && npm test` |
| **Full suite command** | `cargo test --workspace --all-targets --all-features` |
| **Cross-target lint** | `cargo clippy --workspace --all-targets --all-features -- -D warnings -D clippy::unwrap_used` (+ `--target x86_64-unknown-linux-gnu` / `--target x86_64-apple-darwin` per CLAUDE.md if Unix-cfg code is touched) |
| **Estimated runtime** | gate ~30s; nono-ts build+test ~60s; Rust unit ~30s |

---

## Sampling Rate

- **After every task commit (nono-cli):** Run `cargo clippy --workspace --all-targets --all-features -- -D warnings -D clippy::unwrap_used` + `cargo test -p nono-cli`
- **After every task commit (nono-ts):** Run `napi build --platform --release && npm test` on the Win11 host
- **After every plan wave:** Run `cargo test --workspace --all-targets --all-features`
- **Before `/gsd:verify-work`:** `scripts/verify-dark.ps1 --gate wfp-egress-isolation` PASS (or SKIP_HOST_UNAVAILABLE on a host without the WFP service) AND `npm test` green
- **Max feedback latency:** ~60 seconds (napi rebuild dominates)

---

## Per-Task Verification Map

> Task IDs are illustrative — the planner finalizes plan/task numbering. Every requirement maps to an automated command; the two integration gates are host-gated on Win11.

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 79-01-xx | 01 (WFP-01) | 1 | WFP-01 | T-79-01 (false PASS via vacuous WFP-service-down) | block:true test profile installs per-SID WFP deny; absent WFP service → SKIP not PASS | unit | `cargo test -p nono-cli` (profile parse) + `cargo clippy ...` | ❌ W0 | ⬜ pending |
| 79-01-xx | 01 (WFP-01) | 2 | WFP-01 | T-79-02 (SID collision) | two concurrent agents get distinct AppContainer SIDs; A allowed, B denied | integration (gate) | `pwsh scripts/verify-dark.ps1 --gate wfp-egress-isolation` | ❌ W0 | ⬜ pending |
| 79-02-xx | 02 (TSRG-01) | 1 | TSRG-01 | T-79-03 (WriteRestricted reached when broker arm intended → 0xC0000142 DoS) | `nono-ts-default` profile sets `windows_low_il_broker:true`; default injected before validation guard | unit | `cargo test -p nono-cli` (profile parse) + `cargo clippy ...` | ❌ W0 | ⬜ pending |
| 79-02-xx | 02 (TSRG-01) | 2 | TSRG-01 | T-79-04 (auto-cover grants over-broad path) | auto-cover adds ONLY resolved target exe-dir; no cwd/ancestors | integration (napi) | `cd C:\Users\OMack\nono-ts && napi build --platform --release && npm test` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `scripts/gates/wfp-egress-isolation.ps1` — new gate (WFP-01); follows `Test-Precondition`/`Invoke-Gate` contract; precondition probes `\\.\pipe\nono-wfp-control` → SKIP_HOST_UNAVAILABLE if absent
- [ ] `crates/nono-cli/data/policy.json` — 3 new profiles: `nono-ts-wfp-test-open` (block:false), `nono-ts-wfp-test-blocked` (block:true), `nono-ts-default` (broker-arm default)
- [ ] `C:\Users\OMack\nono-ts\tests\test_confined_run_default.js` — new napi integration test (TSRG-01 / SC4)
- [ ] `C:\Users\OMack\nono-ts\package.json` — `"test"` script wired to run the new integration test
- [ ] **MANDATORY verification spike (research A3 / OQ-1):** confirm `nono run --profile <block:true>` on the non-daemon Windows path actually calls `wfp_filter_add` (same shipped machinery). If it does NOT, the gate must route the two agents through the daemon `launch_agent` path (`nono-agentd` running) instead of direct `nono run`. The gate's validity depends on this.

*Existing infrastructure (verify-dark harness, cargo test, npm) covers the rest.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| None — both deliverables have unattended automated gates | WFP-01 / TSRG-01 | Dark-factory mandate: WFP isolation proof and confinedRun path are both machine-verifiable | n/a |

*Note: both gates are HOST-gated (Win11 + nono-wfp-service / Node + napi-rs). On a non-Windows or service-absent host they emit SKIP_HOST_UNAVAILABLE, which is a valid unattended verdict, not a manual step.*

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 60s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
