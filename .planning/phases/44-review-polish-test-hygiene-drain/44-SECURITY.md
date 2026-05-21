---
phase: 44-review-polish-test-hygiene-drain
audited: 2026-05-20
re_audited: 2026-05-21
auditor: gsd-secure-phase
asvs_level: standard
block_on: critical_open
threats_total: 14
threats_closed: 14
threats_open: 0
verdict: SECURED
unregistered_flags: 0
plans_audited:
  - 44-01-review-polish (8 threats)
  - 44-02-test-hygiene-drain (6 threats)
remediation:
  - phase: 44.1-oidc-fail-closed-remediation-req-review-fu-01-t-44-01-cr-01
    closes: T-44-01
    closes_review_finding: CR-01
    commits:
      - 922d1eb7  # test(44.1-01): pin verify-path fail-closed contract for keyless OIDC issuer
      - abe5c09c  # feat(44.1-01): restore fail-closed contract on keyless OIDC issuer verify path
      - f13b0b3a  # refactor(44.1-01): remove dead configured_oidc_issuer + regression-codifying test
      - 68b09e68  # docs(44.1-01): clarify --issuer required-or-env-var contract
---

# Phase 44 Security Audit

**Phase:** 44 — review-polish-test-hygiene-drain
**Closed:** 14/14
**Open:** 0/14
**ASVS Level:** standard
**Verdict:** SECURED — T-44-01 closed by Phase 44.1 remediation (2026-05-21)

## Executive Summary

Phase 44 closes with all 14 threats verified CLOSED.

- **Initial audit (2026-05-20):** 13/14 CLOSED; T-44-01 (Spoofing-class
  regression on keyless `nono trust verify` trust-anchor decision) flagged
  OPEN as BLOCKER, intersecting code-review finding CR-01.
- **Phase 44.1 remediation (2026-05-21):** Helper-extraction Shape A++ —
  the silent-default `configured_oidc_issuer()` library function is
  deleted; both keyless verify callsites in `crates/nono-cli/src/trust_cmd.rs`
  call a new CLI-side helper `read_required_oidc_issuer(Option<&str>)`
  that fails closed when both `--issuer` and `NONO_TRUST_OIDC_ISSUER`
  are absent. The regression-codifying test
  `configured_oidc_issuer_falls_back_to_github_default_when_unset` is
  deleted and replaced by `read_required_oidc_issuer_fails_closed_when_both_unset`
  which pins the fail-closed contract. CLI `--issuer` doc updated to
  reflect the restored REQUIRED-unless-env-var contract.
- **Re-audit (2026-05-21):** All 8 verification checks for T-44-01 pass.
  Threat flips OPEN → CLOSED. Verdict moves from `OPEN_THREATS` →
  `SECURED`.

No unregistered new-attack-surface flags surfaced from any phase's
SUMMARY.md `## Threat Flags` (44-01, 44-02, and 44.1-01 all explicitly
declared "None").

---

## Threat Verification Table

### Plan 44-01 (review-polish)

| Threat ID | Category | Disposition | Status | Evidence |
|-----------|----------|-------------|--------|----------|
| T-44-01 | Spoofing (S) | mitigate | **CLOSED** (2026-05-21 via Phase 44.1) | `crates/nono-cli/src/trust_cmd.rs:937-960` — helper `read_required_oidc_issuer(Option<&str>) -> Result<String, String>`: returns `Ok(s.to_string())` when `user_issuer` is `Some`; reads `NONO_TRUST_OIDC_ISSUER` and fails closed via `ok_or_else(...)?` with canonical message `"keyless bundle requires --issuer <OIDC_URL> or NONO_TRUST_OIDC_ISSUER env-var (exact match against signer's iss claim)"` when env-var is unset or whitespace-only; `Url::parse(&env_value)` URL-shape-validates the env-var. Both verify callsites wired: `verify_multi_subject_file` at `trust_cmd.rs:1036` and `verify_single_file` at `trust_cmd.rs:1231` call `read_required_oidc_issuer(user_issuer)?` (no silent default substitution). Library function `configured_oidc_issuer` and its regression-codifying test `configured_oidc_issuer_falls_back_to_github_default_when_unset` are DELETED from `crates/nono/src/trust/signing.rs` (`grep '^(pub )?fn configured_oidc_issuer' crates/nono/src/trust/signing.rs` → 0 matches; `grep configured_oidc_issuer_falls_back_to_github_default_when_unset crates/nono/src/trust/signing.rs` → 0 matches). PRIMARY regression test `read_required_oidc_issuer_fails_closed_when_both_unset` at `trust_cmd.rs:2272-2284` PASSES; whitespace-only and malformed-URL branches covered by `read_required_oidc_issuer_fails_closed_when_user_unset_and_env_whitespace_only` and `read_required_oidc_issuer_rejects_malformed_env_url`. CLI doc updated at `cli.rs:3046-3056` to reflect REQUIRED-unless-env-var contract. Remaining `GITHUB_ACTIONS_OIDC_ISSUER` reference at `trust_cmd.rs:623` is in `build_keyless_predicate` (sign-time predicate authoring), NOT a verify-side trust-anchor substitution — confirmed acceptable by 44.1-REVIEW.md lines 51-57. Phase 44.1 commits: `922d1eb7`, `abe5c09c`, `f13b0b3a`, `68b09e68`. |
| T-44-02 | Tampering (T) | accept (per D-44-B4) | CLOSED | Doc comment present at `crates/nono/src/undo/snapshot.rs:594-609` ("**Residual race window:**...tracked as follow-up `.planning/todos/pending/44-validate-restore-target-fd-relative-hardening.md`"). Follow-up todo file exists at the cited path with full scope, acceptance criteria, and ~2-3 week estimate. Accept disposition fully documented. |
| T-44-03 | Information disclosure (I) | mitigate | CLOSED | `crates/nono-cli/src/platform.rs:180-194` — malformed REG_DWORD bails to `None` after stripping `0x`/`0X` prefix; regression test `parse_windows_registry_value_rejects_malformed_dword` at lines 801-815 pins the None-return for `"0xZZZ"` and missing-prefix `"abc"`. |
| T-44-04 | Tampering (T) | mitigate | CLOSED | `crates/nono-cli/src/platform.rs:162` — `first.eq_ignore_ascii_case(name)` for value-name comparison. Regression test `parse_windows_registry_value_accepts_case_mismatch` at lines 780-793 pins case-insensitive match for `EditionId`/`EditionID`/`EDITIONID`/`editionid`. |
| T-44-05 | Denial of service (D) | mitigate | CLOSED | `refresh_synchronous` deleted from all source files (grep returns 0 source matches; only planning-doc references remain). Plan 44-01 SUMMARY confirms deletion in Task 5 commit `c6885f4e`. |
| T-44-06 | Elevation of privilege (E) | mitigate | CLOSED | `.github/scripts/check-cli-doc-flags.sh:54-61` — explicit `if (attr ~ /hide[[:space:]]*=[[:space:]]*true/)` skip on hidden flags. Hidden flags stay hidden; parser no longer exits non-zero on intentionally-hidden flags. |
| T-44-07 | Repudiation (R) | mitigate | CLOSED | All 8 Plan 44-01 task commits (c5b89ff5, 085a4461, babf83ca, c6885f4e, 45a6a832, d21157ad, 3f82b9ca, 6ff834b2) carry `Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>` trailer. Verified via `git log -1 --format="%(trailers:key=Signed-off-by,valueonly)"`. (Note: downstream meta-commits 0120c1d5, f18ad61e, 36871ccf, 5883db1b, cfa2c331, 99afc9ca lack DCO trailers but those are outside the threat-model scope which specified "Plan 44-01" task commits.) Phase 44.1 task commits (922d1eb7, abe5c09c, f13b0b3a, 68b09e68) ALSO carry DCO trailers per 44.1-SUMMARY § Commit-discipline gate. |
| T-44-08 | Information disclosure (I) | mitigate | CLOSED | `crates/nono/src/trust/bundle.rs:1146-1160` — pin-test `verification_policy_default_enables_sct_verification` asserts `VerificationPolicy::default().verify_sct == true`. Future minor bump that flips the default fails this test. |

### Plan 44-02 (test-hygiene-drain)

| Threat ID | Category | Disposition | Status | Evidence |
|-----------|----------|-------------|--------|----------|
| T-44-02-01 | Tampering (T) | mitigate | CLOSED | `44-02-SIBLING-COORDINATION.md:6-21` — URL derivation captured at execute-time from `git remote -v`; `DERIVED_ORG=always-further` matches historically-observed value; deviation gate auto-resolved Option A. Existence-check via `gh repo view` confirmed both sibling URLs resolve before clone (lines 23-28). |
| T-44-02-02 | Repudiation (R) | mitigate | CLOSED | Both sibling commits include `Signed-off-by:` trailer per Plan 44-02 SUMMARY § Verification line 204: "7/7 fork-side commits carry DCO `Signed-off-by` trailers". Sibling SHAs `61ee6aa164` (nono-py) + `1df3e16e6a` (nono-ts) verified via `git log -1 --format='%B' \| grep -i 'signed-off-by'` in each sibling worktree per SUMMARY § Automated checks. Fork-side commits 88a6dedd, 92ba36e9, 2bdea8ea, bfe5ea11, fa2f3cee, fc5cf737, d1798ea3, d182b525 all carry DCO sign-off (verified directly). |
| T-44-02-03 | Information disclosure (I) | accept (per D-44-C1) | CLOSED | `crates/nono-cli/tests/deny_overlap_run.rs:111-127` — either-or assertion present with inline comment explaining security equivalence (lines 111-116); assertion #3 `!stdout.contains("fake-test-secret")` is unchanged at lines 124-127 (the load-bearing security check). Follow-up todo `.planning/todos/pending/44-class-d-validator-preflight-investigation.md` files the latent validator bug per D-44-C3. |
| T-44-02-04 | Denial of service (D) | mitigate | CLOSED | `.config/nextest.toml` contains two `[[profile.default.overrides]]` blocks (lines 10-12, 14-16) with `threads-required = 'num-cpus'` for `windows_run_redirects_profile_state_vars_into_writable_allowlist` + `windows_run_redirects_temp_vars_into_writable_allowlist`. Source-side doc comments at `crates/nono-cli/tests/env_vars.rs:681,1046` cross-link to the nextest config. SC#3 50-runs determinism check is PARTIAL pending live CI (cargo-nextest not installed on Windows dev host) — documented in SIBLING-COORDINATION.md lines 94-120. |
| T-44-02-05 | Elevation of privilege (E) | mitigate | CLOSED | Sibling regression test SHAs recorded: nono-py `61ee6aa16449fcbdeccb819aec051dd7492c8b0b` + nono-ts `1df3e16e6ac8ccb676eb6ae7eb7553e715d46303` (both on `44-broker-ffi-lockstep` branches). PyO3 `to_py_err` and napi-rs `to_napi_err` wildcard arms cover the `BrokerNotFound → SandboxInit-equivalent` mapping; skip()-gated contract assertions document the binding boundary until siblings expose direct broker-argv surfaces. Fork-side regressions at `bindings/c/src/lib.rs:285-291` + `crates/nono-shell-broker/src/main.rs:535,562` continue to catch drift at the Rust layer. |
| T-44-02-06 | Spoofing (S) | accept (per D-44-D2) | CLOSED | `44-02-SIBLING-COORDINATION.md:11-17` — derivation flow proven to read from `git remote -v` at execute-time (raw `UPSTREAM_URL` + `DERIVED_ORG` captured in verifier-greppable form). Hard-coded `always-further` literals in PATTERNS.md docs are context-only; deviation gate fires if `DERIVED_ORG` differs. D-44-D2 documented in `44-CONTEXT.md:97`. |

---

## Accepted Risks Log

| Threat ID | Disposition Rationale | Documenting Artifact |
|-----------|------------------------|----------------------|
| T-44-02 | TOCTOU residual race between `validate_restore_target` lexical check and the non-atomic `create_dir_all`/`retrieve_to`/`set_permissions` sequence. Closure requires substantial cross-platform refactor (Linux nix `*at` syscalls + macOS `*at` + Windows NtCreateFile-or-equivalent). Threat is BOUNDED by requiring a local attacker with write access INSIDE the tracked tree. | Doc comment at `crates/nono/src/undo/snapshot.rs:596-609`; follow-up todo `.planning/todos/pending/44-validate-restore-target-fd-relative-hardening.md` with full scope + acceptance criteria. D-44-B4 in `44-CONTEXT.md`. |
| T-44-02-03 | The Class D either-or assertion (`crates/nono-cli/tests/deny_overlap_run.rs:117-123`) accepts EITHER validator pre-flight diagnostic ("Landlock deny-overlap") OR runtime Landlock filesystem denial ("Permission denied" + "No path denials were observed") as equivalent. The mechanism varies but the security guarantee is preserved by the unchanged assertion #3 `!stdout.contains("fake-test-secret")`. The latent validator pre-flight bug is tracked separately. | Inline comment at `deny_overlap_run.rs:111-116`; follow-up todo `.planning/todos/pending/44-class-d-validator-preflight-investigation.md` with 5 hypothesis branches; D-44-C1 + D-44-C3 in `44-CONTEXT.md`. |
| T-44-02-06 | `always-further` org literal appearing in PATTERNS.md / docs is context-only; the actual derivation flow at execute-time reads from `git remote -v`. The deviation gate fires if `DERIVED_ORG` differs from historically-observed value. | Derivation log at `44-02-SIBLING-COORDINATION.md:6-21` (raw `UPSTREAM_URL` + `DERIVED_ORG` captured in verifier-greppable form); D-44-D2 in `44-CONTEXT.md:97`. |

**Note on WR-03 reviewer finding:** 44-REVIEW.md WR-03 raised a concern that
the either-or assertion `runtime_denial` branch requires BOTH "Permission
denied" AND "No path denials were observed" with AND. This is a test-shape
fragility concern, not a security guarantee gap — assertion #3 still proves
the secret is not leaked. T-44-02-03's accept disposition remains valid.

---

## Re-Audit Trail (2026-05-21)

**Trigger:** Phase 44.1 — OIDC fail-closed remediation
(`.planning/phases/44.1-oidc-fail-closed-remediation-req-review-fu-01-t-44-01-cr-01/`)
declared T-44-01 / CR-01 closed and requested re-audit per § Next Actions
of the initial 2026-05-20 audit.

**Scope:** T-44-01 only. The remaining 13 threats verified CLOSED at the
prior audit and their implementation surfaces were not touched by Phase
44.1 (Phase 44.1's diff is confined to `trust_cmd.rs` + `signing.rs` +
`cli.rs`, all of which are T-44-01 surfaces; the other 13 threats touch
deny_overlap_run.rs, env_vars.rs, nextest.toml, platform.rs, snapshot.rs,
sandbox_prepare.rs, etc., none of which Phase 44.1 modified).

**Files loaded for re-audit:**
- `.planning/phases/44-review-polish-test-hygiene-drain/44-SECURITY.md` (prior audit)
- `.planning/phases/44-review-polish-test-hygiene-drain/44-01-review-polish-PLAN.md` (T-44-01 threat-model block)
- `.planning/phases/44-review-polish-test-hygiene-drain/44-CONTEXT.md` (D-44-B3 acceptance spec line 132)
- `.planning/phases/44.1-oidc-fail-closed-remediation-req-review-fu-01-t-44-01-cr-01/44.1-01-oidc-fail-closed-remediation-PLAN.md` (remediation plan)
- `.planning/phases/44.1-oidc-fail-closed-remediation-req-review-fu-01-t-44-01-cr-01/44.1-01-oidc-fail-closed-remediation-SUMMARY.md` (executor evidence)
- `.planning/phases/44.1-oidc-fail-closed-remediation-req-review-fu-01-t-44-01-cr-01/44.1-VERIFICATION.md` (verifier 8/8 PASS)
- `.planning/phases/44.1-oidc-fail-closed-remediation-req-review-fu-01-t-44-01-cr-01/44.1-REVIEW.md` (code reviewer 0 BLOCKER, 1 WARNING unrelated)
- `crates/nono-cli/src/trust_cmd.rs` (helper body + both callsites + 5 new tests)
- `crates/nono/src/trust/signing.rs` (deletion sites confirmed)
- `crates/nono-cli/src/cli.rs` (--issuer doc)

**Verification checks (8/8 PASS):**

| # | Check | Command | Expected | Got |
|---|-------|---------|----------|-----|
| 1 | Library function gone | `grep '^(pub )?fn configured_oidc_issuer' crates/nono/src/trust/signing.rs` | 0 | 0 PASS |
| 2 | Regression-codifying test gone | `grep configured_oidc_issuer_falls_back_to_github_default_when_unset crates/nono/src/trust/signing.rs` | 0 | 0 PASS |
| 3 | New helper exists | `grep -E 'fn read_required_oidc_issuer' crates/nono-cli/src/trust_cmd.rs` | 1+ | 1 (line 937) PASS |
| 4 | Both verify callsites wired | `grep read_required_oidc_issuer crates/nono-cli/src/trust_cmd.rs` | 3+ | 11 matches (def line 937; callsites lines 1036 + 1231; 5 test fn names + 3 supporting refs) PASS |
| 5 | No silent default in verify path | `grep GITHUB_ACTIONS_OIDC_ISSUER crates/nono-cli/src/trust_cmd.rs` | 0 or only sign-time | 2 matches both at line 611/623 in `build_keyless_predicate` (sign-time predicate authoring), NOT verify-side trust-anchor substitution — acceptable per 44.1-REVIEW.md lines 51-57 PASS |
| 6 | Primary regression test PASSES | `cargo test -p nono-cli --bin nono trust_cmd::tests::read_required_oidc_issuer_fails_closed_when_both_unset` | exit 0 | exit 0, 1 passed PASS |
| 7 | Malformed-env test PASSES | `cargo test -p nono-cli --bin nono trust_cmd::tests::read_required_oidc_issuer_rejects_malformed_env_url` | exit 0 | exit 0, 1 passed (verified via combined run: all 5 helper tests PASS) PASS |
| 8 | CLI doc updated | `grep -A 5 'REQUIRED for keyless verify' crates/nono-cli/src/cli.rs` | mentions env-var alternative | cli.rs:3046-3056 contains `"REQUIRED for keyless verify, unless NONO_TRUST_OIDC_ISSUER is explicitly set"` AND `"Either this flag OR a non-empty NONO_TRUST_OIDC_ISSUER env-var MUST be supplied"` AND CLAUDE.md citation PASS |

**Helper body inspection (`crates/nono-cli/src/trust_cmd.rs:937-960`):**
- Returns `Ok(s.to_string())` when `user_issuer == Some(s)` (line 940-942) — explicit flag wins
- Reads `NONO_TRUST_OIDC_ISSUER`, filters out whitespace-only via `.filter(|v| !v.trim().is_empty())` (line 943-945), then `.ok_or_else(...)?` with canonical fail-closed error `"keyless bundle requires --issuer <OIDC_URL> or NONO_TRUST_OIDC_ISSUER env-var (exact match against signer's iss claim)"` (line 946-950) — both inputs absent OR whitespace-only env-var → fail closed
- `Url::parse(&env_value)` URL-shape validates the env-var value (line 956-958) — malformed env-var → structured error
- Returns `Ok(env_value)` on success (line 959) — parsed env-var value returned

**Verify callsite inspection:**
- `verify_multi_subject_file:1036` — `let req_issuer_owned = read_required_oidc_issuer(user_issuer)?;` (replaces the prior `trust::signing::configured_oidc_issuer()` call). Surrounding inline comment block lines 1023-1035 documents the D-32-08 + Phase-44.1 fail-closed restoration rationale and CLAUDE.md citations.
- `verify_single_file:1231` — identical shape; same Phase-44.1 comment block at lines 1218-1230.

**Disposition:** T-44-01 → CLOSED. All declared mitigations are present in
code with exact file:line cites. The mitigation now matches the
disposition: explicit `ok_or_else(...)?` fail-closed at the verify
trust-anchor boundary, preserving the D-44-B3 opt-in env-var contract,
without the silent canonical-default substitution flagged in the initial
audit.

**Phase 44.1 commits (all with DCO `Signed-off-by:` trailer):**
- `922d1eb7` test(44.1-01): pin verify-path fail-closed contract for keyless OIDC issuer
- `abe5c09c` feat(44.1-01): restore fail-closed contract on keyless OIDC issuer verify path
- `f13b0b3a` refactor(44.1-01): remove dead configured_oidc_issuer + regression-codifying test
- `68b09e68` docs(44.1-01): clarify --issuer required-or-env-var contract

**Independent confirmations:**
- 44.1-VERIFICATION.md: 8/8 must-haves PASS (`status: passed`)
- 44.1-REVIEW.md: 0 BLOCKER, 1 WARNING (`WR-01` unrelated to fail-closed contract per `findings.critical: 0`)

**Unregistered Flags Check (re-audit scope):**

44.1-01-SUMMARY.md § Threat Flags explicitly states: *"None — Phase 44.1
surfaces are all defensive (fix-class); no new network endpoints, auth
paths, or schema changes at trust boundaries were introduced. The single
trust boundary modified (`read_required_oidc_issuer` at the keyless verify
callsite) is a hardening change — the boundary's semantics moved from
`silent-default → require-explicit`, which is a security strengthening,
not a new attack surface."* No unregistered flags identified.

---

## Initial Audit Trail (2026-05-20)

**Files loaded** (full required reading):
- `.planning/phases/44-review-polish-test-hygiene-drain/44-01-review-polish-PLAN.md` (offset 1473, threat model block)
- `.planning/phases/44-review-polish-test-hygiene-drain/44-02-test-hygiene-drain-PLAN.md` (offset 1045, threat model block)
- `.planning/phases/44-review-polish-test-hygiene-drain/44-01-SUMMARY.md`
- `.planning/phases/44-review-polish-test-hygiene-drain/44-02-SUMMARY.md`
- `.planning/phases/44-review-polish-test-hygiene-drain/44-CONTEXT.md`
- `.planning/phases/44-review-polish-test-hygiene-drain/44-REVIEW.md`
- `.planning/phases/44-review-polish-test-hygiene-drain/44-02-SIBLING-COORDINATION.md`
- `crates/nono/src/trust/signing.rs` (full file)
- `crates/nono-cli/src/trust_cmd.rs` (verify sites 950-1242)
- `crates/nono/src/undo/snapshot.rs` (validate_restore_target context 580-620)
- `crates/nono/src/trust/bundle.rs` (SCT pin-test 1137-1160)
- `crates/nono-cli/src/platform.rs` (registry parser + tests 140-815)
- `crates/nono-cli/src/pack_update_hint.rs` (refresh_synchronous deletion verification)
- `crates/nono-cli/tests/deny_overlap_run.rs` (full file)
- `crates/nono-cli/tests/env_vars.rs` (REQ-TEST-HYG-02 doc comments)
- `.config/nextest.toml` (full file)
- `.github/scripts/check-cli-doc-flags.sh` (full file)
- `.planning/todos/pending/44-validate-restore-target-fd-relative-hardening.md`
- `CLAUDE.md` (project conventions — § Coding Standards + Security Considerations)

**Initial-audit verification commands run:**
- `git log 34519423..HEAD --format="%H %s %(trailers:key=Signed-off-by)"` — confirmed DCO trailers on all 8 Plan 44-01 + 8 Plan 44-02 task commits
- `Grep refresh_synchronous` — confirmed deletion from all source files (only planning-doc references remain)
- `Grep eq_ignore_ascii_case` on platform.rs — confirmed at line 162
- `Grep REQ-TEST-HYG-02` on env_vars.rs — confirmed cross-link doc comments at both flaky tests
- `Grep TOCTOU|race window` on snapshot.rs — confirmed doc comment at lines 596-609
- `Grep hide.*true` on check-cli-doc-flags.sh — confirmed skip clause at line 58
- `Grep verify_sct` on bundle.rs — confirmed pin-test at lines 1146-1160
- `Grep configured_oidc_issuer` on signing.rs (pre-44.1) — confirmed default-fallback at line 191 (the original regression cite — now REMEDIATED)
- Read trust_cmd.rs:950-1242 (pre-44.1) — confirmed CR-01 / T-44-01 regression at both verify sites (now REMEDIATED)

**Initial-audit unregistered-flags check:**

Both plans' SUMMARY.md `## Threat Flags` sections explicitly state "None":
- Plan 44-01 SUMMARY.md § Threat Flags (line 327-334): "None — Plan 44-01 surfaces are all defensive (fix-class) or documentation-only; no new network endpoints, auth paths, or schema changes at trust boundaries were introduced."
- Plan 44-02 SUMMARY.md § Threat Flags (line 174-184): "None. All threat boundaries from the plan's `<threat_model>` are mitigated as documented" (followed by per-threat verification rows).

No unregistered new-attack-surface flags identified.

---

## Next Actions

1. **Phase 44 may now ship as SECURED.** All 14 threats verified CLOSED; no BLOCKERs outstanding. Verdict: `SECURED`.

2. **Live-CI deferrals** noted in Plan 44-02 SC#3 (50-runs nextest determinism check) and Plan 44-01 cross-target clippy (PARTIAL on Windows host) — and now also Phase 44.1 cross-target clippy (PARTIAL on Windows host per 44.1-SUMMARY § Cross-target Clippy Disposition) — are operational verifications outside the security threat-audit scope; they remain tracked by the verifier / orchestrator.

3. **No further re-audit required** unless implementation surfaces of the closed threats change.

---

*Initially audited: 2026-05-20*
*Re-audited: 2026-05-21*
*Auditor: gsd-secure-phase*
