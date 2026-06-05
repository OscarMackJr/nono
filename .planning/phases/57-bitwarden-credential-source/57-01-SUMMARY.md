---
phase: 57-bitwarden-credential-source
plan: "01"
subsystem: keystore
tags: [credential-source, bitwarden, security, subprocess, zeroize]
dependency_graph:
  requires: []
  provides: [bw-uri-validation, bw-backend-loader, bws-backend-loader, bw-dispatch]
  affects: [load_secret_by_ref, build_mappings_from_list, build_mappings_from_pairs]
tech_stack:
  added: []
  patterns: [op-uri-analog, subprocess-spawn-timeout, zeroizing-string, serde_json-field-extraction]
key_files:
  created: []
  modified:
    - crates/nono/src/keystore.rs
decisions:
  - "Mirror op:// backend pattern exactly for bw:// — validate-then-spawn, no shell, Zeroizing<String>"
  - "BW_SESSION and BWS_ACCESS_TOKEN are env-only; never in argv (T-57-02, D-02)"
  - "TOTP uses separate bw get totp subprocess — login.totp in item JSON is seed not code (Pitfall 1)"
  - "validate_bw_uri removed #[must_use] since Result<()> is already must_use (clippy double_must_use)"
  - "is_bw_uri inserted in load_secret_by_ref after is_apple_password_uri, before is_keyring_uri"
metrics:
  duration: "9 minutes"
  completed: "2026-06-05T20:25:00Z"
  tasks_completed: 2
  files_modified: 1
---

# Phase 57 Plan 01: bw:// Bitwarden Credential Source Summary

Added `bw://` Bitwarden credential source to `crates/nono/src/keystore.rs` alongside the existing `op://`, `keyring://`, `env://`, and `file://` schemes — routing `bw://item/<id>/<selector>` to the `bw` CLI and `bw://secret/<uuid>` to the `bws` Secrets Manager CLI, with all extracted secrets held in `Zeroizing<String>`.

## Task Results

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | URI Constants, Validation Functions, and Unit Tests | 9210939c | crates/nono/src/keystore.rs |
| 2 | Backend Loaders, Dispatch Wiring, and Mappings Branch | 44ce6543 | crates/nono/src/keystore.rs |

## What Was Built

**Task 1 (9210939c):**
- Module-level doc comment updated to list `bw://item` and `bw://secret` schemes
- Constants: `BW_URI_PREFIX`, `BW_ITEM_SEGMENT`, `BW_SECRET_SEGMENT`
- `is_bw_uri(credential_ref: &str) -> bool` with `#[must_use]`
- `validate_bw_uri(uri: &str) -> Result<()>` — strips prefix, rejects FORBIDDEN_URI_CHARS, query strings, fragments, dispatches to segment-specific validators
- `validate_bw_item_uri_segments` — validates item ID via `is_valid_bw_id`, selector (password/username/totp/notes/field), segment counts
- `validate_bws_uri_segments` — validates UUID, rejects any field selector (D-06)
- `is_valid_bw_id` — allowlist: alphanumeric + hyphen, max 64 chars
- `redact_bw_uri(uri: &str) -> String` with `#[must_use]` — `bw://item/<id>/<redacted>` and `bw://secret/<redacted>`
- 30 unit tests covering all valid/invalid URI forms

**Task 2 (44ce6543):**
- `BwSelector` enum: `Password`, `Username`, `Totp`, `Notes`, `CustomField(String)`
- `parse_bw_item_uri` — parses item URI into (id, selector)
- `json_str_field` — walks serde_json::Value by path, rejects empty strings
- `extract_bw_field` — dispatches selector to correct JSON path or TOTP subprocess
- `classify_bw_error` / `classify_bws_error` — stderr substring classification
- `load_from_bw` — `bw get item --nointeraction -- <id>`, BW_SESSION env-only pre-flight
- `load_totp_via_bw_get_totp` — `bw get totp --nointeraction -- <id>`, separate subprocess
- `load_from_bws` — `bws secret get -- <uuid>`, BWS_ACCESS_TOKEN env-only pre-flight
- `load_from_bw_dispatch` — validates URI, routes by first segment, `#[must_use]`
- Dispatch wiring: `is_bw_uri` branch in `load_secret_by_ref` (before `is_keyring_uri`)
- `bw://` branch in `build_mappings_from_list` requiring explicit `=VAR_NAME`
- `bw://` validation in `build_mappings_from_pairs`
- 15 unit tests covering env pre-flight, CLI-not-found, JSON field extraction, mappings

## Verification Results

| Check | Result |
|-------|--------|
| `cargo test -p nono keystore` | 162 passed, 0 failed |
| `cargo test -p nono` | 769 passed, 1 pre-existing fail (sandbox::windows::tests::try_set_mandatory_label, unrelated to this change) |
| `cargo clippy -p nono -- -D warnings -D clippy::unwrap_used` | Clean |
| No `--session` in bw argv | PASS |
| All tracing::debug uses redact_bw_uri | PASS |
| is_bw_uri in dispatch before is_keyring_uri | PASS (line 228 before line 230) |
| All bw functions return Result<Zeroizing<String>> | PASS |
| BW_URI_PREFIX in both build_mappings functions | PASS |
| Module doc references bw://item and bw://secret | PASS |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Removed double #[must_use] on validate_bw_uri**
- **Found during:** Task 1 clippy run
- **Issue:** Plan specified `#[must_use]` on `validate_bw_uri` but `Result<()>` is already `#[must_use]`, triggering `clippy::double_must_use` (error under `-D warnings`)
- **Fix:** Removed the `#[must_use]` attribute from `validate_bw_uri` — the `Result<()>` return type is inherently must-use; calling code still gets the must-use lint
- **Files modified:** crates/nono/src/keystore.rs

## Threat Coverage

All STRIDE threats from the plan's threat model were addressed:

| Threat ID | Status |
|-----------|--------|
| T-57-01 (Tampering — URI injection) | Mitigated: FORBIDDEN_URI_CHARS + is_valid_bw_id allowlist; test_validate_bw_uri_forbidden_char passes |
| T-57-02 (Information Disclosure — BW_SESSION in argv) | Mitigated: no --session arg in any Command spawn |
| T-57-03 (Information Disclosure — secret in logs) | Mitigated: all tracing::debug! use redact_bw_uri(uri) |
| T-57-04 (DoS/Spoofing — fail-open on missing token) | Mitigated: explicit pre-flight returning Err on unset/empty; 3 tests confirm |
| T-57-05 (Information Disclosure — Vec<u8> not zeroized) | Accepted: documented in code comments; same class as op:// |
| T-57-06 (Tampering — D-06 field selector on secret/) | Mitigated: validate_bws_uri_segments rejects >2 segments; test_validate_bw_uri_secret_no_field_selector passes |
| T-57-SC (no new packages) | Accepted: serde_json was already a workspace dep |

## Known Stubs

None. All dispatch paths are wired. The `bw` and `bws` CLI binaries are operator-installed; their absence at runtime produces fail-closed `KeystoreAccess` errors (tested by test_load_from_bw_cli_not_found and test_load_from_bws_cli_not_found).

## Self-Check: PASSED

- crates/nono/src/keystore.rs: FOUND
- Commit 9210939c: FOUND (git log confirmed)
- Commit 44ce6543: FOUND (git log confirmed)
- cargo test -p nono keystore: 162 passed
- cargo clippy -p nono -- -D warnings -D clippy::unwrap_used: Clean
