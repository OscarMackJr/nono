---
phase: 32-sigstore-integration
reviewed: 2026-05-10T00:00:00Z
depth: standard
files_reviewed: 18
files_reviewed_list:
  - crates/nono/src/trust/bundle.rs
  - crates/nono/src/trust/mod.rs
  - crates/nono-cli/Cargo.toml
  - crates/nono-cli/src/cli.rs
  - crates/nono-cli/src/exec_strategy_windows/launch.rs
  - crates/nono-cli/src/package_cmd.rs
  - crates/nono-cli/src/setup.rs
  - crates/nono-cli/src/trust_cmd.rs
  - crates/nono-cli/src/trust_intercept.rs
  - crates/nono-cli/src/trust_scan.rs
  - crates/nono-cli/tests/broker_authenticode.rs
  - crates/nono-cli/tests/keyless_offline_invariant.rs
  - crates/nono-cli/tests/keyless_sign.rs
  - crates/nono-cli/tests/keyless_verify.rs
  - crates/nono-cli/tests/setup_trust_root.rs
  - crates/nono-cli/tests/trust_policy_template.rs
  - docs/templates/trust-policy-keyless-template.json
  - tests/integration/test_upstream_drift.sh
findings:
  critical: 4
  warning: 5
  info: 3
  total: 12
status: issues_found
---

# Phase 32: Code Review Report

**Reviewed:** 2026-05-10T00:00:00Z
**Depth:** standard
**Files Reviewed:** 18
**Status:** issues_found

## Summary

Phase 32 introduces Sigstore integration for instruction-file attestation: keyless signing via Fulcio/Rekor, keyed ECDSA P-256 signing, trust policy evaluation, a production trusted-root cache path, an Authenticode self-trust-anchor gate for the Windows broker, and the pre-exec trust scan. The overall architecture is sound and the fail-secure and offline-verify invariants are correctly implemented. However, four critical defects were found — one incorrect environment variable assignment that would corrupt child processes, one fail-open trust verification path, one security-relevant regex matching bug, and one silent-degradation behavior in the freshness clock. Five warnings cover dead-code suppressions via platform-conditional `#[allow(dead_code)]`, a test function that is defined but never called, a missing blocklist schema field in the template, an unrestored env-var in a test, and a missing negative identity test. Three info items cover minor quality concerns.

---

## Critical Issues

### CR-01: `SystemDrive` set to `System32` path instead of drive letter

**File:** `crates/nono-cli/src/exec_strategy_windows/launch.rs:695-698`

**Issue:** `append_windows_runtime_env` builds the child environment and sets `SystemDrive` to `windows_system32.display()`, which resolves to e.g. `C:\Windows\System32`. The Windows convention for `SystemDrive` is the bare drive specifier, e.g. `C:`. Programs that use `%SystemDrive%` to build paths (e.g. `%SystemDrive%\ProgramData`) will therefore receive `C:\Windows\System32\ProgramData` instead of `C:\ProgramData`, breaking any child process that relies on `SystemDrive`.

```rust
// Broken — windows_system32 is PathBuf::from(system_root).join("System32")
env_pairs.push((
    "SystemDrive".to_string(),
    windows_system32.display().to_string(),  // "C:\Windows\System32"
));
```

**Fix:** Derive the drive letter from `system_root` (which comes from `SystemRoot` / `windir`):

```rust
// Extract drive letter: take the first component of system_root
let system_drive = system_root
    .components()
    .next()
    .map(|c| c.as_os_str().to_string_lossy().trim_end_matches('\\').to_string())
    .unwrap_or_else(|| "C:".to_string());

env_pairs.push((
    "SystemDrive".to_string(),
    system_drive,
));
```

---

### CR-02: Clock-behind-epoch silently passes expiry gate as epoch date

**File:** `crates/nono/src/trust/bundle.rs:248-252`

**Issue:** `check_trusted_root_freshness` uses `duration_since(UNIX_EPOCH).unwrap_or(0)` when the system clock is behind the epoch. If the system clock has drifted before 1970-01-01, `now_secs` becomes 0, `current_date_iso_prefix_for_secs(0)` returns `"1970-01-01"`, and the comparison `"1970-01-01" < end_date` succeeds for any key with a `valid_for.end` after that date (i.e., every real key), so the freshness gate passes silently even for a maximally expired root. Because this is a security gate, the correct behavior on clock failure is to deny, not to approve. The comment says `unwrap_or(0)` but that is exactly the fail-open behavior CLAUDE.md forbids.

```rust
// Fail-open: clock behind epoch → approves expired root unconditionally
let now_secs = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .map(|d| d.as_secs())
    .unwrap_or(0);    // <-- wrong for a security gate
```

**Fix:** Return an error when the clock is unavailable instead of defaulting to epoch zero:

```rust
let now_secs = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .map_err(|_| NonoError::TrustPolicy(
        "system clock is before Unix epoch; cannot verify trusted root freshness".to_string()
    ))?
    .as_secs();
// Propagate via `?` — caller signature is already `Result<()>`
```

Note: this changes `check_trusted_root_freshness` from a non-fallible-clock function to one that can also fail on clock error. The function already returns `Result<()>`, so the `?` propagation is a one-line addition.

---

### CR-03: Identity regex uses partial-match (`find`) — anchored patterns silently pass or fail unexpectedly

**File:** `crates/nono-cli/src/trust_cmd.rs:1012` and `1179`

**Issue:** The `--identity` flag is documented to match against the `workflow` field. Both `verify_single_file` and `verify_multi_subject_file` use `regex.find(workflow).is_none()` to reject mismatches. `regress::Regex::find` returns the *first substring match*, not a full-string match. An operator-provided pattern like `.github/workflows/release.yml` (without anchors) will match any workflow containing that substring — including adversarially crafted workflow paths like `.github/workflows/release.yml.evil@refs/heads/main`. This is a security control bypass.

The plan documentation describes `--identity` as a "regex matched against the bundle's workflow field" and the test `verify_accepts_san_match` uses an anchored pattern `^\.github/workflows/release\.yml$`, but there is no enforcement that the user provides anchors. The typical operator omitting anchors gets a substring match that is weaker than intended for a security gate.

**Fix:** Either enforce full-string match by wrapping the user regex between `^(?:` and `)$` before compiling, or document and enforce that the pattern must contain explicit anchors (fail-closed at parse time if anchors are absent):

```rust
// Option A: always wrap in anchors (implicit full-string match, D-32-08 compliant)
let anchored = format!("^(?:{req_identity})$");
let regex = regress::Regex::new(&anchored)
    .map_err(|e| format!("invalid --identity regex `{req_identity}`: {e}"))?;

// Option B: reject unanchored patterns at parse time (CLI validation)
if !req_identity.starts_with('^') || !req_identity.ends_with('$') {
    return Err(format!(
        "--identity pattern must be anchored (start with ^ and end with $); got: `{req_identity}`"
    ));
}
```

Option A is safer as it is transparent to operators and eliminates the entire class of bypass.

---

### CR-04: `trust_intercept.rs` uses platform-conditional `#[allow(dead_code)]` on the entire `TrustInterceptor` and supporting types — masking unused production code on Windows

**File:** `crates/nono-cli/src/trust_intercept.rs:15,29,39,55,67,67,322,388`

**Issue:** `CacheEntry`, `CachedOutcome`, `TrustVerified`, `TrustInterceptor`, its `impl` block, `load_signer`, and `format_outcome` all carry `#[cfg_attr(target_os = "windows", allow(dead_code))]`. CLAUDE.md states: "`#[allow(dead_code)]` — if code is unused, either remove it or write tests that use it." These suppressed warnings mean that on Windows, the entire trust interception subsystem may be entirely unused in the binary without any compile-time signal. If the Windows supervisor loop was never wired to call `TrustInterceptor::check_path`, the supervisor silently provides no trust enforcement on Windows while the Unix path does. This is a security-relevant gap that the attribute conceals.

**Fix:** Remove the `#[cfg_attr(target_os = "windows", allow(dead_code))]` attributes and either:
(a) Gate the entire module `#[cfg(not(target_os = "windows"))]` if Windows supervisor does not call it yet (documents the gap explicitly), or
(b) Wire `TrustInterceptor` into the Windows supervisor runtime and add an integration test that exercises the Windows code path.

```rust
// Instead of silencing with allow(dead_code), gate the whole module:
#[cfg(not(target_os = "windows"))]
pub struct TrustInterceptor { ... }

// Or add a Windows integration test that calls check_path().
```

---

## Warnings

### WR-01: `_verify_fixture_path` is a dead private function in `keyless_sign.rs`

**File:** `crates/nono-cli/tests/keyless_sign.rs:167`

**Issue:** `fn _verify_fixture_path(_workspace: &Path)` is defined with a leading underscore to suppress the unused-function lint but is never called from any test. It contains a real assertion (`frozen.exists()`) that would fail CI if the fixture were removed, but this protection is invisible because the function never runs. The leading-underscore convention is not listed in CLAUDE.md; it is normally used for intentionally-unused parameters, not for functions. The correct approach per CLAUDE.md is to either call the function or remove it.

**Fix:** Either call it from `mock_servers_only_no_real_network` or `setup_isolated_home`, or delete it. If the intent is to have the frozen fixture always present, add an explicit test:

```rust
#[test]
fn frozen_fixture_exists() {
    let frozen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("nono")
        .join("tests")
        .join("fixtures")
        .join("trust-root-frozen.json");
    assert!(frozen.exists(), "frozen TUF fixture must be committed");
}
```

---

### WR-02: `trust-policy-keyless-template.json` blocklist missing `publishers` key

**File:** `docs/templates/trust-policy-keyless-template.json:24-26`

**Issue:** The `blocklist` object contains only `"digests": []` and is missing `"publishers": []`. If `Blocklist` is deserialized with `#[serde(default)]` on `publishers`, this silently works but leaves users with a template that does not represent the full schema. The test `trust_policy_template.rs` asserts that the template deserializes, which would pass even with the missing field if serde defaults it. Users copying this template to create their own policy will have an incomplete blocklist section that omits publisher blocking, which is a feature of the policy system.

**Fix:**

```json
"blocklist": {
  "digests": [],
  "publishers": []
}
```

---

### WR-03: `verify_rejects_missing_issuer` test acceptance criterion is too weak

**File:** `crates/nono-cli/tests/keyless_verify.rs:88-92`

**Issue:** The test asserts `!output.status.success()` (non-zero exit) as the primary check, but the comment at line 88 notes that "either the keyless arm fires (if bundle found) or the bundle-missing error fires." Both are accepted. This means the test passes even if `nono trust verify` fails for a completely unrelated reason (e.g., binary not found, environment error). The test does not actually prove that the keyless arm enforces `--issuer`; it proves the binary exits non-zero when a stub file with no bundle is passed. The D-32-08 enforcement cannot be distinguished from bundle-not-found in this test.

**Fix:** Supply a real keyless bundle (even the hermetic rcgen-generated one from `verify_accepts_san_match`) to the verify call and assert the specific error message contains `"--issuer"`:

```rust
// Use the hermetic keyless bundle from make_hermetic_keyless_bundle()
// and assert the error message names the missing flag:
assert!(
    combined.contains("--issuer"),
    "D-32-08: error must name the missing --issuer flag; got:\n{combined}"
);
```

---

### WR-04: `setup.rs` has `#[allow(dead_code)]` on a struct field

**File:** `crates/nono-cli/src/setup.rs:23`

**Issue:** `SetupRunner::verbose` is annotated `#[allow(dead_code)]`. CLAUDE.md explicitly prohibits this pattern: "Avoid `#[allow(dead_code)]`. If code is unused, either remove it or write tests that use it." If `verbose` is not used in any method, it should be removed from the struct and not passed through from `SetupArgs`.

**Fix:** Either use `self.verbose` in at least one method (e.g., to gate `println!` vs `eprintln!` verbosity), or remove the field entirely:

```rust
// Remove the field if unused:
pub struct SetupRunner {
    check_only: bool,
    // ... other fields ...
    // verbose: u8,   <-- delete
}
```

---

### WR-05: `check_trusted_root_freshness` ISO-10 string comparison is locale/format-sensitive

**File:** `crates/nono/src/trust/bundle.rs:260-262`

**Issue:** The freshness check compares the ISO-8601 date prefix of `now` with the `valid_for.end` field from the TUF JSON using a raw string `<` comparison. This works correctly when the `end` field is an RFC 3339 / ISO 8601 string in `YYYY-MM-DDTHH:MM:SSZ` form (because lexicographic `<` on `YYYY-MM-DD` is equivalent to chronological order). However, if the upstream TUF root uses a different timestamp format (e.g., without the zero-padded month, or with timezone offsets), the comparison silently breaks and could either always return `true` (stale root treated as fresh) or always return `false` (valid root rejected). The code slices `end[..end.len().min(10)]` to get the date prefix but does not validate the format.

**Fix:** Add a format assertion before the comparison, or parse the ISO string numerically:

```rust
let end_date = &end[..end.len().min(10)];
// Guard: reject non-date-shaped strings to fail closed
if end_date.len() < 10 || !end_date.as_bytes()[4..5].eq(b"-") {
    // Non-standard format — fail closed: treat as expired
    return false;  // inside the .map() closure
}
now_iso10.as_str() < end_date
```

---

## Info

### IN-01: `broker_authenticode.rs` test `broker_valid_signature_spawns` contains a contradictory assertion

**File:** `crates/nono-cli/tests/broker_authenticode.rs:124`

**Issue:** In the `else` branch (dev layout), the test asserts `broker.exists()` with the message `"release broker found at ..."`. This phrasing is grammatically a success message but the assertion is reachable only when the broker *does* exist (line 94-104 returns early if it doesn't). The assertion at line 124 is therefore always true when reached, making it a no-op that could mislead future maintainers into thinking it is a meaningful guard.

**Fix:** Remove the redundant assertion or restructure so it provides a useful invariant:

```rust
// The early-return at line 94-104 already guarantees broker.exists() here.
// Either remove or replace with a doc comment explaining the invariant.
```

---

### IN-02: `keyless_sign.rs` deferred test body calls `panic!` unconditionally

**File:** `crates/nono-cli/tests/keyless_sign.rs:115`

**Issue:** `keyless_sign_then_verify_roundtrip` is marked `#[ignore]` and its body ends with `panic!("not yet implemented — see P32-DEFER-001")`. If someone runs `cargo test -- --ignored` (e.g., in a future CI gate that runs all tests), the panic produces an `FAILED` output with no actionable message beyond the string. While the `#[ignore]` attribute prevents this in normal runs, the panic body is unnecessarily abrupt compared to a structured skip.

**Fix:** Replace `panic!` with a graceful failure that identifies the work item:

```rust
// Instead of panic!, use eprintln! + return for the deferred stub body:
eprintln!("DEFERRED P32-DEFER-001: keyless roundtrip not yet implemented");
// If the test is meant to fail loudly when explicitly run, keep the panic but improve the message:
panic!(
    "P32-DEFER-001: keyless sign+verify roundtrip not yet implemented. \
     See .planning/deferred-items.md and keyless_sign.rs module doc for capture procedure."
);
```

---

### IN-03: `package_cmd.rs` is in-scope but contains no Phase 32 changes — review scope note

**File:** `crates/nono-cli/src/package_cmd.rs`

**Issue:** This file was listed in the review scope but contains no Phase 32 sigstore integration changes. The imports reference `nono::SignerIdentity`, which is likely a pre-existing dependency. No findings are raised for this file beyond noting that its inclusion in the diff is unexpected and may indicate a stale scope list.

**Fix:** Verify the file was intentionally included in the Phase 32 diff; if it was included by mistake, exclude it from the diff base to reduce review surface in future phases.

---

_Reviewed: 2026-05-10T00:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
