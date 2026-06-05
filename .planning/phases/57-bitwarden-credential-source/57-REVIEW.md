---
phase: 57-bitwarden-credential-source
reviewed: 2026-06-05T00:00:00Z
depth: standard
files_reviewed: 1
files_reviewed_list:
  - crates/nono/src/keystore.rs
findings:
  critical: 0
  warning: 3
  info: 4
  total: 7
status: issues_found
---

# Phase 57: Code Review Report

**Reviewed:** 2026-06-05
**Depth:** standard
**Files Reviewed:** 1
**Status:** issues_found

## Summary

Reviewed the `bw://` Bitwarden credential source additions in `crates/nono/src/keystore.rs`
(new functions `is_bw_uri`, `validate_bw_uri`, `validate_bw_item_uri_segments`,
`validate_bws_uri_segments`, `redact_bw_uri`, `is_valid_bw_id`, `parse_bw_item_uri`,
`json_str_field`, `extract_bw_field`, `classify_bw_error`, `classify_bws_error`,
`load_from_bw`, `load_totp_via_bw_get_totp`, `load_from_bws`, `load_from_bw_dispatch`,
plus the `build_mappings_*` and `load_secret_by_ref` dispatch wiring).

The core security properties are well implemented:

- **No secrets in argv.** `BW_SESSION` and `BWS_ACCESS_TOKEN` are never passed as
  CLI flags; both backends rely on the inherited environment. No resolved secret
  value is ever placed in argv. Confirmed against all three `Command::new(...)` sites.
- **Argument-injection defense in depth.** Every subprocess invocation uses the `--`
  argument terminator (`bw get item --nointeraction -- <id>`, `bw get totp ... -- <id>`,
  `bws secret get -- <uuid>`), and `is_valid_bw_id` constrains IDs to `[A-Za-z0-9-]{1,64}`.
  `Command::new` (no shell) is used throughout, so `FORBIDDEN_URI_CHARS` plus the ID
  allowlist plus `--` form a layered defense.
- **Fail-closed.** Missing/empty `BW_SESSION`, missing/empty `BWS_ACCESS_TOKEN`, missing
  CLI binary, locked vault, non-zero exit, invalid JSON, missing field, and empty field
  values all return `Err(...)`. No path returns `Ok` with an empty secret.
- **Zeroizing coverage.** All secret return paths wrap the value in `Zeroizing<String>`
  (`json_str_field`, the `CustomField` arm, `load_totp_via_bw_get_totp`, `load_from_bws`).
- **D-06 enforced.** `validate_bws_uri_segments` rejects any field selector on
  `bw://secret/` URIs (exactly 2 segments required).
- **Redaction.** `redact_bw_uri` is used in tracing and error messages; raw URIs/secrets
  are not logged.
- **No `.unwrap()`/`.expect()`** in production paths; `NonoError` + `?` propagation throughout
  (the `unwrap`/`expect` occurrences are confined to `#[cfg(test)]`).

No BLOCKER-level defects were found. The findings below are robustness and
consistency issues.

## Warnings

### WR-01: Subprocess stderr is echoed verbatim into error messages

**File:** `crates/nono/src/keystore.rs:1602-1607`, `1620-1635`, `1770`
**Issue:** `classify_bw_error` and `classify_bws_error` append the trimmed subprocess
`stderr` directly to the returned `NonoError` message (`Detail: {stderr_trimmed}` and the
fallthrough `... failed for '{}': {}`). While `bw`/`bws` error output is usually benign,
this is an unbounded, untrusted-content channel embedded into an error that propagates to
the user and potentially to logs. If a future `bw`/`bws` version (or a wrapper/shim on
`PATH`) emits the item contents, the otpauth seed, or the access token in a diagnostic line,
it would defeat the careful `redact_bw_uri` discipline applied everywhere else. This is the
same latent issue as the existing `op://` backend (`classify_op_error`), so it is a
pre-existing pattern rather than a regression — but it is being extended to two more
backends here.
**Fix:** Either cap and sanitize the stderr that is surfaced (e.g. truncate to a bounded
length and strip control characters), or stop interpolating raw stderr into user-facing
errors and instead emit it only at `trace!` level behind an explicit debug gate. At minimum,
document the trust assumption inline. Example:

```rust
fn sanitize_stderr(stderr: &str) -> String {
    const MAX: usize = 512;
    let s: String = stderr.trim().chars().filter(|c| !c.is_control() || *c == ' ').take(MAX).collect();
    s
}
// ... NonoError::SecretNotFound(format!("Bitwarden item not found: '{}'. Detail: {}", redacted, sanitize_stderr(stderr)))
```

### WR-02: `bw get totp` failure is misclassified as `classify_bw_error`, hiding the "no TOTP configured" case

**File:** `crates/nono/src/keystore.rs:1768-1771`
**Issue:** When `bw get totp` exits non-zero, the error is routed through
`classify_bw_error`. For TOTP-specific failures (item has no TOTP/2FA configured), `bw`
emits messages like "No TOTP available" / "Not found" that the classifier will fold into
the generic `not found` → `SecretNotFound("Bitwarden item not found")` branch. The user is
told the *item* was not found when in fact the item exists but has no TOTP, which is a
misleading diagnostic for a credential-loading tool. The success path is handled correctly
(empty TOTP → `SecretNotFound` with an accurate message at line 1781-1786), but the
non-zero-exit path is not.
**Fix:** Add a TOTP-aware classification branch (or a dedicated `classify_bw_totp_error`)
that distinguishes "item has no TOTP configured" from "item not found", e.g. match on
`stderr.contains("No TOTP")` / `"Premium"` before falling through to `classify_bw_error`.

### WR-03: `is_valid_bw_id` accepts hyphen-only / leading-hyphen IDs

**File:** `crates/nono/src/keystore.rs:459-461`
**Issue:** `is_valid_bw_id` accepts any string of `[A-Za-z0-9-]` up to 64 chars, including
all-hyphen (`---`) and leading-hyphen (`-rf`, `--version`) values. The `--` argument
terminator in every subprocess call neutralizes the flag-injection risk today, so this is
not a BLOCKER — but it is a fragile invariant: any future call site that forgets `--`
(or a CLI that does not honor `--`) would immediately become flag-injectable, and an
all-hyphen "UUID" is never a legitimate Bitwarden ID. Defense-in-depth argues for
rejecting these at validation time rather than relying solely on the downstream `--`.
**Fix:** Tighten the allowlist to require at least one alphanumeric character and forbid a
leading hyphen:

```rust
fn is_valid_bw_id(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && !s.starts_with('-')
        && s.chars().any(|c| c.is_ascii_alphanumeric())
        && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}
```

## Info

### IN-01: Custom-field name leaks into error message, bypassing redaction policy

**File:** `crates/nono/src/keystore.rs:1569-1574`
**Issue:** The `CustomField` not-found arm of `extract_bw_field` formats the raw field
`name` into the error (`"Custom field '{}' not found in Bitwarden item '{}'"`), while
`redact_bw_uri` deliberately hides the field-name segment everywhere else (verified by
`test_redact_bw_uri_item_custom`). The field name is user-supplied configuration, not the
secret itself, so disclosure risk is low — but it is an inconsistency with the module's
stated "never log the selector/field segment" redaction discipline.
**Fix:** Drop the raw `name` from the message and rely on the already-redacted URI, e.g.
`"Custom field not found in Bitwarden item '{}'"` using `redacted`.

### IN-02: `parse_bw_item_uri` re-validates selectors already guaranteed by `validate_bw_uri`

**File:** `crates/nono/src/keystore.rs:1464-1501`
**Issue:** `parse_bw_item_uri` is only ever reached after `load_from_bw_dispatch` calls
`validate_bw_uri`, yet it re-implements the full selector match including an unreachable
`other =>` error arm and missing-segment checks. This is harmless defensive code but
duplicates the selector vocabulary (`password`/`username`/`totp`/`notes`/`field`) in two
places, so a future selector addition must be made in both `validate_bw_item_uri_segments`
and `parse_bw_item_uri` or the two will silently diverge.
**Fix:** Consider centralizing the selector parse so validation and parsing share one
source of truth, or add a comment noting the two lists must stay in sync.

### IN-03: `load_from_bws` re-parses the UUID by hand instead of using a shared parser

**File:** `crates/nono/src/keystore.rs:1804-1812`
**Issue:** `load_from_bws` strips the prefix and does `path.split('/').nth(1)` to recover the
UUID, duplicating logic that `validate_bws_uri_segments` already performed. The `item`
backend has a dedicated `parse_bw_item_uri` helper; the `secret` backend would benefit from
the same symmetry for maintainability. Behavior is correct (the URI is pre-validated), so
this is style/consistency only.
**Fix:** Extract a small `parse_bws_uri(uri) -> Result<&str>` helper mirroring
`parse_bw_item_uri`.

### IN-04: TOTP backend lacks its own `tracing::debug!` load line

**File:** `crates/nono/src/keystore.rs:1725-1788`
**Issue:** `load_from_bw` (line 1668) and `load_from_bws` (line 1830) each emit a redacted
"Loading secret from ..." debug line, but `load_totp_via_bw_get_totp` does not. The TOTP
path spawns a *second* `bw` subprocess, so for a `bw://item/<id>/totp` reference there is no
trace record of the `bw get totp` invocation. Minor observability gap; no security impact
since redaction would be applied anyway.
**Fix:** Add `tracing::debug!("Loading TOTP from Bitwarden: {}", redact_bw_uri(parent_uri));`
near the top of `load_totp_via_bw_get_totp`.

---

_Reviewed: 2026-06-05_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
