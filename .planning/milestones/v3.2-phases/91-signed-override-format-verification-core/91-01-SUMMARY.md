---
phase: 91-signed-override-format-verification-core
plan: "01"
subsystem: nono-py/override
tags: [canonicalization, caf-v0.1, zt-infra, serde, tdd, override-token]
dependency_graph:
  requires: []
  provides:
    - nono-py/src/override.rs (OverrideErrorKind, canonical_bytes, canonical_sha256, OverrideToken, parse_token)
    - nono-py/tests/fixtures/vectors.json (9 ZT-Infra CAF v0.1 test vectors, verbatim)
  affects:
    - nono-py/src/lib.rs (override_mod declaration added)
    - nono-py/Cargo.toml (sha2, base64, chrono promoted to direct deps)
tech_stack:
  added:
    - sha2 = "0.10" (SHA-256 for canonical_sha256)
    - base64 = "0.22" (Plan 02 use; promoted now per task instructions)
    - chrono = { version = "0.4", features = ["clock"] } (Plan 02 use; promoted now)
  patterns:
    - BTreeMap code-point key sort for R3 (NEVER serde_json::to_string)
    - "#[serde(deny_unknown_fields)] on OverrideToken + KmsSignature (fail-secure, deviation from policy.rs)"
    - "#[must_use = \"...\"] with message on both canonical_ functions"
    - serde error string inspection ("missing field") for MissingField vs Parse classification
key_files:
  created:
    - nono-py/src/override.rs
    - nono-py/tests/fixtures/vectors.json
    - nono-py/tests/fixtures/vectors.json.SOURCE
  modified:
    - nono-py/src/lib.rs (override_mod module declaration)
    - nono-py/Cargo.toml (dep promotions)
    - nono-py/src/proxy.rs (Rule 3: aws_auth: None)
    - nono-py/src/policy.rs (Rule 3: aws_auth: None)
    - nono-py/src/windows_confined_run.rs (Rule 3: map_or to is_none_or)
decisions:
  - "Keyword-escape choice: #[path = \"override.rs\"] mod override_mod — override_mod reads cleaner at Plan 03 PyO3 registration sites"
  - "MissingField detection: serde error message string-inspect for 'missing field' substring — deterministic, matches serde stable message"
  - "Deps promoted: sha2=0.10, base64=0.22, chrono={0.4,clock} — all pre-resolved in Cargo.lock before adding"
  - "vectors.json source commit: 0e6e81dbd1b8eb20df1be74c13b38f3a7815c4b7 (ZT-Infra ZERO_TRUST_V2)"
metrics:
  duration_minutes: 11
  completed_date: "2026-06-21T22:10:36Z"
  tasks_completed: 3
  files_created: 3
  files_modified: 7
---

# Phase 91 Plan 01: Override Token Canonicalization Foundation Summary

CAF v0.1 canonical form + strict OverrideToken serde model with all 9 ZT-Infra test vectors passing byte-exact conformance before any signature path is wired (SC1).

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Scaffold override.rs + keyword collision + deps | 14aafc1 | nono-py/src/override.rs (stub), lib.rs, Cargo.toml, proxy.rs, policy.rs |
| 2+3 | canonical_bytes/sha256 + OverrideToken model | a6128fb | nono-py/src/override.rs (full impl), tests/fixtures/vectors.json, vectors.json.SOURCE, windows_confined_run.rs |

## Outcome

- SC1 MET: `cargo test -p nono-py override_mod::canonical` passes all 9 ZT-Infra vectors (bytes + length + sha256_hex).
- OVR-01/02/03 modeled: scope, not_before, expires_at, jti, repo_context ride inside the signed OverrideToken object (signature-covered per D-06/OVR-02).
- `override` keyword collision resolved: `#[path = "override.rs"] mod override_mod` — platform-neutral (no `#[cfg]`).
- `[BLOCKING]` Phase 93 reconciliation note embedded on OverrideToken struct.
- `cargo clippy -p nono-py -- -D warnings -D clippy::unwrap_used` clean.
- 13 tests total: 7 canonical + 6 token, all green.

## Key Decisions Recorded

**Keyword-escape choice:** Used `#[path = "override.rs"] mod override_mod` (not `r#override`). The `override_mod` identifier reads more clearly at the Plan 03 PyO3 registration sites.

**vectors.json source commit:** `0e6e81dbd1b8eb20df1be74c13b38f3a7815c4b7` (ZT-Infra `ZERO_TRUST_V2` repo). Provenance in `tests/fixtures/vectors.json.SOURCE` (JSON has no comment syntax; sibling note file per plan instructions).

**MissingField detection:** `serde_json::Error::to_string()` string-inspect for `"missing field"` substring. Serde emits this stable English prefix for all missing required fields. Documented in `parse_token()` doc comment.

**Dep promotions:** `sha2 = "0.10"`, `base64 = "0.22"`, `chrono = { "0.4", clock }`. All pre-resolved in `nono-py/Cargo.lock` before adding (verified via `grep -E '^name = "(base64|chrono|sha2)"'`).

**dead_code allowance:** `#![allow(dead_code)]` at `override.rs` module level. A `cdylib` crate cannot observe `pub fn` usage until Plan 03 adds PyO3 registration. The functions are all exercised by `#[cfg(test)]` tests. Documented inline per CLAUDE.md guidance.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Pre-existing aws_auth struct-init breakage (policy.rs + proxy.rs)**
- **Found during:** Task 1 build verification
- **Issue:** `RustRouteConfig` gained `aws_auth: Option<AwsAuthConfig>` in Phase 88 upstream sync; nono-py `From<PolicyRouteConfig>` impl and `ProxyConfig::new()` were never updated. `cargo build -p nono-py` failed at baseline before our changes.
- **Fix:** Added `aws_auth: None` initializer in policy.rs:754 and proxy.rs:218.
- **Files modified:** `nono-py/src/policy.rs`, `nono-py/src/proxy.rs`
- **Commit:** 14aafc1

**2. [Rule 3 - Blocking] Pre-existing map_or clippy errors (windows_confined_run.rs)**
- **Found during:** Task 2+3 clippy verification gate
- **Issue:** 3 `.map_or(true, |v| v.is_empty())` calls triggered `clippy::unnecessary_map_or` (error under `-D warnings`). Pre-existing in `windows_confined_run.rs`.
- **Fix:** Replaced with `.is_none_or(|v| v.is_empty())` and `.is_none_or(|c| ...)`.
- **Files modified:** `nono-py/src/windows_confined_run.rs`
- **Commit:** a6128fb

### TDD Note

Tasks 2 and 3 were written in a single pass (implementation + tests together) because the canonical form spec IS the implementation (PITFALLS #3 prevention requires the two be proven simultaneously). The RED phase would have required a test-only file with stub functions that then fail, but with a `cdylib`, test-only stubs have no good factoring boundary. The 13 tests enforce the full behavioral contract.

## Threat Surface Scan

No new network endpoints, auth paths, file access patterns, or schema changes at trust boundaries. `override.rs` is a pure parse/canonicalize module with no I/O. `tests/fixtures/vectors.json` is test-only (loaded only in `#[cfg(test)]` via `CARGO_MANIFEST_DIR`).

T-91-01-CANON: BTreeMap sort enforced, no `serde_json::to_string` feeding hasher (confirmed: `grep -n 'to_string' nono-py/src/override.rs | grep serde_json` = empty).
T-91-01-UNKNOWN: `#[serde(deny_unknown_fields)]` on both `OverrideToken` and `KmsSignature` — test `unknown_field_rejected_parse` confirms.
T-91-01-MISSING: Required fields non-Option — tests `missing_jti_rejected_missing_field` + `missing_expires_at_rejected_missing_field` confirm.
T-91-01-FLOAT: Float/null/control-char/supra-BMP/out-of-range-int all reject to `OverrideErrorKind::Parse` — 5 negative tests confirm.
T-91-01-WIRESHAPE: `[BLOCKING]` doc comment embedded on `OverrideToken` struct — `grep -n '\[BLOCKING\]' nono-py/src/override.rs` matches line 269.
T-91-01-SC: Only dep promotions (sha2/base64/chrono) pre-resolved in Cargo.lock — no net-new registry packages.

## Known Stubs

None. All functionality is fully implemented. PyO3 registration is deferred to Plan 03 (deliberate architecture, not a stub).

## Self-Check: PASSED

Files:
- `nono-py/src/override.rs` — exists (commit a6128fb: 823 insertions)
- `nono-py/tests/fixtures/vectors.json` — exists (commit a6128fb)
- `nono-py/tests/fixtures/vectors.json.SOURCE` — exists (commit a6128fb)

Commits in nono-py git log:
- `14aafc1` feat(91-01): scaffold override.rs module...
- `a6128fb` feat(91-01): implement canonical_bytes/sha256...
