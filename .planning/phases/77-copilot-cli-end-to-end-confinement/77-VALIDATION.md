---
phase: 77
slug: copilot-cli-end-to-end-confinement
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-06-17
---

# Phase 77 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test runner (`cargo test`) + PowerShell gate (`scripts/verify-dark.ps1`) |
| **Config file** | none — workspace `Cargo.toml` + `Makefile` |
| **Quick run command** | `cargo test -p nono-cli` |
| **Full suite command** | `make ci` (clippy + fmt + tests) |
| **Estimated runtime** | ~120 seconds (host-gated AppContainer tests are `#[ignore]`-marked) |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p nono-cli`
- **After every plan wave:** Run `make ci`
- **Before `/gsd:verify-work`:** Full suite must be green; `scripts/verify-dark.ps1 --gate copilot-e2e` emits PASS or SKIP_HOST_UNAVAILABLE
- **Max feedback latency:** 120 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| {N}-01-01 | 01 | 1 | CPLT-01 | T-77-01 / — | ancestor-RA grant covers only user-ownable ancestors; stops at first non-owned (fail-secure) | unit | `cargo test -p nono-cli` | ❌ W0 | ⬜ pending |
| {N}-02-01 | 02 | — | CPLT-02 | T-77-02 / — | idempotent + non-destructive (adds one allow-ACE, alters/removes no existing or deny ACE) | unit | `cargo test -p nono-cli` | ❌ W0 | ⬜ pending |
| {N}-03-01 | 03 | — | CPLT-03 | T-77-03 / — | gate discriminates confinement FAIL from Copilot/auth/network SKIP_HOST_UNAVAILABLE | manual+script | `scripts/verify-dark.ps1 --gate copilot-e2e` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*
*Planner refines this map with concrete task IDs and per-task commands.*

---

## Wave 0 Requirements

- [ ] Rust unit tests for the ancestor-RA grant helper (mirror existing `dacl_guard.rs` guard tests) — CPLT-01
- [ ] Rust unit tests for the generic `nono setup --grant-ancestors` command: idempotency + non-destructive ACE assertions — CPLT-02
- [ ] `scripts/gates/copilot-e2e.ps1` gate file with `Test-Precondition` / `Invoke-Gate` (PASS/FAIL/SKIP_HOST_UNAVAILABLE) — CPLT-03

*Existing `cargo test` infrastructure covers the unit-testable surface; the gate is new PowerShell.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Copilot completes a real authenticated suggestion under AppContainer with zero STATUS_ACCESS_DENIED / Node module-resolution crash | CPLT-01, CPLT-03 | Requires a real Win11 host + installed `@github/copilot` + GitHub auth + network (host-gated by design) | Run one-time-admin setup, then `scripts/verify-dark.ps1 --gate copilot-e2e` — expect PASS on a provisioned host, SKIP_HOST_UNAVAILABLE otherwise |
| One-time-admin grant on `C:\` / `C:\Users` for `ALL APPLICATION PACKAGES` | CPLT-02 | Requires elevation; cannot be exercised in non-elevated unit tests | Run `nono setup --grant-ancestors --profile copilot-cli` elevated; re-run to confirm idempotency |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 120s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
