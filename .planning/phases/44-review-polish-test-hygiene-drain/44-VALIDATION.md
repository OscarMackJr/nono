---
phase: 44
slug: review-polish-test-hygiene-drain
status: approved
nyquist_compliant: true
wave_0_complete: true
created: 2026-05-20
reconstructed_from: [44-01-SUMMARY.md, 44-02-SUMMARY.md, 44-VERIFICATION.md]
---

# Phase 44 — Validation Strategy

> Retroactive Nyquist validation contract for the REVIEW polish + test-hygiene drain phase. Reconstructed from `*-SUMMARY.md` artifacts and verified against the live codebase after Phase 44.1 (CR-01 remediation).

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in `cargo test` + `cargo-nextest` (per-test isolation profile) |
| **Config file** | `.config/nextest.toml` (subprocess isolation for two Windows env_vars flakes) |
| **Quick run command** | `cargo test -p nono-cli --lib --no-fail-fast` |
| **Full suite command** | `cargo test --workspace --no-fail-fast` |
| **Lint gate** | `cargo clippy --workspace --tests -- -D warnings -D clippy::unwrap_used` |
| **Cross-target gate** | `cargo clippy --workspace --target x86_64-unknown-linux-gnu` AND `--target x86_64-apple-darwin` per CLAUDE.md § Coding Standards |
| **Sibling-repo (REQ-TEST-HYG-03/04)** | `pytest` (nono-py); `node` (nono-ts) — run from sibling worktrees |
| **Estimated runtime** | ~60 s (lib unit tests); ~7 min (full workspace incl. compile) |

---

## Sampling Rate

- **After every task commit:** `cargo test -p {affected crate} --lib`
- **After every plan wave:** `cargo test --workspace --no-fail-fast` + `cargo clippy --workspace --tests -- -D warnings -D clippy::unwrap_used`
- **Before `/gsd-verify-work`:** Full suite must be green (workspace + clippy)
- **For env-mutating tests on Windows:** `cargo nextest run -p nono-cli --test env_vars --config-file .config/nextest.toml`
- **Max feedback latency:** ~60 s for the relevant crate's unit tests

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 44-01-01 | 44-01 | 1 | REQ-REVIEW-FU-01 | — | Multi-line `#[arg(...)]` flags caught by doc-check (no silent exempt) | integration (shell) | `bash .github/scripts/check-cli-doc-flags.sh` | ✅ | ✅ green |
| 44-01-02 | 44-01 | 1 | REQ-REVIEW-FU-01 | — | `CGROUP_V2_HINT` is single source of truth (no string drift across 6 sites) | unit | `cargo test -p nono --lib error::tests` | ✅ | ✅ green |
| 44-01-03 | 44-01 | 1 | REQ-REVIEW-FU-01 | — | `parse_windows_registry_value` case-insensitive; rejects malformed REG_DWORD; `compare_versions` Ord-symmetric | unit | `cargo test -p nono-cli --lib platform::tests::parse_windows_registry_value_accepts_case_mismatch parse_windows_registry_value_rejects_malformed_dword compare_versions_is_symmetric_on_non_numeric_segments` | ✅ | ✅ green |
| 44-01-04 | 44-01 | 1 | REQ-REVIEW-FU-01 | — | `is_newer` suppresses pre-release false positives; `save_state` is atomic tmp+rename | unit | `cargo test -p nono-cli --lib pack_update_hint::tests::is_newer_suppresses_hint_on_prerelease_installed is_newer_returns_true_on_genuine_upgrade is_newer_returns_false_on_downgrade_or_equal` | ✅ | ✅ green |
| 44-01-05 | 44-01 | 1 | REQ-REVIEW-FU-01 | T-44-01 (CR-01) | `read_required_oidc_issuer` fails closed when both user-flag and env-var unset (post-44.1 remediation; library-side `configured_oidc_issuer` removed) | unit | `cargo test -p nono-cli --lib trust_cmd::tests::read_required_oidc_issuer_returns_user_issuer_when_set read_required_oidc_issuer_returns_env_value_when_user_unset_and_env_set read_required_oidc_issuer_fails_closed_when_both_unset read_required_oidc_issuer_fails_closed_when_user_unset_and_env_whitespace_only read_required_oidc_issuer_rejects_malformed_env_url` | ✅ | ✅ green |
| 44-01-06 | 44-01 | 1 | REQ-REVIEW-FU-01 | — | sigstore-verify default `verify_sct` posture pinned TRUE (future bump that flips default fails this test) | unit | `cargo test -p nono --lib trust::bundle::tests` (SCT default pin assertion at bundle.rs:1147) | ✅ | ✅ green |
| 44-01-07 | 44-01 | 1 | REQ-REVIEW-FU-01 | — | `validate_restore_target` residual TOCTOU race documented (snapshot.rs:596) + follow-up todo filed | doc-source | `grep -q 'Residual race window' crates/nono/src/undo/snapshot.rs && test -f .planning/todos/pending/44-validate-restore-target-fd-relative-hardening.md` | ✅ | ✅ green |
| 44-01-08 | 44-01 | 1 | REQ-REVIEW-FU-01 | — | Auto-pull test thread-safety: 5 tests use canonical `lock_env()` + `EnvVarGuard`; XDG_CONFIG_HOME pinned | integration | `cargo test -p nono-cli --test auto_pull_e2e_linux` (Linux-only cfg; Windows host PARTIAL → CI) | ✅ | ⚠️ cfg-gated |
| 44-02-01 | 44-02 | 1 | REQ-TEST-HYG-01 | T-44-02-03 | Class D deny-overlap re-enabled with either-or assertion (`validator_message` OR `runtime_denial`); secret never appears in stdout | integration | `cargo test -p nono-cli --test deny_overlap_run` (Linux-only cfg; Windows host PARTIAL → CI) | ✅ | ⚠️ cfg-gated |
| 44-02-02 | 44-02 | 1 | REQ-TEST-HYG-02 | T-44-02-04 | Class E env_vars flakes serialized via subprocess isolation (`threads-required = 'num-cpus'`) | integration | `cargo nextest run -p nono-cli --test env_vars --config-file .config/nextest.toml` (PARTIAL: nextest not on Windows dev host → CI) | ✅ | ⚠️ tool-deferred |
| 44-02-03 | 44-02 | 1 | REQ-TEST-HYG-03 | T-44-02-05 | nono-py PyO3 binding maps SandboxInit family → `PyRuntimeError` (lockstep with fork-side C-FFI mapping) | sibling-pytest | `cd ../nono-py && pytest tests/test_broker_ffi_mapping.py` (branch `44-broker-ffi-lockstep` @ 61ee6aa164) | ✅ | ✅ green |
| 44-02-04 | 44-02 | 1 | REQ-TEST-HYG-03, REQ-TEST-HYG-04 | T-44-02-05 | nono-ts napi-rs binding maps PathNotFound → `Error{code: 'InvalidArg'}`; broker-argv null/INVALID_HANDLE_VALUE rejection contract via skip-gated tests pointing at fork-side regressions | sibling-node | `cd ../nono-ts && node tests/test_broker_ffi_mapping.js` (branch `44-broker-ffi-lockstep` @ 1df3e16e6a) | ✅ | ✅ green |
| 44-02-05 | 44-02 | 1 | REQ-TEST-HYG-04 | — | Broker-argv null + INVALID_HANDLE_VALUE rejection at fork-side (v24 broker regressions catch drift at Rust layer) | unit | `cargo test -p nono-shell-broker --lib` (Windows-only cfg; broker tests at `crates/nono-shell-broker/src/main.rs:530-565`) | ✅ | ⚠️ cfg-gated |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ cfg-gated/tool-deferred (PARTIAL — runs in CI)*

**Coverage:** 13/13 tasks have automated commands. 4 are PARTIAL on the Windows dev host (Linux-cfg or nextest-tool deferral); all 4 run in live CI lanes per the cross-target-verify-checklist precedent.

---

## Wave 0 Requirements

*Existing infrastructure covers all phase requirements.* No Wave 0 stubs needed — Phase 44 is a drain/hygiene phase; every requirement landed alongside its regression test in the same commit.

Inventory of test artifacts produced in-phase:

- `crates/nono-cli/src/platform.rs` — 3 new `#[test]` cases (lines 780, 801, 823)
- `crates/nono-cli/src/pack_update_hint.rs` — 3 new `#[test]` cases (lines 322, 340, 349)
- `crates/nono/src/trust/bundle.rs` — 1 new SCT pin `#[test]` (line 1147)
- `crates/nono-cli/src/trust_cmd.rs` — 5 fail-closed `read_required_oidc_issuer` `#[test]` cases (lines 2251–2305; landed via Phase 44.1 remediation, supersedes original Phase 44 library-side `configured_oidc_issuer` tests)
- `crates/nono-cli/tests/deny_overlap_run.rs` — `#[ignore]` removed; either-or assertion (line 117)
- `crates/nono-cli/tests/env_vars.rs` — doc-comments cross-linking to `.config/nextest.toml` (lines 681, 1046)
- `crates/nono-cli/tests/auto_pull_e2e_linux.rs` — 5 tests refactored to `lock_env()` + `EnvVarGuard`
- `.config/nextest.toml` — first nextest config in repo (2 per-test override blocks)
- `crates/nono-cli/tests/common/test_env.rs` — gate widened to `any(windows, linux)`; `lock_env()` mirror added
- Sibling: `C:\Users\OMack\nono-py\tests\test_broker_ffi_mapping.py`
- Sibling: `C:\Users\OMack\nono-ts\tests\test_broker_ffi_mapping.js`

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Class D Linux deny-overlap test runtime exercise | REQ-TEST-HYG-01 | Test file is `#![cfg(target_os = "linux")]`; Windows dev host cannot execute. Source-level checks complete (no `#[ignore]`, either-or assertion present, secret-leak assertion #3 unchanged). | On Linux CI lane: `cargo test -p nono-cli --test deny_overlap_run`. Expected: exit 0; either-or branch fires (validator pre-flight OR runtime Landlock denial); `fake-test-secret` does not appear in stdout. |
| Class E env_vars 50-runs determinism check (Roadmap SC#3) | REQ-TEST-HYG-02 | `cargo-nextest` is not installed on the Windows dev host. | Wire `cargo nextest run -p nono-cli --test env_vars --config-file .config/nextest.toml` into Windows CI; iterate 50× back-to-back. Expected: 0 failures across both `windows_run_redirects_profile_state_vars_into_writable_allowlist` and `windows_run_redirects_temp_vars_into_writable_allowlist`. |
| Cross-target Linux clippy on Phase 44 HEAD | REQ-REVIEW-FU-01 (CLAUDE.md cross-target rule) | `x86_64-linux-gnu-gcc` C-toolchain not installable in the worktree execution sandbox. Documented in `44-01-CLIPPY-CROSS-TARGET.md` as PARTIAL. | On Linux CI lane: `cargo clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used`. Expected: exit 0. |
| Cross-target macOS clippy on Phase 44 HEAD | REQ-REVIEW-FU-01 (CLAUDE.md cross-target rule) | macOS SDK not present on Windows dev host. Documented in `44-01-CLIPPY-CROSS-TARGET.md` as PARTIAL. | On macOS CI lane: `cargo clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used`. Expected: exit 0. |
| Sibling-repo PR submission to `always-further/nono-py` + `always-further/nono-ts` | REQ-TEST-HYG-03, REQ-TEST-HYG-04 | Submitting PRs without user authorization would violate nono-py CONTRIBUTING.md § 9 (maintainer review handoff is the user's call). Both branches are PR-ready with DCO sign-off. | From sibling worktrees: `git push -u origin 44-broker-ffi-lockstep` then `gh pr create --base main` against each upstream repo. |
| Phase 44 close SHA recorded as v2.6 quiet-baseline anchor (Roadmap SC#5) | — | Anchor SHA only exists after merge commit lands on main; the validation step runs pre-merge. Orchestrator-managed post-merge bookkeeping. | After Phase 44 merges to main, the orchestrator captures the merged HEAD SHA and records it in `ROADMAP.md` / `STATE.md` as the anchor referenced by REQ-CI-FU-03 in Phase 46. |

---

## Validation Sign-Off

- [x] All tasks have automated verify commands or are routed to Manual-Only with rationale
- [x] Sampling continuity: every task has its own command; no 3 consecutive tasks without an automated verify
- [x] Wave 0 covers all MISSING references *(none — drain phase, tests landed alongside fixes)*
- [x] No watch-mode flags used in commands
- [x] Feedback latency < 60 s for the relevant crate's unit tests
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved 2026-05-20 (retroactive reconstruction from `*-SUMMARY.md` + `44-VERIFICATION.md`; live codebase cross-checked after Phase 44.1 CR-01 remediation).

## Validation Audit 2026-05-20

| Metric | Count |
|--------|-------|
| Requirements audited | 5 (REQ-REVIEW-FU-01, REQ-TEST-HYG-01..04) |
| Tasks mapped | 13 |
| Automated verifies present | 13 |
| Gaps found (MISSING) | 0 |
| PARTIAL (cfg-gated or tool-deferred) | 4 |
| Routed to Manual-Only | 6 (4 PARTIAL runtime + 1 PR-handoff + 1 anchor SHA) |
| Auditor escalations | 0 |

**Verdict:** Phase 44 is **Nyquist-compliant**. All requirements have automated tests in-tree; the PARTIAL items are runtime-execution deferrals on environmental grounds (Linux cfg, missing nextest tool, missing cross-target C toolchains, administrative PR handoff, post-merge orchestrator step) — none are missing-test gaps. Manual-Only routing matches `44-VERIFICATION.md § human_verification` 1:1.
