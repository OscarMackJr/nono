---
phase: 77
slug: copilot-cli-end-to-end-confinement
status: approved
nyquist_compliant: true
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
| 77-01-01 | 01 | 1 | CPLT-01 | T-77-01 / — | RA grant mask is exactly FILE_READ_ATTRIBUTES (0x80), no broader bits; bad-SID fails closed | unit (tdd) | `cargo test -p nono windows::tests::grant_read_attributes` | ❌ W0 | ⬜ pending |
| 77-01-02 | 01 | 1 | CPLT-01 | T-77-01b / T-77-01c | guard grants RA on owned ancestors, stops at first non-owned (`Ok(false)=>break`), reverts on Drop/Err | unit (tdd) | `cargo test -p nono-cli dacl_guard::tests::ancestor_read_attributes` | ❌ W0 | ⬜ pending |
| 77-01-03 | 01 | 1 | CPLT-01 | T-77-01d | copilot-cli profile declares `node.exe` coverage; stale native-PE test inverted | unit | `cargo test -p nono-cli profile::builtin::tests::copilot_cli` | ❌ W0 | ⬜ pending |
| 77-02-01 | 02 | 2 | CPLT-02 | T-77-02 / — | `--grant-ancestors --profile <p>` is generic (not Copilot-specific), `requires=grant_ancestors` | unit | `cargo test -p nono-cli` (cli arg-parse) | ❌ W0 | ⬜ pending |
| 77-02-02 | 02 | 2 | CPLT-02 | T-77-02 / — | grantee = `S-1-15-2-1`; admin-gated; idempotent (GetAce check) + non-destructive (no existing/deny ACE altered) | unit (tdd) | `cargo test -p nono-cli` (setup grant-ancestors) | ❌ W0 | ⬜ pending |
| 77-03-01 | 03 | 3 | CPLT-03 | T-77-03 / — | gate discriminates confinement FAIL from Copilot/auth/network SKIP_HOST_UNAVAILABLE; no `exit`, locked verdict shape | script | `pwsh -File scripts/gates/copilot-e2e.ps1` contract-load + `verify-dark.ps1 --gate copilot-e2e` | ❌ W0 | ⬜ pending |
| 77-03-02 | 03 | 3 | CPLT-03 | — | permanent non-destructive admin grant documented (D-09) | doc | grep DESIGN-engine-abstraction.md for grant doc section | ❌ W0 | ⬜ pending |
| 77-03-03 | 03 | 3 | CPLT-03 | T-77-03 | [BLOCKING] real authenticated `copilot` suggestion under AppContainer: zero STATUS_ACCESS_DENIED, zero Node module-resolution crash | manual (host-gated checkpoint) | `scripts/verify-dark.ps1 --gate copilot-e2e` (elevated Win11 + Copilot authed) | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*
*Note: a `SKIP_HOST_UNAVAILABLE` verdict on 77-03-03 does NOT close CPLT-03 SC1/SC4 — a real PASS on a provisioned host is required to fully close the requirement (verify-work gate condition, per plan-checker warning #2).*

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

- [x] All tasks have `<automated>` verify or Wave 0 dependencies (77-03-03 is the host-gated checkpoint; all `auto` tasks carry scoped `<automated>` commands)
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references (new test files identified for CPLT-01/02; new gate file for CPLT-03)
- [x] No watch-mode flags
- [x] Feedback latency < 120s (scoped `cargo test -p <crate> <module>` commands)
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved 2026-06-17
