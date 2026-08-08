---
phase: 115-v3-6-carry-forward-drain
plan: 03
subsystem: auth
tags: [rust, profile-validation, fail-secure, aws-sigv4, oauth2, credential-injection]

# Dependency graph
requires:
  - phase: 115-v3-6-carry-forward-drain
    provides: "Plan 115-01's CustomCredentialDef Option<T> inject-field shape and split D-03 regression tests (both confirmed unaffected by this plan's validation rejections)"
provides:
  - "validate_custom_credential unconditional aws_auth rejection (D-13, closes DRAIN-04/NEW-03's aws_auth half)"
  - "validate_oauth2_auth unconditional plain-client_credentials rejection (D-14, closes DRAIN-04/NEW-03's OAuth2 half)"
  - "108-DIVERGENCE-LEDGER.md D-13 carry-forward note: a future SigV4 absorb must un-reject aws_auth"
affects: [116, 117, 118]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Reject-at-validation-time over accept-then-501/silently-degrade for config surfaces with no working implementation"
    - "Unconditional rejection supersedes (and is placed ahead of) a narrower mutual-exclusion check for the same field"

key-files:
  created: []
  modified:
    - crates/nono-cli/src/profile/mod.rs
    - crates/nono-cli/data/nono-profile.schema.json
    - .planning/phases/108-upst12-divergence-audit/108-DIVERGENCE-LEDGER.md

key-decisions:
  - "D-13: aws_auth.is_some() alone (regardless of what else is set) is now an unconditional NonoError::ProfileParse rejection in validate_custom_credential, superseding the prior aws_auth/credential_key/auth mutual-exclusion-only check"
  - "D-14: auth.client_assertion.is_none() is now an unconditional rejection in validate_oauth2_auth, inserted between the client_assertion early-return and the client_id/client_secret emptiness checks"
  - "Downstream aws_auth/client_id/client_secret validation logic (validate_aws_auth's call site, the client_id/client_secret emptiness checks) is deliberately left in place even though it is now unreachable at runtime — kept ready for reactivation when a future plan un-rejects the respective mechanism, mirroring D-13's own divergence-ledger obligation"

requirements-completed: [DRAIN-04]

# Metrics
duration: ~35min
completed: 2026-08-08
---

# Phase 115 Plan 03: aws_auth / Plain-OAuth2 Validation Rejections Summary

**`validate_custom_credential` and `validate_oauth2_auth` now reject `aws_auth` and plain (assertion-less) OAuth2 `client_credentials` unconditionally at profile-load time, closing DRAIN-04/NEW-03's fail-secure gap where both mechanisms validated successfully and only failed (or, for plain OAuth2, silently degraded with zero token injection) at request time.**

## Performance

- **Duration:** ~35 min (commit-timestamp span; session included upfront context reading and a task-isolation revert/reapply pass to produce clean per-task commits)
- **Started:** 2026-08-08T22:21:39+01:00 (Task 1 commit)
- **Completed:** 2026-08-08T22:23:08+01:00 (Task 2 commit)
- **Tasks:** 2/2 completed
- **Files modified:** 3

## Accomplishments

- `aws_auth`-bearing credentials are now rejected at `validate_custom_credential` time with an error naming AWS SigV4 as unimplemented, regardless of what else is set on the credential — closing the gap where `reverse.rs`'s unconditional 501 was only discovered at request time (D-13).
- Plain OAuth2 `client_credentials` (client_id/client_secret, no `client_assertion`) is now rejected at `validate_oauth2_auth` time — RESEARCH Q2's source trace confirmed this flow is genuinely unwired: `CredentialStore::load`'s oauth2 branch only handles `Some(ClientAssertionConfig::SpiffeJwt {..})`, so a plain-credentials route is inserted into none of `credentials`/`aws_routes`/`spiffe_assertion_routes` and the request reaches upstream with **zero token injection** — strictly worse than a 501 (D-14).
- Both rejections were bite-proof verified: the check was temporarily disabled (`if false && ...`), the new dedicated test for each rejection was confirmed to fail with the exact expected panic message, and the check was restored.
- Confirmed (by grep + read, not assumption) that Plan 115-01's two split D-03 regression tests are unaffected: `platform_overrides_custom_credential_merge_exhaustive_over_every_field` calls `merge_custom_credential_def` directly and never reaches either validator; `platform_overrides_custom_credential_collision_inherits_via_pipeline` routes through the real pipeline but never sets `aws_auth` or a client-assertion-less `auth`. Both pass unmodified after this plan's changes.
- `nono-profile.schema.json`'s `aws_auth` property descriptions (6 locations: top-level `CustomCredentialDef` description, `credential_key`, `auth`, `aws_auth`, `spiffe`, `capture`) now document the D-13 rejection.
- `108-DIVERGENCE-LEDGER.md` carries a new "D-13 Carry-Forward Note (Phase 115, DRAIN-04)" section (appended, existing content undisturbed) recording the obligation that a future absorb of real AWS SigV4 signing must un-reject `aws_auth` in `validate_custom_credential`.

## Task Commits

Each task was committed atomically:

1. **Task 1: D-13 reject aws_auth unconditionally + schema + ledger note** - `84d04bb1` (fix)
2. **Task 2: D-14 reject plain OAuth2 client_credentials** - `4b40d82a` (fix)

**Plan metadata:** SUMMARY commit follows (see final commit in this response).

## Files Created/Modified

- `crates/nono-cli/src/profile/mod.rs` - `validate_custom_credential`'s unconditional `aws_auth` rejection (D-13); `validate_oauth2_auth`'s unconditional plain-`client_credentials` rejection (D-14); 1 new + 1 updated test for D-13; 2 new + 4 updated tests for D-14
- `crates/nono-cli/data/nono-profile.schema.json` - `aws_auth`/`credential_key`/`auth`/`spiffe`/`capture` property descriptions and the top-level `CustomCredentialDef` description updated to document the D-13 rejection (6 locations)
- `.planning/phases/108-upst12-divergence-audit/108-DIVERGENCE-LEDGER.md` - appended "D-13 Carry-Forward Note (Phase 115, DRAIN-04)" recording the future SigV4-absorb un-reject obligation

## Decisions Made

- D-13 (locked in CONTEXT.md): reject *any* `aws_auth` route unconditionally, not just the `aws_auth` + `credential_key`/`auth` combination the old mutual-exclusion check caught — the old check is superseded, not kept alongside the new one, since keeping both would be redundant (the unconditional check always fires first).
- D-14 (locked in CONTEXT.md, evidence-gated by RESEARCH Q2): reject plain OAuth2 `client_credentials` on the same rule as D-13, confirmed via source trace (not assumed) that the flow is genuinely unwired before writing the rejection.
- Downstream validation logic that becomes unreachable as a consequence of the new unconditional rejections (`validate_aws_auth`'s call site inside `validate_custom_credential`; the `client_id.is_empty()`/`client_secret.is_empty()`/literal-secret-warning checks inside `validate_oauth2_auth`) is deliberately left in place rather than removed. This mirrors D-13's own divergence-ledger framing: when a future plan un-rejects either mechanism (real SigV4 signing, or plain-OAuth2 route-wiring), the detailed field-level validation it will need is already present and correct, not something that has to be re-derived. `rustc`/clippy do not flag this as dead code (the unreachability is a semantic consequence of two complementary `Option::is_some()`/`is_none()` branches, not something the compiler's static dead-code analysis detects), so no lint suppression was needed either way.
- Chose to isolate Task 1 and Task 2's changes into two clean, non-overlapping commits despite both touching `crates/nono-cli/src/profile/mod.rs`: implemented both tasks' edits together first (to run the full test suite once and confirm 329/329 passing), then temporarily reverted Task 2's edits back to their pre-plan text, committed Task 1 alone (327/329 tests — the 2 missing being Task 2's new tests), and re-applied Task 2's edits verbatim from the same text used the first time, committing Task 2 alone (329/329 restored). This keeps `git log` and per-task bite-proof verification honest per-task rather than producing one combined commit for two independently-verified tasks.

## Deviations from Plan

None - plan executed exactly as written. Both D-13 and D-14 were pre-confirmed by RESEARCH.md before this plan started (Q1/Q2 both resolved with no blocking evidence gap), so no evidence-gathering deviation was needed during execution itself.

## Issues Encountered

None. The 11 pre-existing baseline test failures observed during a full `cargo test -p nono-sandbox-cli --bin nono` run (`audit_session::tests::discover_sessions_does_not_warn_when_legacy_audit_root_is_empty`, `config::tests::*` x5, `profile_cmd::tests::test_init_allowed_when_pack_has_same_short_name`, `protected_paths::tests::*` x3) exactly match the documented pre-existing Windows baseline (11 failures, unrelated to `profile/mod.rs` or this plan's files) — not chased per the durable lesson on this baseline. All 329 `profile::` tests, all 301 `nono-sandbox-proxy --lib` tests, and all 826 `nono-sandbox --lib` tests pass.

## Bite-Proof Verification Transcripts

**D-13 (`custom_credential_aws_auth_rejected_unconditionally`), Task 1:** Temporarily changed `if cred.aws_auth.is_some() {` to `if false && cred.aws_auth.is_some() {`. The new test failed with the exact expected panic: `aws_auth-bearing credential must be rejected unconditionally: ()`. Restored the check; the test and the full `profile::` suite (329/329) passed again.

**D-14 (`oauth2_plain_client_credentials_no_client_assertion_rejected`), Task 2:** Temporarily changed `if auth.client_assertion.is_none() {` to `if false && auth.client_assertion.is_none() {`. The new test failed with the exact expected panic: `plain client_credentials with no client_assertion must be rejected: ()`. Restored the check; the test and the full `profile::` suite (329/329) passed again.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- DRAIN-04 (this plan's sole requirement) is closed: both config surfaces with no working implementation (`aws_auth`, plain OAuth2 `client_credentials`) are now rejected at profile-load time rather than discovered as a 501 or, worse, silently degraded to unauthenticated passthrough at request time.
- `108-DIVERGENCE-LEDGER.md`'s new D-13 carry-forward note is the durable record a future AWS SigV4 absorb plan must read before un-rejecting `aws_auth` — cross-referenced from `SEC-01` (`0ecc476b`)'s existing "won't-sync (target subsystem absent)" row in the same ledger.
- No cross-repo (`../nono-py`) or `.planning/STATE.md`/`.planning/ROADMAP.md` changes were made or needed, per this plan's file scope and the phase's standing SDK-state-writer ban.
- Plans 115-04 (DRAIN-05, `../nono-py` route-config parity) and 115-05/06 (DRAIN-02/06) are unaffected by this plan's changes — no shared files, no shared symbols.

---
*Phase: 115-v3-6-carry-forward-drain*
*Completed: 2026-08-08*

## Self-Check: PASSED

- FOUND: `.planning/phases/115-v3-6-carry-forward-drain/115-03-SUMMARY.md`
- FOUND: `84d04bb1` (Task 1 commit)
- FOUND: `4b40d82a` (Task 2 commit)
- FOUND: `f73951a9` (this summary's own commit, verified after the fact)
