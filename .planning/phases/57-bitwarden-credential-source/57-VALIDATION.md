---
phase: 57
slug: bitwarden-credential-source
status: validated
nyquist_compliant: true
wave_0_complete: true
created: 2026-06-05
updated: 2026-06-05
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
| 57-01-01 | 01 | 1 | REQ-CRED-01 | T-57-01 / — | `bw://` URI validation rejects injection chars / query / fragment; reserved vs custom-field selectors do not collide | unit | `cargo test -p nono keystore` | ✅ | ✅ green |
| 57-01-02 | 01 | 1 | REQ-CRED-01 | T-57-02 / T-57-03 / T-57-04 | `BW_SESSION`/`BWS_ACCESS_TOKEN` and secret values never in argv (env-only); `bw://` ref redacted in logs; fail-closed (missing token / CLI / not-found aborts, never defaults) | unit | `cargo test -p nono keystore` | ✅ | ✅ green |

> **Covering tests (verified green 2026-06-05, 46 `bw` test fns within 162 keystore tests):**
> - 57-01-01: `test_is_bw_uri_{true,false}`, `test_validate_bw_uri_*` (forbidden_char, id_with_injection_char, query_rejected, fragment_rejected, secret_no_field_selector, item_missing_selector, unknown_first_segment, unknown_selector, item_{password,username,notes,totp,custom_field}, secret), `test_redact_bw_uri_*`
> - 57-01-02: `test_load_from_bw_{no_session,empty_session}`, `test_load_from_bws_no_token`, `test_load_from_bw{,s}_cli_not_found`, `test_extract_bw_field_*`, `test_build_mappings_bw_uri_{with,without}_var`, `test_load_secret_by_ref_dispatches_bw`

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

---

## Validation Audit 2026-06-05

| Metric | Count |
|--------|-------|
| Gaps found | 0 |
| Resolved | 0 |
| Escalated | 0 |

Post-execution audit (State A). Both Per-Task Map rows moved ⬜ pending → ✅ green: all 46 `bw` test functions (within 162 keystore tests) pass under `cargo test -p nono keystore`; `cargo clippy -p nono -- -D warnings -D clippy::unwrap_used` clean. REQ-CRED-01 has automated unit coverage for every secure behavior except the live-vault end-to-end path, which remains correctly Manual-Only (tracked in 57-HUMAN-UAT.md — requires real `BW_SESSION`/`BWS_ACCESS_TOKEN` + installed CLIs). Phase is Nyquist-compliant.
