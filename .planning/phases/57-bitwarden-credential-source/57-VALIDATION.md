---
phase: 57
slug: bitwarden-credential-source
status: ready
nyquist_compliant: true
wave_0_complete: true
created: 2026-06-05
---

# Phase 57 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test runner (`cargo test`) |
| **Config file** | none — workspace `Cargo.toml`, tests are `#[cfg(test)]` modules in `crates/nono/src/keystore.rs` |
| **Quick run command** | `cargo test -p nono keystore` |
| **Full suite command** | `cargo test -p nono` |
| **Estimated runtime** | ~30 seconds (lib unit tests) |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p nono keystore`
- **After every plan wave:** Run `cargo test -p nono` + `cargo clippy -p nono -- -D warnings -D clippy::unwrap_used`
- **Before `/gsd:verify-work`:** Full suite + clippy must be green
- **Max feedback latency:** 60 seconds

---

## Per-Task Verification Map

> Filled by the planner / executor as tasks are defined. One row per task; map each to REQ-CRED-01 and the relevant threat ref.

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 57-01-01 | 01 | 1 | REQ-CRED-01 | T-57-01 / — | `bw://` URI validation rejects injection chars / query / fragment; reserved vs custom-field selectors do not collide | unit | `cargo test -p nono keystore::tests::bw` | ✅ | ⬜ pending |
| 57-01-02 | 01 | 1 | REQ-CRED-01 | T-57-02 / T-57-03 / T-57-04 | `BW_SESSION`/`BWS_ACCESS_TOKEN` and secret values never in argv (env-only); `bw://` ref redacted in logs; fail-closed (missing token / CLI / not-found aborts, never defaults) | unit | `cargo test -p nono keystore::tests::bw` | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- Existing infrastructure covers all phase requirements. The `op://` path already has `#[cfg(test)]` unit tests in `keystore.rs`; new `bw://` tests follow the same in-module pattern using a mock/trait-injected command runner (no new framework, no new test file required).

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| End-to-end resolution against a live Bitwarden vault (`bw`) and Secrets Manager (`bws`) | REQ-CRED-01 | Requires real `BW_SESSION` / `BWS_ACCESS_TOKEN` and installed CLIs; cannot run in CI without secrets | Operator: `bw unlock --raw` → export `BW_SESSION`; run `nono run --credential 'bw://item/<id>/password=SECRET' -- <cmd>`; confirm secret reaches child env and never appears in logs/argv. Repeat for `bws` with `BWS_ACCESS_TOKEN` + `bw://secret/<uuid>`. |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references (none — existing in-module test infra covers both tasks)
- [x] No watch-mode flags
- [x] Feedback latency < 60s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved 2026-06-05
