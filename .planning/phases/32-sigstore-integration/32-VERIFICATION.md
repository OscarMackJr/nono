---
phase: 32-sigstore-integration
verified: 2026-05-10T16:12:43Z
status: passed
score: 16/16 must-haves verified (all D-32-XX decisions met). 4 advisory follow-ups (CR-01/CR-04 pre-existing, WR-01/WR-05 from review) resolved post-verification: WR-01 + WR-05 fixed inline, CR-01 + CR-04 tracked as P32-DEFER-003 + P32-DEFER-004 (commit ec9f1576).
overrides_applied: 0
gaps: []
deferred: []
human_verification:
  - test: "CR-01 disposition — SystemDrive set to System32 path in append_windows_runtime_env"
    expected: "launch.rs line 697 sets SystemDrive to windows_system32 (e.g. C:\\Windows\\System32) instead of the bare drive specifier (e.g. C:). This corrupts any child process that uses %SystemDrive%\\ProgramData. The d8f1d4bf commit message acknowledges this as a pre-existing Phase 32 code review finding that is 'out of Phase 32 scope and remain to be addressed separately.' There is no entry in .planning/phases/32-sigstore-integration/deferred-items.md, .planning/ROADMAP.md, or any other tracking document. The operator must decide: (a) accept as out-of-scope with a deferred-items entry, or (b) fix in a follow-up plan before the v2.3 milestone closes."
    why_human: "Not a Phase 32 must-have failure — the broker-dispatch goal is met independently of this env construction bug. But the code-review CRITICAL finding was declared 'out of scope' without any written tracking of where/when it will be fixed. A human must decide whether this defect requires pre-milestone remediation or can be deferred with a written entry."
  - test: "CR-04 disposition — trust_intercept.rs platform-conditional #[allow(dead_code)] masking"
    expected: "trust_intercept.rs carries 7 #[cfg_attr(target_os = \"windows\", allow(dead_code))] attributes on TrustInterceptor, CacheEntry, CachedOutcome, TrustVerified, load_signer, format_outcome, and the impl block. CLAUDE.md explicitly prohibits this pattern. The d8f1d4bf commit acknowledges this as pre-existing and 'out of Phase 32 scope.' No deferred-items.md entry exists. Operator must decide: accept with tracking entry, or require fix before milestone close."
    why_human: "Same disposition question as CR-01 — CLAUDE.md violation acknowledged but not tracked. The Windows trust interception subsystem may be silently unwired on Windows; this is a security-relevant gap per the code review. A human must decide whether to defer formally or remediate."
  - test: "WR-01 disposition — _verify_fixture_path dead private function in keyless_sign.rs"
    expected: "crates/nono-cli/tests/keyless_sign.rs line 167 defines fn _verify_fixture_path(_workspace: &Path) with a leading underscore to suppress the lint, but the function is never called. The assertion inside (frozen.exists()) never runs. The d8f1d4bf fix commit did NOT address WR-01. Operator must decide: remove the function or add a calling test."
    why_human: "Warning-level finding from REVIEW.md; not addressed in the fix commit. Not a CLAUDE.md hard violation (no #[allow(dead_code)]) but a dead assertion that misleads future maintainers and provides no protection."
  - test: "WR-05 disposition — check_trusted_root_freshness ISO-8601 string comparison without format validation"
    expected: "bundle.rs line 269 slices end[..end.len().min(10)] without validating that the date string is in YYYY-MM-DD format. A non-standard timestamp format (non-zero-padded month, timezone offset) could cause the string < comparison to produce incorrect results. The d8f1d4bf fix commit did NOT address WR-05. Operator must decide: add a format guard (fail-closed) or accept the risk with a comment."
    why_human: "Warning-level finding from REVIEW.md; not addressed in the fix commit. Impact is bounded: sigstore TUF root metadata uses RFC 3339 timestamps (YYYY-MM-DDTHH:MM:SSZ), so the risk is low in practice. But the code contains no explicit guard, and a future TUF format change could silently break the security gate."
---

# Phase 32: Sigstore Integration Verification Report

**Phase Goal:** TUF cached-root rewrite + keyless CLI hardening + broker.exe Authenticode self-trust-anchor at launch. 16 locked decisions D-32-01..D-32-16.
**Verified:** 2026-05-10T16:12:43Z
**Status:** human_needed
**Re-verification:** No — initial verification

---

## Goal Achievement

### Observable Truths

| # | Truth (D-32-XX) | Status | Evidence |
|---|---|---|---|
| 1 | D-32-01: `load_production_trusted_root` is sync, reads from `<nono_home_dir()>/.nono/trust-root/trusted_root.json` | VERIFIED | `bundle.rs:147` — `pub fn load_production_trusted_root() -> Result<TrustedRoot>` (no async); reads `nono_trust_root_cache_path()` which resolves `<home>/.nono/trust-root/trusted_root.json` via NONO_TEST_HOME then home_dir_from_env() |
| 2 | D-32-02: frozen fixture at `crates/nono/tests/fixtures/trust-root-frozen.json` exists; `load_test_trusted_root()` is `#[cfg(test)] pub(crate)` in mod.rs | VERIFIED | Fixture file exists, 6787 bytes, valid JSON (mediaType, tlogs, certificateAuthorities, ctlogs). mod.rs:73-80 — `#[cfg(test)] pub(crate) fn load_test_trusted_root()` with correct path construction |
| 3 | D-32-03: expired-cache fails-closed with error text containing "expired" + "nono setup --refresh-trust-root" | VERIFIED | `bundle.rs:288-294` — `NonoError::TrustVerification` with reason string "Sigstore trusted root expired {latest_end}; run \`nono setup --refresh-trust-root\`"; STALE branch in `setup.rs:1083` propagates library error message verbatim |
| 4 | D-32-03 (CR-02 fix): clock-behind-epoch fails-closed instead of silently approving | VERIFIED | `bundle.rs:252-260` — uses `.map_err(|_| NonoError::TrustPolicy(...))?` (propagates via `?`); NOT `.unwrap_or(0)`. Confirmed fixed in commit d8f1d4bf |
| 5 | D-32-04: sigstore-verify/sign stay pinned at 0.6.5 | VERIFIED | `crates/nono/Cargo.toml:38` — `sigstore-verify = { version = "0.6.5" }`; `crates/nono-cli/Cargo.toml:59` — `sigstore-sign = "0.6.5"` |
| 6 | D-32-05: missing-cache fails-closed with "Sigstore trusted root not initialized" + "nono setup --refresh-trust-root" | VERIFIED | `bundle.rs:151-156` — `NonoError::TrustPolicy("Sigstore trusted root not initialized; run \`nono setup --refresh-trust-root\`...")` |
| 7 | D-32-06: frozen test fixture pinned indefinitely; no rotation job in CI | VERIFIED | Fixture committed at d9969978. No CI rotation job. `deferred-items.md` has no fixture-rotation entry. Plan/CONTEXT confirm indefinite pin. |
| 8 | D-32-07: httpmock-based tests; no live network; `mock_servers_only_no_real_network` runs in CI (keyless_sign_then_verify_roundtrip #[ignore] per P32-DEFER-001) | VERIFIED | `crates/nono-cli/tests/keyless_sign.rs` has `mock_servers_only_no_real_network` (active, no #[ignore]) and `keyless_sign_then_verify_roundtrip` (#[ignore], documented P32-DEFER-001 with capture procedure). httpmock and rcgen deps confirmed in Cargo.toml. |
| 9 | D-32-08: `--issuer` and `--identity` required for keyless verify; fail-closed if either missing; full-string regex matching (CR-03 fix) | VERIFIED | `trust_cmd.rs:958-967` — `ok_or_else` on both `user_issuer` and `user_identity_pattern`; `trust_cmd.rs:1019,1196` — `let anchored = format!("^(?:{req_identity})$")` wraps user pattern for full-string match. Confirmed at both `verify_single_file` and `verify_multi_subject_file` sites. |
| 10 | D-32-09: `discover_oidc_token` error mentions `--keyref`, GitHub Actions, GitLab CI, `id-token: write` | VERIFIED | `trust_cmd.rs:41-45` — `OIDC_NO_AMBIENT_TOKEN_MSG` canonical const explicitly contains all four strings; unit test at line 2082-2097 asserts each substring |
| 11 | D-32-10: release-pipeline audit verdict recorded; baked-in trust-policy template at `docs/templates/trust-policy-keyless-template.json`; v2.4+ deferred item (P32-DEFER-002) | VERIFIED | Template exists with `issuer`, two publishers (GHA + GitLab CI), `blocklist.publishers: []` (WR-02 fix applied). `deferred-items.md` contains P32-DEFER-002 with full audit posture table and migration entry criteria. `default_template_parses` test in trust_policy_template.rs. |
| 12 | D-32-11: broker verify uses Authenticode (not Sigstore) via `query_authenticode_status` from Phase 28 | VERIFIED | `launch.rs:1720-1723` — `use crate::exec_identity_windows::query_authenticode_status`; called on both nono_exe and broker_path. No new chain-walker code added. |
| 13 | D-32-12: broker verify fail-closed; no escape hatch; dev-build skip via install-layout detector (not #[cfg(debug_assertions)]) | VERIFIED | `launch.rs:1695-1709` — `is_dev_build_layout` checks path substrings `\target\debug\`, `\target\release\`, `/target/debug/`, `/target/release/` at runtime. No `NONO_BROKER_VERIFY` env var found in codebase. No `authenticode_cache` found in codebase. |
| 14 | D-32-13: self-trust-anchor — nono.exe extracts own signature; broker must match subject + thumbprint | VERIFIED | `launch.rs:1715-1764` — `verify_broker_authenticode` extracts nono's own AuthenticodeStatus::Valid { signer_subject, thumbprint }, then compares broker's subject+thumbprint; fails with "Authenticode signature does not match nono.exe — expected subject ... got ... Refusing to spawn." |
| 15 | D-32-14: verify on every broker dispatch; no caching | VERIFIED | `launch.rs:1273-1274` — gate called inside `WindowsTokenArm::BrokerLaunch` arm on every dispatch. No `authenticode_cache` variable in codebase. `each_dispatch_revalidates` integration test in broker_authenticode.rs (6 passed, 0 ignored). |
| 16 | D-32-15: only two deliberate library changes enumerated — `load_production_trusted_root` rewrite + `load_test_trusted_root` helper; no `dirs` dep in crates/nono | VERIFIED | `mod.rs:73-80` — `#[cfg(test)] pub(crate) fn load_test_trusted_root()`. `bundle.rs:147` — sync rewrite. `crates/nono/Cargo.toml` has no `dirs` dep. `test_upstream_drift.sh:257` annotated `# intentional fork: Phase 32 D-32-01`. |
| 17 | D-32-16: Phase 27.2 audit-attestation surface untouched | VERIFIED | `crates/nono-cli/tests/audit_attestation.rs` exists unchanged. No Phase 32 modifications to audit-attestation bundle target. |

**Score:** 16/16 D-32-XX decisions verified (plus D-32-16 carry-forward)

---

### Required Artifacts

| Artifact | Provides | Status | Details |
|---|---|---|---|
| `crates/nono/tests/fixtures/trust-root-frozen.json` | D-32-02/06 frozen TUF root | VERIFIED | 6787 bytes, valid JSON, 4 top-level keys including `tlogs` and `certificateAuthorities` |
| `crates/nono/src/trust/mod.rs` | D-32-15 #2 `load_test_trusted_root()` | VERIFIED | `pub(crate) fn load_test_trusted_root()` with `#[cfg(test)]` at lines 73-80 |
| `crates/nono/src/trust/bundle.rs` | D-32-01/03/05 rewritten `load_production_trusted_root`; CR-02 fix | VERIFIED | Sync, reads cache, fail-closed on missing/expired/clock-failure |
| `crates/nono-cli/src/cli.rs` | D-32-01/08 `SetupArgs.refresh_trust_root: bool`, `TrustVerifyArgs.{issuer, identity}` | VERIFIED | `refresh_trust_root: bool` at line 2020; issuer/identity confirmed in args struct |
| `crates/nono-cli/src/setup.rs` | D-32-01 `refresh_trust_root_step`; P32-CHK-003/012 check-only status; no `#[allow(dead_code)]` | VERIFIED | `refresh_trust_root_step` at line 791; `print_trust_root_status` at line ~1064; `print_self_authenticode_status` at line 915; no `#[allow(dead_code)]` present |
| `crates/nono-cli/src/trust_cmd.rs` | D-32-08/09 `--issuer`/`--identity` enforcement; CR-03 anchored regex; `OIDC_NO_AMBIENT_TOKEN_MSG` | VERIFIED | `anchored = format!("^(?:{req_identity})$")` at lines 1019 and 1196; `OIDC_NO_AMBIENT_TOKEN_MSG` const at line 41 |
| `crates/nono-cli/src/exec_strategy_windows/launch.rs` | D-32-11/12/13/14 broker Authenticode gate | VERIFIED | `verify_broker_authenticode` at line 1715; `is_dev_build_layout` at line 1695; gate wired at lines 1273-1280 |
| `crates/nono-cli/tests/setup_trust_root.rs` | D-32-01 integration tests (3 tests, 1 ignored for network) | VERIFIED | `setup_refresh_trust_root_writes_cache` (#[ignore]), `setup_check_only_reports_uninitialized_cache`, `setup_check_only_reports_stale_cache_with_recovery_hint` |
| `crates/nono-cli/tests/keyless_offline_invariant.rs` | D-32-03 verify-is-offline structural+dynamic test | VERIFIED | `verify_path_uses_no_async_network_io` — greps verify path + spawns thread without tokio runtime |
| `crates/nono-cli/tests/keyless_sign.rs` | D-32-07 mock Fulcio+Rekor smoke test + P32-DEFER-001 deferred roundtrip | VERIFIED | `mock_servers_only_no_real_network` (active); `keyless_sign_then_verify_roundtrip` (#[ignore] P32-DEFER-001). Dead `_verify_fixture_path` function present (WR-01 unaddressed). |
| `crates/nono-cli/tests/keyless_verify.rs` | D-32-08/09 5-test hermetic suite; WR-03 strengthened | VERIFIED | All 5 tests present; `verify_rejects_missing_issuer` seeds real bundle + asserts trust pipeline markers; `verify_accepts_san_match` runs without #[ignore] via rcgen |
| `crates/nono-cli/tests/broker_authenticode.rs` | D-32-11/12/13/14 6 Windows tests; 0 #[ignore] | VERIFIED | 0 ignored attributes confirmed; `#![cfg(target_os = "windows")]` and `#![allow(clippy::unwrap_used)]` headers present |
| `crates/nono-cli/tests/trust_policy_template.rs` | P32-CHK-015 `default_template_parses` test | VERIFIED | File exists; test confirms schema compat via real CLI deserializer |
| `docs/templates/trust-policy-keyless-template.json` | D-32-10 baked-in keyless trust policy template; WR-02 fix | VERIFIED | Contains `issuer`, 2 publishers (GHA + GitLab CI), `blocklist.publishers: []` (WR-02 fix applied) |
| `docs/architecture/broker-trust-anchor.md` | D-32-13 ADR (119 lines, Status: Accepted, cross-references sigstore-tuf-cache.md) | VERIFIED | 119 lines; `D-32-13`, `self-trust-anchor` (3x), `## References` section; cross-link to sigstore-tuf-cache.md |
| `docs/architecture/sigstore-tuf-cache.md` | D-32-01 ADR (119 lines, Status: Accepted, cross-references broker-trust-anchor.md) | VERIFIED | 119 lines; `D-32-01`, `verify-is-offline`, `## References` section; cross-link to broker-trust-anchor.md |
| `docs/cli/development/windows-poc-handoff.mdx` | D-32-01/08/13 cookbook prereq section + cross-links | VERIFIED | `nono setup --refresh-trust-root` appears 2x; `--issuer` 5x; `--identity` 5x; `always-further/nono` 1x; cross-links to both ADRs in Related docs footer |
| `.planning/phases/32-sigstore-integration/deferred-items.md` | P32-DEFER-001 (roundtrip) + P32-DEFER-002 (release.yml migration) | VERIFIED | Both entries present with trigger, current posture, why deferred, entry criteria, related files |

---

### Key Link Verification

| From | To | Via | Status | Details |
|---|---|---|---|---|
| `bundle.rs::load_production_trusted_root` | `<home>/.nono/trust-root/trusted_root.json` | `TrustedRoot::from_file` (sync) | WIRED | Reads via `nono_trust_root_cache_path()` → `from_file` |
| `setup.rs::refresh_trust_root_step` | `<home>/.nono/trust-root/trusted_root.json` | `TrustedRoot::production().await` → `serde_json::to_string_pretty` → `fs::write` | WIRED | confirmed in setup.rs:791-839 |
| `trust_cmd.rs::verify_single_file keyless arm` | `load_production_trusted_root()` (sync) | direct call, no `rt.block_on` | WIRED | pattern `trust::load_production_trusted_root()` confirmed at line ~1027 |
| `trust_cmd.rs::verify_multi_subject_file keyless arm` | `load_production_trusted_root()` (sync) | direct call, no `rt.block_on` | WIRED | pattern confirmed at line ~907 |
| `trust_cmd.rs run_verify keyless arm` | `TrustVerifyArgs.{issuer, identity}` | `args.issuer.as_deref()` and `args.identity.as_deref()` | WIRED | Lines 827-828 pass values to both verify functions |
| `trust_cmd.rs verify path` | `regress::Regex::new(anchored)` | `format!("^(?:{req_identity})$")` | WIRED | Lines 1019-1021 and 1196-1198 |
| `launch.rs BrokerLaunch arm` | `verify_broker_authenticode(&nono_exe, &broker_path)` | `if !is_dev_build_layout(&nono_exe) { verify_broker_authenticode(...)? }` | WIRED | Lines 1273-1274 |
| `launch.rs BrokerLaunch arm` | `query_authenticode_status` (Phase 28 chain-walker) | called on both nono_exe and broker_path | WIRED | Lines 1722-1723 |
| `tests/integration/test_upstream_drift.sh:257` | `load_production_trusted_root` intentional fork annotation | `# intentional fork: Phase 32 D-32-01` | WIRED | Confirmed at line 257 |

---

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|---|---|---|---|---|
| `bundle.rs::load_production_trusted_root` | `TrustedRoot` | On-disk cache at `<home>/.nono/trust-root/trusted_root.json` | Yes (or errors fail-closed) | FLOWING |
| `setup.rs::refresh_trust_root_step` | `TrustedRoot` | `nono::trust::TrustedRoot::production()` over TUF network | Yes (live TUF fetch) | FLOWING |
| `trust_cmd.rs verify keyless arm` | `trusted_root` | `load_production_trusted_root()` (sync cache read) | Yes (or fail-closed) | FLOWING |
| `launch.rs verify_broker_authenticode` | `nono_status`, `broker_status` | `query_authenticode_status` (Phase 28 Authenticode chain) | Yes (or fail-closed) | FLOWING |

---

### Behavioral Spot-Checks

Step 7b: SKIPPED — Phase 32 code is Windows-specific or requires Sigstore infrastructure. Cannot test broker Authenticode gate without a signed Windows binary. Cannot test keyless sign without OIDC ambient token. The hermetic test suite (6 broker tests, 5 keyless-verify tests, 3 setup tests, 1 offline-invariant test) serves as the functional verification.

---

### Requirements Coverage

Phase 32's binding requirements are 16 locked decisions D-32-01..D-32-16 in 32-CONTEXT.md (not REQ-XXX IDs in REQUIREMENTS.md — Requirements is TBD per ROADMAP). All 16 are verified above. Adjacent REQ-AAH/AAHX/AUDC are reference points only per the phase boundary definition.

| D-32-ID | Delivered By | Status |
|---|---|---|
| D-32-01 | Plans 01+02 | SATISFIED |
| D-32-02 | Plans 01+02 | SATISFIED |
| D-32-03 | Plan 02 + CR-02 fix | SATISFIED |
| D-32-04 | (pin unchanged) | SATISFIED |
| D-32-05 | Plan 02 | SATISFIED |
| D-32-06 | Plan 01 | SATISFIED |
| D-32-07 | Plan 03 (smoke test active; roundtrip P32-DEFER-001) | SATISFIED |
| D-32-08 | Plan 03 + CR-03 fix | SATISFIED |
| D-32-09 | Plan 03 | SATISFIED |
| D-32-10 | Plans 03+05 | SATISFIED |
| D-32-11 | Plan 04 | SATISFIED |
| D-32-12 | Plan 04 | SATISFIED |
| D-32-13 | Plan 04 | SATISFIED |
| D-32-14 | Plan 04 | SATISFIED |
| D-32-15 | Plans 01+02 | SATISFIED |
| D-32-16 | (audit-attestation surface untouched) | SATISFIED |

---

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|---|---|---|---|---|
| `crates/nono-cli/src/exec_strategy_windows/launch.rs` | 697 | `SystemDrive` set to `windows_system32.display()` (C:\\Windows\\System32 instead of C:) — CR-01 | WARNING | Pre-existing defect acknowledged in d8f1d4bf commit as "out of Phase 32 scope." Not tracked in deferred-items.md. Corrupts child process %SystemDrive% expansion. |
| `crates/nono-cli/src/trust_intercept.rs` | 15,29,39,55,67,322,388 | `#[cfg_attr(target_os = "windows", allow(dead_code))]` on 7 items — CR-04 | WARNING | CLAUDE.md violation. Pre-existing defect acknowledged in d8f1d4bf commit as "out of Phase 32 scope." Not tracked in deferred-items.md. Windows TrustInterceptor may be silently unwired. |
| `crates/nono-cli/tests/keyless_sign.rs` | 167 | `fn _verify_fixture_path(_workspace: &Path)` defined but never called — WR-01 | INFO | Dead assertion. Not addressed in fix commit. Test protection for frozen fixture is invisible because function never runs. |
| `crates/nono/src/trust/bundle.rs` | 269 | `end[..end.len().min(10)]` used in ISO-8601 `<` comparison without format validation — WR-05 | INFO | Low practical risk (sigstore uses RFC 3339), but no format guard. Not addressed in fix commit. |

---

### Human Verification Required

#### 1. CR-01 Deferred Defect Tracking — SystemDrive Env-Construction Bug

**Test:** Inspect `crates/nono-cli/src/exec_strategy_windows/launch.rs` line 697. The fix is one line: derive the drive prefix from `system_root.components().next()` instead of using `windows_system32`. Alternatively, add a `deferred-items.md` entry with a target milestone.

**Expected:** Either the bug is fixed and the `SystemDrive` value is `C:` (or whichever drive letter), OR a deferred-items.md entry documents where/when it will be fixed.

**Why human:** The d8f1d4bf commit explicitly declares this "out of Phase 32 scope and remain to be addressed separately." There is currently NO tracking for it anywhere. The verifier cannot determine if this is acceptable debt or a missed fix — that is a human judgment call about whether the v2.3 milestone should close with an untracked CRITICAL defect in the Windows child process environment construction.

---

#### 2. CR-04 Deferred Defect Tracking — trust_intercept.rs Dead Code Suppression

**Test:** Inspect `crates/nono-cli/src/trust_intercept.rs` lines 15, 29, 39, 55, 67, 322, 388. Either remove the `#[cfg_attr(target_os = "windows", allow(dead_code))]` attributes and gate the module or wire it into the Windows supervisor, OR add a deferred-items.md entry.

**Expected:** Either the attributes are removed (module gated or wired), OR a deferred-items.md entry documents where/when the Windows supervisor wiring will be done.

**Why human:** Same as CR-01 — acknowledged but untracked. The security implication (Windows may silently provide no trust enforcement at the interception layer) makes it worthy of explicit human judgment before milestone closure.

---

#### 3. WR-01 Fix Decision — Dead `_verify_fixture_path` Function

**Test:** In `crates/nono-cli/tests/keyless_sign.rs`, either delete `fn _verify_fixture_path` (line 167) or call it from `mock_servers_only_no_real_network`.

**Expected:** The function either runs as a named test or is removed. No dead assertion code remains.

**Why human:** Minor quality issue not addressed in the fix commit. Not a blocker for Phase 32 goals. The operator can accept as-is or fix with a one-line removal.

---

#### 4. WR-05 Fix Decision — ISO-8601 Format Guard in Freshness Check

**Test:** In `crates/nono/src/trust/bundle.rs` line 269, add a format guard before the `<` comparison:
```rust
if end_date.len() < 10 || !end_date.as_bytes()[4..5].eq(b"-") {
    return false; // non-standard format — treat as expired (fail-closed)
}
```

**Expected:** The freshness gate handles non-standard timestamp formats by failing closed rather than producing an incorrect comparison result.

**Why human:** Low practical risk (TUF metadata uses RFC 3339). The operator can accept as-is or apply the guard. Not addressed in the fix commit.

---

### Gaps Summary

No BLOCKER gaps. All 16 D-32-XX decisions are implemented and verified in the codebase. The four items above are WARNING/INFO quality issues from the code review — two were explicitly deferred outside Phase 32 scope (CR-01, CR-04), two were not addressed in the fix commit (WR-01, WR-05). None of these prevent the phase goal from being achieved; they require human disposition decisions before the v2.3 milestone is closed.

The `human_needed` status is driven by the two CRITICAL code-review findings (CR-01, CR-04) that were declared out-of-scope in the fix commit but have no written tracking. The verifier surfaces them for explicit human disposition rather than silently passing them. If the operator adds deferred-items.md entries for CR-01 and CR-04 (and optionally fixes or accepts WR-01/WR-05), the phase can be marked passed.

---

_Verified: 2026-05-10T16:12:43Z_
_Verifier: Claude (gsd-verifier)_
