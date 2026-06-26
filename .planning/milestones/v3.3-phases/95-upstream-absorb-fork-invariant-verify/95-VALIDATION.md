---
phase: 95
slug: upstream-absorb-fork-invariant-verify
status: approved
nyquist_compliant: true
wave_0_complete: true
created: 2026-06-26
---

# Phase 95 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test runner (`cargo test`) via `make` targets |
| **Config file** | `Makefile` (targets), `Cargo.toml` (workspace) — no separate test config |
| **Quick run command** | `cargo test -p nono --lib` (or the touched crate) |
| **Full suite command** | `make test` (workspace) |
| **Estimated runtime** | ~120–300 seconds (full workspace build + test) |

---

## Sampling Rate

- **After every task commit:** Run the touched crate's tests (`cargo test -p <crate>`)
- **After every plan wave:** Run `make build` + `make test` (Windows host)
- **Before `/gsd:verify-work`:** `make ci` (clippy + fmt + tests) green; no-new-failures vs the captured baseline (D-04)
- **Max feedback latency:** ~300 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 95-01-01 | 01 | 1 | UPST10-02 | — | Windows baseline-red set frozen before any cherry-pick (D-04) | gate | `grep -c "FAILED" ci-logs-local/baseline-95/baseline-before-cherry-picks.txt` (== 5) | ✅ | ⬜ pending |
| 95-01-02 | 01 | 1 | UPST10-02 | T-95-A (AF_UNIX dup2 bypass) | Cluster A AF_UNIX deadlock fix absorbed, DCO-signed, fork mediation intact | integration | `git log --format=%B HEAD \| grep -c "cherry picked from commit 9ce74e92"` + DCO grep | ✅ | ⬜ pending |
| 95-02-01 | 02 | 2 | UPST10-02 | T-95-B (audit-bypass / non-additive audit.rs) | Cluster B shared-surface hunks staged; tool-sandbox + tls_intercept SKIPPED (D-01) | source | `git status --short \| grep -v "^??" \| wc -l` + prohibited-path absence grep | ✅ | ⬜ pending |
| 95-02-02 | 02 | 2 | UPST10-02 | T-95-B | CR-02 audit invariant byte-intact (`records_verified: event_count > 0`) | unit | `cargo test -p nono --lib -- audit::tests::verify_empty_log_with_no_stored_metadata_is_not_valid` | ✅ | ⬜ pending |
| 95-03-01 | 03 | 2 | UPST10-02 | T-95-C (credential leak via proxy-activation regression) | Cluster C credentials_intent fix only; Phase 89 fail-secure divergence preserved (D-02) | unit | `cargo test -p nono-cli --lib -- proxy_runtime::tests::proxy_activates_with_custom_credentials_only` | ✅ | ⬜ pending |
| 95-03-02 | 03 | 2 | UPST10-02 | T-95-C | Full gate green, no new failures vs baseline | gate | `make ci` | ✅ | ⬜ pending |
| 95-04-01 | 04 | 3 | UPST10-03 | T-95-D (fork Windows-invariant regression) | One checklist entry per fork invariant, none regressed (SC3) + SC4 security note | integration | guard tests (CR-02 + proxy sentinel) + `git diff` invariant gates | ✅ | ⬜ pending |
| 95-04-02 | 04 | 3 | UPST10-02, UPST10-03 | T-95-D | DIVERGENCE-LEDGER closed (no will-sync row open); PARTIAL→96 recorded; full gate green | gate | `make ci` | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

No standalone Wave 0 needed — existing Rust test infrastructure and the guard tests
(`verify_empty_log_with_no_stored_metadata_is_not_valid`, `proxy_activates_with_custom_credentials_only`)
already exist in the tree. The D-04 baseline-red capture serves as the Wave-0-equivalent and is the
FIRST task of Wave 1 (`95-01-01`), gating all downstream cherry-picks.

- [x] Baseline-red capture wired as Wave 1 Task 1 (`95-01-01`). Known baseline (verified live at
  `0e3d1a68`): 5 reds — `try_set_mandatory_label` (nono lib) + `test_init_allowed_when_pack_has_same_short_name`
  + 3 `protected_paths` tests (nono-cli). Recorded so "no new failures" is provable.

*Existing Rust test infrastructure covers all phase requirements — no framework install needed.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Cross-target clippy (`x86_64-unknown-linux-gnu`, `x86_64-apple-darwin`) | UPST10-03 | Toolchain stand-up deferred to Phase 96 (D-03); recorded PARTIAL→96 per cross-target-verify-checklist | Deferred — Phase 95 records the PARTIAL→96 note per cfg-gated commit |
| Fork-invariant: `exec_strategy_windows/` denial-rendering untouched | UPST10-03 | git-diff inspection gate | `git diff <base>..HEAD -- crates/nono-cli/src/exec_strategy_windows/` shows no unintended change |

*Most phase behaviors have automated verification (test names + grep gates pinned in RESEARCH.md).*

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies (8/8 tasks carry automated commands)
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references (baseline-red capture wired as `95-01-01`)
- [x] No watch-mode flags
- [x] Feedback latency < 300s (`make ci` is the wave gate; per-task verifies are sub-30s)
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved 2026-06-26
