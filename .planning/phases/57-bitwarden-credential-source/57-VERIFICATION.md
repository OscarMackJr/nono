---
phase: 57-bitwarden-credential-source
verified: 2026-06-05T21:00:00Z
status: human_needed
score: 3/3 must-haves verified
overrides_applied: 0
human_verification:
  - test: "Run nono with a bw:// credential on a machine with bw CLI installed and an unlocked vault"
    expected: "The sandboxed child process receives the secret as an environment variable; no raw secret appears in ps output, /proc/<pid>/cmdline, or any tracing log line"
    why_human: "Live subprocess call to the bw binary with a real BW_SESSION token is required to confirm end-to-end secret delivery and confirm no token/value in argv or logs at runtime"
  - test: "Run nono with a bw://secret/<uuid> credential on a machine with bws CLI installed and BWS_ACCESS_TOKEN set"
    expected: "The sandboxed child process receives the .value field as an environment variable; no raw token appears in ps output or tracing logs"
    why_human: "Live subprocess call to the bws binary with a real BWS_ACCESS_TOKEN is required to confirm end-to-end delivery for the Secrets Manager path"
---

# Phase 57: Bitwarden Credential Source Verification Report

**Phase Goal:** Operators can load credentials from Bitwarden via `bw://` URIs alongside the existing keystore backends.
**Verified:** 2026-06-05T21:00:00Z
**Status:** human_needed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | A `bw://` URI resolves a secret from Bitwarden (bw CLI or bws CLI) and makes it available to the child without exposing the raw secret in any log, audit entry, or process argument list | VERIFIED (automated) + ? UNCERTAIN (live runtime) | `load_from_bw_dispatch` wired at line 228-229 of `load_secret_by_ref`, before `is_keyring_uri` (line 230). All `tracing::debug!` calls in bw paths use `redact_bw_uri(uri)`. No `"--session"` literal appears in any `Command::new` args block. `BW_SESSION` env pre-flight at lines 1657-1666; `BWS_ACCESS_TOKEN` pre-flight at lines 1815-1828. Live end-to-end with a real Bitwarden vault requires human verification. |
| 2 | Secret fields are held in `Zeroizing<String>` and cleared on drop; `cargo clippy -D clippy::unwrap_used` exits 0 with no exceptions | VERIFIED | `load_from_bw`, `load_from_bws`, `load_totp_via_bw_get_totp`, `json_str_field`, and `extract_bw_field` all return `Result<Zeroizing<String>>` (confirmed at lines 1651, 1725, 1802, 1508, 1539). `cargo clippy -p nono -- -D warnings -D clippy::unwrap_used` exits 0 (clean, no output). |
| 3 | `bw://` behaves identically to `keyring://`/`env://`/`file://` at the `load_secret_by_ref` dispatch boundary with no platform-specific code paths above the keystore layer | VERIFIED | `is_bw_uri` branch inserted at line 228 of `load_secret_by_ref`, before `is_keyring_uri` at line 230. `bw://` branch added to `build_mappings_from_list` (lines 2132-2156) and `build_mappings_from_pairs` (lines 2203-2204). No `#[cfg(target_os = ...)]` wrapping any bw-related function. |

**Score:** 3/3 truths verified (automated portion); live runtime tests require human.

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/nono/src/keystore.rs` | `fn is_bw_uri(` with `#[must_use]` | VERIFIED | Line 298-301: `#[must_use]` pub fn `is_bw_uri` present |
| `crates/nono/src/keystore.rs` | `fn validate_bw_uri(` | VERIFIED | Line 320: `pub fn validate_bw_uri` present; no double `#[must_use]` (correct — `Result<()>` is inherently must-use; SUMMARY notes this as an auto-fixed deviation from plan) |
| `crates/nono/src/keystore.rs` | `fn load_from_bw_dispatch(` | VERIFIED | Line 1899: present with `#[must_use = "..."]` |
| `crates/nono/src/keystore.rs` | `fn load_from_bw(` | VERIFIED | Line 1651 |
| `crates/nono/src/keystore.rs` | `fn load_from_bws(` | VERIFIED | Line 1802 |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `load_secret_by_ref` | `load_from_bw_dispatch` | `is_bw_uri` branch inserted before `is_keyring_uri` | WIRED | Line 228: `} else if is_bw_uri(credential_ref) {` precedes `is_keyring_uri` at line 230. Confirmed by test `test_load_secret_by_ref_dispatches_bw` (passes). |
| `build_mappings_from_list` | `validate_bw_uri` | `BW_URI_PREFIX` `starts_with` branch | WIRED | Lines 2132-2156: `entry.starts_with(BW_URI_PREFIX)` branch calls `validate_bw_uri(uri)?`. `BW_URI_PREFIX` constant used (not literal string). |
| `load_from_bw` | `Zeroizing::new` | all extracted field values wrapped before return | WIRED | `json_str_field` returns `Zeroizing::new(s.to_string())` at line 1524. `extract_bw_field` CustomField arm returns `Zeroizing::new(v.to_string())` at line 1568. `load_totp_via_bw_get_totp` returns `Zeroizing::new(trimmed)` at line 1787. `load_from_bws` returns `Zeroizing::new(s.to_string())` at line 1885. |

### Data-Flow Trace (Level 4)

Not applicable — no UI/rendering components. The data flow terminates at environment-variable injection into child processes, which is verified at the API level by unit tests and requires live subprocess execution for end-to-end confirmation (see Human Verification below).

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| `cargo test -p nono keystore` | `cargo test -p nono --lib keystore` | 162 passed, 0 failed | PASS |
| `cargo clippy -p nono -- -D warnings -D clippy::unwrap_used` | (executed) | No output (clean) | PASS |
| `cargo test -p nono` (full lib suite) | `cargo test -p nono` | 769 passed, 1 pre-existing fail (`sandbox::windows::tests::try_set_mandatory_label_surfaces_directive_when_user_owned_apply_fails`) | PASS (pre-existing failure excluded per phase instructions) |
| `--session` absent from bw Command args | `grep '"--session"' keystore.rs` | No output | PASS |
| `is_bw_uri` before `is_keyring_uri` in dispatch | Line check | Line 228 before line 230 | PASS |
| All bw functions return `Result<Zeroizing<String>>` | `grep` for return types | Lines 1651, 1725, 1802, 1508, 1539 | PASS |
| `BW_URI_PREFIX` in both `build_mappings` functions | `grep BW_URI_PREFIX` | Lines 2132 (`build_mappings_from_list`) and 2203 (`build_mappings_from_pairs`) | PASS |
| Module doc references `bw://item` and `bw://secret` | `grep "bw://item\|bw://secret" keystore.rs` | Lines 13, 14 of module doc | PASS |

### Probe Execution

No probes declared in PLAN.md or SUMMARY.md. Step 7c: SKIPPED (no probe files for this phase).

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| REQ-CRED-01 | 57-01-PLAN.md | A `bw://` Bitwarden credential source resolves secrets through the keystore abstraction alongside existing schemes, with `Zeroizing<String>` fields | SATISFIED (automated) | All three success criteria verified above. REQUIREMENTS.md traceability table still shows `Pending` — that field needs updating to `Complete` as a housekeeping follow-up; it does not affect the implementation verification. |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `crates/nono/src/keystore.rs` | 459-461 | `is_valid_bw_id` accepts leading-hyphen IDs (e.g. `--version`, `---`) | Warning | WR-03 from code review. Defense-in-depth gap: the `--` argument terminator in all three subprocess invocations (`bw get item`, `bw get totp`, `bws secret get`) currently prevents exploitation. If any future call site omits `--`, a leading-hyphen "ID" becomes a flag-injection vector. The fix is one line: `!s.starts_with('-') && s.chars().any(|c| c.is_ascii_alphanumeric())`. |
| `crates/nono/src/keystore.rs` | 1602-1607, 1620-1635 | `classify_bw_error` / `classify_bws_error` echo raw subprocess stderr into error messages | Warning | WR-01 from code review. Inherited pattern from `classify_op_error`. Unbounded stderr from a manipulated `bw`/`bws` shim on PATH could surface sensitive content in error messages. Pre-existing pattern; same class as `op://`. |
| `crates/nono/src/keystore.rs` | 1725-1788 | `load_totp_via_bw_get_totp` has no `tracing::debug!` load line | Info | IN-04 from code review. No security impact; minor observability gap for the `bw get totp` subprocess call. |
| `crates/nono/src/keystore.rs` | 1569-1574 | Custom-field `name` leaks into `SecretNotFound` error message, inconsistent with `redact_bw_uri` discipline elsewhere | Info | IN-01 from code review. Field name is operator-supplied config, not the secret value itself; low disclosure risk. |

No TBD, FIXME, or XXX debt markers found in any new bw-related code.

### Human Verification Required

#### 1. Live bw CLI End-to-End Secret Delivery

**Test:** On a machine with `bw` CLI installed and a Bitwarden vault, set `BW_SESSION=$(bw unlock --raw)`, configure a profile or `--credential` with `bw://item/<real-id>/password=MY_SECRET`, and run `nono run --profile <profile> -- printenv MY_SECRET`.
**Expected:** The child process prints the password value. No raw password appears in `ps aux` output, `/proc/<pid>/cmdline` (Linux), or `tracing::debug` log output. `BW_SESSION` does not appear in the `bw get item` process argv.
**Why human:** Live subprocess call to the real Bitwarden CLI with a real session token and vault item is required to confirm end-to-end secret delivery and absence of credential leakage in argv or logs at runtime. No mock can substitute for this.

#### 2. Live bws CLI End-to-End Secret Delivery

**Test:** On a machine with `bws` CLI installed and a Bitwarden Secrets Manager project, set `BWS_ACCESS_TOKEN=<real-token>`, configure `bw://secret/<real-uuid>=MY_SECRET`, and run `nono run -- printenv MY_SECRET`.
**Expected:** The child process prints the `.value` field of the secret. No raw token appears in `ps aux` output or tracing logs.
**Why human:** Live subprocess call to the real `bws` CLI with a real access token and secret UUID is required.

### Gaps Summary

No code-level blockers were found. All automated verifications pass. The two review warnings (WR-01 stderr echo, WR-03 leading-hyphen ID allowance) are pre-existing pattern issues documented in the code review, neither of which blocks the phase goal. The sole gap category is live runtime verification that requires a real Bitwarden vault — this is the standard operator-environment gate for any credential-backend phase.

---

_Verified: 2026-06-05T21:00:00Z_
_Verifier: Claude (gsd-verifier)_
