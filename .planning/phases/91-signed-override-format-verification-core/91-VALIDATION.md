---
phase: 91
slug: signed-override-format-verification-core
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-06-21
---

# Phase 91 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Derived from 91-RESEARCH.md § "Validation Architecture". Per-task map is finalized by the planner.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust `cargo test` (nono-py crate, `#[cfg(test)]` modules) + optional pytest for PyO3 exception-type assertions |
| **Config file** | `nono-py/Cargo.toml` |
| **Quick run command** | `cargo test -p nono-py override::` |
| **Full suite command** | `cargo test -p nono-py` |
| **Estimated runtime** | ~30 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p nono-py override::`
- **After every plan wave:** Run `cargo test -p nono-py`
- **Before `/gsd:verify-work`:** Full suite + `cargo clippy -p nono-py -- -D warnings -D clippy::unwrap_used` must be green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 91-01-01 | 01 | 1 | OVR-03 | — | `canonical_bytes()` over ZT vectors matches reference SHA-256 digests (SC1) | unit | `cargo test -p nono-py override::canonical` | ❌ W0 | ⬜ pending |
| 91-02-01 | 02 | 2 | VFY-02/03/04/05/06/07 | T-91-02 | valid token → `Ok(OverrideGrant)`; every failure mode → `Err` (SC2) | unit | `cargo test -p nono-py override::verify` | ❌ W0 | ⬜ pending |
| 91-02-02 | 02 | 2 | VFY-06 | T-91-03 | consumed `jti` rejected on 2nd verify in-process (SC3) | unit | `cargo test -p nono-py override::replay` | ❌ W0 | ⬜ pending |
| 91-03-01 | 03 | 3 | VFY-07 | — | `NonoOverrideError` raised at PyO3 boundary for every `Err` (SC4) | unit/integration | `cargo test -p nono-py override::pyo3` | ❌ W0 | ⬜ pending |
| 91-03-02 | 03 | 3 | VFY-07 | — | `#[must_use]` on verify `Result` triggers compile warning if ignored (SC5) | compile-check | `cargo build -p nono-py 2>&1 \| grep must_use` (negative-control test) | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky · Planner refines per-task IDs to match final PLAN.md task breakdown.*

---

## Wave 0 Requirements

- [ ] `nono-py/src/override.rs` `#[cfg(test)]` module — test scaffolding for OVR/VFY requirements
- [ ] Committed local ECDSA P-256 test keypair + test-only DER pubkey injection path (D-01; test-gated, never a production trust anchor)
- [ ] Snapshot of ZT-Infra `test-vectors/canonical-form/vectors.json` into nono-py test tree (OQ-2 — recommend snapshot over live-repo read)

*Rust crate already has a test runner — no framework install needed.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Reconciliation of nono-side token wire shape with real KMS-issued tokens | OVR-01 (D-06) | Real KMS tokens are a Phase 93 live-arm concern; Phase 91 is offline with local-test-keypair fixtures | Deferred to Phase 93 — `[BLOCKING]`-flagged in PLAN.md, not a Phase 91 gate |

*All Phase 91 offline behaviors have automated verification.*

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
